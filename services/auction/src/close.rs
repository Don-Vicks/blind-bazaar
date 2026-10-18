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
    /// Refund times are rounded up to a multiple of this (0 = no rounding).
    pub refund_window_secs: u64,
}

/// When a refund queued now should leave: a random delay in `[delay_min, delay_max]`, rounded up to the
/// next window boundary so refunds from different lots share a transaction. Timing alone then says
/// only "some lots closed in the last `delay_max` seconds".
pub fn refund_time<R: rand::Rng>(rng: &mut R, now: i64, cfg: &CloseConfig) -> i64 {
    let delay = rng.random_range(cfg.delay_min..=cfg.delay_max.max(cfg.delay_min)) as i64;
    let t = now + delay;
    if cfg.refund_window_secs == 0 {
        return t;
    }
    let w = cfg.refund_window_secs as i64;
    ((t + w - 1) / w) * w
}

/// Viewing key used to disclose winning memos (reserve-met proof), if the operator enabled it.
pub type MemoKey<'a> = Option<&'a zcash_keys::keys::UnifiedFullViewingKey>;

/// Auction formats. `reserve_zats` is a floor for the selling formats and the buyer's budget (a
/// ceiling) for a request for quotes.
pub const FORMATS: [&str; 4] = ["first_price", "uniform", "vickrey", "rfq"];

/// True for a request for quotes: the lot creator is a buyer, bids are quotes, the lowest wins.
pub fn is_rfq(format: &str) -> bool {
    format == "rfq"
}

/// Eligible bids, best first. Selling formats: at or above reserve, highest amount first. RFQ: at or
/// below budget, lowest quote first. Ties go to the bid mined first (height, txid, output index).
pub fn rank<'a>(valid: &[&'a store::Bid], format: &str, reserve_zats: u64, exclude: &[i64]) -> Vec<&'a store::Bid> {
    let rfq = is_rfq(format);
    let mut v: Vec<&store::Bid> = valid
        .iter()
        .copied()
        .filter(|b| !exclude.contains(&b.id))
        .filter(|b| {
            let amount = b.bid_zats.unwrap_or(0);
            if rfq { amount > 0 && amount <= reserve_zats } else { amount >= reserve_zats }
        })
        .collect();
    v.sort_by(|a, b| {
        let by_amount = if rfq { a.bid_zats.cmp(&b.bid_zats) } else { b.bid_zats.cmp(&a.bid_zats) };
        // Ties go to the bid the chain saw first: block height, then txid, then output index. Every
        // process that reads the chain orders the same way; wall-clock receipt order would not.
        by_amount
            .then_with(|| a.mined_height.cmp(&b.mined_height))
            .then_with(|| a.txid.cmp(&b.txid))
            .then_with(|| a.output_index.cmp(&b.output_index))
    });
    v
}

pub fn pick_winner<'a>(valid: &[&'a store::Bid], format: &str, reserve_zats: u64, exclude: &[i64]) -> Option<&'a store::Bid> {
    rank(valid, format, reserve_zats, exclude).into_iter().next()
}

/// What the winner at `position` in `ranked` pays (total, deposit included), and where that price
/// comes from.
pub fn price_for(format: &str, reserve_zats: u64, ranked: &[&store::Bid], position: usize) -> (u64, &'static str) {
    let own = ranked.get(position).and_then(|b| b.bid_zats).unwrap_or(0);
    match format {
        "uniform" => (own, "lowest_winning"),
        "vickrey" => match ranked.get(position + 1).and_then(|b| b.bid_zats) {
            Some(next) if next >= reserve_zats => (next, "second_bid"),
            _ => (reserve_zats, "reserve"),
        },
        "rfq" => (own, "lowest_quote"),
        _ => (own, "own_bid"),
    }
}

