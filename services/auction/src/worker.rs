//! The single task that owns the wallet DB: sync, enhance, ingest, close, pay out.

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{error, info, warn};
use zcash_client_backend::data_api::{WalletRead, wallet::ConfirmationsPolicy};
use zcash_proofs::prover::LocalTxProver;
use zcash_protocol::consensus::Network;

use crate::{
    close,
    config::Config,
    ingest,
    state::{SyncStatus, WalletCmd},
    store,
    wallet::{self, send::Outgoing},
};

const MAX_PAYOUT_ATTEMPTS: i64 = 5;

pub struct Worker {
    pub cfg: Config,
    pub params: Network,
    pub store: Arc<Mutex<Connection>>,
    pub status: Arc<RwLock<SyncStatus>>,
    pub rx: mpsc::Receiver<WalletCmd>,
}

impl Worker {
    pub async fn run(mut self) -> Result<()> {
        let wallet_dir = self.cfg.wallet_dir();
        let mut db = wallet::open_db(self.params, &wallet_dir)?;
        let db_cache = wallet::cache::MemBlockCache::default();
        let (_, data_db_path) = wallet::db_paths(&wallet_dir);
        let wallet_ro = ingest::open_wallet_readonly(&data_db_path)?;

        let mut client = wallet::connect(&self.cfg.lightwalletd_url).await?;
        let (seed, _created) = wallet::load_or_create_seed(&self.cfg.operator_seed_file)?;
        let operator =
            wallet::ensure_operator_account(&mut db, &mut client, &self.params, &seed, self.cfg.birthday_height)
                .await?;

        // Make sure the operator has at least one address so the health endpoint can show it.
        if wallet::operator_address(&db, &self.params, operator.account_id)?.is_none() {
            let (addr, _) = wallet::next_auction_address(&mut db, &self.params, operator.account_id)?;
            info!("operator address: {addr}");
        }
        {
            let mut s = self.status.write().await;
            s.operator_address = wallet::operator_address(&db, &self.params, operator.account_id)?;
        }

        info!("loading prover parameters");
        let prover = LocalTxProver::bundled();

        let mut interval = tokio::time::interval(Duration::from_secs(self.cfg.sync_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                Some(cmd) = self.rx.recv() => match cmd {
                    WalletCmd::NewAddress { reply } => {
                        let r = wallet::next_auction_address(&mut db, &self.params, operator.account_id);
                        let _ = reply.send(r);
                    }
                },
                _ = interval.tick() => {
                    if let Err(e) = self.pass(&mut client, &db_cache, &mut db, &wallet_ro, &prover, &operator).await {
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
        prover: &LocalTxProver,
        operator: &wallet::Operator,
    ) -> Result<()> {
        wallet::sync::sync_once(client, &self.params, db_cache, db).await?;
        let enhanced = wallet::sync::enhance(client, &self.params, db).await?;
        if enhanced > 0 {
            info!("enhanced {enhanced} transactions");
        }

        let tip = wallet::chain_tip(client).await.ok();
        let height = wallet::wallet_height(db)?;
        let balance = db
            .get_wallet_summary(ConfirmationsPolicy::default())
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
        }

        {
            let store = self.store.lock().await;
            let n = ingest::ingest(&store, wallet_ro)?;
            if n > 0 {
                info!("ingested {n} new outputs");
            }
            close::close_due(&store, self.cfg.refund_delay_min_secs, self.cfg.refund_delay_max_secs)?;
        }

        self.process_payouts(client, db, prover, operator).await?;
        Ok(())
    }

    async fn process_payouts(
        &self,
        client: &mut wallet::Client,
        db: &mut wallet::Db,
        prover: &LocalTxProver,
        operator: &wallet::Operator,
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
                operator.account_id,
                &operator.usk,
                &out,
            )
            .await;
            let store = self.store.lock().await;
            match result {
                Ok(txid) => {
                    store::mark_payout_sent(&store, p.id, &txid)?;
                    if p.kind == "seller" {
                        if let Some(aid) = &p.auction_id {
                            store::set_auction_payout(&store, aid, &txid)?;
                        }
                    }
                }
                Err(e) => {
                    warn!("payout {} failed: {e:#}", p.id);
                    let backoff = 60 * (1 << p.attempts.min(5));
                    store::mark_payout_failed(&store, p.id, &format!("{e:#}"), store::now() + backoff)?;
                }
            }
        }
        Ok(())
    }
}
