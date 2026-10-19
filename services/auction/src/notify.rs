//! Optional notifications: a bidder or seller who leaves an email is told when something needs them.
//!
//! Privacy: an email ties a person to a bid, which the rest of the design avoids, so it is strictly
//! opt-in, shown to nobody, never returned by any endpoint, and purged a week after the lot reaches a
//! final state. Mail goes through the operator's SMTP (`NOTIFY_SMTP_URL`); without it, notifications
//! are recorded and logged but not sent, so the product works the same with or without a mail server.

use anyhow::{Result, anyhow};
use rusqlite::Connection;
use tracing::{info, warn};

use crate::store;

/// Messages derived from one event. Returns (subscription id, subject, body) triples.
pub fn messages_for(store: &Connection, ev: &store::Event, site: &str) -> Result<Vec<(i64, String, String)>> {
    let Some(aid) = ev.auction_id.as_deref() else { return Ok(vec![]) };
    let Some(auction) = store::get_auction(store, aid)? else { return Ok(vec![]) };
    let lot = format!("{site}/a/{aid}");
    let status = format!("{site}/a/{aid}/status");
    let title = &auction.title;
    let receipt = ev.payload.get("receipt").or_else(|| ev.payload.get("bid_receipt")).and_then(|v| v.as_str());
    let mut out = Vec::new();
    let bidder = |r: &str| store::subscriptions_for_receipt(store, aid, r);
    match ev.kind.as_str() {
        "bid_received" => {
            if let Some(r) = receipt {
                for s in bidder(r)? {
                    out.push((s, format!("Your bid on \"{title}\" was received"), format!("The auction read your sealed bid. Nothing to do until the lot closes.\n\nStanding: {status}")));
                }
            }
        }
        "reveal_open" => {
            let until = ev.payload.get("reveal_until").and_then(|v| v.as_i64()).unwrap_or(0);
            for s in store::subscriptions_for_bid_status(store, aid, "sealed")? {
                out.push((s, format!("Reveal your bid on \"{title}\""), format!("The lot has closed. Reveal your bid before {} (unix {until}) or your deposit is returned and the bid cannot win.\n\nReveal here: {status}", fmt_time(until))));
            }
        }
        "closed" => {
            for s in store::subscriptions_for_bid_status(store, aid, "won")? {
                out.push((s, format!("You won \"{title}\""), format!("Pay the balance before the deadline to settle. Your standing page has the amount and a payment link.\n\n{status}")));
            }
            for s in store::subscriptions_for_bid_status(store, aid, "valid")? {
                out.push((s, format!("\"{title}\" has closed"), format!("Your bid did not win. Your deposit returns to the refund address you sealed, after a short random delay.\n\n{status}")));
            }
            for s in store::subscriptions_for_seller(store, auction.seller_id)? {
                let price = ev.payload.get("clearing_zats").and_then(|v| v.as_u64());
                out.push((s, format!("Your lot \"{title}\" closed"), match price {
                    Some(p) => format!("Closed at {p} zatoshis. Once the winner pays you will be paid; post the delivery note for the winner on your dashboard.\n\n{site}/dashboard/{aid}"),
                    None => format!("Closed with no bid at or above the reserve. Deposits are being returned.\n\n{site}/dashboard/{aid}"),
                }));
            }
        }
        "second_chance" => {
            if let Some(r) = receipt {
                for s in bidder(r)? {
                    out.push((s, format!("Second chance on \"{title}\""), format!("The winner did not pay. The lot is yours if you pay the balance before the deadline.\n\n{status}")));
                }
            }
        }
        "settled" => {
            for s in store::subscriptions_for_seller(store, auction.seller_id)? {
                out.push((s, format!("The winner of \"{title}\" has paid"), format!("Your payout is queued. Post the delivery note now if you have not.\n\n{site}/dashboard/{aid}")));
            }
        }
        "seller_paid" => {
            let txid = ev.payload.get("txid").and_then(|v| v.as_str()).unwrap_or("");
            for s in store::subscriptions_for_seller(store, auction.seller_id)? {
                out.push((s, format!("You were paid for \"{title}\""), format!("Payout sent in transaction {txid}.\n\n{site}/dashboard/{aid}")));
            }
        }
        "refund_sent" => {
            if let Some(pid) = ev.payload.get("payout_id").and_then(|v| v.as_i64()) {
                for s in store::subscriptions_for_payout(store, pid)? {
                    out.push((s, format!("Refund sent for \"{title}\""), format!("Your deposit is on its way to the address you sealed.\n\n{status}")));
                }
            }
        }
        "forfeited" => {
            if let Some(r) = receipt {
                for s in bidder(r)? {
                    out.push((s, format!("Deadline missed on \"{title}\""), format!("The balance was not paid in time; the deposit went to the seller.\n\n{lot}")));
                }
            }
        }
        _ => {}
    }
    Ok(out)
}

fn fmt_time(t: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(t)
        .map(|d| d.format(&time::format_description::well_known::Rfc2822).unwrap_or_default())
        .unwrap_or_default()
}

