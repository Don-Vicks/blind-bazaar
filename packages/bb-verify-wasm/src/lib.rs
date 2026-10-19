//! Browser-side verifier for Blind Bazaar note-opening disclosures. Pure math: rebuilds an Orchard
//! note from the disclosed opening and returns its extracted commitment, and can decrypt one note
//! ciphertext with a disclosed symmetric key. No network, no keys of its own.

use orchard::note::{ExtractedNoteCommitment, Note, NoteVersion, RandomSeed, Rho};
use orchard::value::NoteValue;
use wasm_bindgen::prelude::*;

fn bytes32(hex_str: &str, what: &str) -> Result<[u8; 32], JsValue> {
    hex::decode(hex_str)
        .map_err(|e| JsValue::from_str(&format!("{what}: {e}")))?
        .try_into()
        .map_err(|_| JsValue::from_str(&format!("{what} must be 32 bytes")))
}

/// Recompute the extracted note commitment (cmx) for a disclosed opening. Returns lowercase hex.
#[wasm_bindgen]
pub fn compute_cmx(orchard_receiver_hex: &str, value_zats: u64, rho_hex: &str, rseed_hex: &str, note_version: u8) -> Result<String, JsValue> {
    let receiver: [u8; 43] = hex::decode(orchard_receiver_hex)
        .map_err(|e| JsValue::from_str(&format!("receiver: {e}")))?
        .try_into()
        .map_err(|_| JsValue::from_str("receiver must be 43 bytes"))?;
    let addr = Option::from(orchard::Address::from_raw_address_bytes(&receiver))
        .ok_or_else(|| JsValue::from_str("invalid orchard receiver"))?;
    let rho = Option::from(Rho::from_bytes(&bytes32(rho_hex, "rho")?)).ok_or_else(|| JsValue::from_str("invalid rho"))?;
    let rseed = Option::from(RandomSeed::from_bytes(bytes32(rseed_hex, "rseed")?, &rho)).ok_or_else(|| JsValue::from_str("invalid rseed"))?;
    let version = match note_version {
        2 => NoteVersion::V2,
        3 => NoteVersion::V3,
        v => return Err(JsValue::from_str(&format!("unknown note version {v}"))),
    };
    let note: Note = Option::from(Note::from_parts(addr, NoteValue::from_raw(value_zats), rho, rseed, version))
        .ok_or_else(|| JsValue::from_str("opening does not form a valid note"))?;
    Ok(hex::encode(ExtractedNoteCommitment::from(note.commitment()).to_bytes()))
}

/// Decrypt one Orchard note ciphertext (580 bytes, hex) with a disclosed 32-byte key. Returns JSON
/// `{version, diversifier, value, rseed, memo}`; `memo` is the UTF-8 text memo or null.
#[wasm_bindgen]
pub fn decrypt_note(key_hex: &str, enc_ciphertext_hex: &str) -> Result<String, JsValue> {
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::Aead};
    let key = bytes32(key_hex, "key")?;
    let enc = hex::decode(enc_ciphertext_hex).map_err(|e| JsValue::from_str(&format!("ciphertext: {e}")))?;
    let cipher = ChaCha20Poly1305::new((&key).into());
    let pt = cipher
        .decrypt((&[0u8; 12]).into(), enc.as_ref())
        .map_err(|_| JsValue::from_str("ciphertext did not open with this key"))?;
    if pt.len() != 564 {
        return Err(JsValue::from_str("unexpected plaintext length"));
    }
    let memo_bytes = &pt[52..564];
    let memo = if memo_bytes[0] <= 0xF4 {
        let end = memo_bytes.iter().rposition(|b| *b != 0).map(|i| i + 1).unwrap_or(0);
        std::str::from_utf8(&memo_bytes[..end]).ok().map(|s| s.to_string())
    } else {
        None
    };
    let value = u64::from_le_bytes(pt[12..20].try_into().unwrap());
    let memo_json = match memo {
        Some(m) => format!("\"{}\"", m.replace('\\', "\\\\").replace('"', "\\\"")),
        None => "null".into(),
    };
    Ok(format!(
        "{{\"version\":{},\"diversifier\":\"{}\",\"value\":{},\"rseed\":\"{}\",\"memo\":{}}}",
        pt[0],
        hex::encode(&pt[1..12]),
        value,
        hex::encode(&pt[20..52]),
        memo_json
    ))
}

/// A receiver wallet made in the browser: the same ZIP-32 derivation the service and the mobile wallets use
/// (Orchard spending key from the 24-word phrase's seed, coin type 133 on mainnet, account 0, diversifier 0),
/// so the phrase imports into Zodl, Zingo or YWallet unchanged. Entropy comes from the caller
/// (`crypto.getRandomValues`, 32 bytes hex). The transparent receiver is derived by the caller (BIP 44
/// `m/44'/133'/0'/0/0`; secp256k1 does not build for wasm here) and passed in as its 20-byte public key hash;
/// this function assembles the unified address from the Orchard receiver and that hash. Nothing here keeps
/// state or touches the network. Returns JSON `{phrase, seed, address, orchard_only}`.
#[wasm_bindgen]
pub fn receiver_from_entropy(network: &str, entropy_hex: &str, transparent_pkh_hex: &str) -> Result<String, JsValue> {
    receiver_json(network, entropy_hex, transparent_pkh_hex).map_err(|e| JsValue::from_str(&e))
}

