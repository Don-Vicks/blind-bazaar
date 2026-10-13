//! Close due auctions, pick winners, queue refunds with random delays.

use anyhow::Result;
use rand::RngExt;
use rusqlite::Connection;
use tracing::info;

use crate::{disclosure, memo, store};
use crate::wallet::net::Net as Network;

pub struct CloseConfig {
    pub delay_min: u64,
    pub delay_max: u64,
    pub settlement_window_secs: i64,
}

/// Viewing key used to disclose winning memos (reserve-met proof), if the operator enabled it.
pub type MemoKey<'a> = Option<&'a zcash_keys::keys::UnifiedFullViewingKey>;

/// Valid bids at or above reserve, best first: highest amount, then earliest received.
pub fn rank<'a>(valid: &[&'a store::Bid], reserve_zats: u64, exclude: &[i64]) -> Vec<&'a store::Bid> {
    let mut v: Vec<&store::Bid> = valid
        .iter()
        .copied()
        .filter(|b| !exclude.contains(&b.id))
        .filter(|b| b.bid_zats.unwrap_or(0) >= reserve_zats)
        .collect();
    v.sort_by(|a, b| {
        b.bid_zats
            .cmp(&a.bid_zats)
            .then_with(|| a.received_at.cmp(&b.received_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    v
}

pub fn pick_winner<'a>(valid: &[&'a store::Bid], reserve_zats: u64, exclude: &[i64]) -> Option<&'a store::Bid> {
    rank(valid, reserve_zats, exclude).into_iter().next()
}

