//! 2-of-3 FROST threshold signing for Orchard spend authorization.
//!
//! The group verifying key doubles as the Orchard spend validating key `ak`; the Zcash wallet scans
//! and proposes spends with a full viewing key assembled from `ak || nk || rivk`, and every spend's
//! signature is produced by two of three signers over the PCZT's shielded sighash, re-randomized by
//! that action's `alpha`. Nobody ever holds the group secret.

use std::collections::BTreeMap;

use reddsa::frost::redpallas as frost;
use reddsa::frost::redpallas::rerandomized;
use serde::{Deserialize, Serialize};

pub use frost::Identifier;
pub use frost::SigningPackage;
pub use frost::keys::PublicKeyPackage;
pub use frost::round1::SigningNonces;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("frost: {0}")]
    Frost(#[from] frost::Error),
    #[error("{0}")]
    Encoding(String),
}

type Result<T> = std::result::Result<T, Error>;

/// One signer's share plus the public package. Serialize with `to_json` for storage on that signer's host only.
#[derive(Clone)]
pub struct SignerShare {
    pub identifier: Identifier,
    pub key_package: frost::keys::KeyPackage,
    pub public: frost::keys::PublicKeyPackage,
}

#[derive(Serialize, Deserialize)]
struct SignerShareJson {
    identifier: String,
    key_package: String,
    public: String,
}

impl SignerShare {
    pub fn to_json(&self) -> Result<String> {
        let j = SignerShareJson {
            identifier: hex::encode(self.identifier.serialize()),
            key_package: hex::encode(self.key_package.serialize().map_err(Error::Frost)?),
            public: hex::encode(self.public.serialize().map_err(Error::Frost)?),
        };
        serde_json::to_string(&j).map_err(|e| Error::Encoding(e.to_string()))
    }

    pub fn from_json(s: &str) -> Result<Self> {
        let j: SignerShareJson = serde_json::from_str(s).map_err(|e| Error::Encoding(e.to_string()))?;
        Ok(Self {
            identifier: Identifier::deserialize(&unhex(&j.identifier)?)?,
            key_package: frost::keys::KeyPackage::deserialize(&unhex(&j.key_package)?)?,
            public: frost::keys::PublicKeyPackage::deserialize(&unhex(&j.public)?)?,
        })
    }

    /// The group verifying key, which is the Orchard spend validating key `ak` (32 bytes).
    pub fn group_ak(&self) -> [u8; 32] {
        group_ak(&self.public)
    }
}

fn unhex(s: &str) -> Result<Vec<u8>> {
    hex::decode(s).map_err(|e| Error::Encoding(e.to_string()))
}

/// Trusted-dealer key generation for `max` signers with threshold `min`. Used at escrow setup on an
/// air-gapped machine; the dealer output is split and the dealer state discarded. (DKG is the follow-up.)
pub fn generate(max: u16, min: u16) -> Result<Vec<SignerShare>> {
    let mut rng = rand_core_old::OsRng;
    let (shares, public) = frost::keys::generate_with_dealer(max, min, frost::keys::IdentifierList::Default, &mut rng)?;
    let mut out = Vec::with_capacity(shares.len());
    for (identifier, secret) in shares {
        let key_package = frost::keys::KeyPackage::try_from(secret)?;
        out.push(SignerShare { identifier, key_package, public: public.clone() });
    }
    Ok(out)
}

/// Distributed key generation: three rounds, no dealer. Each participant keeps its secret packages on its
/// own host and exchanges only the public packages. The result is the same `SignerShare` the dealer path
/// produces, so signing sessions do not care which way the group key was made.
pub mod dkg {
    use super::*;
    use frost::keys::dkg as fd;

    pub type Round1Secret = fd::round1::SecretPackage;
    pub type Round1Package = fd::round1::Package;
    pub type Round2Secret = fd::round2::SecretPackage;
    pub type Round2Package = fd::round2::Package;

    pub fn identifier(n: u16) -> Result<Identifier> {
        Identifier::try_from(n).map_err(Error::Frost)
    }

    /// Round 1: commit to a random polynomial. Broadcast the package; keep the secret.
    pub fn part1(id: Identifier, max: u16, min: u16) -> Result<(Round1Secret, Round1Package)> {
        fd::part1(id, max, min, rand_core_old::OsRng).map_err(Error::Frost)
    }

