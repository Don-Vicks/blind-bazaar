//! The single task that owns the wallet DB: sync, enhance, ingest, close, pay out.

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{error, info, warn};
use zcash_client_backend::data_api::WalletRead;
use zcash_proofs::prover::LocalTxProver;
use crate::wallet::net::Net as Network;

use crate::{
    close,
    config::{Config, Role},
    ingest,
    state::{SyncStatus, WalletCmd},
    store,
    wallet::{self, send::{Authority, Outgoing}},
};

const MAX_PAYOUT_ATTEMPTS: i64 = 5;

pub struct Worker {
    pub cfg: Config,
    pub params: Network,
    pub network_name: &'static str,
    pub store: Arc<Mutex<Connection>>,
    pub status: Arc<RwLock<SyncStatus>>,
    pub rx: mpsc::Receiver<WalletCmd>,
}

impl Worker {
    pub async fn run(mut self) -> Result<()> {
        let wallet_dir = self.cfg.wallet_dir();
        let mut db = wallet::open_db(self.params, &wallet_dir)?;
        let (_, data_db_path) = wallet::db_paths(&wallet_dir);
        let wallet_ro = ingest::open_wallet_readonly(&data_db_path)?;

        let mut client = wallet::connect(&self.cfg.lightwalletd_url).await?;
        let operator = match self.cfg.role {
            Role::Viewer => {
                let ufvk = self
                    .cfg
                    .operator_ufvk
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("ROLE=viewer needs OPERATOR_UFVK"))?;
                wallet::ensure_viewer_account(&mut db, &mut client, &self.params, ufvk, self.cfg.birthday_height).await?
            }
            Role::All | Role::Spender => match &self.cfg.frost_escrow {
                Some(path) => {
                    let escrow = crate::frost::Escrow::load(path)?;
                    wallet::ensure_escrow_account(&mut db, &mut client, &self.params, &escrow, self.cfg.birthday_height).await?
                }
                None => {
                    let (seed, _created) = wallet::load_or_create_seed(&self.cfg.operator_seed_file)?;
                    wallet::ensure_operator_account(&mut db, &mut client, &self.params, &seed, self.cfg.birthday_height)
                        .await?
                }
            },
        };
        let frost = match (&self.cfg.frost_escrow, self.cfg.role) {
            (Some(path), Role::All | Role::Spender) => Some(self.build_frost(path)?),
            _ => None,
        };
        if self.cfg.role == Role::Spender && self.cfg.viewer_url.is_none() {
            return Err(anyhow::anyhow!("ROLE=spender needs VIEWER_URL"));
        }
        let signing = match (&frost, operator.usk.is_some()) {
            (Some(f), _) => Some(f.describe()),
            (None, true) => Some("seed".to_string()),
            (None, false) => None,
        };
        info!("running as {:?}, signing: {}", self.cfg.role, signing.as_deref().unwrap_or("none (viewer)"));

        // Address derivation records the exposure height, so the wallet must know the chain tip first.
        let db_cache = wallet::cache::MemBlockCache::default();
        wallet::sync::sync_once(&mut client, &self.params, &db_cache, &mut db).await?;

        // A dedicated receive address for the operator (tips, Go Dark demo, funding), persisted so
        // it is stable across restarts and never confused with an auction address.
        let operator_addr = {
            let store = self.store.lock().await;
            match store::get_meta(&store, "operator_address")? {
                Some(a) => a,
                None => {
                    let (addr, _) = wallet::next_auction_address(&mut db, &self.params, operator.account_id)?;
                    store::set_meta(&store, "operator_address", &addr)?;
                    addr
                }
            }
        };
        info!("operator address: {operator_addr}");
        let (ver, chain, height) = wallet::sync::lightd_info(&mut client).await?;
        info!("lightwalletd {ver} chain={chain} height={height}");
        {
            let mut st = self.status.write().await;
            st.operator_address = Some(operator_addr);
            st.lightwalletd_version = Some(format!("{ver} ({chain})"));
            st.signing = signing;
        }

