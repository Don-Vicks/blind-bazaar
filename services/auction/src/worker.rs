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
/// Upper bound on one sync/ingest/payout pass. A stalled lightwalletd stream otherwise blocks the worker forever.
const PASS_TIMEOUT: Duration = Duration::from_secs(300);

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
        let mut seed: Option<secrecy::SecretVec<u8>> = None;
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
                    let (s, _created) = wallet::load_or_create_seed(&self.cfg.operator_seed_file)?;
                    let op = wallet::ensure_operator_account(&mut db, &mut client, &self.params, &s, self.cfg.birthday_height).await?;
                    seed = Some(s);
                    op
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
        // Every network step is bounded: a lightwalletd call that never answers must not hang the worker.
        let db_cache = wallet::cache::MemBlockCache::default();
        tokio::time::timeout(PASS_TIMEOUT, wallet::sync::sync_once(&mut client, &self.params, &db_cache, &mut db))
            .await
            .map_err(|_| anyhow::anyhow!("initial sync timed out after {}s", PASS_TIMEOUT.as_secs()))??;

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
                    WalletCmd::NewAuditAccount { name, reply } => {
                        let r = match &seed {
                            Some(s) => wallet::create_lot_account(&mut db, &mut client, &self.params, s, &name).await,
                            None => Err(anyhow::anyhow!("auditable lots need the seed-backed operator; this instance runs a viewer or escrow account")),
                        };
                        let _ = reply.send(r);
                    }
                },
                _ = interval.tick() => {
                    let outcome = tokio::time::timeout(
                        PASS_TIMEOUT,
                        self.pass(&mut client, &db_cache, &mut db, &wallet_ro, prover.as_ref(), &operator, frost.as_ref(), seed.as_ref()),
                    )
                    .await
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("pass timed out after {}s; reconnecting", PASS_TIMEOUT.as_secs())));
                    if let Err(e) = outcome {
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
        seed: Option<&secrecy::SecretVec<u8>>,
    ) -> Result<()> {
        let memo_key = if self.cfg.disclose_winning_memo { operator.ufvk.clone() } else { None };
        if self.cfg.role == Role::Spender {
            self.mirror_auctions().await?;
            self.mirror_envelopes().await?;
            self.mirror_reveals().await?;
            self.mirror_manual_payouts().await?;
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
            let reorg = ingest::reorg_check(&store, wallet_ro)?;
            if !reorg.reorged.is_empty() || !reorg.reinstated.is_empty() {
                warn!("reorg: {} bid(s) set aside, {} reinstated, {} after close", reorg.reorged.len(), reorg.reinstated.len(), reorg.after_close.len());
            }
            let n = ingest::ingest(&store, wallet_ro, height, self.cfg.ingest_confirmations)?;
            if n > 0 {
                info!("ingested {n} new outputs");
            }
            let close_cfg = close::CloseConfig {
                delay_min: self.cfg.refund_delay_min_secs,
                delay_max: self.cfg.refund_delay_max_secs,
                refund_window_secs: self.cfg.refund_window_secs,
                settlement_window_secs: self.cfg.settlement_window_secs,
            };
            close::close_due(&store, Some(wallet_ro), &self.params, self.network_name, &close_cfg, memo_key.as_ref())?;
            close::finalize_reveals(&store, Some(wallet_ro), &self.params, self.network_name, &close_cfg, memo_key.as_ref())?;
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
            if let Some(seed) = seed {
                self.process_sweeps(client, db, prover, seed).await?;
            }
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

    /// Auditable lots keep deposits in their own account; once a lot closes (and again after settlement) the
    /// whole balance is swept into the operator pool, from which refunds and payouts are paid as usual. The
    /// sweep is the only spend an auditor sees from the lot's published viewing key.
    async fn process_sweeps(
        &self,
        client: &mut wallet::Client,
        db: &mut wallet::Db,
        prover: &LocalTxProver,
        seed: &secrecy::SecretVec<u8>,
    ) -> Result<()> {
        use secrecy::ExposeSecret;
        use zcash_client_backend::data_api::{Account, WalletRead};
        let due: Vec<store::Payout> = {
            let store = self.store.lock().await;
            store::due_payouts(&store, store::now(), MAX_PAYOUT_ATTEMPTS)?.into_iter().filter(|p| p.from_account.is_some()).collect()
        };
        for p in due {
            let uuid = p.from_account.clone().unwrap_or_default();
            let account_id = match uuid.parse::<uuid::Uuid>() {
                Ok(u) => zcash_client_sqlite::AccountUuid::from_uuid(u),
                Err(_) => {
                    let store = self.store.lock().await;
                    store::mark_payout_dead(&store, p.id, "bad from_account")?;
                    continue;
                }
            };
            let policy = wallet::send::confirmations_policy(self.cfg.min_confirmations);
            let spendable = db
                .get_wallet_summary(policy)?
                .and_then(|s| s.account_balances().get(&account_id).map(|b| u64::from(b.spendable_value())))
                .unwrap_or(0);
            if spendable < 20_000 {
                // Nothing (or only dust) to move yet; look again next pass without burning an attempt.
                continue;
            }
            let derivation = match db.get_account(account_id)?.and_then(|a| a.source().key_derivation().cloned()) {
                Some(d) => d,
                None => {
                    let store = self.store.lock().await;
                    store::mark_payout_dead(&store, p.id, "lot account has no derivation")?;
                    continue;
                }
            };
            let usk = zcash_keys::keys::UnifiedSpendingKey::from_seed(&self.params, seed.expose_secret(), derivation.account_index())
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            // Send everything minus the fee. ZIP 317 charges per logical action; try the cheapest estimate first
            // and step down if the proposal says the balance does not cover it.
            let mut result = Err(anyhow::anyhow!("no attempt"));
            let mut amount = spendable.saturating_sub(10_000);
            for _ in 0..12 {
                if amount < 10_000 {
                    break;
                }
                let out = Outgoing { address: &p.to_address, zats: amount, memo: None };
                result = wallet::send::send(db, &self.params, client, prover, account_id, &Authority::Seed(&usk), &out, self.cfg.min_confirmations).await;
                match &result {
                    Err(e) if format!("{e:#}").contains("InsufficientFunds") || format!("{e:#}").contains("Insufficient") => amount = amount.saturating_sub(5_000),
                    _ => break,
                }
            }
            let store = self.store.lock().await;
            match result {
                Ok(sent) => {
                    info!("swept {amount} zats from lot account {uuid} in {}", sent.txid);
                    store::mark_payout_sent(&store, p.id, &sent.txid)?;
                    store::insert_event(&store, p.auction_id.as_deref(), "swept", &serde_json::json!({ "txid": sent.txid, "value_zats": amount }))?;
                }
                Err(e) => {
                    warn!("sweep {} failed: {e:#}", p.id);
                    let backoff = 60 * (1 << p.attempts.min(5));
                    store::mark_payout_failed(&store, p.id, &format!("{e:#}"), store::now() + backoff)?;
                }
            }
        }
        Ok(())
    }

    /// Assemble the escrow coordinator from config: HTTP signer hosts, plus any in-process shares.
    fn build_frost(&self, escrow_path: &std::path::Path) -> Result<crate::frost::FrostSpender> {
        use crate::frost::{Escrow, FrostSpender, HttpSigner, LocalSigner, SignerHost, SignerPolicy, SignerTransport};
        let escrow = Escrow::load(escrow_path)?;
        let mut signers: Vec<Box<dyn SignerTransport>> = Vec::new();
        for entry in &self.cfg.frost_signers {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            // `url|token` gives each signer its own credential; a bare url falls back to FROST_TOKEN.
            let (url, token) = match entry.split_once('|') {
                Some((u, t)) => (u.trim(), Some(t.trim().to_string())),
                None => (entry, self.cfg.frost_token.clone()),
            };
            signers.push(Box::new(HttpSigner::new(url, token)?));
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

    /// Spender: adopt sealed envelopes from the viewer so rail deliveries to their addresses become bids here too.
    async fn mirror_envelopes(&self) -> Result<()> {
        let base = self.cfg.viewer_url.as_deref().unwrap_or_default();
        let after: i64 = {
            let store = self.store.lock().await;
            store::get_meta(&store, "mirrored_envelope_at")?.and_then(|v| v.parse().ok()).unwrap_or(0)
        };
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        let envelopes: Vec<store::Envelope> = client
            .get(format!("{base}/admin/envelopes?after={after}"))
            .header("x-admin-token", &self.cfg.admin_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if envelopes.is_empty() {
            return Ok(());
        }
        let store = self.store.lock().await;
        let mut last = after;
        for e in &envelopes {
            store::insert_envelope(&store, &store::Envelope { matched_bid_id: None, ..e.clone() })?;
            last = last.max(e.created_at);
        }
        store::set_meta(&store, "mirrored_envelope_at", &last.to_string())?;
        info!("mirrored {} envelope(s)", envelopes.len());
        Ok(())
    }

    /// Spender: adopt operator-initiated (manual) payouts queued on the viewer, so they exist in the ledger the
    /// signers check before the spender tries to pay them.
    async fn mirror_manual_payouts(&self) -> Result<()> {
        let base = self.cfg.viewer_url.as_deref().unwrap_or_default();
        let after: i64 = {
            let store = self.store.lock().await;
            store::get_meta(&store, "mirrored_manual_payout_id")?.and_then(|v| v.parse().ok()).unwrap_or(0)
        };
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        let list: Vec<crate::frost::ExpectedPayout> = client
            .get(format!("{base}/ledger/payouts?after={after}&kind=manual"))
            .header("x-admin-token", &self.cfg.admin_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if list.is_empty() {
            return Ok(());
        }
        let store = self.store.lock().await;
        let mut last = after;
        for p in list {
            last = last.max(p.id);
            store::queue_payout(&store, p.auction_id.as_deref(), None, "manual", &p.to_address, p.value_zats, Some("viewer manual payout"), store::now())?;
        }
        store::set_meta(&store, "mirrored_manual_payout_id", &last.to_string())?;
        info!("mirrored manual payouts up to id {last}");
        Ok(())
    }

    /// Spender: adopt reveals from the viewer, re-checking each commitment against its own sealed bid.
    async fn mirror_reveals(&self) -> Result<()> {
        let base = self.cfg.viewer_url.as_deref().unwrap_or_default();
        let after: i64 = {
            let store = self.store.lock().await;
            store::get_meta(&store, "mirrored_reveal_at")?.and_then(|v| v.parse().ok()).unwrap_or(0)
        };
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
        #[derive(serde::Deserialize)]
        struct R { auction_id: Option<String>, nonce: Option<String>, bid_zats: Option<u64>, salt: Option<String>, revealed_at: Option<i64> }
        let reveals: Vec<R> = client
            .get(format!("{base}/admin/reveals?after={after}"))
            .header("x-admin-token", &self.cfg.admin_token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if reveals.is_empty() {
            return Ok(());
        }
        let store = self.store.lock().await;
        let mut last = after;
        for r in reveals {
            let (Some(aid), Some(nonce), Some(bid), Some(salt), Some(at)) = (r.auction_id, r.nonce, r.bid_zats, r.salt, r.revealed_at) else { continue };
            last = last.max(at);
            if let Some(b) = store::get_bid_by_nonce(&store, &aid, &nonce)? {
                if b.status == "sealed" && b.commitment.as_deref() == Some(crate::memo::commitment(&aid, bid, &salt).as_str()) {
                    store::reveal_bid(&store, b.id, bid, &salt)?;
                } else if b.status == "sealed" {
                    warn!("viewer sent a reveal for {aid} that does not match the commitment; ignored");
                }
            }
        }
        store::set_meta(&store, "mirrored_reveal_at", &last.to_string())?;
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
        // Refunds that are due together leave together: one transaction with several outputs, so a chain
        // observer sees one shielded transaction per window, not one per bidder, and refunds for several
        // lots share it. Seller payouts and forfeits go one per transaction so each has its own txid.
        let mut batches: Vec<Vec<store::Payout>> = Vec::new();
        let mut refunds: Vec<store::Payout> = Vec::new();
        for p in due {
            if p.from_account.is_some() {
                continue; // sweeps from lot accounts are handled by process_sweeps
            }
            if matches!(p.kind.as_str(), "refund" | "invalid_refund") {
                refunds.push(p);
            } else {
                batches.push(vec![p]);
            }
        }
        let max = self.cfg.refund_batch_max.max(1);
        while !refunds.is_empty() {
            let mut batch: Vec<store::Payout> = Vec::new();
            let mut rest = Vec::new();
            for p in refunds.drain(..) {
                // One output per address per transaction; a repeated address waits for the next batch.
                if batch.len() < max && !batch.iter().any(|q| q.to_address == p.to_address) {
                    batch.push(p);
                } else {
                    rest.push(p);
                }
            }
            batches.push(batch);
            refunds = rest;
        }
        for batch in batches {
            let ids: Vec<i64> = batch.iter().map(|p| p.id).collect();
            let total: u64 = batch.iter().map(|p| p.value_zats).sum();
            info!("sending {} payout(s) {:?} totalling {} zats", batch.len(), ids, total);
            let outs: Vec<Outgoing<'_>> = batch
                .iter()
                .map(|p| Outgoing { address: &p.to_address, zats: p.value_zats, memo: p.memo.as_deref() })
                .collect();
            let request = match wallet::send::build_batch_request(&outs) {
                Ok(r) => r,
                Err(e) => {
                    let store = self.store.lock().await;
                    for p in &batch {
                        store::mark_payout_dead(&store, p.id, &format!("bad payout: {e:#}"))?;
                    }
                    continue;
                }
            };
            let result = wallet::send::send_request(
                db,
                &self.params,
                client,
                prover,
                account_id,
                auth,
                request,
                self.cfg.min_confirmations,
            )
            .await;
            let store = self.store.lock().await;
            match result {
                Ok(sent) => {
                    let txid = sent.txid;
                    for p in &batch {
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
                            &serde_json::json!({ "payout_id": p.id, "txid": txid, "value_zats": p.value_zats, "batch_size": batch.len(), "cosigners": sent.cosigners }),
                        )?;
                    }
                }
                Err(e) => {
                    warn!("payout batch {ids:?} failed: {e:#}");
                    for p in &batch {
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
        }
        Ok(())
    }
}