    /// Round 2: with every other participant's round-1 package, produce one secret share per peer.
    /// Each returned package goes to exactly the peer it is keyed by, over a private channel.
    pub fn part2(secret: Round1Secret, round1: &BTreeMap<Identifier, Round1Package>) -> Result<(Round2Secret, BTreeMap<Identifier, Round2Package>)> {
        fd::part2(secret, round1).map_err(Error::Frost)
    }

    /// Round 3: with every peer's round-1 package and the round-2 packages addressed to this
    /// participant, derive the long-lived share and the public key package.
    pub fn part3(secret: &Round2Secret, round1: &BTreeMap<Identifier, Round1Package>, round2: &BTreeMap<Identifier, Round2Package>) -> Result<SignerShare> {
        let (key_package, public) = fd::part3(secret, round1, round2).map_err(Error::Frost)?;
        Ok(SignerShare { identifier: *key_package.identifier(), key_package, public })
    }

    pub fn round1_secret_to_hex(s: &Round1Secret) -> Result<String> { Ok(hex::encode(s.serialize()?)) }
    pub fn round1_secret_from_hex(s: &str) -> Result<Round1Secret> { Ok(Round1Secret::deserialize(&unhex(s)?)?) }
    pub fn round1_package_to_hex(p: &Round1Package) -> Result<String> { Ok(hex::encode(p.serialize()?)) }
    pub fn round1_package_from_hex(s: &str) -> Result<Round1Package> { Ok(Round1Package::deserialize(&unhex(s)?)?) }
    pub fn round2_secret_to_hex(s: &Round2Secret) -> Result<String> { Ok(hex::encode(s.serialize()?)) }
    pub fn round2_secret_from_hex(s: &str) -> Result<Round2Secret> { Ok(Round2Secret::deserialize(&unhex(s)?)?) }
    pub fn round2_package_to_hex(p: &Round2Package) -> Result<String> { Ok(hex::encode(p.serialize()?)) }
    pub fn round2_package_from_hex(s: &str) -> Result<Round2Package> { Ok(Round2Package::deserialize(&unhex(s)?)?) }
}

pub fn group_ak(public: &frost::keys::PublicKeyPackage) -> [u8; 32] {
    let bytes = public.verifying_key().serialize().expect("verifying key serializes");
    bytes.try_into().expect("32-byte RedPallas point")
}

/// Round 1: a signer commits to nonces for one signing session. Keep `nonces` secret; broadcast `commitments`.
pub fn round1(share: &SignerShare) -> (frost::round1::SigningNonces, frost::round1::SigningCommitments) {
    let mut rng = rand_core_old::OsRng;
    frost::round1::commit(share.key_package.signing_share(), &mut rng)
}

/// Build the signing package from the message (the shielded sighash) and every participant's commitments.
pub fn signing_package(sighash: &[u8; 32], commitments: BTreeMap<Identifier, frost::round1::SigningCommitments>) -> frost::SigningPackage {
    frost::SigningPackage::new(commitments, sighash)
}

/// The per-action randomizer: Orchard's `alpha` for the spend being authorized (32-byte little-endian scalar).
pub fn randomizer_from_alpha(alpha_bytes: &[u8; 32]) -> Result<rerandomized::Randomizer> {
    rerandomized::Randomizer::deserialize(alpha_bytes).map_err(Error::Frost)
}

/// Round 2: a signer produces its share over the package, re-randomized by `alpha`.
pub fn round2(
    share: &SignerShare,
    nonces: &frost::round1::SigningNonces,
    package: &frost::SigningPackage,
    randomizer: &rerandomized::Randomizer,
) -> Result<frost::round2::SignatureShare> {
    frost_rerandomized::sign::<frost::PallasBlake2b512>(package, nonces, &share.key_package, *randomizer).map_err(Error::Frost)
}

/// Aggregate shares into the 64-byte RedPallas signature valid under `rk = ak + alpha·G`.
pub fn aggregate(
    public: &frost::keys::PublicKeyPackage,
    package: &frost::SigningPackage,
    shares: &BTreeMap<Identifier, frost::round2::SignatureShare>,
    randomizer: &rerandomized::Randomizer,
) -> Result<[u8; 64]> {
    let params = rerandomized::RandomizedParams::from_randomizer(public.verifying_key(), *randomizer);
    let sig = rerandomized::aggregate(package, shares, public, &params)?;
    let bytes = sig.serialize().map_err(Error::Frost)?;
    bytes.try_into().map_err(|_| Error::Encoding("signature is not 64 bytes".into()))
}

