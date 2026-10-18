use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};

pub enum WalletCmd {
    NewAddress {
        reply: oneshot::Sender<anyhow::Result<(String, String)>>,
    },
    /// Fresh Go Dark receive address: unified address plus its transparent receiver.
    NewReceiveAddress {
        reply: oneshot::Sender<anyhow::Result<(String, Option<String>)>>,
    },
    /// Fetch a transaction by display txid and trial-decrypt it (debug aid; mined or mempool).
    ScanTx {
        txid: String,
        reply: oneshot::Sender<anyhow::Result<String>>,
    },
    InspectBlock {
        height: u32,
        reply: oneshot::Sender<anyhow::Result<serde_json::Value>>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SyncStatus {
    pub chain_tip: Option<u32>,
    pub wallet_height: Option<u32>,
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
    pub operator_address: Option<String>,
    pub balance_zats: Option<u64>,
    pub lightwalletd_version: Option<String>,
    pub mempool_seen: usize,
    pub pending_payouts_zats: u64,
    pub dead_payouts: i64,
    pub low_balance: bool,
    /// "seed" or "frost 2-of-3 (...)"; None for a viewer.
    pub signing: Option<String>,
}

pub struct AppState {
    pub network_name: &'static str,
    pub admin_token: String,
    pub cors_origin: Option<String>,
    /// Trust the first `x-forwarded-for` entry as the client address (set behind a reverse proxy).
    pub trust_proxy: bool,
    pub store: Arc<Mutex<Connection>>,
    pub wallet: mpsc::Sender<WalletCmd>,
    pub status: Arc<RwLock<SyncStatus>>,
}
