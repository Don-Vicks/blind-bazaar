//! `bb-auction audit`: recompute an auditable lot's result from the chain alone.
//!
//! Inputs: the lot's published full viewing key and birthday, the public lot definition (deposit,
//! reserve, format, close time) and, optionally, the published result to compare against. The tool
//! imports the key into a throwaway view-only wallet, scans from the birthday with whatever
//! lightwalletd the auditor trusts, recovers every memo, applies the same acceptance rules as the
//! service, ranks the bids, and prints what the chain says the result is. Nothing from the service is
//! trusted beyond the public lot definition, and even that is checked against what the chain shows.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::{close, ingest, memo, store, wallet};
use crate::wallet::net::Net as Network;

#[derive(Deserialize)]
struct Lot {
    id: String,
    reserve_zats: u64,
    deposit_zats: u64,
    close_at: i64,
    #[serde(default = "first_price")]
    format: String,
    #[serde(default)]
    units: u32,
    #[serde(default)]
    reveal_secs: i64,
}
fn first_price() -> String {
    "first_price".into()
}

#[derive(Deserialize, Default)]
struct Published {
    #[serde(default)]
    ledger_hash: Option<String>,
    #[serde(default)]
    clearing_zats: Option<u64>,
    #[serde(default)]
    winning_txids: Vec<String>,
    #[serde(default)]
    audit: Option<PublishedAudit>,
}

#[derive(Deserialize, Default)]
struct PublishedAudit {
    #[serde(default)]
    reveals: Vec<Reveal>,
}

#[derive(Deserialize, Clone)]
struct Reveal {
    receipt: String,
    bid_zats: u64,
    salt: String,
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
    let sealed = lot.reveal_secs > 0;
    let reveals: Vec<Reveal> = published.audit.map(|x| x.reveals).unwrap_or_default();
    let mut bids: Vec<store::Bid> = Vec::new();
    let mut rejected = 0;
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
        let mined_at = out.mined_height.and_then(|h| ingest::block_time(&ro, h).ok().flatten());
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
        println!(
            "  {} {} h={:?} value={} {}{}",
            &out.txid[..16],
            status,
            out.mined_height,
            out.value_zats,
            amount.map(|v| format!("bid={v} ")).unwrap_or_default(),
            &receipt[..12]
        );
        if status != "valid" {
            rejected += 1;
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
    let valid: Vec<&store::Bid> = bids.iter().filter(|b| b.status == "valid").collect();
    println!("{} received outputs, {} bids read, {} rejected, {} valid", outputs.len(), bids.len(), rejected, valid.len());

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
    println!("\nrecomputed from the chain:");
    println!("  ledger root  {ledger_hash}");
    println!("  clearing     {clearing:?}");
    for w in &winners {
        println!("  winner       {} (receipt {})", w.txid, w.receipt.clone().unwrap_or_default());
    }

    let mut ok = true;
    if a.result.is_some() {
        let check = |name: &str, same: bool, ours: String, theirs: String| {
            println!("  {:<13}{}  chain={ours}  published={theirs}", name, if same { "MATCH" } else { "MISMATCH" });
            same
        };
        ok &= check("ledger root", published.ledger_hash.as_deref() == Some(ledger_hash.as_str()), ledger_hash.clone(), published.ledger_hash.clone().unwrap_or_default());
        ok &= check("clearing", published.clearing_zats == clearing, format!("{clearing:?}"), format!("{:?}", published.clearing_zats));
        let ours: Vec<String> = winners.iter().map(|w| w.txid.clone()).collect();
        let theirs = published.winning_txids.clone();
        ok &= check("winners", ours == theirs, ours.join(","), theirs.join(","));
        println!("\n{}", if ok { "OK: the published result is what the chain says" } else { "FAIL: the published result does not match the chain" });
    }
    Ok(ok)
}

#[allow(dead_code)]
pub fn is_dir(p: &Path) -> bool {
    p.is_dir()
}