/// Close due auctions. First-price: the top bid wins at its own price. Uniform: the top `units` bids
/// win and every winner pays the lowest winning bid. Losers are refunded with random delays.
pub fn close_due(
    store: &Connection,
    wallet_ro: Option<&Connection>,
    params: &Network,
    network_name: &str,
    cfg: &CloseConfig,
    memo_key: MemoKey<'_>,
) -> Result<usize> {
    let (delay_min, delay_max) = (cfg.delay_min, cfg.delay_max);
    let now = store::now();
    let mut closed = 0;
    for auction in store::due_open_auctions(store, now)? {
        let bids = store::bids_for_auction(store, &auction.id)?;
        let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();

        let ledger: Vec<(String, u64)> = valid
            .iter()
            .map(|b| (b.receipt.clone().unwrap_or_default(), b.bid_zats.unwrap_or(0)))
            .collect();
        let ledger_hash = memo::ledger_hash(&ledger);

        let ranked = rank(&valid, auction.reserve_zats, &[]);
        let units = auction.units.max(1) as usize;
        let winners: Vec<&store::Bid> = ranked.iter().copied().take(units).collect();
        let clearing = match auction.format.as_str() {
            "uniform" => winners.last().and_then(|b| b.bid_zats),
            _ => winners.first().and_then(|b| b.bid_zats),
        };

        let status = if winners.is_empty() { "no_sale" } else { "closed" };
        let settle_by = (!winners.is_empty()).then(|| now + cfg.settlement_window_secs);
        store::close_auction(
            store,
            &auction.id,
            status,
            winners.first().map(|b| b.id),
            &ledger_hash,
            settle_by,
            clearing,
        )?;
        for w in &winners {
            let price = if auction.format == "uniform" { clearing.unwrap_or(0) } else { w.bid_zats.unwrap_or(0) };
            store::set_bid_won(store, w.id, price, settle_by.unwrap_or(now))?;
        }
        store::insert_event(
            store,
            Some(&auction.id),
            if winners.is_empty() { "no_sale" } else { "closed" },
            &serde_json::json!({
                "valid_bids": valid.len(),
                "ledger_hash": ledger_hash,
                "units": units,
                "units_sold": winners.len(),
                "format": auction.format,
                "clearing_zats": clearing,
                "winning_amount_zats": clearing,
                "settle_by": settle_by,
            }),
        )?;

        if let Some(wallet_ro) = wallet_ro {
            let mut disclosures = Vec::new();
            for w in &winners {
                match disclosure::build(
                    wallet_ro,
                    params,
                    network_name,
                    &w.txid,
                    w.output_index,
                    memo_key.map(|k| (k, auction.reserve_zats)),
                ) {
                    Ok(d) => disclosures.push(d),
                    Err(e) => tracing::warn!("could not build disclosure for {} bid {}: {e:#}", auction.id, w.id),
                }
            }
            if !disclosures.is_empty() {
                let verified = disclosures.iter().filter(|d| d.verified_against_chain).count();
                info!("auction {}: {}/{} disclosures verified against chain", auction.id, verified, disclosures.len());
                store::set_disclosure(store, &auction.id, &serde_json::to_string(&disclosures)?)?;
            }
        }

        let mut rng = rand::rng();
        let winner_ids: Vec<i64> = winners.iter().map(|b| b.id).collect();
        for b in &valid {
            if winner_ids.contains(&b.id) {
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
        // A winner whose deposit already covers the price owes nothing: settle on the spot.
        for w in &winners {
            let price = if auction.format == "uniform" { clearing.unwrap_or(0) } else { w.bid_zats.unwrap_or(0) };
            if price <= w.value_zats {
                settle_winner(store, &auction, w.id, price, w.value_zats, "deposit covers price")?;
            }
        }
        info!(
            "auction {} closed ({}): {} valid bids, {} of {} units sold at {:?}, ledger {}",
            auction.id,
            auction.format,
            valid.len(),
            winners.len(),
            units,
            clearing,
            ledger_hash
        );
        closed += 1;
    }
    Ok(closed)
}

/// Record a winner's settlement: mark the bid settled, pay the seller the price, refund any excess the
/// winner overpaid, and advance the auction to `settled` once every winner has paid.
pub fn settle_winner(
    store: &Connection,
    auction: &store::Auction,
    bid_id: i64,
    price_zats: u64,
    paid_total_zats: u64,
    source: &str,
) -> Result<()> {
    store::set_bid_status(store, bid_id, "settled")?;
    let memo_text = format!("BB1P|{}", auction.id);
    store::queue_payout(
        store,
        Some(&auction.id),
        Some(bid_id),
        "seller",
        &auction.seller_address,
        price_zats,
        Some(&memo_text),
        store::now() + 30,
    )?;
    if paid_total_zats > price_zats {
        if let Some(b) = store::get_bid(store, bid_id)? {
            if let Some(addr) = b.refund_address.as_deref() {
                let memo_text = format!("BB1R|{}", auction.id);
                store::queue_payout(
                    store,
                    Some(&auction.id),
                    Some(bid_id),
                    "refund",
                    addr,
                    paid_total_zats - price_zats,
                    Some(&memo_text),
                    store::now() + 30,
                )?;
            }
        }
    }
    store::insert_event(
        store,
        Some(&auction.id),
        "settled",
        &serde_json::json!({ "price_zats": price_zats, "source": source }),
    )?;
    if store::count_bids_with_status(store, &auction.id, "won")? == 0 {
        store::set_auction_status(store, &auction.id, "settled")?;
    }
    Ok(())
}

/// A winner who has not paid by `settle_by` forfeits the deposit to the seller. The next-ranked bidder gets a
/// second-chance offer at their own bid if their refund has not left yet; otherwise the unit stays unsold.
pub fn enforce_settlement_deadlines(store: &Connection, cfg: &CloseConfig) -> Result<usize> {
    let now = store::now();
    let mut acted = 0;
    for winner in store::overdue_winners(store, now)? {
        let Some(auction_id) = winner.auction_id.clone() else { continue };
        let Some(auction) = store::get_auction(store, &auction_id)? else { continue };
        if auction.status != "closed" {
            continue;
        }
        store::set_bid_status(store, winner.id, "forfeited")?;
        let memo_text = format!("BB1F|{}", auction.id);
        store::queue_payout(
            store,
            Some(&auction.id),
            Some(winner.id),
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

        let bids = store::bids_for_auction(store, &auction.id)?;
        let taken: Vec<i64> = bids
            .iter()
            .filter(|b| matches!(b.status.as_str(), "won" | "settled" | "forfeited"))
            .map(|b| b.id)
            .collect();
        let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();
        match pick_winner(&valid, auction.reserve_zats, &taken) {
            Some(r) if store::cancel_queued_payout_for_bid(store, r.id)? => {
                let settle_by = now + cfg.settlement_window_secs;
                let price = r.bid_zats.unwrap_or(0);
                store::set_bid_won(store, r.id, price, settle_by)?;
                store::insert_event(
                    store,
                    Some(&auction.id),
                    "second_chance",
                    &serde_json::json!({ "bid_receipt": r.receipt, "winning_amount_zats": price, "settle_by": settle_by }),
                )?;
                info!("auction {}: winner forfeited, second chance offered to bid {}", auction.id, r.id);
                if price <= r.value_zats {
                    settle_winner(store, &auction, r.id, price, r.value_zats, "deposit covers price")?;
                }
            }
            _ => {
                info!("auction {}: winner forfeited, no eligible runner-up for that unit", auction.id);
                if store::count_bids_with_status(store, &auction.id, "won")? == 0 {
                    let settled = store::count_bids_with_status(store, &auction.id, "settled")?;
                    store::set_auction_status(store, &auction.id, if settled > 0 { "settled" } else { "unsettled" })?;
                    if settled == 0 {
                        store::insert_event(store, Some(&auction.id), "unsettled", &serde_json::json!({}))?;
                    }
                }
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
                image_url: None,
                units: 1,
                format: "first_price",
            },
        )
        .unwrap();
    }

    fn uniform_auction(store: &Connection, id: &str, units: u32) {
        store::insert_auction(
            store,
            &NewAuction {
                id,
                title: "edition",
                description: "",
                reserve_zats: 2_000_000,
                deposit_zats: 1_000_000,
                seller_address: UA,
                address: UA,
                diversifier_index: None,
                close_at: store::now() - 1,
                seller_id: None,
                image_url: None,
                units,
                format: "uniform",
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
        let n = close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(n, 1);
        let a = store::get_auction(&store, "aaaaaaaa").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(a.winner_bid_id, Some(b2));
        assert_eq!(a.clearing_zats, Some(5_000_000));
        assert_eq!(store::get_bid(&store, b2).unwrap().unwrap().status, "won");
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
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(store::get_auction(&store, "bbbbbbbb").unwrap().unwrap().winner_bid_id, Some(first));
    }

    #[test]
    fn no_bid_at_reserve_means_no_sale_and_everyone_refunded() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "cccccccc", store::now() - 1);
        bid(&store, "cccccccc", "21", 1_000_000, "0000000000000021", "valid");
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
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
        close_due(&store, None, &Network::Test, "test", &c, None).unwrap();
        assert_eq!(store::get_auction(&store, "dddddddd").unwrap().unwrap().winner_bid_id, Some(b2));

        // Winner b2 never pays: forfeit, and b1's queued refund is cancelled for a second chance.
        assert_eq!(enforce_settlement_deadlines(&store, &c).unwrap(), 1);
        let a = store::get_auction(&store, "dddddddd").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(store::get_bid(&store, b1).unwrap().unwrap().status, "won", "runner-up now owes at their own bid");
        assert_eq!(store::get_bid(&store, b1).unwrap().unwrap().price_zats, Some(3_000_000));
        let payouts = store::payouts_for_auction(&store, "dddddddd").unwrap();
        assert!(payouts.iter().any(|p| p.kind == "forfeit" && p.bid_id == Some(b2)));
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(b1) && p.status == "cancelled"));
        assert_eq!(store::get_bid(&store, b2).unwrap().unwrap().status, "forfeited");

        // b1 also never pays: nobody left, auction ends unsettled with b1 forfeited too.
        assert_eq!(enforce_settlement_deadlines(&store, &c).unwrap(), 1);
        let a = store::get_auction(&store, "dddddddd").unwrap().unwrap();
        assert_eq!(a.status, "unsettled");
        assert_eq!(store::get_bid(&store, b1).unwrap().unwrap().status, "forfeited");
    }

    #[test]
    fn uniform_price_top_n_pay_the_lowest_winning_bid() {
        let store = store::open_in_memory().unwrap();
        uniform_auction(&store, "eeeeeeee", 3);
        let b1 = bid(&store, "eeeeeeee", "41", 9_000_000, "0000000000000041", "valid");
        let b2 = bid(&store, "eeeeeeee", "42", 7_000_000, "0000000000000042", "valid");
        let b3 = bid(&store, "eeeeeeee", "43", 4_000_000, "0000000000000043", "valid");
        let b4 = bid(&store, "eeeeeeee", "44", 3_000_000, "0000000000000044", "valid"); // loses
        let _b5 = bid(&store, "eeeeeeee", "45", 1_500_000, "0000000000000045", "valid"); // below reserve
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        let a = store::get_auction(&store, "eeeeeeee").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(a.clearing_zats, Some(4_000_000), "clearing is the lowest winning bid");
        for id in [b1, b2, b3] {
            let b = store::get_bid(&store, id).unwrap().unwrap();
            assert_eq!(b.status, "won");
            assert_eq!(b.price_zats, Some(4_000_000), "every winner pays the clearing price");
        }
        let payouts = store::payouts_for_auction(&store, "eeeeeeee").unwrap();
        let refunded: Vec<i64> = payouts.iter().filter(|p| p.kind == "refund").filter_map(|p| p.bid_id).collect();
        assert!(refunded.contains(&b4));
        assert_eq!(refunded.len(), 2, "the two non-winners are refunded");
    }

    #[test]
    fn uniform_settlement_per_winner_then_settled() {
        let store = store::open_in_memory().unwrap();
        uniform_auction(&store, "ffffffff", 2);
        let b1 = bid(&store, "ffffffff", "51", 6_000_000, "0000000000000051", "valid");
        let b2 = bid(&store, "ffffffff", "52", 5_000_000, "0000000000000052", "valid");
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        let a = store::get_auction(&store, "ffffffff").unwrap().unwrap();
        settle_winner(&store, &a, b1, 5_000_000, 5_000_000, "test").unwrap();
        assert_eq!(store::get_auction(&store, "ffffffff").unwrap().unwrap().status, "closed", "one winner still owes");
        settle_winner(&store, &a, b2, 5_000_000, 5_200_000, "test").unwrap();
        assert_eq!(store::get_auction(&store, "ffffffff").unwrap().unwrap().status, "settled");
        let payouts = store::payouts_for_auction(&store, "ffffffff").unwrap();
        assert_eq!(payouts.iter().filter(|p| p.kind == "seller").count(), 2);
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(b2) && p.value_zats == 200_000), "overpayment refunded");
    }

    #[test]
    fn deposit_covering_the_price_settles_at_close() {
        let store = store::open_in_memory().unwrap();
        // deposit 1.0 (bid helper uses value 1_000_000) and reserve 2_000_000: make a lot whose deposit equals the bid.
        store::insert_auction(
            &store,
            &NewAuction {
                id: "gggggggg", title: "lot", description: "", reserve_zats: 500_000, deposit_zats: 1_000_000,
                seller_address: UA, address: UA, diversifier_index: None, close_at: store::now() - 1,
                seller_id: None, image_url: None, units: 1, format: "first_price",
            },
        )
        .unwrap();
        let b = bid(&store, "gggggggg", "61", 1_000_000, "0000000000000061", "valid");
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(store::get_bid(&store, b).unwrap().unwrap().status, "settled");
        assert_eq!(store::get_auction(&store, "gggggggg").unwrap().unwrap().status, "settled");
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
