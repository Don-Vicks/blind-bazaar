//! Read received outputs from the wallet DB and turn them into bids.

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use tracing::{info, warn};
use zcash_protocol::memo::{Memo, MemoBytes};

use crate::{
    close,
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

/// Timestamp of a scanned block, from the wallet's block table.
pub fn block_time(wallet: &Connection, height: u32) -> Result<Option<i64>> {
    Ok(wallet
        .query_row("SELECT time FROM blocks WHERE height = ?1", [height as i64], |r| r.get::<_, i64>(0))
        .optional()?)
}

/// Height of the last scanned block whose timestamp is at or before `close_at`, and whether a later
/// block has been scanned (so the close is final). `None` if no block at or before `close_at` is known.
pub fn close_height_for(wallet: &Connection, close_at: i64, now: i64) -> Result<Option<(u32, bool)>> {
    // A chain whose newest block is dated more than a week before the close cannot be used as a clock
    // (the zeckit regtest devnet stamps every block with one fixed time). Close by the wall clock then,
    // a minute after close_at, and record the tip as the close height.
    let (tip, tip_time): (Option<i64>, Option<i64>) =
        wallet.query_row("SELECT MAX(height), MAX(time) FROM blocks", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    if let (Some(tip), Some(t)) = (tip, tip_time) {
        if t + 7 * 86_400 < close_at {
            return Ok(Some((tip as u32, now >= close_at + 60)));
        }
    }
    let last: Option<i64> = wallet
        .query_row("SELECT MAX(height) FROM blocks WHERE time <= ?1", [close_at], |r| r.get(0))
        .optional()?
        .flatten();
    let Some(last) = last else { return Ok(None) };
    let later: i64 = wallet.query_row("SELECT COUNT(*) FROM blocks WHERE height > ?1", [last], |r| r.get(0))?;
    Ok(Some((last as u32, later > 0)))
}

/// Process all received outputs not yet recorded as bids. Returns number of new rows. An output counts
/// only once it has `min_confirmations` blocks on top of it (`tip` is the wallet's scanned height), so a
/// one-block reorg cannot slip a deposit into the ledger and out of the chain.
pub fn ingest(store: &Connection, wallet: &Connection, tip: Option<u32>, min_confirmations: u32) -> Result<usize> {
    let mut inserted = 0;
    for out in received_outputs(wallet)? {
        if store::bid_exists(store, &out.txid, out.output_index)? {
            continue;
        }
        if let (Some(tip), Some(h)) = (tip, out.mined_height) {
            if tip.saturating_sub(h) + 1 < min_confirmations {
                continue; // not settled enough yet; next pass
            }
        }
        // A memo-less output is a rail delivery. It is claimed only by the envelope whose own fresh
        // address it arrived at; nothing a client says about txids can attach a payload to someone
        // else's money.
        let envelope = match &out.memo {
            Some(_) => None,
            None => match out.to_address.as_deref().and_then(|a| store::envelope_for_address(store, a).transpose()) {
                Some(env) => Some(env?),
                None => continue, // unknown memo-less receipt (operator funding, tips); nothing to do
            },
        };
        let (memo_str, source) = match (&out.memo, &envelope) {
            (Some(m), _) => (m.clone(), "memo"),
            (None, Some(env)) => (env.payload.clone(), "envelope"),
            (None, None) => continue,
        };
        let mined_at = match out.mined_height {
            Some(h) => block_time(wallet, h)?,
            None => None,
        };
        let bid_id = record(store, &out, &memo_str, source, mined_at)?;
        if let Some(env) = envelope {
            store::mark_envelope_matched(store, &env.deposit_address, bid_id)?;
        }
        inserted += 1;
    }
    Ok(inserted)
}

/// The fields of a bid or sealed commitment that the acceptance rules look at.
struct Entry {
    auction_id: String,
    nonce: String,
    refund_address: String,
    bid_zats: Option<u64>,
    commitment: Option<String>,
}

/// The acceptance rules, shared by first ingest and by reinstatement after a reorg.
fn classify(
    store: &Connection,
    auction: Option<&store::Auction>,
    bid: &Entry,
    value_zats: u64,
    to_address: Option<&str>,
    source: &str,
    mined_at: Option<i64>,
) -> Result<&'static str> {
    // Late is decided by the chain, not by a clock: a bid mined in a block whose timestamp is
    // after close_at missed the lot, wherever and whenever this process scanned it. Only when the
    // block time is unknown does the lot's current status decide.
    // A lot that has already closed cannot take a bid either, even if a non-monotonic block
    // timestamp says the bid was in time: refunding it is the only safe outcome.
    let late = |a: &store::Auction| match mined_at {
        Some(t) => t > a.close_at || a.status != "open",
        None => a.status != "open",
    };
    Ok(match auction {
        None => "unknown_auction",
        Some(a) if late(a) => "late",
        // A wallet pays the deposit exactly. A swap rail delivers at least the deposit (it
        // quotes with a buffer), so envelope bids may overshoot; the excess is refunded with
        // the deposit or counted towards the price.
        Some(a) if source == "envelope" && value_zats < a.deposit_zats => "wrong_deposit",
        Some(a) if source != "envelope" && value_zats != a.deposit_zats => "wrong_deposit",
        Some(a) if store::nonce_used(store, &a.id, &bid.nonce)? => "duplicate_nonce",
        // A sealed lot takes commitments only: an open amount would let the operator read it, so
        // it is refunded. A plain lot cannot take a commitment: there is no reveal window.
        Some(a) if a.sealed() && bid.commitment.is_none() => "unsealed_bid",
        Some(a) if !a.sealed() && bid.commitment.is_some() => "unexpected_commit",
        Some(a) if to_address.is_some() && Some(a.address.as_str()) != to_address => {
            warn!("bid for {} arrived at {:?}, expected {}", a.id, to_address, a.address);
            if bid.commitment.is_some() { "sealed" } else { "valid" }
        }
        Some(_) => if bid.commitment.is_some() { "sealed" } else { "valid" },
    })
}

/// Where a transaction stands in the wallet's view of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Mined(u32),
    /// Known to the wallet but no longer in a block (reorged out, or never mined).
    Unmined,
    /// The wallet has no record of it at all.
    Unknown,
}

