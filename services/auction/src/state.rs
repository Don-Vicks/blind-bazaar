use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};

pub enum WalletCmd {
    NewAddress {
        reply: oneshot::Sender<anyhow::Result<(String, String)>>,
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
}

pub struct AppState {
    pub network_name: &'static str,
    pub admin_token: String,
    pub store: Arc<Mutex<Connection>>,
    pub wallet: mpsc::Sender<WalletCmd>,
    pub status: Arc<RwLock<SyncStatus>>,
}
