//! Close due auctions, pick winners, queue refunds with random delays.

use anyhow::Result;
use rand::RngExt;
use rusqlite::Connection;
use tracing::info;

use crate::{disclosure, memo, store};
use zcash_protocol::consensus::Network;

pub struct CloseConfig {
    pub delay_min: u64,
    pub delay_max: u64,
    pub settlement_window_secs: i64,
}

/// Pick the winner among valid bids: highest at or above reserve; ties go to the earliest received.
pub fn pick_winner<'a>(valid: &[&'a store::Bid], reserve_zats: u64, exclude: &[i64]) -> Option<&'a store::Bid> {
    valid
        .iter()
        .copied()
        .filter(|b| !exclude.contains(&b.id))
        .filter(|b| b.bid_zats.unwrap_or(0) >= reserve_zats)
        .max_by(|a, b| {
            a.bid_zats
                .cmp(&b.bid_zats)
                .then_with(|| b.received_at.cmp(&a.received_at))
                .then_with(|| b.id.cmp(&a.id))
        })
}

pub fn close_due(
    store: &Connection,
    wallet_ro: Option<&Connection>,
    params: &Network,
    network_name: &str,
    cfg: &CloseConfig,
) -> Result<usize> {
    let (delay_min, delay_max) = (cfg.delay_min, cfg.delay_max);
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

        let winner = pick_winner(&valid, auction.reserve_zats, &[]).map(|b| b.id);

        let status = if winner.is_some() { "closed" } else { "no_sale" };
        let settle_by = winner.map(|_| now + cfg.settlement_window_secs);
        store::close_auction(store, &auction.id, status, winner, &ledger_hash, settle_by)?;
        store::insert_event(
            store,
            Some(&auction.id),
            if winner.is_some() { "closed" } else { "no_sale" },
            &serde_json::json!({
                "valid_bids": valid.len(),
                "ledger_hash": ledger_hash,
                "winning_amount_zats": winner.and_then(|w| valid.iter().find(|b| b.id == w)).and_then(|b| b.bid_zats),
                "settle_by": settle_by,
            }),
        )?;

        if let (Some(wid), Some(wallet_ro)) = (winner, wallet_ro) {
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

/// A winner who has not paid by `settle_by` forfeits the deposit to the seller. The runner-up gets a
/// second-chance offer if their refund has not left the wallet yet; otherwise the auction ends unsettled.
pub fn enforce_settlement_deadlines(store: &Connection, cfg: &CloseConfig) -> Result<usize> {
    let now = store::now();
    let mut acted = 0;
    for auction in store::settlement_overdue(store, now)? {
        let Some(wid) = auction.winner_bid_id else { continue };
        let bids = store::bids_for_auction(store, &auction.id)?;
        let forfeited: Vec<i64> = bids.iter().filter(|b| b.status == "forfeited").map(|b| b.id).collect();
        let Some(winner) = bids.iter().find(|b| b.id == wid) else { continue };

        store::set_bid_status(store, wid, "forfeited")?;
        let memo_text = format!("BB1F|{}", auction.id);
        store::queue_payout(
            store,
            Some(&auction.id),
            Some(wid),
            "forfeit",
            &auction.seller_address,
            winner.value_zats,
            Some(&memo_text),
            now + 30,
        )?;
        store::insert_event(
            store,
            Some(&auction.id),
            "forfeited",
            &serde_json::json!({ "bid_receipt": winner.receipt, "deposit_zats": winner.value_zats }),
        )?;

        let mut exclude = forfeited.clone();
        exclude.push(wid);
        let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();
        let runner_up = pick_winner(&valid, auction.reserve_zats, &exclude);
        match runner_up {
            Some(r) if store::cancel_queued_payout_for_bid(store, r.id)? => {
                let settle_by = now + cfg.settlement_window_secs;
                store::set_winner(store, &auction.id, Some(r.id), "closed", Some(settle_by))?;
                store::insert_event(
                    store,
                    Some(&auction.id),
                    "second_chance",
                    &serde_json::json!({ "bid_receipt": r.receipt, "winning_amount_zats": r.bid_zats, "settle_by": settle_by }),
                )?;
                info!("auction {}: winner forfeited, second chance offered to bid {}", auction.id, r.id);
            }
            _ => {
                store::set_winner(store, &auction.id, None, "unsettled", None)?;
                store::insert_event(store, Some(&auction.id), "unsettled", &serde_json::json!({}))?;
                info!("auction {}: winner forfeited, no eligible runner-up", auction.id);
            }
        }
        acted += 1;
    }
    Ok(acted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{NewAuction, NewBid};

    const UA: &str = "utest1zkkkjfxkamagznjr6ayemffj2d2gacdwpzcyw669pvg06xevzqslpmm27zjsctlkstl2vsw62xrjktmzqcu4yu9zdhdxqz3kafa4j2q85y6mv74rzjcgjg8c0ytrg7dwyzwtgnuc76h";

    fn cfg() -> CloseConfig {
        CloseConfig { delay_min: 0, delay_max: 0, settlement_window_secs: 3600 }
    }

    fn auction(store: &Connection, id: &str, close_at: i64) {
        store::insert_auction(
            store,
            &NewAuction {
                id,
                title: "lot",
                description: "",
                reserve_zats: 2_000_000,
                deposit_zats: 1_000_000,
                seller_address: UA,
                address: UA,
                diversifier_index: None,
                close_at,
                seller_id: None,
            },
        )
        .unwrap();
    }

    fn bid(store: &Connection, auction_id: &str, txid: &str, bid_zats: u64, nonce: &str, status: &str) -> i64 {
        let receipt = crate::memo::receipt(nonce);
        store::insert_bid(
            store,
            &NewBid {
                auction_id: Some(auction_id),
                txid,
                output_index: 0,
                pool: 4,
                mined_height: Some(1),
                value_zats: 1_000_000,
                memo: None,
                bid_zats: Some(bid_zats),
                refund_address: Some(UA),
                nonce: Some(nonce),
                receipt: Some(&receipt),
                status,
                source: "memo",
            },
        )
        .unwrap()
    }

    #[test]
    fn highest_bid_wins_and_losers_are_refunded() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "aaaaaaaa", store::now() - 1);
        let b1 = bid(&store, "aaaaaaaa", "01", 3_000_000, "0000000000000001", "valid");
        let b2 = bid(&store, "aaaaaaaa", "02", 5_000_000, "0000000000000002", "valid");
        let _b3 = bid(&store, "aaaaaaaa", "03", 1_500_000, "0000000000000003", "valid"); // below reserve
        let _bad = bid(&store, "aaaaaaaa", "04", 9_000_000, "0000000000000004", "wrong_deposit");
        let n = close_due(&store, None, &Network::TestNetwork, "test", &cfg()).unwrap();
        assert_eq!(n, 1);
        let a = store::get_auction(&store, "aaaaaaaa").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(a.winner_bid_id, Some(b2));
        assert!(a.settle_by.is_some());
        let payouts = store::payouts_for_auction(&store, "aaaaaaaa").unwrap();
        let refunded: Vec<i64> = payouts.iter().filter(|p| p.kind == "refund").filter_map(|p| p.bid_id).collect();
        assert!(refunded.contains(&b1));
        assert_eq!(refunded.len(), 2, "both losing valid bids get a refund");
        assert!(a.ledger_hash.unwrap().len() == 64);
    }

    #[test]
    fn tie_goes_to_earliest_bid() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "bbbbbbbb", store::now() - 1);
        let first = bid(&store, "bbbbbbbb", "11", 4_000_000, "0000000000000011", "valid");
        let _second = bid(&store, "bbbbbbbb", "12", 4_000_000, "0000000000000012", "valid");
        close_due(&store, None, &Network::TestNetwork, "test", &cfg()).unwrap();
        assert_eq!(store::get_auction(&store, "bbbbbbbb").unwrap().unwrap().winner_bid_id, Some(first));
    }

    #[test]
    fn no_bid_at_reserve_means_no_sale_and_everyone_refunded() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "cccccccc", store::now() - 1);
        bid(&store, "cccccccc", "21", 1_000_000, "0000000000000021", "valid");
        close_due(&store, None, &Network::TestNetwork, "test", &cfg()).unwrap();
        let a = store::get_auction(&store, "cccccccc").unwrap().unwrap();
        assert_eq!(a.status, "no_sale");
        assert_eq!(store::payouts_for_auction(&store, "cccccccc").unwrap().len(), 1);
    }

    #[test]
    fn forfeiture_offers_second_chance_then_unsettled() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "dddddddd", store::now() - 1);
        let b1 = bid(&store, "dddddddd", "31", 3_000_000, "0000000000000031", "valid");
        let b2 = bid(&store, "dddddddd", "32", 5_000_000, "0000000000000032", "valid");
        let c = CloseConfig { settlement_window_secs: -1, ..cfg() }; // deadline already passed
        close_due(&store, None, &Network::TestNetwork, "test", &c).unwrap();
        assert_eq!(store::get_auction(&store, "dddddddd").unwrap().unwrap().winner_bid_id, Some(b2));

        // Winner b2 never pays: forfeit, and b1's queued refund is cancelled for a second chance.
        assert_eq!(enforce_settlement_deadlines(&store, &c).unwrap(), 1);
        let a = store::get_auction(&store, "dddddddd").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(a.winner_bid_id, Some(b1));
        let payouts = store::payouts_for_auction(&store, "dddddddd").unwrap();
        assert!(payouts.iter().any(|p| p.kind == "forfeit" && p.bid_id == Some(b2)));
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(b1) && p.status == "cancelled"));
        assert_eq!(store::get_bid(&store, b2).unwrap().unwrap().status, "forfeited");

        // b1 also never pays: nobody left, auction ends unsettled with b1 forfeited too.
        assert_eq!(enforce_settlement_deadlines(&store, &c).unwrap(), 1);
        let a = store::get_auction(&store, "dddddddd").unwrap().unwrap();
        assert_eq!(a.status, "unsettled");
        assert_eq!(a.winner_bid_id, None);
    }

    #[test]
    fn payout_retry_and_dead_letter() {
        let store = store::open_in_memory().unwrap();
        let id = store::queue_payout(&store, None, None, "manual", UA, 10, None, 0).unwrap();
        store::mark_payout_failed(&store, id, "boom", 0).unwrap();
        store::mark_payout_dead(&store, id, "gave up").unwrap();
        assert!(store::due_payouts(&store, store::now() + 1, 5).unwrap().is_empty());
        assert!(store::requeue_payout(&store, id).unwrap());
        assert_eq!(store::due_payouts(&store, store::now() + 1, 5).unwrap().len(), 1);
    }
}