/// The randomized verifying key `rk` for `alpha`, as the Orchard action will carry it (32 bytes).
pub fn randomized_rk(public: &frost::keys::PublicKeyPackage, randomizer: &rerandomized::Randomizer) -> [u8; 32] {
    let params = rerandomized::RandomizedParams::from_randomizer(public.verifying_key(), *randomizer);
    let bytes = params.randomized_verifying_key().serialize().expect("point serializes");
    bytes.try_into().expect("32 bytes")
}

/// Mint the non-spending halves of an Orchard full viewing key: `nk` (a Pallas base element) and
/// `rivk` (a Pallas scalar). Together with the group `ak` they form the 96-byte raw FVK the wallet imports.
pub fn mint_nk_rivk() -> ([u8; 32], [u8; 32]) {
    use ff::{Field, PrimeField};
    let nk = pasta_curves::Fp::random(rand_core_old::OsRng);
    let rivk = pasta_curves::Fq::random(rand_core_old::OsRng);
    (nk.to_repr(), rivk.to_repr())
}

pub fn fvk_bytes(ak: &[u8; 32], nk: &[u8; 32], rivk: &[u8; 32]) -> [u8; 96] {
    let mut out = [0u8; 96];
    out[..32].copy_from_slice(ak);
    out[32..64].copy_from_slice(nk);
    out[64..].copy_from_slice(rivk);
    out
}

/// Serialized round-trip helpers for transport between signer hosts.
pub mod wire {
    use super::*;