/// Close due auctions. First-price: the top bid wins at its own price. Uniform: the top `units` bids
/// win and every winner pays the lowest winning bid. Vickrey: the top bid wins and pays the second
/// bid (or the reserve). RFQ: the lowest quote within budget wins at its own quote and the buyer pays
/// it. Losers are refunded with random delays.
pub fn close_due(
    store: &Connection,
    wallet_ro: Option<&Connection>,
    params: &Network,
    network_name: &str,
    cfg: &CloseConfig,
    memo_key: MemoKey<'_>,
) -> Result<usize> {
    let now = store::now();
    let mut closed = 0;
    for auction in store::due_open_auctions(store, now)? {
        // Close only once the chain has moved past close_at: every bid that can be in the ledger is
        // then mined and scanned, and any process reading the same chain closes on the same set.
        let close_height = match wallet_ro {
            Some(w) => match crate::ingest::close_height_for(w, auction.close_at)? {
                Some((h, true)) => Some(h),
                _ => {
                    tracing::debug!("auction {}: waiting for a block after close_at before closing", auction.id);
                    continue;
                }
            },
            None => None,
        };
        queue_sweep(store, &auction, 30)?;
        if auction.sealed() {
            // Sealed-reveal: fix the set of commitments now, then give bidders the reveal window.
            // Nothing about any amount is known yet, not even to the operator.
            let bids = store::bids_for_auction(store, &auction.id)?;
            let commits: Vec<(String, String)> = bids
                .iter()
                .filter(|b| b.status == "sealed")
                .map(|b| (b.receipt.clone().unwrap_or_default(), b.commitment.clone().unwrap_or_default()))
                .collect();
            let commit_hash = crate::merkle::commit_root(&commits);
            let reveal_until = now + auction.reveal_secs;
            store::open_reveal_window(store, &auction.id, reveal_until, &commit_hash, close_height)?;
            store::insert_event(
                store,
                Some(&auction.id),
                "reveal_open",
                &serde_json::json!({ "sealed_bids": commits.len(), "commit_hash": commit_hash, "reveal_until": reveal_until, "close_height": close_height }),
            )?;
            info!("auction {}: {} sealed bids committed ({}), reveal window until {}", auction.id, commits.len(), commit_hash, reveal_until);
            closed += 1;
            continue;
        }
        close_lot(store, wallet_ro, params, network_name, cfg, memo_key, &auction, close_height)?;
        closed += 1;
    }
    Ok(closed)
}

/// Auditable lots: move the lot account's balance into the operator pool. Queued at close and after each
/// settlement payment; the worker sends the whole spendable balance when the row comes due.
pub fn queue_sweep(store: &Connection, auction: &store::Auction, delay_secs: i64) -> Result<()> {
    let Some(account) = auction.audit_account.as_deref() else { return Ok(()) };
    let Some(to) = store::get_meta(store, "operator_address")? else {
        tracing::warn!("auction {}: no operator address recorded; sweep not queued", auction.id);
        return Ok(());
    };
    store::queue_payout_from(store, Some(&auction.id), "sweep", account, &to, 0, store::now() + delay_secs)?;
    Ok(())
}

/// Sealed-reveal lots whose reveal window has ended: unrevealed commitments are refunded in full, then
/// the lot closes over the revealed bids exactly like a plain lot.
pub fn finalize_reveals(
    store: &Connection,
    wallet_ro: Option<&Connection>,
    params: &Network,
    network_name: &str,
    cfg: &CloseConfig,
    memo_key: MemoKey<'_>,
) -> Result<usize> {
    let now = store::now();
    let mut done = 0;
    for auction in store::due_revealing_auctions(store, now)? {
        let bids = store::bids_for_auction(store, &auction.id)?;
        let mut unrevealed = 0;
        let mut rng = rand::rng();
        for b in bids.iter().filter(|b| b.status == "sealed") {
            store::set_bid_status(store, b.id, "unrevealed")?;
            let memo_text = format!("BB1R|{}", auction.id);
            store::queue_payout(
                store,
                Some(&auction.id),
                Some(b.id),
                "refund",
                b.refund_address.as_deref().unwrap_or_default(),
                b.value_zats,
                Some(&memo_text),
                refund_time(&mut rng, now, cfg),
            )?;
            unrevealed += 1;
        }
        store::insert_event(
            store,
            Some(&auction.id),
            "reveal_closed",
            &serde_json::json!({ "unrevealed": unrevealed, "revealed": bids.iter().filter(|b| b.status == "valid").count() }),
        )?;
        close_lot(store, wallet_ro, params, network_name, cfg, memo_key, &auction, auction.close_height)?;
        done += 1;
    }
    Ok(done)
}