        let prover = if self.cfg.role == Role::Viewer {
            None
        } else {
            info!("loading prover parameters");
            Some(LocalTxProver::bundled())
        };

        let mut interval = tokio::time::interval(Duration::from_secs(self.cfg.sync_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                Some(cmd) = self.rx.recv() => match cmd {
                    WalletCmd::NewAddress { reply } => {
                        let r = wallet::next_auction_address(&mut db, &self.params, operator.account_id);
                        let _ = reply.send(r);
                    }
                    WalletCmd::NewReceiveAddress { reply } => {
                        let r = wallet::next_receive_address(&mut db, &self.params, operator.account_id);
                        let _ = reply.send(r);
                    }
                    WalletCmd::ScanTx { txid, reply } => {
                        let r = wallet::sync::scan_txid(&mut client, &self.params, &mut db, &txid).await;
                        let _ = reply.send(r);
                    }
                    WalletCmd::InspectBlock { height, reply } => {
                        let r = wallet::sync::inspect_block(&mut client, &self.params, height).await;
                        let _ = reply.send(r);
                    }
                },
                _ = interval.tick() => {
                    if let Err(e) = self.pass(&mut client, &db_cache, &mut db, &wallet_ro, prover.as_ref(), &operator, frost.as_ref()).await {
                        error!("pass failed: {e:#}");
                        self.status.write().await.last_error = Some(format!("{e:#}"));
                        // Reconnect on the next tick in case the channel died.
                        match wallet::connect(&self.cfg.lightwalletd_url).await {
                            Ok(c) => client = c,
                            Err(e) => warn!("reconnect failed: {e:#}"),
                        }
                    }
                }
            }
        }
    }

    async fn pass(
        &self,
        client: &mut wallet::Client,
        db_cache: &wallet::cache::MemBlockCache,
        db: &mut wallet::Db,
        wallet_ro: &Connection,
        prover: Option<&LocalTxProver>,
        operator: &wallet::Operator,
        frost: Option<&crate::frost::FrostSpender>,
    ) -> Result<()> {
        let memo_key = if self.cfg.disclose_winning_memo { operator.ufvk.clone() } else { None };
        if self.cfg.role == Role::Spender {
            self.mirror_auctions().await?;
        }
        wallet::sync::sync_once(client, &self.params, db_cache, db).await?;
        let enhanced = wallet::sync::enhance(client, &self.params, db).await?;
        if enhanced > 0 {
            info!("enhanced {enhanced} transactions");
        }
        let mempool = match wallet::sync::scan_mempool(client, &self.params, db).await {
            Ok(n) => n,
            Err(e) => {
                warn!("mempool scan failed: {e:#}");
                0
            }
        };

        let tip = wallet::chain_tip(client).await.ok();
        let height = wallet::wallet_height(db)?;
        let balance = db
            .get_wallet_summary(wallet::send::confirmations_policy(self.cfg.min_confirmations))
            .ok()
            .flatten()
            .and_then(|s| {
                s.account_balances()
                    .get(&operator.account_id)
                    .map(|b| u64::from(b.total()))
            });
        {
            let mut s = self.status.write().await;
            s.chain_tip = tip;
            s.wallet_height = height;
            s.last_sync_at = Some(store::now());
            s.last_error = None;
            s.balance_zats = balance;
            s.mempool_seen = mempool;
        }

        {
            let store = self.store.lock().await;
            let n = ingest::ingest(&store, wallet_ro)?;
            if n > 0 {
                info!("ingested {n} new outputs");
            }
            let close_cfg = close::CloseConfig {
                delay_min: self.cfg.refund_delay_min_secs,
                delay_max: self.cfg.refund_delay_max_secs,
                settlement_window_secs: self.cfg.settlement_window_secs,
            };
            close::close_due(&store, Some(wallet_ro), &self.params, self.network_name, &close_cfg, memo_key.as_ref())?;
            close::enforce_settlement_deadlines(&store, &close_cfg)?;
        }

        let auth = match (operator.usk.as_ref(), frost) {
            (Some(usk), _) => Some(Authority::Seed(usk)),
            (None, Some(f)) => Some(Authority::Frost(f)),
            (None, None) => None,
        };
        if let (Some(prover), Some(auth)) = (prover, auth) {
            // Anything a rail delivered to a transparent receiver gets shielded before the auction logic
            // sees it. The escrow key is Orchard-only, so nothing can land unshielded there.
            let unshielded = db
                .get_wallet_summary(wallet::send::confirmations_policy(self.cfg.min_confirmations))
                .ok()
                .flatten()
                .and_then(|s| s.account_balances().get(&operator.account_id).map(|b| u64::from(b.unshielded())))
                .unwrap_or(0);
            if let (true, Some(usk)) = (unshielded > 0, operator.usk.as_ref()) {
                match wallet::send::shield_transparent(db, &self.params, client, prover, operator.account_id, usk, self.cfg.min_confirmations).await {
                    Ok(txids) if !txids.is_empty() => {
                        let store = self.store.lock().await;
                        store::insert_event(&store, None, "shielded", &serde_json::json!({ "txids": txids, "value_zats": unshielded }))?;
                    }
                    Ok(_) => {}
                    Err(e) => warn!("auto-shield failed: {e:#}"),
                }
            }
            self.process_payouts(client, db, prover, operator.account_id, &auth).await?;
        }
        if self.cfg.role == Role::Spender {
            self.report_payouts().await?;
        }
        self.deliver_events().await?;
        {
            let store = self.store.lock().await;
            let t = store::totals(&store)?;
            let mut s = self.status.write().await;
            s.pending_payouts_zats = t.payouts_pending_zats;
            s.dead_payouts = t.payouts_dead;
            s.low_balance = match s.balance_zats {
                Some(b) => b < t.payouts_pending_zats + self.cfg.fee_reserve_zats,
                None => false,
            };
        }
        Ok(())
    }

    /// Assemble the escrow coordinator from config: HTTP signer hosts, plus any in-process shares.
    fn build_frost(&self, escrow_path: &std::path::Path) -> Result<crate::frost::FrostSpender> {
        use crate::frost::{Escrow, FrostSpender, HttpSigner, LocalSigner, SignerHost, SignerPolicy, SignerTransport};
        let escrow = Escrow::load(escrow_path)?;
        let mut signers: Vec<Box<dyn SignerTransport>> = Vec::new();
        for url in &self.cfg.frost_signers {
            if !url.trim().is_empty() {
                signers.push(Box::new(HttpSigner::new(url.trim(), self.cfg.frost_token.clone())?));
            }
        }
        for path in &self.cfg.frost_signer_shares {
            warn!("running signer share {} in-process: no separation from the spender (development only)", path.display());
            let host = SignerHost::load(path, &escrow, self.params, SignerPolicy::default())?;
            signers.push(Box::new(LocalSigner { host: Arc::new(host), plan: None }));
        }
        FrostSpender::new(escrow, signers)
    }

    async fn deliver_events(&self) -> Result<()> {
        let Some(url) = &self.cfg.webhook_url else { return Ok(()) };
        let pending = {
            let store = self.store.lock().await;
            store::undelivered_events(&store, 8)?
        };
        if pending.is_empty() {
            return Ok(());
        }
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        for ev in pending {
            let body = serde_json::json!({
                "id": ev.id, "kind": ev.kind, "auction_id": ev.auction_id,
                "payload": ev.payload, "created_at": ev.created_at,
            });
            let ok = client.post(url).json(&body).send().await.map(|r| r.status().is_success()).unwrap_or(false);
            let store = self.store.lock().await;
            store::mark_event_delivered(&store, ev.id, ok)?;
            if !ok {
                warn!("webhook delivery failed for event {}", ev.id);
            }
        }
        Ok(())
    }

    /// Spender: adopt auction definitions from the viewer (first sight wins; never updated).
    async fn mirror_auctions(&self) -> Result<()> {
        let base = self.cfg.viewer_url.as_deref().unwrap_or_default();
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        let auctions: Vec<store::Auction> = client
            .get(format!("{base}/admin/auctions"))
            .header("x-admin-token", &self.cfg.admin_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let store = self.store.lock().await;
        for a in &auctions {
            if store::mirror_auction(&store, a)? {
                info!("mirrored auction {} ({})", a.id, a.title);
            }
        }
        Ok(())
    }

    /// Spender: tell the viewer which payouts went out so its public state matches.
    async fn report_payouts(&self) -> Result<()> {
        let base = self.cfg.viewer_url.as_deref().unwrap_or_default();
        let (after, batch) = {
            let store = self.store.lock().await;
            let after: i64 = store::get_meta(&store, "reported_payout_id")?.and_then(|v| v.parse().ok()).unwrap_or(0);
            (after, store::sent_payouts_after(&store, after)?)
        };
        if batch.is_empty() {
            return Ok(());
        }
        let body: Vec<serde_json::Value> = batch
            .iter()
            .map(|(p, bid_txid)| {
                serde_json::json!({
                    "auction_id": p.auction_id, "kind": p.kind, "bid_txid": bid_txid,
                    "to_address": p.to_address, "value_zats": p.value_zats, "txid": p.txid,
                })
            })
            .collect();
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        client
            .post(format!("{base}/admin/payouts/sync"))
            .header("x-admin-token", &self.cfg.admin_token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let last = batch.iter().map(|(p, _)| p.id).max().unwrap_or(after);
        let store = self.store.lock().await;
        store::set_meta(&store, "reported_payout_id", &last.to_string())?;
        Ok(())
    }

    async fn process_payouts(
        &self,
        client: &mut wallet::Client,
        db: &mut wallet::Db,
        prover: &LocalTxProver,
        account_id: zcash_client_sqlite::AccountUuid,
        auth: &Authority<'_>,
    ) -> Result<()> {
        let due = {
            let store = self.store.lock().await;
            store::due_payouts(&store, store::now(), MAX_PAYOUT_ATTEMPTS)?
        };
        for p in due {
            info!("sending payout {} ({}) of {} zats", p.id, p.kind, p.value_zats);
            let out = Outgoing {
                address: &p.to_address,
                zats: p.value_zats,
                memo: p.memo.as_deref(),
            };
            let result = wallet::send::send(
                db,
                &self.params,
                client,
                prover,
                account_id,
                auth,
                &out,
                self.cfg.min_confirmations,
            )
            .await;
            let store = self.store.lock().await;
            match result {
                Ok(sent) => {
                    let txid = sent.txid;
                    store::mark_payout_sent(&store, p.id, &txid)?;
                    if p.kind == "seller" {
                        if let Some(aid) = &p.auction_id {
                            if store::pending_seller_payouts(&store, aid)? == 0 {
                                if let Some(a) = store::get_auction(&store, aid)? {
                                    if a.status == "settled" {
                                        store::set_auction_payout(&store, aid, &txid)?;
                                    }
                                }
                            }
                        }
                    }
                    store::insert_event(
                        &store,
                        p.auction_id.as_deref(),
                        match p.kind.as_str() { "seller" => "seller_paid", "forfeit" => "forfeit_paid", _ => "refund_sent" },
                        &serde_json::json!({ "payout_id": p.id, "txid": txid, "value_zats": p.value_zats, "cosigners": sent.cosigners }),
                    )?;
                }
                Err(e) => {
                    warn!("payout {} failed: {e:#}", p.id);
                    if p.attempts + 1 >= MAX_PAYOUT_ATTEMPTS {
                        store::mark_payout_dead(&store, p.id, &format!("{e:#}"))?;
                        store::insert_event(
                            &store,
                            p.auction_id.as_deref(),
                            "payout_dead",
                            &serde_json::json!({ "payout_id": p.id, "error": format!("{e:#}") }),
                        )?;
                    } else {
                        let backoff = 60 * (1 << p.attempts.min(5));
                        store::mark_payout_failed(&store, p.id, &format!("{e:#}"), store::now() + backoff)?;
                    }
                }
            }
        }
        Ok(())
    }
}
