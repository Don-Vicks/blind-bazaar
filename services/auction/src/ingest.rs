//! Read received outputs from the wallet DB and turn them into bids.

use std::path::Path;

use anyhow::{Result, anyhow};
use rusqlite::{Connection, OpenFlags};
use tracing::{info, warn};
use zcash_protocol::memo::{Memo, MemoBytes};

use crate::{
    memo::{self, Parsed},
    store::{self, NewBid},
};

pub struct ReceivedOutput {
    pub txid: String,
    pub mined_height: Option<u32>,
    pub pool: i64,
    pub output_index: u32,
    pub to_address: Option<String>,
    pub value_zats: u64,
    pub memo: Option<String>,
}

/// Open the wallet DB read-only on a separate connection for ingest queries.
pub fn open_wallet_readonly(data_db: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        data_db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    Ok(conn)
}

pub fn received_outputs(wallet: &Connection) -> Result<Vec<ReceivedOutput>> {
    let mut stmt = wallet.prepare(
        "SELECT lower(hex(t.txid)), t.mined_height, o.output_pool, o.output_index, o.to_address, o.value, o.memo
         FROM v_tx_outputs o
         JOIN transactions t ON t.txid = o.txid
         WHERE o.to_account_uuid IS NOT NULL
           AND o.from_account_uuid IS NULL
           AND o.is_change = 0
           AND t.mined_height IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |r| {
        let memo_bytes: Option<Vec<u8>> = r.get(6)?;
        Ok(ReceivedOutput {
            txid: reverse_txid_hex(&r.get::<_, String>(0)?),
            mined_height: r.get::<_, Option<i64>>(1)?.map(|h| h as u32),
            pool: r.get(2)?,
            output_index: r.get::<_, i64>(3)? as u32,
            to_address: r.get(4)?,
            value_zats: r.get::<_, i64>(5)? as u64,
            memo: memo_bytes.and_then(|b| memo_text(&b)),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Wallet stores txid bytes in internal (little-endian) order; explorers show them reversed.
fn reverse_txid_hex(internal_hex: &str) -> String {
    let bytes = hex::decode(internal_hex).unwrap_or_default();
    hex::encode(bytes.iter().rev().copied().collect::<Vec<_>>())
}

fn memo_text(bytes: &[u8]) -> Option<String> {
    match MemoBytes::from_bytes(bytes).ok().and_then(|m| Memo::try_from(m).ok())? {
        Memo::Text(t) => Some(t.to_string()),
        Memo::Empty => None,
        _ => None,
    }
}

/// Process all received outputs not yet recorded as bids. Returns number of new rows.
pub fn ingest(store: &Connection, wallet: &Connection) -> Result<usize> {
    let mut inserted = 0;
    for out in received_outputs(wallet)? {
        if store::bid_exists(store, &out.txid, out.output_index)? {
            continue;
        }
        let (memo_str, source) = match &out.memo {
            Some(m) => (m.clone(), "memo"),
            None => match store::envelope_for_txid(store, &out.txid)? {
                Some(env) => (env.payload.clone(), "envelope"),
                None => {
                    // Memo-less receipt with no envelope yet; re-check next pass.
                    continue;
                }
            },
        };
        let bid_id = record(store, &out, &memo_str, source)?;
        if source == "envelope" {
            if let Some(env) = store::envelope_for_txid(store, &out.txid)? {
                store::mark_envelope_matched(store, &env.deposit_address, bid_id)?;
            }
        }
        inserted += 1;
    }
    Ok(inserted)
}

fn record(store: &Connection, out: &ReceivedOutput, memo_str: &str, source: &str) -> Result<i64> {
    let mut nb = NewBid {
        auction_id: None,
        txid: &out.txid,
        output_index: out.output_index,
        pool: out.pool,
        mined_height: out.mined_height,
        value_zats: out.value_zats,
        memo: Some(memo_str),
        bid_zats: None,
        refund_address: None,
        nonce: None,
        receipt: None,
        status: "malformed",
        source,
    };

    match memo::parse(memo_str) {
        Ok(Parsed::Bid(bid)) => {
            let receipt = memo::receipt(&bid.nonce);
            nb.bid_zats = Some(bid.bid_zats);
            nb.refund_address = Some(&bid.refund_address);
            nb.nonce = Some(&bid.nonce);
            nb.receipt = Some(&receipt);
            let auction = store::get_auction(store, &bid.auction_id)?;
            nb.status = match &auction {
                None => "unknown_auction",
                Some(a) if a.status != "open" => "late",
                Some(a) if out.value_zats != a.deposit_zats => "wrong_deposit",
                Some(a) if store::nonce_used(store, &a.id, &bid.nonce)? => "duplicate_nonce",
                Some(a) if a.address != out.to_address.clone().unwrap_or_default() => {
                    warn!(
                        "bid for {} arrived at {:?}, expected {}",
                        a.id, out.to_address, a.address
                    );
                    "valid"
                }
                Some(_) => "valid",
            };
            if auction.is_some() {
                nb.auction_id = Some(&bid.auction_id);
            }
            let id = store::insert_bid(store, &nb)?;
            info!(
                "bid {} for auction {} status {} ({} zats, {})",
                id, bid.auction_id, nb.status, out.value_zats, out.txid
            );
            store::insert_event(
                store,
                nb.auction_id,
                if nb.status == "valid" { "bid_received" } else { "bid_rejected" },
                &serde_json::json!({ "receipt": receipt, "status": nb.status, "source": source }),
            )?;
            // Anything that is not a valid bid but has a refund address gets its deposit back.
            if nb.status != "valid" {
                let memo_text = format!("BB1R|{}", bid.auction_id);
                store::queue_payout(
                    store,
                    nb.auction_id,
                    Some(id),
                    "invalid_refund",
                    &bid.refund_address,
                    out.value_zats,
                    Some(&memo_text),
                    store::now() + 60,
                )?;
            }
            Ok(id)
        }
        Ok(Parsed::Settlement(s)) => {
            nb.nonce = Some(&s.nonce);
            nb.status = "settlement";
            let auction = store::get_auction(store, &s.auction_id)?;
            let id = match &auction {
                Some(a) if a.status == "closed" => {
                    nb.auction_id = Some(&s.auction_id);
                    let id = store::insert_bid(store, &nb)?;
                    settle(store, a, &s.nonce, out, id)?;
                    id
                }
                _ => {
                    nb.status = "unexpected_settlement";
                    store::insert_bid(store, &nb)?
                }
            };
            Ok(id)
        }
        Err(e) => {
            warn!("malformed memo in {}: {e}", out.txid);
            Ok(store::insert_bid(store, &nb)?)
        }
    }
}

fn settle(store: &Connection, auction: &store::Auction, nonce: &str, out: &ReceivedOutput, _row_id: i64) -> Result<()> {
    let winner_id = auction
        .winner_bid_id
        .ok_or_else(|| anyhow!("closed auction without winner"))?;
    let winner = store::get_bid(store, winner_id)?.ok_or_else(|| anyhow!("winner bid missing"))?;
    if winner.nonce.as_deref() != Some(nonce) {
        warn!("settlement nonce mismatch for auction {}", auction.id);
        return Ok(());
    }
    let bid_zats = winner.bid_zats.unwrap_or(0);
    let owed = bid_zats.saturating_sub(auction.deposit_zats);
    if out.value_zats < owed {
        warn!(
            "settlement for {} underpaid: got {} need {}",
            auction.id, out.value_zats, owed
        );
        return Ok(());
    }
    store::set_auction_settled(store, &auction.id, &out.txid)?;
    store::insert_event(
        store,
        Some(&auction.id),
        "settled",
        &serde_json::json!({ "settlement_txid": out.txid, "winning_amount_zats": bid_zats }),
    )?;
    let memo_text = format!("BB1P|{}", auction.id);
    store::queue_payout(
        store,
        Some(&auction.id),
        Some(winner_id),
        "seller",
        &auction.seller_address,
        bid_zats,
        Some(&memo_text),
        store::now() + 30,
    )?;
    info!("auction {} settled by {}", auction.id, out.txid);
    Ok(())
}