/// Close one lot over its valid bids: rank, price, publish the ledger root, queue refunds, settle any
/// winner whose deposit already covers the price.
#[allow(clippy::too_many_arguments)]
fn close_lot(
    store: &Connection,
    wallet_ro: Option<&Connection>,
    params: &Network,
    network_name: &str,
    cfg: &CloseConfig,
    memo_key: MemoKey<'_>,
    auction: &store::Auction,
    close_height: Option<u32>,
) -> Result<()> {
    let now = store::now();
    let bids = store::bids_for_auction(store, &auction.id)?;
    let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();

    let ledger: Vec<crate::merkle::Entry> = valid
        .iter()
        .map(|b| (b.receipt.clone().unwrap_or_default(), b.bid_zats.unwrap_or(0), b.nonce.clone().unwrap_or_default()))
        .collect();
    let ledger_hash = crate::merkle::root(&ledger);

    let ranked = rank(&valid, &auction.format, auction.reserve_zats, &[]);
    let units = if auction.format == "uniform" { auction.units.max(1) as usize } else { 1 };
    let winners: Vec<&store::Bid> = ranked.iter().copied().take(units).collect();
    let clearing = match auction.format.as_str() {
        "uniform" => winners.last().and_then(|b| b.bid_zats),
        _ if winners.is_empty() => None,
        f => Some(price_for(f, auction.reserve_zats, &ranked, 0).0),
    };
    let price_of = |position: usize| -> u64 {
        match auction.format.as_str() {
            "uniform" => clearing.unwrap_or(0),
            f => price_for(f, auction.reserve_zats, &ranked, position).0,
        }
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
    if let Some(h) = close_height {
        store::set_auction_close_height(store, &auction.id, h)?;
    }
    for (i, w) in winners.iter().enumerate() {
        store::set_bid_won(store, w.id, price_of(i), settle_by.unwrap_or(now))?;
    }
    store::insert_event(
        store,
        Some(&auction.id),
        if winners.is_empty() { "no_sale" } else { "closed" },
        &serde_json::json!({
            "valid_bids": valid.len(),
            "close_height": close_height,
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
        // Vickrey keeps the winning bid sealed: the price is proven from the ledger, not the memo.
        let memo_key = if auction.format == "vickrey" { None } else { memo_key };
        for w in &winners {
            match disclosure::build(
                wallet_ro,
                params,
                network_name,
                &w.txid,
                w.output_index,
                memo_key.map(|k| (k, auction.reserve_zats)),
            ) {
                Ok(mut d) => {
                    if is_rfq(&auction.format) && d.reserve_zats.is_some() {
                        d.reserve_rule = Some("at_most".into());
                    }
                    disclosures.push(d)
                }
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
        let memo_text = format!("BB1R|{}", auction.id);
        store::queue_payout(
            store,
            Some(&auction.id),
            Some(b.id),
            "refund",
            b.refund_address.as_deref().unwrap_or_default(),
            b.value_zats,
            Some(&memo_text),
            refund_time(&mut rng, now, cfg),
        )?;
    }
    // A winner whose deposit already covers the price owes nothing: settle on the spot. In an RFQ the
    // buyer pays, so the quoter's bond never counts towards the price.
    if !is_rfq(&auction.format) {
        for (i, w) in winners.iter().enumerate() {
            let price = price_of(i);
            if price <= w.value_zats {
                settle_winner(store, auction, w.id, price, w.value_zats, "deposit covers price")?;
            }
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
    Ok(())
}

/// Record a winner's settlement: mark the bid settled, pay the seller the price, refund any excess the
/// winner overpaid, and advance the auction to `settled` once every winner has paid.
///
/// RFQ: the buyer (lot creator) is the payer. `paid_total_zats` is what the buyer sent; the winning
/// quoter receives the price plus their bond back, and any overpayment returns to the buyer.
pub fn settle_winner(
    store: &Connection,
    auction: &store::Auction,
    bid_id: i64,
    price_zats: u64,
    paid_total_zats: u64,
    source: &str,
) -> Result<()> {
    store::set_bid_status(store, bid_id, "settled")?;
    queue_sweep(store, auction, 30)?;
    let bid = store::get_bid(store, bid_id)?;
    let memo_text = format!("BB1P|{}", auction.id);
    let (payee, payee_amount, overpay_to) = if is_rfq(&auction.format) {
        let quoter = bid.as_ref().and_then(|b| b.refund_address.clone()).unwrap_or_default();
        let bond = bid.as_ref().map(|b| b.value_zats).unwrap_or(0);
        (quoter, price_zats + bond, Some(auction.seller_address.clone()))
    } else {
        (auction.seller_address.clone(), price_zats, bid.as_ref().and_then(|b| b.refund_address.clone()))
    };
    store::queue_payout(
        store,
        Some(&auction.id),
        Some(bid_id),
        "seller",
        &payee,
        payee_amount,
        Some(&memo_text),
        store::now() + 30,
    )?;
    if paid_total_zats > price_zats {
        if let Some(addr) = overpay_to.as_deref() {
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
        if is_rfq(&auction.format) {
            // The buyer never paid: the quoter did nothing wrong, so the bond goes back and the request lapses.
            store::set_bid_status(store, winner.id, "expired")?;
            let memo_text = format!("BB1R|{}", auction.id);
            store::queue_payout(
                store,
                Some(&auction.id),
                Some(winner.id),
                "refund",
                winner.refund_address.as_deref().unwrap_or_default(),
                winner.value_zats,
                Some(&memo_text),
                now + 30,
            )?;
            store::set_auction_status(store, &auction.id, "unsettled")?;
            store::insert_event(
                store,
                Some(&auction.id),
                "buyer_defaulted",
                &serde_json::json!({ "bid_receipt": winner.receipt, "quote_zats": winner.price_zats }),
            )?;
            info!("rfq {}: buyer did not pay, bond refunded, request lapsed", auction.id);
            acted += 1;
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
        let ranked = rank(&valid, &auction.format, auction.reserve_zats, &taken);
        match ranked.first().copied() {
            Some(r) if store::cancel_queued_payout_for_bid(store, r.id)? => {
                let settle_by = now + cfg.settlement_window_secs;
                let price = price_for(&auction.format, auction.reserve_zats, &ranked, 0).0;
                store::set_bid_won(store, r.id, price, settle_by)?;
                store::set_auction_clearing(store, &auction.id, price)?;
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
        CloseConfig { delay_min: 0, delay_max: 0, settlement_window_secs: 3600, refund_window_secs: 0 }
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
                reveal_secs: 0,
                audit: None,
            },
        )
        .unwrap();
    }

    fn uniform_auction(store: &Connection, id: &str, units: u32) {
        formatted_auction(store, id, units, "uniform");
    }

    fn formatted_auction(store: &Connection, id: &str, units: u32, format: &str) {
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
                close_at: store::now() - 1,
                seller_id: None,
                image_url: None,
                units,
                format,
                reveal_secs: 0,
                audit: None,
            },
        )
        .unwrap();
    }

    fn bid(store: &Connection, auction_id: &str, txid: &str, bid_zats: u64, nonce: &str, status: &str) -> i64 {
        bid_at(store, auction_id, txid, bid_zats, nonce, status, 1)
    }

    fn bid_at(store: &Connection, auction_id: &str, txid: &str, bid_zats: u64, nonce: &str, status: &str, height: u32) -> i64 {
        let receipt = crate::memo::receipt(nonce);
        store::insert_bid(
            store,
            &NewBid {
                auction_id: Some(auction_id),
                txid,
                output_index: 0,
                pool: 4,
                mined_height: Some(height),
                value_zats: 1_000_000,
                memo: None,
                bid_zats: Some(bid_zats),
                refund_address: Some(UA),
                nonce: Some(nonce),
                receipt: Some(&receipt),
                status,
                source: "memo",
                commitment: None,
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
    fn tie_goes_to_the_bid_mined_first() {
        let store = store::open_in_memory().unwrap();
        auction(&store, "bbbbbbbb", store::now() - 1);
        // Inserted second, mined earlier: the chain order wins, not the order this process saw them.
        let _later = bid_at(&store, "bbbbbbbb", "11", 4_000_000, "0000000000000011", "valid", 120);
        let earlier = bid_at(&store, "bbbbbbbb", "12", 4_000_000, "0000000000000012", "valid", 119);
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(store::get_auction(&store, "bbbbbbbb").unwrap().unwrap().winner_bid_id, Some(earlier));
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
                seller_id: None, image_url: None, units: 1, format: "first_price", reveal_secs: 0, audit: None,
            },
        )
        .unwrap();
        let b = bid(&store, "gggggggg", "61", 1_000_000, "0000000000000061", "valid");
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(store::get_bid(&store, b).unwrap().unwrap().status, "settled");
        assert_eq!(store::get_auction(&store, "gggggggg").unwrap().unwrap().status, "settled");
    }

    #[test]
    fn vickrey_winner_pays_the_second_bid_and_stays_sealed() {
        let store = store::open_in_memory().unwrap();
        formatted_auction(&store, "hhhhhhhh", 1, "vickrey");
        let b1 = bid(&store, "hhhhhhhh", "71", 3_000_000, "0000000000000071", "valid");
        let b2 = bid(&store, "hhhhhhhh", "72", 5_000_000, "0000000000000072", "valid");
        let _b3 = bid(&store, "hhhhhhhh", "73", 1_000_000, "0000000000000073", "valid"); // below reserve
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        let a = store::get_auction(&store, "hhhhhhhh").unwrap().unwrap();
        assert_eq!(a.winner_bid_id, Some(b2));
        assert_eq!(a.clearing_zats, Some(3_000_000), "price is the runner-up's bid");
        assert_eq!(store::get_bid(&store, b2).unwrap().unwrap().price_zats, Some(3_000_000));
        let refunded: Vec<i64> = store::payouts_for_auction(&store, "hhhhhhhh").unwrap().iter().filter(|p| p.kind == "refund").filter_map(|p| p.bid_id).collect();
        assert!(refunded.contains(&b1));
        // The ledger lets anyone check the price is a committed bid without opening the runner-up's note.
        let entries = store::ledger_entries(&store, "hhhhhhhh").unwrap();
        let receipt = crate::memo::receipt("0000000000000071");
        let proof = crate::merkle::prove(&entries, &receipt).unwrap();
        assert_eq!(proof.root, a.ledger_hash.unwrap());
        assert!(crate::merkle::verify(&receipt, 3_000_000, "0000000000000071", &proof));
    }

    #[test]
    fn vickrey_single_bid_pays_the_reserve_and_second_chance_reprices() {
        let store = store::open_in_memory().unwrap();
        formatted_auction(&store, "iiiiiiii", 1, "vickrey");
        let only = bid(&store, "iiiiiiii", "81", 5_000_000, "0000000000000081", "valid");
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        assert_eq!(store::get_bid(&store, only).unwrap().unwrap().price_zats, Some(2_000_000), "no second bid: pay the reserve");

        let store = store::open_in_memory().unwrap();
        formatted_auction(&store, "jjjjjjjj", 1, "vickrey");
        let b1 = bid(&store, "jjjjjjjj", "91", 3_000_000, "0000000000000091", "valid");
        let b2 = bid(&store, "jjjjjjjj", "92", 4_000_000, "0000000000000092", "valid");
        let b3 = bid(&store, "jjjjjjjj", "93", 6_000_000, "0000000000000093", "valid");
        let c = CloseConfig { settlement_window_secs: -1, ..cfg() };
        close_due(&store, None, &Network::Test, "test", &c, None).unwrap();
        assert_eq!(store::get_bid(&store, b3).unwrap().unwrap().price_zats, Some(4_000_000));
        enforce_settlement_deadlines(&store, &c).unwrap();
        let r = store::get_bid(&store, b2).unwrap().unwrap();
        assert_eq!(r.status, "won");
        assert_eq!(r.price_zats, Some(3_000_000), "runner-up now pays the third bid, not their own");
        assert_eq!(store::get_auction(&store, "jjjjjjjj").unwrap().unwrap().clearing_zats, Some(3_000_000), "published price follows");
        assert_eq!(store::get_bid(&store, b1).unwrap().unwrap().status, "valid");
    }

    #[test]
    fn rfq_lowest_quote_within_budget_wins_and_the_buyer_pays() {
        let store = store::open_in_memory().unwrap();
        formatted_auction(&store, "kkkkkkkk", 1, "rfq"); // budget 2_000_000, bond 1_000_000
        let q1 = bid(&store, "kkkkkkkk", "a1", 1_500_000, "00000000000000a1", "valid");
        let q2 = bid(&store, "kkkkkkkk", "a2", 1_200_000, "00000000000000a2", "valid");
        let _q3 = bid(&store, "kkkkkkkk", "a3", 2_500_000, "00000000000000a3", "valid"); // over budget
        close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap();
        let a = store::get_auction(&store, "kkkkkkkk").unwrap().unwrap();
        assert_eq!(a.status, "closed");
        assert_eq!(a.winner_bid_id, Some(q2));
        assert_eq!(a.clearing_zats, Some(1_200_000));
        let w = store::get_bid(&store, q2).unwrap().unwrap();
        assert_eq!(w.status, "won", "the quoter's bond does not count as payment; the buyer must pay");
        let payouts = store::payouts_for_auction(&store, "kkkkkkkk").unwrap();
        assert_eq!(payouts.iter().filter(|p| p.kind == "refund").count(), 2, "losing and over-budget quotes get their bond back");
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(q1)));

        // The buyer pays the quote (plus a little): the quoter gets quote + bond, the buyer the excess.
        settle_winner(&store, &a, q2, 1_200_000, 1_250_000, "test").unwrap();
        let payouts = store::payouts_for_auction(&store, "kkkkkkkk").unwrap();
        let to_quoter = payouts.iter().find(|p| p.kind == "seller").unwrap();
        assert_eq!(to_quoter.value_zats, 2_200_000);
        assert_eq!(to_quoter.to_address, UA);
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(q2) && p.value_zats == 50_000));
        assert_eq!(store::get_auction(&store, "kkkkkkkk").unwrap().unwrap().status, "settled");
    }

    #[test]
    fn rfq_buyer_default_returns_the_bond_and_lapses() {
        let store = store::open_in_memory().unwrap();
        formatted_auction(&store, "llllllll", 1, "rfq");
        let q = bid(&store, "llllllll", "b1", 1_000_000, "00000000000000b1", "valid");
        let c = CloseConfig { settlement_window_secs: -1, ..cfg() };
        close_due(&store, None, &Network::Test, "test", &c, None).unwrap();
        assert_eq!(enforce_settlement_deadlines(&store, &c).unwrap(), 1);
        assert_eq!(store::get_bid(&store, q).unwrap().unwrap().status, "expired");
        assert_eq!(store::get_auction(&store, "llllllll").unwrap().unwrap().status, "unsettled");
        let payouts = store::payouts_for_auction(&store, "llllllll").unwrap();
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(q) && p.value_zats == 1_000_000));
        assert!(payouts.iter().all(|p| p.kind != "forfeit"));
        assert_eq!(store::ledger_entries(&store, "llllllll").unwrap().len(), 1, "expired quotes stay in the ledger");
    }

    fn sealed_auction(store: &Connection, id: &str, reveal_secs: i64) {
        store::insert_auction(
            store,
            &NewAuction {
                id, title: "sealed", description: "", reserve_zats: 2_000_000, deposit_zats: 1_000_000,
                seller_address: UA, address: UA, diversifier_index: None, close_at: store::now() - 1,
                seller_id: None, image_url: None, units: 1, format: "first_price", reveal_secs, audit: None,
            },
        )
        .unwrap();
    }

    fn sealed_bid(store: &Connection, auction_id: &str, txid: &str, bid_zats: u64, salt: &str, nonce: &str) -> i64 {
        let receipt = crate::memo::receipt(nonce);
        let commitment = crate::memo::commitment(auction_id, bid_zats, salt);
        store::insert_bid(
            store,
            &NewBid {
                auction_id: Some(auction_id), txid, output_index: 0, pool: 4, mined_height: Some(1), value_zats: 1_000_000,
                memo: None, bid_zats: None, refund_address: Some(UA), nonce: Some(nonce), receipt: Some(&receipt),
                status: "sealed", source: "memo", commitment: Some(&commitment),
            },
        )
        .unwrap()
    }

    #[test]
    fn sealed_lot_commits_at_close_then_closes_over_reveals_and_refunds_the_silent() {
        let store = store::open_in_memory().unwrap();
        sealed_auction(&store, "ssssssss", 60);
        let salt_a = "0000000000000000000000000000000a";
        let salt_b = "0000000000000000000000000000000b";
        let a = sealed_bid(&store, "ssssssss", "s1", 3_000_000, salt_a, "00000000000000s1");
        let b = sealed_bid(&store, "ssssssss", "s2", 5_000_000, salt_b, "00000000000000s2");
        let c = sealed_bid(&store, "ssssssss", "s3", 9_000_000, "0000000000000000000000000000000c", "00000000000000s3");

        // Close: nothing is ranked yet; the commitment root is fixed and the reveal window opens.
        assert_eq!(close_due(&store, None, &Network::Test, "test", &cfg(), None).unwrap(), 1);
        let lot = store::get_auction(&store, "ssssssss").unwrap().unwrap();
        assert_eq!(lot.status, "revealing");
        assert!(lot.commit_hash.is_some());
        assert!(lot.winner_bid_id.is_none());
        assert!(store::payouts_for_auction(&store, "ssssssss").unwrap().is_empty(), "no money moves before reveals");

        // A reveal with the wrong salt is rejected by the commitment check the API performs.
        let wrong = crate::memo::commitment("ssssssss", 5_000_000, salt_a);
        assert_ne!(Some(wrong.as_str()), store::get_bid(&store, b).unwrap().unwrap().commitment.as_deref());
        // A and B reveal; C never does.
        store::reveal_bid(&store, a, 3_000_000, salt_a).unwrap();
        store::reveal_bid(&store, b, 5_000_000, salt_b).unwrap();
        assert_eq!(store::get_bid(&store, b).unwrap().unwrap().status, "valid");

        // Reveal window still open: nothing happens.
        assert_eq!(finalize_reveals(&store, None, &Network::Test, "test", &cfg(), None).unwrap(), 0);
        store.execute("UPDATE auctions SET reveal_until = ?1 WHERE id = 'ssssssss'", [store::now() - 1]).unwrap();
        assert_eq!(finalize_reveals(&store, None, &Network::Test, "test", &cfg(), None).unwrap(), 1);

        let lot = store::get_auction(&store, "ssssssss").unwrap().unwrap();
        assert_eq!(lot.status, "closed");
        assert_eq!(lot.winner_bid_id, Some(b), "the highest revealed bid wins; the unrevealed 0.09 does not");
        assert_eq!(lot.clearing_zats, Some(5_000_000));
        assert_eq!(store::get_bid(&store, c).unwrap().unwrap().status, "unrevealed");
        let payouts = store::payouts_for_auction(&store, "ssssssss").unwrap();
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(c) && p.value_zats == 1_000_000), "silent bidder gets the deposit back");
        assert!(payouts.iter().any(|p| p.kind == "refund" && p.bid_id == Some(a)), "loser refunded");
        let entries = store::ledger_entries(&store, "ssssssss").unwrap();
        assert_eq!(entries.len(), 2, "ledger covers revealed bids only");
    }

    #[test]
    fn refund_times_land_on_window_boundaries_within_the_delay_range() {
        let c = CloseConfig { delay_min: 30, delay_max: 300, settlement_window_secs: 3600, refund_window_secs: 60 };
        let mut rng = rand::rng();
        for _ in 0..200 {
            let t = refund_time(&mut rng, 1_000_000, &c);
            assert_eq!(t % 60, 0, "rounded up to the window");
            assert!(t >= 1_000_030 && t <= 1_000_360, "within [min, max + window): {t}");
        }
        let plain = CloseConfig { refund_window_secs: 0, ..c };
        let t = refund_time(&mut rng, 1_000_000, &plain);
        assert!((1_000_030..=1_000_300).contains(&t));
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
