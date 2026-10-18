//! `bb-auction audit`: recompute an auditable lot's result from the chain alone.
//!
//! Inputs: the lot's published full viewing key and birthday, the public lot definition (deposit,
//! reserve, format, close time) and, optionally, the published result to compare against. The tool
//! imports the key into a throwaway view-only wallet, scans from the birthday with whatever
//! lightwalletd the auditor trusts, recovers every memo, applies the same acceptance rules as the
//! service, ranks the bids, and prints what the chain says the result is. Nothing from the service is
//! trusted beyond the public lot definition, and even that is checked against what the chain shows.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{close, ingest, memo, store, wallet};
use crate::wallet::net::Net as Network;

#[derive(Deserialize)]
pub struct Lot {
    pub id: String,
    pub reserve_zats: u64,
    pub deposit_zats: u64,
    pub close_at: i64,
    #[serde(default = "first_price")]
    pub format: String,
    #[serde(default)]
    pub units: u32,
    #[serde(default)]
    pub reveal_secs: i64,
}
fn first_price() -> String {
    "first_price".into()
}

#[derive(Deserialize, Default)]
pub struct Published {
    #[serde(default)]
    pub ledger_hash: Option<String>,
    #[serde(default)]
    pub clearing_zats: Option<u64>,
    #[serde(default)]
    pub winning_txids: Vec<String>,
    #[serde(default)]
    pub audit: Option<PublishedAudit>,
}

#[derive(Deserialize, Default)]
pub struct PublishedAudit {
    #[serde(default)]
    pub reveals: Vec<Reveal>,
}

#[derive(Deserialize, Clone)]
pub struct Reveal {
    pub receipt: String,
    pub bid_zats: u64,
    pub salt: String,
}

/// Fetch a local file or an http(s) URL as text.
async fn fetch(src: &str) -> Result<String> {
    if src.starts_with("http://") || src.starts_with("https://") {
        Ok(reqwest::get(src).await?.error_for_status()?.text().await?)
    } else {
        std::fs::read_to_string(src).with_context(|| format!("read {src}"))
    }
}

pub struct AuditArgs {
    pub ufvk: String,
    pub birthday: u32,
    pub lot: String,
    pub result: Option<String>,
    pub dir: std::path::PathBuf,
}

pub async fn run(params: Network, lightwalletd_url: &str, a: &AuditArgs) -> Result<bool> {
    let lot: Lot = serde_json::from_str(&fetch(&a.lot).await?).context("lot definition JSON")?;
    let published: Published = match &a.result {
        Some(r) => serde_json::from_str(&fetch(r).await?).context("result JSON")?,
        None => Published::default(),
    };
    println!("lot {}: deposit {} reserve {} format {} close_at {}", lot.id, lot.deposit_zats, lot.reserve_zats, lot.format, lot.close_at);

    // A throwaway view-only wallet holding only this lot's key.
    std::fs::create_dir_all(&a.dir)?;
    let mut db = wallet::open_db(params, &a.dir)?;
    let mut client = wallet::connect(lightwalletd_url).await?;
    let me = wallet::ensure_viewer_account(&mut db, &mut client, &params, &a.ufvk, Some(a.birthday)).await?;
    let _ = me;
    let cache = wallet::cache::MemBlockCache::default();
    println!("scanning from {} with {lightwalletd_url} …", a.birthday);
    wallet::sync::sync_once(&mut client, &params, &cache, &mut db).await?;
    let enhanced = wallet::sync::enhance(&mut client, &params, &mut db).await?;
    println!("scanned to {:?}; {enhanced} transactions enhanced for memos", wallet::wallet_height(&db)?);

    // Every received output, judged by the same rules the service uses.
    let (_, data_db) = wallet::db_paths(&a.dir);
    let ro = ingest::open_wallet_readonly(&data_db)?;
    let outputs = ingest::received_outputs(&ro)?;
    let reveals: Vec<Reveal> = published.audit.as_ref().map(|x| x.reveals.clone()).unwrap_or_default();
    let bids = judge(&lot, &outputs, &reveals, |h| ingest::block_time(&ro, h).ok().flatten());
    for b in &bids {
        println!(
            "  {} {} h={:?} value={} {}{}",
            &b.txid[..16],
            b.status,
            b.mined_height,
            b.value_zats,
            b.bid_zats.map(|v| format!("bid={v} ")).unwrap_or_default(),
            &b.receipt.as_deref().unwrap_or("")[..12]
        );
    }
    let rejected = bids.iter().filter(|b| b.status != "valid").count();
    let r = recompute(&lot, &bids);
    println!("{} received outputs, {} bids read, {} rejected, {} valid", outputs.len(), bids.len(), rejected, bids.len() - rejected);
    println!("\nrecomputed from the chain:");
    println!("  ledger root  {}", r.ledger_hash);
    println!("  clearing     {:?}", r.clearing);
    for (txid, receipt) in &r.winners {
        println!("  winner       {txid} (receipt {receipt})");
    }

    let mut ok = true;
    if a.result.is_some() {
        for c in compare(&r, &published) {
            println!("  {:<13}{}  chain={}  published={}", c.name, if c.same { "MATCH" } else { "MISMATCH" }, c.ours, c.theirs);
            ok &= c.same;
        }
        println!("\n{}", if ok { "OK: the published result is what the chain says" } else { "FAIL: the published result does not match the chain" });
    }
    Ok(ok)
}