/// Native-testable body of `receiver_from_entropy` (JsValue cannot be built off wasm).
pub fn receiver_json(network: &str, entropy_hex: &str, transparent_pkh_hex: &str) -> Result<String, String> {
    use orchard::keys::{FullViewingKey, Scope, SpendingKey};
    use zcash_address::unified::{Address as UA, Encoding, Receiver, Revision, Uitem};
    use zcash_protocol::consensus::NetworkType;
    let (net, coin_type) = match network {
        "main" => (NetworkType::Main, 133u32),
        "test" => (NetworkType::Test, 1u32),
        other => return Err(format!("unknown network {other}")),
    };
    let entropy = bytes32(entropy_hex, "entropy").map_err(|e| e.as_string().unwrap_or_default())?;
    let mnemonic = bip0039::Mnemonic::<bip0039::English>::from_entropy(entropy.to_vec()).map_err(|e| format!("mnemonic: {e}"))?;
    let seed = mnemonic.to_seed("");
    let sk = Option::from(SpendingKey::from_zip32_seed(&seed, coin_type, zip32::AccountId::ZERO).ok())
        .flatten()
        .ok_or_else(|| "orchard key derivation failed".to_string())?;
    let fvk = FullViewingKey::from(&sk);
    let orchard = fvk.address_at(0u32, Scope::External).to_raw_address_bytes();
    let mut items = vec![Uitem::Data(Receiver::Orchard(orchard))];
    if !transparent_pkh_hex.is_empty() {
        let pkh: [u8; 20] = hex::decode(transparent_pkh_hex)
            .map_err(|e| format!("pkh: {e}"))?
            .try_into()
            .map_err(|_| "pkh must be 20 bytes".to_string())?;
        items.push(Uitem::Data(Receiver::P2pkh(pkh)));
    }
    let full = UA::try_from_items(Revision::R0, items).map_err(|e| format!("{e}"))?;
    let orchard_only = UA::try_from_items(Revision::R0, vec![Uitem::Data(Receiver::Orchard(orchard))]).map_err(|e| format!("{e}"))?;
    let out = serde_json::json!({
        "phrase": mnemonic.phrase(),
        "seed": hex::encode(seed),
        "address": full.encode(&net),
        "orchard_only": orchard_only.encode(&net),
    });
    Ok(out.to_string())
}

#[cfg(test)]
mod receiver_tests {
    use super::*;

    /// The browser derivation must give the same first address as the service's wallet code for the same
    /// phrase, or an import into a mobile wallet would show a different address from the one the user shared.
    #[test]
    fn browser_receiver_matches_the_service_derivation() {
        use zcash_keys::encoding::AddressCodec;
        use zcash_keys::keys::{UnifiedAddressRequest, UnifiedSpendingKey};
        use zcash_protocol::consensus::Network;
        let entropy = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";
        for (net, params) in [("main", Network::MainNetwork), ("test", Network::TestNetwork)] {
            let out: serde_json::Value = serde_json::from_str(&receiver_json(net, entropy, "").unwrap()).unwrap();
            let phrase = out["phrase"].as_str().unwrap();
            assert_eq!(phrase.split(' ').count(), 24);
            let seed = bip0039::Mnemonic::<bip0039::English>::from_phrase(phrase).unwrap().to_seed("");
            assert_eq!(hex::encode(seed), out["seed"].as_str().unwrap());
            let usk = UnifiedSpendingKey::from_seed(&params, &seed, zip32::AccountId::ZERO).unwrap();
            let ufvk = usk.to_unified_full_viewing_key();
            // Lot addresses are derived with `UnifiedAddressRequest::ORCHARD` (services/auction/src/wallet/mod.rs).
            let (ua, _) = ufvk.find_address(zip32::DiversifierIndex::new(), UnifiedAddressRequest::ORCHARD).unwrap();
            assert_eq!(ua.encode(&params), out["orchard_only"].as_str().unwrap(), "{net}");
            assert_eq!(out["address"], out["orchard_only"], "no pkh given: the full address is the Orchard-only one");
            // With a transparent receiver the address is longer and still decodes to the same Orchard receiver.
            let with_t: serde_json::Value = serde_json::from_str(&receiver_json(net, entropy, &"ab".repeat(20)).unwrap()).unwrap();
            let full = with_t["address"].as_str().unwrap();
            assert!(full.len() > ua.encode(&params).len());
            assert_eq!(with_t["orchard_only"], out["orchard_only"]);
        }
        assert!(receiver_json("main", entropy, "abcd").is_err(), "a short pkh is refused");
        assert!(receiver_json("regtest", entropy, "").is_err());
    }
}