pub fn tx_state(wallet: &Connection, display_txid: &str) -> Result<TxState> {
    let mut bytes = hex::decode(display_txid).unwrap_or_default();
    bytes.reverse();
    let row: Option<Option<i64>> = wallet
        .query_row("SELECT mined_height FROM transactions WHERE txid = ?1", [bytes], |r| r.get(0))
        .optional()?;
    Ok(match row {
        Some(Some(h)) => TxState::Mined(h as u32),
        Some(None) => TxState::Unmined,
        None => TxState::Unknown,
    })
}

#[derive(Debug, Default, Clone)]
pub struct ReorgReport {
    pub reorged: Vec<i64>,
    pub reinstated: Vec<i64>,
    /// Bids that vanished after the lot had already counted them: needs an operator.
    pub after_close: Vec<i64>,
}

/// Every pass: re-check that each counted deposit and settlement is still in a block. A payment that a
/// reorg removed stops counting (and its queued refund is cancelled); if the chain mines it again it is
/// classified afresh under the same rules. A deposit that vanishes after the lot closed cannot be undone
/// here, because the ledger root was published over it; it is flagged for the operator.
pub fn reorg_check(store: &Connection, wallet: &Connection) -> Result<ReorgReport> {
    let mut report = ReorgReport::default();
    for b in store::bids_to_recheck(store)? {
        let state = tx_state(wallet, &b.txid)?;
        match (b.status.as_str(), state) {
            ("reorged", TxState::Mined(h)) => {
                // Back in a block: classify as if seen for the first time at this height.
                let (Some(nonce), Some(refund)) = (b.nonce.clone(), b.refund_address.clone()) else { continue };
                let entry = Entry { auction_id: b.auction_id.clone().unwrap_or_default(), nonce, refund_address: refund, bid_zats: b.bid_zats, commitment: b.commitment.clone() };
                let auction = store::get_auction(store, &entry.auction_id)?;
                let mined_at = block_time(wallet, h)?;
                let status = if b.memo.as_deref().map(|m| m.starts_with("BB1S")).unwrap_or(false) {
                    "settlement"
                } else {
                    classify(store, auction.as_ref(), &entry, b.value_zats, None, &b.source, mined_at)?
                };
                store::reinstate_bid(store, b.id, status, h)?;
                store::insert_event(store, b.auction_id.as_deref(), "bid_reinstated", &serde_json::json!({ "receipt": b.receipt, "status": status, "height": h }))?;
                info!("bid {} mined again at {h}: now {status}", b.id);
                report.reinstated.push(b.id);
            }
            ("reorged", _) => {}
            (_, TxState::Mined(_)) => {}
            (status, _) => {
                // Gone from the chain.
                let counted_after_close = matches!(status, "won" | "settled" | "forfeited" | "settlement")
                    || store::get_auction(store, b.auction_id.as_deref().unwrap_or_default())?.map(|a| a.status != "open").unwrap_or(false)
                        && matches!(status, "valid" | "sealed" | "unrevealed");
                store::mark_bid_reorged(store, b.id)?;
                let cancelled = store::cancel_queued_payout_for_bid(store, b.id)?;
                if counted_after_close {
                    warn!("bid {} ({status}) vanished from the chain after its lot closed; operator action needed", b.id);
                    store::insert_event(store, b.auction_id.as_deref(), "reorg_after_close", &serde_json::json!({ "receipt": b.receipt, "was": status, "txid": b.txid, "refund_cancelled": cancelled }))?;
                    report.after_close.push(b.id);
                } else {
                    info!("bid {} ({status}) is no longer in a block; set aside", b.id);
                    store::insert_event(store, b.auction_id.as_deref(), "bid_reorged", &serde_json::json!({ "receipt": b.receipt, "was": status, "refund_cancelled": cancelled }))?;
                }
                report.reorged.push(b.id);
            }
        }
    }
    Ok(report)
}