/// Judge every received output by the service's acceptance rules. `block_time` answers the mined time of a
/// height from whatever chain view the auditor trusts.
pub fn judge(lot: &Lot, outputs: &[ingest::ReceivedOutput], reveals: &[Reveal], block_time: impl Fn(u32) -> Option<i64>) -> Vec<store::Bid> {
    let sealed = lot.reveal_secs > 0;
    let mut bids: Vec<store::Bid> = Vec::new();
    for (i, out) in outputs.iter().enumerate() {
        let Some(m) = out.memo.as_deref() else { continue };
        let parsed = match memo::parse(m) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let (nonce, refund, bid_zats, commitment) = match parsed {
            memo::Parsed::Bid(b) if b.auction_id == lot.id => (b.nonce, b.refund_address, Some(b.bid_zats), None),
            memo::Parsed::Commit(c) if c.auction_id == lot.id => (c.nonce, c.refund_address, None, Some(c.commitment)),
            _ => continue,
        };
        let mined_at = out.mined_height.and_then(&block_time);
        let late = mined_at.map(|t| t > lot.close_at).unwrap_or(false);
        let receipt = memo::receipt(&nonce);
        let mut status = "valid";
        let mut amount = bid_zats;
        if late {
            status = "late";
        } else if out.value_zats != lot.deposit_zats {
            status = "wrong_deposit";
        } else if sealed != commitment.is_some() {
            status = if sealed { "unsealed_bid" } else { "unexpected_commit" };
        } else if let Some(c) = &commitment {
            match reveals.iter().find(|r| r.receipt == receipt) {
                Some(r) if memo::commitment(&lot.id, r.bid_zats, &r.salt) == *c => amount = Some(r.bid_zats),
                Some(_) => status = "bad_reveal",
                None => status = "unrevealed",
            }
        }
        if bids.iter().any(|b| b.nonce.as_deref() == Some(&nonce)) && status == "valid" {
            status = "duplicate_nonce";
        }
        bids.push(store::Bid {
            id: i as i64,
            auction_id: Some(lot.id.clone()),
            txid: out.txid.clone(),
            output_index: out.output_index,
            pool: out.pool,
            mined_height: out.mined_height,
            value_zats: out.value_zats,
            memo: Some(m.to_string()),
            bid_zats: amount,
            refund_address: Some(refund),
            nonce: Some(nonce),
            receipt: Some(receipt),
            status: status.to_string(),
            source: "chain".into(),
            received_at: mined_at.unwrap_or(0),
            price_zats: None,
            settle_by: None,
            commitment,
            salt: None,
            revealed_at: None,
        });
    }
    bids
}

/// What the chain says the result is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomputed {
    pub ledger_hash: String,
    pub clearing: Option<u64>,
    /// (txid, receipt) in rank order.
    pub winners: Vec<(String, String)>,
}

