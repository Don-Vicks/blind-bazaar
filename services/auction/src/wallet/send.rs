//! Build, prove and broadcast a single shielded payment from the operator account.

use std::{num::NonZeroUsize, str::FromStr};

use anyhow::anyhow;
use tracing::info;
use zcash_address::ZcashAddress;
use zcash_client_backend::{
    data_api::{
        WalletRead,
        wallet::{
            ConfirmationsPolicy, SpendingKeys, create_proposed_transactions,
            input_selection::{GreedyInputSelector, SpendPolicy},
            propose_transfer,
        },
    },
    fees::{DustOutputPolicy, SplitPolicy, StandardFeeRule, standard::MultiOutputChangeStrategy},
    proto::service,
    wallet::OvkPolicy,
};
use zcash_client_sqlite::AccountUuid;
use zcash_client_backend::util::SystemClock;
use zcash_keys::keys::UnifiedSpendingKey;
use zcash_proofs::prover::LocalTxProver;
use zcash_protocol::{
    ShieldedPool,
    memo::{Memo, MemoBytes},
    value::Zatoshis,
};
use zip321::{Payment, TransactionRequest};

use super::{Client, Db};
use super::net::Net as Network;

pub struct Outgoing<'a> {
    pub address: &'a str,
    pub zats: u64,
    pub memo: Option<&'a str>,
}

pub fn build_request(out: &Outgoing<'_>) -> anyhow::Result<TransactionRequest> {
    let payment = Payment::new(
        ZcashAddress::from_str(out.address).map_err(|_| anyhow!("invalid recipient address"))?,
        Some(Zatoshis::from_u64(out.zats).map_err(|_| anyhow!("invalid amount"))?),
        out.memo
            .map(Memo::from_str)
            .transpose()
            .map_err(|_| anyhow!("invalid memo"))?
            .map(MemoBytes::from),
        None,
        None,
        vec![],
    )
    .map_err(|e| anyhow!("{e:?}"))?;
    TransactionRequest::new(vec![payment]).map_err(|e| anyhow!("{e:?}"))
}

pub fn confirmations_policy(min: Option<u32>) -> ConfirmationsPolicy {
    match min.and_then(std::num::NonZeroU32::new) {
        Some(n) => ConfirmationsPolicy::new_symmetrical(n, true),
        None => ConfirmationsPolicy::default(),
    }
}

pub async fn send(
    db: &mut Db,
    params: &Network,
    client: &mut Client,
    prover: &LocalTxProver,
    account_id: AccountUuid,
    usk: &UnifiedSpendingKey,
    out: &Outgoing<'_>,
    min_confirmations: Option<u32>,
) -> anyhow::Result<String> {
    let request = build_request(out)?;
    send_request(db, params, client, prover, account_id, usk, request, min_confirmations).await
}

#[allow(clippy::too_many_arguments)]
pub async fn send_request(
    db: &mut Db,
    params: &Network,
    client: &mut Client,
    prover: &LocalTxProver,
    account_id: AccountUuid,
    usk: &UnifiedSpendingKey,
    request: TransactionRequest,
    min_confirmations: Option<u32>,
) -> anyhow::Result<String> {
    let change_strategy = MultiOutputChangeStrategy::new(
        StandardFeeRule::Zip317,
        None,
        ShieldedPool::Orchard,
        DustOutputPolicy::default(),
        SplitPolicy::with_min_output_value(
            NonZeroUsize::new(4).unwrap(),
            Zatoshis::from_u64(10_000_000).unwrap(),
        ),
    );
    let input_selector = GreedyInputSelector::new();

    let proposal = propose_transfer::<_, _, _, _, std::convert::Infallible>(
        db,
        params,
        account_id,
        &input_selector,
        &change_strategy,
        request,
        confirmations_policy(min_confirmations),
        &SpendPolicy::default(),
        None,
        None,
    )
    .map_err(|e| anyhow!("propose_transfer: {e:?}"))?;

    let txids = create_proposed_transactions::<_, _, std::convert::Infallible, _, std::convert::Infallible, _>(
        db,
        params,
        &SystemClock,
        &mut super::rng(),
        prover,
        prover,
        &SpendingKeys::from_unified_spending_key(usk.clone()),
        OvkPolicy::Sender,
        &proposal,
        None,
    )
    .map_err(|e| anyhow!("create_proposed_transactions: {e:?}"))?;

    if txids.len() > 1 {
        return Err(anyhow!("multi-transaction proposals are not supported"));
    }
    let txid = *txids.first();

    let (txid, raw_tx) = db
        .get_transaction(txid)?
        .map(|tx| {
            let mut raw = service::RawTransaction::default();
            tx.write(&mut raw.data).expect("in-memory write");
            (tx.txid(), raw)
        })
        .ok_or_else(|| anyhow!("transaction {txid} not found after creation"))?;

    let response = client.send_transaction(raw_tx).await?.into_inner();
    if response.error_code != 0 {
        return Err(anyhow!(
            "send_transaction failed ({}): {}",
            response.error_code,
            response.error_message
        ));
    }
    info!("sent {txid}");
    Ok(txid.to_string())
}
