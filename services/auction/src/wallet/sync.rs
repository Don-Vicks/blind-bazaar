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