pub fn recompute(lot: &Lot, bids: &[store::Bid]) -> Recomputed {
    let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();
    let ledger: Vec<crate::merkle::Entry> = valid
        .iter()
        .map(|b| (b.receipt.clone().unwrap_or_default(), b.bid_zats.unwrap_or(0), b.nonce.clone().unwrap_or_default()))
        .collect();
    let ledger_hash = crate::merkle::root(&ledger);
    let ranked = close::rank(&valid, &lot.format, lot.reserve_zats, &[]);
    let units = if lot.format == "uniform" { lot.units.max(1) as usize } else { 1 };
    let winners: Vec<&store::Bid> = ranked.iter().copied().take(units).collect();
    let clearing = match lot.format.as_str() {
        "uniform" => winners.last().and_then(|b| b.bid_zats),
        _ if winners.is_empty() => None,
        f => Some(close::price_for(f, lot.reserve_zats, &ranked, 0).0),
    };
    Recomputed {
        ledger_hash,
        clearing,
        winners: winners.iter().map(|w| (w.txid.clone(), w.receipt.clone().unwrap_or_default())).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: &'static str,
    pub same: bool,
    pub ours: String,
    pub theirs: String,
}

/// Line up the chain's answer against the published one. Every field the operator publishes about the outcome is
/// checked; a mismatch in any of them means the published result is not what the chain says.
pub fn compare(r: &Recomputed, published: &Published) -> Vec<Check> {
    let ours: Vec<String> = r.winners.iter().map(|w| w.0.clone()).collect();
    vec![
        Check { name: "ledger root", same: published.ledger_hash.as_deref() == Some(r.ledger_hash.as_str()), ours: r.ledger_hash.clone(), theirs: published.ledger_hash.clone().unwrap_or_default() },
        Check { name: "clearing", same: published.clearing_zats == r.clearing, ours: format!("{:?}", r.clearing), theirs: format!("{:?}", published.clearing_zats) },
        Check { name: "winners", same: ours == published.winning_txids, ours: ours.join(","), theirs: published.winning_txids.join(",") },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA: &str = "utest1phxskj2u3phwse5pql5dazp4katjgtx265pnxgputh9p6vgjpc4pt33gmwgmdmqd3gnucf7f073cxlk6c7f6gkzxu3yvgwuysyplaj24";

    fn lot(format: &str) -> Lot {
        Lot { id: "a0d1t0t5".into(), reserve_zats: 1_000_000, deposit_zats: 500_000, close_at: 1_000, format: format.into(), units: 1, reveal_secs: 0 }
    }

    fn out(i: u8, height: u32, value: u64, memo: Option<String>) -> ingest::ReceivedOutput {
        ingest::ReceivedOutput { txid: format!("{i:064x}"), mined_height: Some(height), pool: 4, output_index: 0, to_address: None, value_zats: value, memo }
    }

    fn bid_memo(nonce: &str, bid: u64) -> Option<String> {
        Some(memo::encode_bid(&memo::Bid { auction_id: "a0d1t0t5".into(), bid_zats: bid, refund_address: UA.into(), nonce: nonce.into() }))
    }

    /// Three bids before close at 3, 2 and 1 ZEC, one late, one with the wrong deposit, one duplicate nonce.
    fn chain() -> Vec<ingest::ReceivedOutput> {
        vec![
            out(1, 10, 500_000, bid_memo("00000000000000a1", 3_000_000)),
            out(2, 11, 500_000, bid_memo("00000000000000a2", 2_000_000)),
            out(3, 12, 500_000, bid_memo("00000000000000a3", 1_000_000)),
            out(4, 20, 500_000, bid_memo("00000000000000a4", 9_000_000)), // mined after close
            out(5, 12, 400_000, bid_memo("00000000000000a5", 8_000_000)), // short deposit
            out(6, 13, 500_000, bid_memo("00000000000000a1", 7_000_000)), // nonce reused
            out(7, 13, 500_000, None),                                     // no memo: a plain payment
        ]
    }

    fn time(h: u32) -> Option<i64> {
        Some(if h >= 20 { 2_000 } else { 100 })
    }

    fn honest(lot: &Lot) -> (Recomputed, Published) {
        let bids = judge(lot, &chain(), &[], time);
        let r = recompute(lot, &bids);
        let p = Published { ledger_hash: Some(r.ledger_hash.clone()), clearing_zats: r.clearing, winning_txids: r.winners.iter().map(|w| w.0.clone()).collect(), audit: None };
        (r, p)
    }

    #[test]
    fn judges_by_the_service_rules() {
        let bids = judge(&lot("first_price"), &chain(), &[], time);
        let statuses: Vec<(&str, &str)> = bids.iter().map(|b| (&b.txid[63..], b.status.as_str())).collect();
        assert_eq!(statuses, vec![("1", "valid"), ("2", "valid"), ("3", "valid"), ("4", "late"), ("5", "wrong_deposit"), ("6", "duplicate_nonce")]);
        let r = recompute(&lot("first_price"), &bids);
        assert_eq!(r.clearing, Some(3_000_000));
        assert_eq!(r.winners.len(), 1);
        assert!(r.winners[0].0.ends_with('1'));
        let v = recompute(&lot("vickrey"), &bids);
        assert_eq!(v.clearing, Some(2_000_000), "vickrey pays the second bid");
        assert_eq!(v.winners, r.winners);
    }

    #[test]
    fn an_honest_result_matches() {
        let l = lot("first_price");
        let (r, p) = honest(&l);
        assert!(compare(&r, &p).iter().all(|c| c.same));
    }

    #[test]
    fn a_suppressed_winning_bid_is_caught() {
        // The operator publishes a result that pretends the 3 ZEC bid never happened: the runner-up wins at 2 ZEC,
        // with a ledger root over the remaining bids. The chain still shows the 3 ZEC payment.
        let l = lot("first_price");
        let (r, _) = honest(&l);
        let mut without = chain();
        without.remove(0);
        let forged = recompute(&l, &judge(&l, &without, &[], time));
        let p = Published { ledger_hash: Some(forged.ledger_hash.clone()), clearing_zats: forged.clearing, winning_txids: forged.winners.iter().map(|w| w.0.clone()).collect(), audit: None };
        let checks = compare(&r, &p);
        assert!(checks.iter().all(|c| !c.same), "root, price and winner all differ: {checks:?}");
    }

    #[test]
    fn a_lowered_price_is_caught_even_with_the_right_winner() {
        let l = lot("first_price");
        let (r, mut p) = honest(&l);
        p.clearing_zats = Some(2_500_000);
        let checks = compare(&r, &p);
        assert!(checks.iter().find(|c| c.name == "clearing").map(|c| !c.same).unwrap());
        assert!(checks.iter().filter(|c| c.name != "clearing").all(|c| c.same));
    }

    #[test]
    fn a_swapped_winner_is_caught() {
        let l = lot("first_price");
        let (r, mut p) = honest(&l);
        p.winning_txids = vec![format!("{:064x}", 2)];
        assert!(!compare(&r, &p).iter().find(|c| c.name == "winners").unwrap().same);
    }

    #[test]
    fn a_planted_bid_is_caught() {
        // A ledger root that includes a bid the chain never carried (a shill the operator invented).
        let l = lot("first_price");
        let (r, mut p) = honest(&l);
        let mut entries: Vec<crate::merkle::Entry> = judge(&l, &chain(), &[], time)
            .iter()
            .filter(|b| b.status == "valid")
            .map(|b| (b.receipt.clone().unwrap(), b.bid_zats.unwrap(), b.nonce.clone().unwrap()))
            .collect();
        entries.push((memo::receipt("00000000000000ff"), 2_900_000, "00000000000000ff".into()));
        p.ledger_hash = Some(crate::merkle::root(&entries));
        assert!(!compare(&r, &p).iter().find(|c| c.name == "ledger root").unwrap().same);
    }

    #[test]
    fn sealed_lots_need_matching_reveals() {
        let mut l = lot("first_price");
        l.reveal_secs = 600;
        let salt = "0123456789abcdef0123456789abcdef";
        let commit = |nonce: &str, bid: u64| {
            Some(memo::encode_commit(&memo::Commit { auction_id: "a0d1t0t5".into(), commitment: memo::commitment("a0d1t0t5", bid, salt), refund_address: UA.into(), nonce: nonce.into() }))
        };
        let outs = vec![out(1, 10, 500_000, commit("00000000000000b1", 3_000_000)), out(2, 10, 500_000, commit("00000000000000b2", 2_000_000))];
        let reveals = vec![
            Reveal { receipt: memo::receipt("00000000000000b1"), bid_zats: 3_000_000, salt: salt.into() },
            // The second reveal claims a different amount from what was committed.
            Reveal { receipt: memo::receipt("00000000000000b2"), bid_zats: 5_000_000, salt: salt.into() },
        ];
        let bids = judge(&l, &outs, &reveals, time);
        assert_eq!(bids[0].status, "valid");
        assert_eq!(bids[0].bid_zats, Some(3_000_000));
        assert_eq!(bids[1].status, "bad_reveal");
        let none = judge(&l, &outs, &[], time);
        assert!(none.iter().all(|b| b.status == "unrevealed"));
    }
}