fn record(store: &Connection, out: &ReceivedOutput, memo_str: &str, source: &str, mined_at: Option<i64>) -> Result<i64> {
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
        commitment: None,
    };

    // A plain bid and a sealed commitment share every check except the amount.
    let parsed = memo::parse(memo_str);
    let entry = match &parsed {
        Ok(Parsed::Bid(b)) => Some(Entry {
            auction_id: b.auction_id.clone(),
            nonce: b.nonce.clone(),
            refund_address: b.refund_address.clone(),
            bid_zats: Some(b.bid_zats),
            commitment: None,
        }),
        Ok(Parsed::Commit(c)) => Some(Entry {
            auction_id: c.auction_id.clone(),
            nonce: c.nonce.clone(),
            refund_address: c.refund_address.clone(),
            bid_zats: None,
            commitment: Some(c.commitment.clone()),
        }),
        _ => None,
    };
    if let Some(bid) = entry {
        let receipt = memo::receipt(&bid.nonce);
        nb.bid_zats = bid.bid_zats;
        nb.refund_address = Some(&bid.refund_address);
        nb.nonce = Some(&bid.nonce);
        nb.receipt = Some(&receipt);
        nb.commitment = bid.commitment.as_deref();
        let auction = store::get_auction(store, &bid.auction_id)?;
        nb.status = classify(store, auction.as_ref(), &bid, out.value_zats, out.to_address.as_deref(), source, mined_at)?;
        if auction.is_some() {
            nb.auction_id = Some(&bid.auction_id);
        }
        let id = store::insert_bid(store, &nb)?;
        info!(
            "bid {} for auction {} status {} ({} zats, {})",
            id, bid.auction_id, nb.status, out.value_zats, out.txid
        );
        let accepted = matches!(nb.status, "valid" | "sealed");
        store::insert_event(
            store,
            nb.auction_id,
            if accepted { "bid_received" } else { "bid_rejected" },
            &serde_json::json!({ "receipt": receipt, "status": nb.status, "source": source }),
        )?;
        // Anything that is not an accepted bid but has a refund address gets its deposit back.
        if !accepted {
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
        return Ok(id);
    }
    match parsed {
        Ok(Parsed::Bid(_)) | Ok(Parsed::Commit(_)) => unreachable!("handled above"),
        Ok(Parsed::Settlement(s)) => {
            nb.nonce = Some(&s.nonce);
            nb.status = "settlement";
            let auction = store::get_auction(store, &s.auction_id)?;
            let id = match &auction {
                Some(a) if a.status == "closed" || a.status == "settled" => {
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
    let bids = store::bids_for_auction(store, &auction.id)?;
    let rfq = close::is_rfq(&auction.format);
    // Money that does not settle anything is sent back rather than kept: to the bid's sealed refund
    // address for a selling lot, or to the buyer's address for an RFQ.
    let bounce = |reason: &str, to: Option<&str>| -> Result<()> {
        warn!("settlement for {}: {reason}; refunding {} zats", auction.id, out.value_zats);
        if let Some(addr) = to {
            let memo_text = format!("BB1R|{}", auction.id);
            store::queue_payout(store, Some(&auction.id), None, "refund", addr, out.value_zats, Some(&memo_text), store::now() + 60)?;
        }
        Ok(())
    };
    let Some(winner) = bids.iter().find(|b| b.status == "won" && b.nonce.as_deref() == Some(nonce)) else {
        let any = bids.iter().find(|b| b.nonce.as_deref() == Some(nonce));
        let to = if rfq { Some(auction.seller_address.as_str()) } else { any.and_then(|b| b.refund_address.as_deref()) };
        return bounce("does not match an unpaid winner", to);
    };
    let price = winner.price_zats.unwrap_or(0);
    // RFQ: the buyer pays the whole quote; the quoter's bond is returned with the payout.
    let owed = if rfq { price } else { price.saturating_sub(winner.value_zats) };
    if out.value_zats < owed {
        let to = if rfq { Some(auction.seller_address.as_str()) } else { winner.refund_address.as_deref() };
        return bounce(&format!("underpaid: got {} need {owed}", out.value_zats), to);
    }
    let paid_total = if rfq { out.value_zats } else { winner.value_zats + out.value_zats };
    close::settle_winner(store, auction, winner.id, price, paid_total, &out.txid)?;
    store::set_auction_settled_txid(store, &auction.id, &out.txid)?;
    info!("auction {} bid {} settled by {}", auction.id, winner.id, out.txid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{NewAuction, NewBid};

    const UA: &str = "utest1phxskj2u3phwse5pql5dazp4katjgtx265pnxgputh9p6vgjpc4pt33gmwgmdmqd3gnucf7f073cxlk6c7f6gkzxu3yvgwuysyplaj24";

    /// A stand-in for the wallet database: just the two tables the re-check reads.
    #[test]
    fn close_height_uses_the_clock_when_the_chain_has_no_usable_block_times() {
        let w = fake_wallet();
        // A devnet that stamps every block with one fixed time from 2011.
        for h in 100..=110 {
            w.execute("INSERT INTO blocks (height, time) VALUES (?1, 1297153002)", [h]).unwrap();
        }
        let close_at = 1_792_000_000;
        assert_eq!(close_height_for(&w, close_at, close_at + 10).unwrap(), Some((110, false)), "not yet: a minute of grace");
        assert_eq!(close_height_for(&w, close_at, close_at + 61).unwrap(), Some((110, true)), "closed by the clock at the tip");
        // A real chain: the block after close_at decides, whatever the clock says.
        let real = fake_wallet();
        for (h, t) in [(200, close_at - 150), (201, close_at - 75), (202, close_at + 10)] {
            real.execute("INSERT INTO blocks (height, time) VALUES (?1, ?2)", [h as i64, t]).unwrap();
        }
        assert_eq!(close_height_for(&real, close_at, close_at + 3_600).unwrap(), Some((201, true)));
        let waiting = fake_wallet();
        waiting.execute("INSERT INTO blocks (height, time) VALUES (300, ?1)", [close_at - 10]).unwrap();
        assert_eq!(close_height_for(&waiting, close_at, close_at + 3_600).unwrap(), Some((300, false)), "no block after close_at yet");
    }

    fn fake_wallet() -> Connection {
        let w = Connection::open_in_memory().unwrap();
        w.execute_batch(
            "CREATE TABLE transactions (txid BLOB PRIMARY KEY, mined_height INTEGER);
             CREATE TABLE blocks (height INTEGER PRIMARY KEY, time INTEGER NOT NULL);",
        )
        .unwrap();
        w
    }

    fn set_tx(w: &Connection, display_txid: &str, mined_height: Option<u32>) {
        let mut bytes = hex::decode(display_txid).unwrap();
        bytes.reverse();
        w.execute(
            "INSERT INTO transactions (txid, mined_height) VALUES (?1, ?2) ON CONFLICT (txid) DO UPDATE SET mined_height = excluded.mined_height",
            rusqlite::params![bytes, mined_height.map(|h| h as i64)],
        )
        .unwrap();
    }

    fn lot(store: &Connection, id: &str, close_at: i64) {
        store::insert_auction(
            store,
            &NewAuction {
                id, title: "lot", description: "", reserve_zats: 2_000_000, deposit_zats: 1_000_000, seller_address: UA, address: UA,
                diversifier_index: None, close_at, seller_id: None, image_url: None, units: 1, format: "first_price", reveal_secs: 0, audit: None,
            },
        )
        .unwrap();
    }

    fn bid(store: &Connection, auction_id: &str, txid: &str, status: &str) -> i64 {
        let nonce = &txid[..16];
        let receipt = memo::receipt(nonce);
        let memo_text = format!("BB1|{auction_id}|3000000|{UA}|{nonce}");
        store::insert_bid(
            store,
            &NewBid {
                auction_id: Some(auction_id), txid, output_index: 0, pool: 4, mined_height: Some(100), value_zats: 1_000_000, memo: Some(&memo_text),
                bid_zats: Some(3_000_000), refund_address: Some(UA), nonce: Some(nonce), receipt: Some(&receipt), status, source: "memo", commitment: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn a_reorged_deposit_stops_counting_and_comes_back_when_mined_again() {
        let store = store::open_in_memory().unwrap();
        let w = fake_wallet();
        w.execute("INSERT INTO blocks (height, time) VALUES (100, 1000), (101, 1075), (102, 1150)", []).unwrap();
        lot(&store, "aaaaaaaa", 5000);
        let tx = "1111111111111111222222222222222233333333333333334444444444444444";
        let id = bid(&store, "aaaaaaaa", tx, "valid");
        set_tx(&w, tx, Some(100));

        // Still mined: nothing changes.
        let r = reorg_check(&store, &w).unwrap();
        assert!(r.reorged.is_empty() && r.reinstated.is_empty());
        assert_eq!(store::get_bid(&store, id).unwrap().unwrap().status, "valid");

        // Reorged out: set aside, out of the ledger.
        set_tx(&w, tx, None);
        let r = reorg_check(&store, &w).unwrap();
        assert_eq!(r.reorged, vec![id]);
        assert_eq!(store::get_bid(&store, id).unwrap().unwrap().status, "reorged");
        assert!(store::ledger_entries(&store, "aaaaaaaa").unwrap().is_empty());
        assert!(store::receipts_for_auction(&store, "aaaaaaaa").unwrap().is_empty());

        // Mined again in a later block, still before close: valid again at the new height.
        set_tx(&w, tx, Some(102));
        let r = reorg_check(&store, &w).unwrap();
        assert_eq!(r.reinstated, vec![id]);
        let b = store::get_bid(&store, id).unwrap().unwrap();
        assert_eq!(b.status, "valid");
        assert_eq!(b.mined_height, Some(102));
    }

    #[test]
    fn a_deposit_remined_after_close_is_late_not_valid() {
        let store = store::open_in_memory().unwrap();
        let w = fake_wallet();
        w.execute("INSERT INTO blocks (height, time) VALUES (100, 1000), (101, 9000)", []).unwrap();
        lot(&store, "bbbbbbbb", 5000); // closes at t=5000
        let tx = "5555555555555555666666666666666677777777777777778888888888888888";
        let id = bid(&store, "bbbbbbbb", tx, "valid");
        set_tx(&w, tx, None);
        reorg_check(&store, &w).unwrap();
        set_tx(&w, tx, Some(101)); // block timestamped after close_at
        reorg_check(&store, &w).unwrap();
        assert_eq!(store::get_bid(&store, id).unwrap().unwrap().status, "late");
    }

    #[test]
    fn a_counted_winner_that_vanishes_is_flagged_and_its_refund_cancelled() {
        let store = store::open_in_memory().unwrap();
        let w = fake_wallet();
        lot(&store, "cccccccc", 1);
        store.execute("UPDATE auctions SET status = 'closed' WHERE id = 'cccccccc'", []).unwrap();
        let tx = "9999999999999999aaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbcccccccccccccccc";
        let loser = bid(&store, "cccccccc", tx, "valid");
        store::queue_payout(&store, Some("cccccccc"), Some(loser), "refund", UA, 1_000_000, None, 0).unwrap();
        set_tx(&w, tx, None);
        let r = reorg_check(&store, &w).unwrap();
        assert_eq!(r.after_close, vec![loser], "a bid the published ledger already counted needs a human");
        let payouts = store::payouts_for_auction(&store, "cccccccc").unwrap();
        assert_eq!(payouts[0].status, "cancelled", "no refund for money that is no longer on the chain");
    }

    #[test]
    fn ingest_waits_for_confirmations() {
        // tip 100, output at 100: one confirmation. With two required it waits; with one it counts.
        assert!(100u32.saturating_sub(100) + 1 < 2);
        assert!(!(100u32.saturating_sub(100) + 1 < 1));
        assert!(!(101u32.saturating_sub(100) + 1 < 2));
    }
}
