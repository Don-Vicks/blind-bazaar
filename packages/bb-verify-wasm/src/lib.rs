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