/// What the operator is told about. Edge-triggered: an alert goes out when its condition appears and not again
/// until it has cleared, so a long outage is one mail, not one per pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperatorState {
    pub low_balance: bool,
    pub balance_zats: Option<u64>,
    pub pending_payouts_zats: u64,
    pub fee_reserve_zats: u64,
    pub dead_payouts: i64,
    pub frozen: Option<String>,
}

/// Queue operator alerts for conditions that are newly true. Returns how many were queued.
pub fn operator_alerts(store: &Connection, subscription_id: i64, st: &OperatorState, site: &str) -> Result<usize> {
    let mut queued = 0;
    let mut edge = |key: &str, now_true: bool, subject: String, body: String| -> Result<()> {
        let was = store::get_meta(store, key)?.as_deref() == Some("1");
        if now_true && !was {
            store::insert_notification(store, subscription_id, 0, "operator_alert", &subject, &body)?;
            queued += 1;
        }
        if now_true != was {
            store::set_meta(store, key, if now_true { "1" } else { "0" })?;
        }
        Ok(())
    };
    edge(
        "alert_low_balance",
        st.low_balance,
        "Operator float is low".into(),
        format!(
            "The operator wallet holds {} zatoshis; pending payouts need {} plus a fee reserve of {}. Refunds and seller payouts will wait until it is topped up.\n\n{site}/dashboard",
            st.balance_zats.unwrap_or(0),
            st.pending_payouts_zats,
            st.fee_reserve_zats
        ),
    )?;
    edge(
        "alert_dead_payouts",
        st.dead_payouts > 0,
        format!("{} payout(s) need a human", st.dead_payouts),
        format!("{} payout(s) exhausted their retries. Check the admin payout list, fix the cause, and retry them.\n\n{site}/dashboard", st.dead_payouts),
    )?;
    edge(
        "alert_frozen",
        st.frozen.is_some(),
        "The service has parked itself".into(),
        format!("Reason: {}. Nothing moves on chain until it is restarted on a build that follows the chain.\n\n{site}", st.frozen.clone().unwrap_or_default()),
    )?;
    Ok(queued)
}

pub struct Mailer {
    url: String,
    from: String,
}

impl Mailer {
    pub fn from_env(url: Option<String>, from: Option<String>) -> Option<Self> {
        match (url, from) {
            (Some(url), Some(from)) if !url.is_empty() => Some(Self { url, from }),
            _ => None,
        }
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        use lettre::{AsyncTransport, Message};
        let email = Message::builder()
            .from(self.from.parse().map_err(|e| anyhow!("NOTIFY_FROM: {e}"))?)
            .to(to.parse().map_err(|e| anyhow!("recipient: {e}"))?)
            .subject(subject)
            .body(body.to_string())?;
        let transport = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(&self.url)?.build();
        transport.send(email).await?;
        Ok(())
    }
}

