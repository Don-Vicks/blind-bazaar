mod api;
mod close;
mod config;
mod disclosure;
mod frost;
mod audit;
mod linkability;
mod notify;
mod ratelimit;
mod ingest;
mod memo;
mod merkle;
mod state;
mod store;
mod wallet;
mod worker;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::info;
use crate::wallet::net::Net as Network;

use crate::config::Config;

/// `bb-auction --verify-disclosure result.json`: fetch the named transaction from lightwalletd and
/// check that the disclosed note opening matches the on-chain commitment. Needs no wallet.
async fn verify_disclosure_file(cfg: &Config, path: &std::path::Path) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let list: Vec<disclosure::Disclosure> = match serde_json::from_str::<serde_json::Value>(&text)? {
        serde_json::Value::Array(_) => serde_json::from_str(&text)?,
        _ => vec![serde_json::from_str(&text)?],
    };
    let mut failures = 0;
    for d in &list {
        if !verify_one(cfg, d).await? {
            failures += 1;
        }
    }
    if failures > 0 {
        println!("FAIL: {failures} of {} disclosures did not match", list.len());
        std::process::exit(2);
    }
    println!("OK: all {} disclosure(s) match the chain", list.len());
    Ok(())
}

async fn verify_one(cfg: &Config, d: &disclosure::Disclosure) -> anyhow::Result<bool> {
    use zcash_client_backend::proto::service;
    let params = match d.network.as_str() {
        "main" => Network::Main,
        "regtest" => cfg.net(),
        _ => Network::Test,
    };
    let mut client = wallet::connect(&cfg.lightwalletd_url).await?;
    let mut hash = hex::decode(&d.txid)?;
    hash.reverse();
    let raw = client
        .get_transaction(service::TxFilter { hash, ..Default::default() })
        .await?
        .into_inner();
    let mined = (raw.height > 0).then_some(raw.height as u32);
    let tx = disclosure::read_tx(&params, &raw.data, mined)?;
    let cmx = hex::encode(disclosure::compute_cmx(d)?);
    match disclosure::verify_against_tx(d, &tx)? {
        Some(pool) => {
            println!("ok   action {} of {} ({pool} pool): {} zatoshis to {}", d.action_index, d.txid, d.value_zats, &d.recipient_address[..20]);
            println!("     cmx {cmx}");
            if let Some(k) = &d.enc_key {
                let key: [u8; 32] = hex::decode(k)?.try_into().map_err(|_| anyhow::anyhow!("enc_key must be 32 bytes"))?;
                let note = disclosure::decrypt_note_with_key(&tx, d.action_index, pool, &key)?;
                let bid = disclosure::memo_bid_matches(d, &note)?;
                match (bid, d.reserve_zats) {
                    (Some(b), Some(r)) if d.reserve_rule.as_deref() == Some("at_most") => println!("     memo decrypted: quote {b} zatoshis, budget {r}: {}", if b <= r { "within budget" } else { "OVER BUDGET" }),
                    (Some(b), Some(r)) => println!("     memo decrypted: bid {b} zatoshis, reserve {r}: {}", if b >= r { "reserve met" } else { "BELOW RESERVE" }),
                    (Some(b), None) => println!("     memo decrypted: bid {b} zatoshis"),
                    _ => println!("     memo decrypted but carries no BB1 bid"),
                }
            }
            Ok(true)
        }
        None => {
            println!("FAIL action {} of {}: computed cmx {cmx} does not match", d.action_index, d.txid);
            Ok(false)
        }
    }
}

