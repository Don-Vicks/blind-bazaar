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
