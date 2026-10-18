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
    /// Auditable lot: a wallet account of its own. Replies (account uuid, first address, encoded UFVK, birthday).
    NewAuditAccount {
        name: String,
        reply: oneshot::Sender<anyhow::Result<AuditAccount>>,
    },
}

#[derive(Debug, Clone)]
pub struct AuditAccount {
    pub account: String,
    pub address: String,
    pub ufvk: String,
    pub birthday: u32,
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
    pub freeze: crate::freeze::FreezeStatus,
}

pub struct AppState {
    pub network_name: &'static str,
    pub admin_token: String,
    /// Read-only token for the ledger endpoints; None falls back to the admin token.
    pub ledger_token: Option<String>,
    pub cors_origin: Option<String>,
    /// Trust the first `x-forwarded-for` entry as the client address (set behind a reverse proxy).
    pub trust_proxy: bool,
    pub freeze: crate::freeze::FreezePlan,
    /// Accept deposits off the standard menu (`denom.rs`). Off by default: an unusual deposit marks its refunds.
    pub allow_any_deposit: bool,
    pub settlement_window_secs: i64,
    pub store: Arc<Mutex<Connection>>,
    pub wallet: mpsc::Sender<WalletCmd>,
    pub status: Arc<RwLock<SyncStatus>>,
}