/// Standalone test wallet: `bb-auction wallet --dir data/bidder1 --seed-file data/bidder1.seed [--pay zcash:...]`.
async fn wallet_cmd(
    cfg: &Config,
    dir: std::path::PathBuf,
    seed_file: std::path::PathBuf,
    pay: Option<String>,
    wait_balance: Option<u64>,
) -> anyhow::Result<()> {
    use zcash_client_backend::data_api::WalletRead;
    let params = cfg.net();
    let mut db = wallet::open_db(params, &dir)?;
    let cache = wallet::cache::MemBlockCache::default();
    let mut client = wallet::connect(&cfg.lightwalletd_url).await?;
    let (ver, chain, height) = wallet::sync::lightd_info(&mut client).await?;
    println!("server: {ver} chain={chain} height={height}");
    let (seed, _) = wallet::load_or_create_seed(&seed_file)?;
    let me = wallet::ensure_operator_account(&mut db, &mut client, &params, &seed, cfg.birthday_height).await?;
    let address = match db.get_last_generated_address_matching(me.account_id, wallet::orchard_only_request())? {
        Some(ua) => ua.encode(&params),
        None => wallet::next_auction_address(&mut db, &params, me.account_id)?.0,
    };
    println!("address: {address}");
    let policy = wallet::send::confirmations_policy(cfg.min_confirmations);
    loop {
        wallet::sync::sync_once(&mut client, &params, &cache, &mut db).await?;
        wallet::sync::enhance(&mut client, &params, &mut db).await?;
        let summary = db.get_wallet_summary(policy)?;
        let (total, spendable) = summary
            .as_ref()
            .and_then(|s| s.account_balances().get(&me.account_id))
            .map(|b| (u64::from(b.total()), u64::from(b.spendable_value())))
            .unwrap_or((0, 0));
        println!(
            "height: {:?}  balance: {} zats (spendable {})",
            db.chain_height()?.map(u32::from),
            total,
            spendable
        );
        match wait_balance {
            Some(want) if spendable < want => {
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                continue;
            }
            _ => break,
        }
    }
    if let Some(uri) = pay {
        let request = zip321::TransactionRequest::from_uri(&uri).map_err(|e| anyhow::anyhow!("bad URI: {e:?}"))?;
        info!("paying {} payment(s) from {uri}", request.payments().len());
        let prover = zcash_proofs::prover::LocalTxProver::bundled();
        let usk = me.usk.as_ref().expect("wallet subcommand always has a seed");
        let sent = wallet::send::send_request(
            &mut db,
            &params,
            &mut client,
            &prover,
            me.account_id,
            &wallet::send::Authority::Seed(usk),
            request,
            cfg.min_confirmations,
        )
        .await?;
        println!("txid: {}", sent.txid);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zcash_client_backend=warn".into()),
        )
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls crypto provider");

    let cfg = Config::parse();
    if let Some(path) = &cfg.verify_disclosure {
        return verify_disclosure_file(&cfg, path).await;
    }
    if let Some(config::Cmd::Frost { cmd }) = cfg.cmd.clone() {
        match cmd {
            config::FrostCmd::Keygen { max, min, out } => {
                let e = frost::keygen(max, min, &out)?;
                let (ufvk, ua) = frost::show(&e, &cfg.net())?;
                println!("escrow {}-of-{} written to {}", e.threshold, e.signers, out.display());
                println!("ak    {}", e.ak);
                println!("ufvk  {ufvk}");
                println!("first address {ua}");
                println!("keep signer-*.json on separate hosts; share escrow.json with the viewer");
            }
            config::FrostCmd::Show { escrow } => {
                let e = frost::Escrow::load(&escrow)?;
                let (ufvk, ua) = frost::show(&e, &cfg.net())?;
                println!("ufvk  {ufvk}");
                println!("first address {ua}");
            }
            config::FrostCmd::Dkg { step } => {
                use config::DkgStep::*;
                use frost::dkg_ceremony as c;
                match step {
                    Round1 { id, max, min, dir } => c::round1(id, max, min, &dir)?,
                    Round2 { id, dir } => c::round2(id, &dir)?,
                    Finish { id, dir } => c::finish(id, &dir)?,
                    Escrow { id, dir } => {
                        let e = c::escrow(id, &dir)?;
                        let (ufvk, ua) = frost::show(&e, &cfg.net())?;
                        println!("escrow {}-of-{} written to {}", e.threshold, e.signers, dir.join("escrow.json").display());
                        println!("ak    {}", e.ak);
                        println!("ufvk  {ufvk}");
                        println!("first address {ua}");
                    }
                }
            }
            config::FrostCmd::Signer { share, escrow, bind, token, max_external_zats, ledger_url, ledger_token } => {
                let e = frost::Escrow::load(&escrow)?;
                let mut host = frost::SignerHost::load(&share, &e, cfg.net(), frost::SignerPolicy { max_external_zats, ..frost::SignerPolicy::default() })?;
                match (ledger_url, ledger_token) {
                    (Some(url), Some(tok)) => {
                        host = host.with_ledger(frost::LedgerClient::new(&url, &tok)?);
                        info!("ledger check on: every output must match a payout from {url}");
                    }
                    (Some(_), None) => anyhow::bail!("FROST_LEDGER_URL needs FROST_LEDGER_TOKEN"),
                    _ => tracing::warn!("ledger check off: this signer enforces only the value and fee caps"),
                }
                info!("frost signer {} for group ak {}", host.identifier(), e.ak);
                frost::run_signer(std::sync::Arc::new(host), &bind, token).await?;
            }
        }
        return Ok(());
    }
    if let Some(config::Cmd::Audit { ufvk, birthday, lot, result, dir }) = cfg.cmd.clone() {
        let ok = audit::run(cfg.net(), &cfg.lightwalletd_url, &audit::AuditArgs { ufvk, birthday, lot, result, dir }).await?;
        if !ok {
            std::process::exit(2);
        }
        return Ok(());
    }
    if let Some(config::Cmd::SimulateRefunds { losers_per_lot, delay_min, delay_max, window_secs, batch_max, seed }) = cfg.cmd.clone() {
        println!("delay {delay_min}-{delay_max}s, window {window_secs}s, batch max {batch_max}, {losers_per_lot} losers per lot, 48 h, seed {seed}\n");
        print!("{}", linkability::table(seed, losers_per_lot, delay_min, delay_max, window_secs, batch_max));
        return Ok(());
    }
    if let Some(config::Cmd::Derive { seed_file, count, check, ufvk: print_ufvk }) = cfg.cmd.clone() {
        use secrecy::ExposeSecret;
        use zcash_keys::keys::UnifiedSpendingKey;
        let params = cfg.net();
        let (seed, _) = wallet::load_or_create_seed(&seed_file)?;
        let usk = UnifiedSpendingKey::from_seed(&params, seed.expose_secret(), zip32::AccountId::ZERO)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let ufvk = usk.to_unified_full_viewing_key();
        if print_ufvk {
            println!("ufvk: {}", ufvk.encode(&params).map_err(|e| anyhow::anyhow!("{e:?}"))?);
        }
        let mut j = zip32::DiversifierIndex::new();
        for _ in 0..count {
            let (ua, idx) = ufvk
                .find_address(j, wallet::orchard_only_request())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            println!("{} {}", hex::encode(idx.as_bytes()), ua.encode(&params));
            j = idx;
            j.increment().map_err(|_| anyhow::anyhow!("diversifier overflow"))?;
        }
        if !check.is_empty() {
            use zcash_address::unified::{Address as UA, Container, Encoding, Receiver};
            use orchard::keys::Scope;
            let ivk = ufvk
                .orchard()
                .ok_or_else(|| anyhow::anyhow!("no orchard key"))?
                .to_ivk(Scope::External);
            for ua in check {
                let (_, _, decoded) = UA::decode(&ua).map_err(|e| anyhow::anyhow!("{e}"))?;
                let recv = decoded.items().into_iter().find_map(|r| match r {
                    Receiver::Orchard(b) => Some(b),
                    _ => None,
                });
                let owned = recv
                    .and_then(|b| Option::from(orchard::Address::from_raw_address_bytes(&b)))
                    .and_then(|a| ivk.diversifier_index(&a));
                println!(
                    "{}… owned_by_this_key={}",
                    &ua[..14],
                    owned.map(|i| hex::encode(i.as_bytes())).unwrap_or_else(|| "NO".into())
                );
            }
        }
        return Ok(());
    }
    if let Some(config::Cmd::Wallet { dir, seed_file, pay, wait_balance }) = cfg.cmd.clone() {
        return wallet_cmd(&cfg, dir, seed_file, pay, wait_balance).await;
    }
    let params = cfg.net();
    let network_name = cfg.network_name();
    if cfg.admin_token.trim().len() < 8 {
        anyhow::bail!("ADMIN_TOKEN must be set to at least 8 characters; an empty token would let anyone queue payouts");
    }
    std::fs::create_dir_all(&cfg.data_dir)?;

    let store = Arc::new(Mutex::new(store::open(&cfg.store_path())?));
    let status = Arc::new(RwLock::new(state::SyncStatus::default()));
    let (tx, rx) = mpsc::channel(16);

    let worker = worker::Worker {
        cfg: cfg.clone(),
        params,
        network_name,
        store: store.clone(),
        status: status.clone(),
        rx,
    };
    // The wallet DB and SQLite connections are !Sync, so the worker gets its own OS thread
    // with a current-thread runtime. This also keeps proving work off the API runtime.
    std::thread::Builder::new()
        .name("wallet-worker".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("worker runtime");
            if let Err(e) = rt.block_on(worker.run()) {
                tracing::error!("wallet worker exited: {e:#}");
                std::process::exit(1);
            }
        })?;

    let app_state = Arc::new(state::AppState {
        network_name,
        admin_token: cfg.admin_token.clone(),
        ledger_token: cfg.ledger_token.clone(),
        cors_origin: cfg.cors_origin.clone(),
        trust_proxy: cfg.trust_proxy,
        store,
        wallet: tx,
        status,
    });

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    info!("bb-auction listening on {} ({network_name})", cfg.bind);
    axum::serve(
        listener,
        api::router(app_state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
