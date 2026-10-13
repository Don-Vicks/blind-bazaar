//! Sync + enhancement pass. `sync::run` scans compact blocks; the enhancement loop fetches
//! full transactions so memos become available (compact blocks do not carry memos).

use std::collections::BTreeSet;

use anyhow::anyhow;
use futures_util::StreamExt;
use tonic::Code;
use tracing::{debug, info};
use zcash_client_backend::{
    data_api::{
        TransactionDataRequest, TransactionStatus, WalletRead, WalletWrite,
        wallet::decrypt_and_store_transaction,
    },
    proto::service::{self, BlockRange, RawTransaction},
};
use zcash_keys::encoding::AddressCodec;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::{BlockHeight, BranchId, Network};

use super::{Client, Db, cache::MemBlockCache};
use std::time::Duration;

const BATCH_SIZE: u32 = 1_000;

pub async fn sync_once(client: &mut Client, params: &Network, db_cache: &MemBlockCache, db: &mut Db) -> anyhow::Result<()> {
    zcash_client_backend::sync::run(client, params, db_cache, db, BATCH_SIZE)
        .await
        .map_err(|e| anyhow!("sync failed: {e:?}"))?;
    Ok(())
}

fn parse_raw_transaction(
    params: &Network,
    chain_tip: BlockHeight,
    tx: RawTransaction,
) -> anyhow::Result<(Transaction, Option<BlockHeight>)> {
    let mined_height = (tx.height > 0 && tx.height <= u64::from(u32::MAX))
        .then(|| BlockHeight::from_u32(u32::try_from(tx.height).unwrap()));
    let tx = Transaction::read(
        &tx.data[..],
        BranchId::for_height(params, mined_height.unwrap_or(chain_tip)),
    )?;
    Ok((tx, mined_height))
}

async fn fetch_transaction(
    client: &mut Client,
    params: &Network,
    chain_tip: BlockHeight,
    txid: TxId,
) -> anyhow::Result<Option<(Transaction, Option<BlockHeight>)>> {
    let request = service::TxFilter {
        hash: txid.as_ref().to_vec(),
        ..Default::default()
    };
    let raw_tx = match client.get_transaction(request).await {
        Ok(response) => Some(response.into_inner()),
        Err(status) if status.code() == Code::NotFound => None,
        Err(status) => return Err(status.into()),
    };
    raw_tx
        .map(|raw_tx| parse_raw_transaction(params, chain_tip, raw_tx))
        .transpose()
}

pub async fn enhance(client: &mut Client, params: &Network, db: &mut Db) -> anyhow::Result<usize> {
    let Some(chain_tip) = db.chain_height()? else {
        return Ok(0);
    };
    let mut satisfied = BTreeSet::new();
    let mut enhanced = 0usize;
    loop {
        let mut new_request = false;
        for req in db.transaction_data_requests()? {
            if satisfied.contains(&req) {
                continue;
            }
            new_request = true;
            debug!("tx data request {req:?}");
            match &req {
                TransactionDataRequest::GetStatus(txid) => {
                    let status = fetch_transaction(client, params, chain_tip, *txid)
                        .await?
                        .map_or(TransactionStatus::TxidNotRecognized, |(_, h)| {
                            h.map_or(TransactionStatus::NotInMainChain, TransactionStatus::Mined)
                        });
                    db.set_transaction_status(*txid, status)?;
                }
                TransactionDataRequest::Enhancement(txid) => {
                    match fetch_transaction(client, params, chain_tip, *txid).await? {
                        None => {
                            db.set_transaction_status(*txid, TransactionStatus::TxidNotRecognized)?;
                        }
                        Some((tx, mined_height)) => {
                            info!("enhancing tx {txid} (mined at {mined_height:?})");
                            decrypt_and_store_transaction(params, db, &tx, mined_height)?;
                            enhanced += 1;
                        }
                    }
                }
                TransactionDataRequest::TransactionsInvolvingAddress(tia) => {
                    let address = tia.address().encode(params);
                    let request = service::TransparentAddressBlockFilter {
                        address: address.clone(),
                        range: Some(BlockRange {
                            start: Some(service::BlockId {
                                height: u64::from(tia.block_range_start()),
                                ..Default::default()
                            }),
                            end: tia.block_range_end().map(|h| service::BlockId {
                                height: u64::from(h - 1),
                                ..Default::default()
                            }),
                            pool_types: Default::default(),
                        }),
                    };
                    let mut stream = client.get_taddress_txids(request).await?.into_inner();
                    while let Some(raw_tx) = stream.next().await {
                        let (tx, mined_height) = parse_raw_transaction(params, chain_tip, raw_tx?)?;
                        decrypt_and_store_transaction(params, db, &tx, mined_height)?;
                    }
                }
            }
            satisfied.insert(req);
        }
        if !new_request {
            break;
        }
    }
    Ok(enhanced)
}

