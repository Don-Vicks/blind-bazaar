mod api;
mod close;
mod config;
mod disclosure;
mod ingest;
mod memo;
mod state;
mod store;
mod wallet;
mod worker;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::info;
use zcash_protocol::consensus::Network;

use crate::config::{Config, NetworkName};

/// `bb-auction --verify-disclosure result.json`: fetch the named transaction from lightwalletd and
/// check that the disclosed note opening matches the on-chain commitment. Needs no wallet.
async fn verify_disclosure_file(cfg: &Config, path: &std::path::Path) -> anyhow::Result<()> {
    use zcash_client_backend::proto::service;
    let d: disclosure::Disclosure = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let params = match d.network.as_str() {
        "main" => Network::MainNetwork,
        _ => Network::TestNetwork,
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
    let cmx = hex::encode(disclosure::compute_cmx(&d)?);
    match disclosure::verify_against_tx(&d, &tx)? {
        Some(pool) => {
            println!("OK: note opening matches action {} of {} in the {pool} pool", d.action_index, d.txid);
            println!("    recipient {}", d.recipient_address);
            println!("    value     {} zatoshis", d.value_zats);
            println!("    cmx       {cmx}");
            Ok(())
        }
        None => {
            println!("FAIL: computed cmx {cmx} does not match any action at index {}", d.action_index);
            std::process::exit(2);
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
    let params = match cfg.network {
        NetworkName::Test => Network::TestNetwork,
        NetworkName::Main => Network::MainNetwork,
    };
    let mut db = wallet::open_db(params, &dir)?;
    let cache = wallet::cache::MemBlockCache::default();
    let mut client = wallet::connect(&cfg.lightwalletd_url).await?;
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
        let txid = wallet::send::send_request(
            &mut db,
            &params,
            &mut client,
            &prover,
            me.account_id,
            &me.usk,
            request,
            cfg.min_confirmations,
        )
        .await?;
        println!("txid: {txid}");
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
    if let Some(config::Cmd::Wallet { dir, seed_file, pay, wait_balance }) = cfg.cmd.clone() {
        return wallet_cmd(&cfg, dir, seed_file, pay, wait_balance).await;
    }
    let (params, network_name) = match cfg.network {
        NetworkName::Test => (Network::TestNetwork, "test"),
        NetworkName::Main => (Network::MainNetwork, "main"),
    };
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
        store,
        wallet: tx,
        status,
    });

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    info!("bb-auction listening on {} ({network_name})", cfg.bind);
    axum::serve(listener, api::router(app_state)).await?;
    Ok(())
}