    pub fn commitments_to_hex(c: &frost::round1::SigningCommitments) -> Result<String> {
        Ok(hex::encode(c.serialize()?))
    }
    pub fn commitments_from_hex(s: &str) -> Result<frost::round1::SigningCommitments> {
        Ok(frost::round1::SigningCommitments::deserialize(&unhex(s)?)?)
    }
    pub fn share_to_hex(s: &frost::round2::SignatureShare) -> String {
        hex::encode(s.serialize())
    }
    pub fn share_from_hex(s: &str) -> Result<frost::round2::SignatureShare> {
        Ok(frost::round2::SignatureShare::deserialize(&unhex(s)?)?)
    }
    pub fn package_to_hex(p: &frost::SigningPackage) -> Result<String> {
        Ok(hex::encode(p.serialize()?))
    }
    pub fn package_from_hex(s: &str) -> Result<frost::SigningPackage> {
        Ok(frost::SigningPackage::deserialize(&unhex(s)?)?)
    }
    pub fn identifier_to_hex(i: &Identifier) -> String {
        hex::encode(i.serialize())
    }
    pub fn identifier_from_hex(s: &str) -> Result<Identifier> {
        Ok(Identifier::deserialize(&unhex(s)?)?)
    }
    pub fn public_to_hex(p: &frost::keys::PublicKeyPackage) -> Result<String> {
        Ok(hex::encode(p.serialize()?))
    }
    pub fn public_from_hex(s: &str) -> Result<frost::keys::PublicKeyPackage> {
        Ok(frost::keys::PublicKeyPackage::deserialize(&unhex(s)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a full 3-party DKG in process (each party's packages serialized through the wire helpers,
    /// as they would be between hosts) and check the resulting 2-of-3 shares sign under the group key.
    #[test]
    fn dkg_produces_shares_that_sign_like_dealer_shares() {
        let ids: Vec<Identifier> = (1..=3).map(|n| dkg::identifier(n).unwrap()).collect();
        let mut secrets1 = BTreeMap::new();
        let mut r1 = BTreeMap::new();
        for id in &ids {
            let (s, p) = dkg::part1(*id, 3, 2).unwrap();
            let p = dkg::round1_package_from_hex(&dkg::round1_package_to_hex(&p).unwrap()).unwrap();
            secrets1.insert(*id, dkg::round1_secret_from_hex(&dkg::round1_secret_to_hex(&s).unwrap()).unwrap());
            r1.insert(*id, p);
        }
        let mut secrets2 = BTreeMap::new();
        let mut inbox: BTreeMap<Identifier, BTreeMap<Identifier, dkg::Round2Package>> = BTreeMap::new();
        for id in &ids {
            let others: BTreeMap<_, _> = r1.iter().filter(|(k, _)| *k != id).map(|(k, v)| (*k, v.clone())).collect();
            let (s2, out) = dkg::part2(secrets1.remove(id).unwrap(), &others).unwrap();
            secrets2.insert(*id, s2);
            for (to, pkg) in out {
                let pkg = dkg::round2_package_from_hex(&dkg::round2_package_to_hex(&pkg).unwrap()).unwrap();
                inbox.entry(to).or_default().insert(*id, pkg);
            }
        }
        let mut shares = Vec::new();
        for id in &ids {
            let others: BTreeMap<_, _> = r1.iter().filter(|(k, _)| *k != id).map(|(k, v)| (*k, v.clone())).collect();
            shares.push(dkg::part3(&secrets2[id], &others, &inbox[id]).unwrap());
        }
        let ak = shares[0].group_ak();
        assert!(shares.iter().all(|s| s.group_ak() == ak), "every participant derives the same group key");

        // 2-of-3 signing with participants 2 and 3, re-randomized by a random alpha.
        let msg = [7u8; 32];
        let (_, alpha) = mint_nk_rivk();
        let r = randomizer_from_alpha(&alpha).unwrap();
        let (n2, c2) = round1(&shares[1]);
        let (n3, c3) = round1(&shares[2]);
        let mut commitments = BTreeMap::new();
        commitments.insert(shares[1].identifier, c2);
        commitments.insert(shares[2].identifier, c3);
        let pkg = signing_package(&msg, commitments);
        let mut sig_shares = BTreeMap::new();
        sig_shares.insert(shares[1].identifier, round2(&shares[1], &n2, &pkg, &r).unwrap());
        sig_shares.insert(shares[2].identifier, round2(&shares[2], &n3, &pkg, &r).unwrap());
        let sig = aggregate(&shares[1].public, &pkg, &sig_shares, &r).unwrap();
        let rk = randomized_rk(&shares[1].public, &r);
        let vk = reddsa::VerificationKey::<reddsa::orchard::SpendAuth>::try_from(rk).unwrap();
        assert!(vk.verify(&msg, &reddsa::Signature::from(sig)).is_ok());
    }

    #[test]
    fn two_of_three_signs_and_verifies_under_randomized_key() {
        let shares = generate(3, 2).unwrap();
        let ak = shares[0].group_ak();
        assert_eq!(ak, shares[2].group_ak());
        let sighash = [7u8; 32];
        let alpha = {
            use ff::PrimeField;
            pasta_curves::Fq::from(123456789u64).to_repr()
        };
        let r = randomizer_from_alpha(&alpha).unwrap();

        // Signers 1 and 3 participate.
        let (n1, c1) = round1(&shares[0]);
        let (n3, c3) = round1(&shares[2]);
        let mut comms = BTreeMap::new();
        comms.insert(shares[0].identifier, c1);
        comms.insert(shares[2].identifier, c3);
        let pkg = signing_package(&sighash, comms);
        let mut sigs = BTreeMap::new();
        sigs.insert(shares[0].identifier, round2(&shares[0], &n1, &pkg, &r).unwrap());
        sigs.insert(shares[2].identifier, round2(&shares[2], &n3, &pkg, &r).unwrap());
        let sig = aggregate(&shares[0].public, &pkg, &sigs, &r).unwrap();

        // Verify with the reddsa primitive under rk = ak + alpha*G, exactly as an Orchard action does.
        let rk = randomized_rk(&shares[0].public, &r);
        let vk = reddsa::VerificationKey::<reddsa::orchard::SpendAuth>::try_from(rk).unwrap();
        let s = reddsa::Signature::<reddsa::orchard::SpendAuth>::from(sig);
        assert!(vk.verify(&sighash, &s).is_ok(), "aggregate signature verifies under the randomized key");
        let bad = reddsa::VerificationKey::<reddsa::orchard::SpendAuth>::try_from(ak).unwrap();
        assert!(bad.verify(&sighash, &s).is_err(), "and not under the unrandomized key");
    }

    #[test]
    fn share_json_round_trips() {
        let shares = generate(3, 2).unwrap();
        let j = shares[1].to_json().unwrap();
        let back = SignerShare::from_json(&j).unwrap();
        assert_eq!(back.identifier, shares[1].identifier);
        assert_eq!(back.group_ak(), shares[1].group_ak());
    }

    #[test]
    fn one_signer_is_not_enough() {
        let shares = generate(3, 2).unwrap();
        let sighash = [1u8; 32];
        let r = randomizer_from_alpha(&[0u8; 32]).unwrap_or_else(|_| {
            use ff::PrimeField;
            randomizer_from_alpha(&pasta_curves::Fq::from(5u64).to_repr()).unwrap()
        });
        let (n1, c1) = round1(&shares[0]);
        let mut comms = BTreeMap::new();
        comms.insert(shares[0].identifier, c1);
        let pkg = signing_package(&sighash, comms);
        // FROST refuses to even produce a share for a session below the threshold.
        assert!(round2(&shares[0], &n1, &pkg, &r).is_err());
    }
}