/// Pull the current mempool and trial-decrypt each transaction so bids show up before they are
/// mined. lightwalletd keeps the stream open until the next block, so we stop after a short window.
pub async fn scan_mempool(client: &mut Client, params: &Network, db: &mut Db) -> anyhow::Result<usize> {
    let Some(chain_tip) = db.chain_height()? else {
        return Ok(0);
    };
    let mut stream = client
        .get_mempool_stream(service::Empty {})
        .await?
        .into_inner();
    let mut seen = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let next = tokio::time::timeout_at(deadline, stream.next()).await;
        match next {
            Ok(Some(Ok(raw))) => {
                let (tx, _) = parse_raw_transaction(params, chain_tip + 1, raw)?;
                decrypt_and_store_transaction(params, db, &tx, None)?;
                seen += 1;
            }
            Ok(Some(Err(e))) => {
                debug!("mempool stream error: {e}");
                break;
            }
            Ok(None) | Err(_) => break,
        }
    }
    Ok(seen)
}

pub async fn lightd_info(client: &mut Client) -> anyhow::Result<(String, String, u64)> {
    let info = client.get_lightd_info(service::Empty {}).await?.into_inner();
    Ok((info.version, info.chain_name, info.block_height))
}

/// Fetch one transaction by display (big-endian hex) txid and trial-decrypt it into the wallet.
pub async fn scan_txid(client: &mut Client, params: &Network, db: &mut Db, txid_display: &str) -> anyhow::Result<String> {
    let chain_tip = db.chain_height()?.ok_or_else(|| anyhow!("wallet has no chain height yet"))?;
    let mut bytes = hex::decode(txid_display)?;
    bytes.reverse();
    let txid = TxId::from_bytes(bytes.try_into().map_err(|_| anyhow!("txid must be 32 bytes"))?);
    match fetch_transaction(client, params, chain_tip, txid).await? {
        None => Ok("lightwalletd does not know this txid".into()),
        Some((tx, mined_height)) => {
            let before = db.get_wallet_summary(zcash_client_backend::data_api::wallet::ConfirmationsPolicy::MIN)?;
            decrypt_and_store_transaction(params, db, &tx, mined_height)?;
            let after = db.get_wallet_summary(zcash_client_backend::data_api::wallet::ConfirmationsPolicy::MIN)?;
            let total = |s: Option<zcash_client_backend::data_api::WalletSummary<zcash_client_sqlite::AccountUuid>>| {
                s.map(|s| s.account_balances().values().map(|b| u64::from(b.total())).sum::<u64>()).unwrap_or(0)
            };
            Ok(format!(
                "mined_height={mined_height:?} sapling_outputs={} orchard_actions={} ironwood_actions={} balance {} -> {}",
                tx.sapling_bundle().map(|b| b.shielded_outputs().len()).unwrap_or(0),
                tx.orchard_bundle().map(|b| b.actions().len()).unwrap_or(0),
                tx.ironwood_bundle().map(|b| b.actions().len()).unwrap_or(0),
                total(before),
                total(after)
            ))
        }
    }
}
