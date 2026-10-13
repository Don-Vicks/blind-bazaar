//! Note-opening disclosure (ZIP 311-style) for the winning output only.
//!
//! The wallet already holds the decrypted note plaintext (recipient, value, rho, rseed). Publishing
//! those lets anyone rebuild the note commitment and compare it to the `cmx` of the named action in
//! the on-chain transaction, proving the amount and recipient of that one output without revealing
//! the viewing key or any other bid.

use anyhow::{Result, anyhow};
use orchard::note::{ExtractedNoteCommitment, Note, NoteVersion, RandomSeed, Rho};
use orchard::value::NoteValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use zcash_address::unified::{Address as UnifiedAddr, Container, Encoding, Receiver};
use zcash_primitives::transaction::Transaction;
use zcash_protocol::consensus::{BlockHeight, BranchId, Network};

pub const VERSION: &str = "bb-note-opening-1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disclosure {
    pub version: String,
    pub network: String,
    pub txid: String,
    pub action_index: u32,
    pub pool: String,
    pub recipient_address: String,
    pub orchard_receiver: String,
    pub value_zats: u64,
    pub rho: String,
    pub rseed: String,
    pub note_version: u8,
    pub memo: Option<String>,
    pub cmx: String,
    pub verified_against_chain: bool,
}

fn txid_internal_bytes(display_hex: &str) -> Result<Vec<u8>> {
    let mut b = hex::decode(display_hex)?;
    b.reverse();
    Ok(b)
}

fn orchard_receiver_from_ua(ua: &str) -> Result<[u8; 43]> {
    let (_, _, addr) = UnifiedAddr::decode(ua).map_err(|e| anyhow!("bad unified address: {e}"))?;
    addr.items()
        .into_iter()
        .find_map(|r| match r {
            Receiver::Orchard(bytes) => Some(bytes),
            _ => None,
        })
        .ok_or_else(|| anyhow!("unified address has no Orchard receiver"))
}

fn rebuild_note(d: &Disclosure) -> Result<Note> {
    let receiver: [u8; 43] = hex::decode(&d.orchard_receiver)?
        .try_into()
        .map_err(|_| anyhow!("orchard receiver must be 43 bytes"))?;
    let addr = Option::from(orchard::Address::from_raw_address_bytes(&receiver))
        .ok_or_else(|| anyhow!("invalid orchard receiver"))?;
    let rho_bytes: [u8; 32] = hex::decode(&d.rho)?.try_into().map_err(|_| anyhow!("rho must be 32 bytes"))?;
    let rho = Option::from(Rho::from_bytes(&rho_bytes)).ok_or_else(|| anyhow!("invalid rho"))?;
    let rseed_bytes: [u8; 32] = hex::decode(&d.rseed)?.try_into().map_err(|_| anyhow!("rseed must be 32 bytes"))?;
    let rseed = Option::from(RandomSeed::from_bytes(rseed_bytes, &rho)).ok_or_else(|| anyhow!("invalid rseed"))?;
    let version = match d.note_version {
        2 => NoteVersion::V2,
        3 => NoteVersion::V3,
        v => return Err(anyhow!("unknown note version {v}")),
    };
    Option::from(Note::from_parts(addr, NoteValue::from_raw(d.value_zats), rho, rseed, version))
        .ok_or_else(|| anyhow!("note parts do not form a valid note"))
}

pub fn compute_cmx(d: &Disclosure) -> Result<[u8; 32]> {
    let note = rebuild_note(d)?;
    Ok(ExtractedNoteCommitment::from(note.commitment()).to_bytes())
}

/// Compare the rebuilt commitment with the named action in `tx`. Returns the pool it matched.
pub fn verify_against_tx(d: &Disclosure, tx: &Transaction) -> Result<Option<&'static str>> {
    let cmx = compute_cmx(d)?;
    let idx = d.action_index as usize;
    if let Some(b) = tx.ironwood_bundle() {
        if let Some(a) = b.actions().get(idx) {
            if a.cmx().to_bytes() == cmx {
                return Ok(Some("ironwood"));
            }
        }
    }
    if let Some(b) = tx.orchard_bundle() {
        if let Some(a) = b.actions().get(idx) {
            if a.cmx().to_bytes() == cmx {
                return Ok(Some("orchard"));
            }
        }
    }
    Ok(None)
}

pub fn read_tx(params: &Network, raw: &[u8], mined_height: Option<u32>) -> Result<Transaction> {
    let branch = match mined_height {
        Some(h) => BranchId::for_height(params, BlockHeight::from_u32(h)),
        None => BranchId::Nu6_3,
    };
    Ok(Transaction::read(raw, branch)?)
}

/// Build the disclosure for a received output from the wallet DB and self-verify it against the
/// raw transaction stored by the enhancement step.
pub fn build(wallet_ro: &Connection, params: &Network, network_name: &str, txid_display: &str, action_index: u32) -> Result<Disclosure> {
    let txid = txid_internal_bytes(txid_display)?;
    let row = wallet_ro
        .query_row(
            "SELECT n.value, n.rho, n.rseed, n.note_version, n.memo, a.address, t.raw, t.mined_height
             FROM orchard_received_notes n
             JOIN transactions t ON t.id_tx = n.transaction_id
             LEFT JOIN addresses a ON a.id = n.address_id
             WHERE t.txid = ?1 AND n.action_index = ?2",
            params![txid, action_index as i64],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<Vec<u8>>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("no received note for {txid_display}:{action_index}"))?;
    let (value, rho, rseed, note_version, memo, address, raw, mined_height) = row;
    let address = address.ok_or_else(|| anyhow!("wallet did not record the receiving address"))?;
    let receiver = orchard_receiver_from_ua(&address)?;
    let memo = memo.and_then(|b| {
        zcash_protocol::memo::MemoBytes::from_bytes(&b)
            .ok()
            .and_then(|m| zcash_protocol::memo::Memo::try_from(m).ok())
            .and_then(|m| match m {
                zcash_protocol::memo::Memo::Text(t) => Some(t.to_string()),
                _ => None,
            })
    });
    let mut d = Disclosure {
        version: VERSION.into(),
        network: network_name.into(),
        txid: txid_display.into(),
        action_index,
        pool: "unknown".into(),
        recipient_address: address,
        orchard_receiver: hex::encode(receiver),
        value_zats: value as u64,
        rho: hex::encode(rho),
        rseed: hex::encode(rseed),
        note_version: note_version as u8,
        memo,
        cmx: String::new(),
        verified_against_chain: false,
    };
    d.cmx = hex::encode(compute_cmx(&d)?);
    if let Some(raw) = raw {
        let tx = read_tx(params, &raw, mined_height.map(|h| h as u32))?;
        if let Some(pool) = verify_against_tx(&d, &tx)? {
            d.pool = pool.into();
            d.verified_against_chain = true;
        }
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_receiver_length() {
        let d = Disclosure {
            version: VERSION.into(),
            network: "test".into(),
            txid: "00".repeat(32),
            action_index: 0,
            pool: "ironwood".into(),
            recipient_address: String::new(),
            orchard_receiver: "ab".repeat(10),
            value_zats: 1,
            rho: "00".repeat(32),
            rseed: "00".repeat(32),
            note_version: 3,
            memo: None,
            cmx: String::new(),
            verified_against_chain: false,
        };
        assert!(compute_cmx(&d).is_err());
    }
}
