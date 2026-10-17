//! FROST escrow: the spender's Orchard spend authority is a 2-of-3 group key. This module holds the
//! escrow description that the viewer and spender share (no secrets) and the key-generation command.

use std::path::Path;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use zcash_keys::keys::UnifiedFullViewingKey;

use crate::wallet::net::Net as Network;

/// Public escrow description. Safe to share with the viewer; contains no signing material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escrow {
    pub version: u32,
    pub threshold: u16,
    pub signers: u16,
    /// Group verifying key = Orchard spend validating key `ak` (hex, 32 bytes).
    pub ak: String,
    /// Minted nullifier deriving key (hex, 32 bytes).
    pub nk: String,
    /// Minted commit-ivk randomness (hex, 32 bytes).
    pub rivk: String,
    /// Signer identifiers (hex) in share order.
    pub identifiers: Vec<String>,
}

impl Escrow {
    pub fn fvk_bytes(&self) -> Result<[u8; 96]> {
        let h = |s: &str| -> Result<[u8; 32]> {
            hex::decode(s)?.try_into().map_err(|_| anyhow!("escrow field must be 32 bytes"))
        };
        Ok(bb_frost::fvk_bytes(&h(&self.ak)?, &h(&self.nk)?, &h(&self.rivk)?))
    }

    pub fn ufvk(&self) -> Result<UnifiedFullViewingKey> {
        let fvk = orchard::keys::FullViewingKey::from_bytes(&self.fvk_bytes()?)
            .ok_or_else(|| anyhow!("escrow keys do not form a valid Orchard full viewing key"))?;
        UnifiedFullViewingKey::from_orchard_fvk(fvk).map_err(|e| anyhow!("{e:?}"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

/// Generate shares and the escrow file. Mints `nk`/`rivk` until the resulting FVK validates.
pub fn keygen(max: u16, min: u16, out: &Path) -> Result<Escrow> {
    std::fs::create_dir_all(out)?;
    let shares = bb_frost::generate(max, min).map_err(|e| anyhow!("{e}"))?;
    let ak = shares[0].group_ak();
    let (nk, rivk) = loop {
        let (nk, rivk) = bb_frost::mint_nk_rivk();
        if orchard::keys::FullViewingKey::from_bytes(&bb_frost::fvk_bytes(&ak, &nk, &rivk)).is_some() {
            break (nk, rivk);
        }
    };
    let escrow = Escrow {
        version: 1,
        threshold: min,
        signers: max,
        ak: hex::encode(ak),
        nk: hex::encode(nk),
        rivk: hex::encode(rivk),
        identifiers: shares.iter().map(|s| bb_frost::wire::identifier_to_hex(&s.identifier)).collect(),
    };
    for (i, s) in shares.iter().enumerate() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = out.join(format!("signer-{}.json", i + 1));
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
        f.write_all(s.to_json().map_err(|e| anyhow!("{e}"))?.as_bytes())?;
    }
    std::fs::write(out.join("escrow.json"), serde_json::to_string_pretty(&escrow)?)?;
    Ok(escrow)
}

pub fn show(escrow: &Escrow, params: &Network) -> Result<(String, String)> {
    use zcash_keys::keys::UnifiedAddressRequest;
    let ufvk = escrow.ufvk()?;
    let encoded = ufvk.encode(params).map_err(|e| anyhow!("{e:?}"))?;
    let (ua, _) = ufvk
        .find_address(zip32::DiversifierIndex::new(), UnifiedAddressRequest::ORCHARD)
        .map_err(|e| anyhow!("{e:?}"))?;
    Ok((encoded, ua.encode(params)))
}
