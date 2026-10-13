mod api;
mod close;
mod config;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zcash_client_backend=warn".into()),
        )
        .init();

    let cfg = Config::parse();
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
        store: store.clone(),
        status: status.clone(),
        rx,
    };
    tokio::spawn(async move {
        if let Err(e) = worker.run().await {
            tracing::error!("wallet worker exited: {e:#}");
            std::process::exit(1);
        }
    });

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