/// Turn new events into notifications, send what the mailer can, purge stale subscriptions.
pub async fn run_pass(store: &tokio::sync::Mutex<Connection>, mailer: Option<&Mailer>, site: &str) -> Result<()> {
    let (pending, last) = {
        let s = store.lock().await;
        let last: i64 = store::get_meta(&s, "notified_event_id")?.and_then(|v| v.parse().ok()).unwrap_or(0);
        let events = store::events_after(&s, last, 100)?;
        let mut newest = last;
        let mut pending = Vec::new();
        for ev in &events {
            newest = newest.max(ev.id);
            for (sub, subject, body) in messages_for(&s, ev, site)? {
                let id = store::insert_notification(&s, sub, ev.id, &ev.kind, &subject, &body)?;
                pending.push(id);
            }
        }
        if newest != last {
            store::set_meta(&s, "notified_event_id", &newest.to_string())?;
        }
        store::purge_stale_subscriptions(&s)?;
        (pending, newest)
    };
    let _ = last;
    let due = {
        let s = store.lock().await;
        store::unsent_notifications(&s, 20)?
    };
    for n in due {
        let result = match mailer {
            Some(m) => m.send(&n.email, &n.subject, &n.body).await,
            None => {
                info!("notification (no mailer configured): to <redacted> \"{}\"", n.subject);
                Ok(())
            }
        };
        let s = store.lock().await;
        match result {
            Ok(()) => store::mark_notification_sent(&s, n.id, None)?,
            Err(e) => {
                warn!("notification {} failed: {e:#}", n.id);
                store::mark_notification_sent(&s, n.id, Some(&format!("{e:#}")))?;
            }
        }
    }
    let _ = pending;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{NewAuction, NewBid};

    const UA: &str = "utest1phxskj2u3phwse5pql5dazp4katjgtx265pnxgputh9p6vgjpc4pt33gmwgmdmqd3gnucf7f073cxlk6c7f6gkzxu3yvgwuysyplaj24";

    #[test]
    fn operator_alerts_fire_on_the_edge_and_again_after_clearing() {
        let s = store::open_in_memory().unwrap();
        let id = store::ensure_operator_subscription(&s, "ops@example.test").unwrap();
        assert_eq!(store::ensure_operator_subscription(&s, "ops2@example.test").unwrap(), id, "one row, updated in place");
        let low = OperatorState { low_balance: true, balance_zats: Some(5), pending_payouts_zats: 100, fee_reserve_zats: 10, ..Default::default() };
        assert_eq!(operator_alerts(&s, id, &low, "x").unwrap(), 1);
        assert_eq!(operator_alerts(&s, id, &low, "x").unwrap(), 0, "still low: no repeat");
        let fine = OperatorState { fee_reserve_zats: 10, ..Default::default() };
        assert_eq!(operator_alerts(&s, id, &fine, "x").unwrap(), 0, "clearing sends nothing");
        assert_eq!(operator_alerts(&s, id, &low, "x").unwrap(), 1, "low again: a new alert");
        let dead_and_frozen = OperatorState { dead_payouts: 2, frozen: Some("paused by the operator".into()), ..low.clone() };
        assert_eq!(operator_alerts(&s, id, &dead_and_frozen, "x").unwrap(), 2, "two new conditions, one mail each");
        let pending = store::unsent_notifications(&s, 10).unwrap();
        assert_eq!(pending.len(), 4);
        assert!(pending.iter().all(|n| n.email == "ops2@example.test"));
        assert!(pending.iter().any(|n| n.subject.contains("need a human")));
        assert!(pending.iter().any(|n| n.subject.contains("parked")));
        // The operator row survives the purge that removes stale bidder subscriptions.
        store::purge_stale_subscriptions(&s).unwrap();
        assert_eq!(store::unsent_notifications(&s, 10).unwrap().len(), 4);
    }

    #[test]
    fn events_become_the_right_mails_for_the_right_people() {
        let s = store::open_in_memory().unwrap();
        let sid = store::insert_seller(&s, "alice", "hash").unwrap();
        store::insert_auction(
            &s,
            &NewAuction {
                id: "aaaaaaaa", title: "Signed print", description: "", reserve_zats: 2_000_000, deposit_zats: 1_000_000, seller_address: UA, address: UA,
                diversifier_index: None, close_at: 1, seller_id: Some(sid), image_url: None, units: 1, format: "first_price", reveal_secs: 0, audit: None,
            },
        )
        .unwrap();
        let mut ids = Vec::new();
        for (i, (nonce, status)) in [("00000000000000a1", "won"), ("00000000000000a2", "valid"), ("00000000000000a3", "valid")].iter().enumerate() {
            let receipt = crate::memo::receipt(nonce);
            ids.push(store::insert_bid(&s, &NewBid {
                auction_id: Some("aaaaaaaa"), txid: &format!("{i:064x}"), output_index: 0, pool: 4, mined_height: Some(1), value_zats: 1_000_000, memo: None,
                bid_zats: Some(3_000_000), refund_address: Some(UA), nonce: Some(nonce), receipt: Some(&receipt), status, source: "memo", commitment: None,
            }).unwrap());
        }
        // Winner and one loser opted in; the other loser did not. The seller opted in.
        store::subscribe_bidder(&s, "aaaaaaaa", "00000000000000a1", "w@example.test").unwrap();
        store::subscribe_bidder(&s, "aaaaaaaa", "00000000000000a2", "l@example.test").unwrap();
        store::subscribe_seller(&s, sid, Some("alice@example.test")).unwrap();

        let ev = |kind: &str, payload: serde_json::Value| store::Event { id: 1, auction_id: Some("aaaaaaaa".into()), kind: kind.into(), payload, created_at: 0 };
        let closed = messages_for(&s, &ev("closed", serde_json::json!({ "clearing_zats": 3_000_000 })), "https://bb.example").unwrap();
        let subjects: Vec<&str> = closed.iter().map(|m| m.1.as_str()).collect();
        assert_eq!(closed.len(), 3, "winner, opted-in loser, seller: {subjects:?}");
        assert!(subjects.iter().any(|t| t.starts_with("You won")));
        assert!(subjects.iter().any(|t| t.contains("has closed")));
        assert!(subjects.iter().any(|t| t.starts_with("Your lot")));
        assert!(closed.iter().all(|m| m.2.contains("https://bb.example/")), "every mail links back to the site");

        let received = messages_for(&s, &ev("bid_received", serde_json::json!({ "receipt": crate::memo::receipt("00000000000000a3") })), "x").unwrap();
        assert!(received.is_empty(), "a bidder who did not opt in gets nothing");

        store::queue_payout(&s, Some("aaaaaaaa"), Some(ids[1]), "refund", UA, 1_000_000, None, 0).unwrap();
        let refund = messages_for(&s, &ev("refund_sent", serde_json::json!({ "payout_id": 1 })), "x").unwrap();
        assert_eq!(refund.len(), 1);
        assert!(refund[0].1.starts_with("Refund sent"));

        // Nothing about an address is exposed through the standing or any public struct: the subscription
        // table is the only place it lives, and a purge removes it once the lot is final and a week old.
        s.execute("UPDATE auctions SET status = 'paid', closed_at = ?1 WHERE id = 'aaaaaaaa'", [store::now() - 8 * 86_400]).unwrap();
        assert_eq!(store::purge_stale_subscriptions(&s).unwrap(), 2, "both bidder subscriptions purged; the seller's stays");
    }
}
