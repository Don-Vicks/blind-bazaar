//! Close due auctions, pick winners, queue refunds with random delays.

use anyhow::Result;
use rand::RngExt;
use rusqlite::Connection;
use tracing::info;

use crate::{disclosure, memo, store};
use zcash_protocol::consensus::Network;

pub fn close_due(
    store: &Connection,
    wallet_ro: &Connection,
    params: &Network,
    network_name: &str,
    delay_min: u64,
    delay_max: u64,
) -> Result<usize> {
    let now = store::now();
    let mut closed = 0;
    for auction in store::due_open_auctions(store, now)? {
        let bids = store::bids_for_auction(store, &auction.id)?;
        let valid: Vec<_> = bids.iter().filter(|b| b.status == "valid").collect();

        let ledger: Vec<(String, u64)> = valid
            .iter()
            .map(|b| (b.receipt.clone().unwrap_or_default(), b.bid_zats.unwrap_or(0)))
            .collect();
        let ledger_hash = memo::ledger_hash(&ledger);

        // Highest bid at or above reserve; ties go to the earliest received.
        let winner = valid
            .iter()
            .filter(|b| b.bid_zats.unwrap_or(0) >= auction.reserve_zats)
            .max_by(|a, b| {
                a.bid_zats
                    .cmp(&b.bid_zats)
                    .then_with(|| b.received_at.cmp(&a.received_at))
                    .then_with(|| b.id.cmp(&a.id))
            })
            .map(|b| b.id);

        let status = if winner.is_some() { "closed" } else { "no_sale" };
        store::close_auction(store, &auction.id, status, winner, &ledger_hash)?;

        if let Some(wid) = winner {
            if let Some(w) = valid.iter().find(|b| b.id == wid) {
                match disclosure::build(wallet_ro, params, network_name, &w.txid, w.output_index) {
                    Ok(d) => {
                        info!(
                            "disclosure for {} built (verified against chain: {})",
                            auction.id, d.verified_against_chain
                        );
                        store::set_disclosure(store, &auction.id, &serde_json::to_string(&d)?)?;
                    }
                    Err(e) => tracing::warn!("could not build disclosure for {}: {e:#}", auction.id),
                }
            }
        }

        let mut rng = rand::rng();
        for b in &valid {
            if Some(b.id) == winner {
                continue;
            }
            let delay = rng.random_range(delay_min..=delay_max.max(delay_min)) as i64;
            let memo_text = format!("BB1R|{}", auction.id);
            store::queue_payout(
                store,
                Some(&auction.id),
                Some(b.id),
                "refund",
                b.refund_address.as_deref().unwrap_or_default(),
                b.value_zats,
                Some(&memo_text),
                now + delay,
            )?;
        }
        info!(
            "auction {} closed: {} valid bids, winner {:?}, ledger {}",
            auction.id,
            valid.len(),
            winner,
            ledger_hash
        );
        closed += 1;
    }
    Ok(closed)
}
