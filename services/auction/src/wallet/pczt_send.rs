//! The FROST send path: proposal → PCZT → proof → threshold signing session → extraction → broadcast.
//! Mirrors `send.rs` but never touches a spending key; the escrow account only has a viewing key.

use std::{convert::Infallible, sync::OnceLock};

use anyhow::anyhow;
use orchard::circuit::{OrchardCircuitVersion, VerifyingKey};
use tracing::info;
use zcash_client_backend::{
    data_api::wallet::{create_pczt_from_proposal, extract_and_store_transaction_from_pczt},
    fees::StandardFeeRule,
    proposal::Proposal,
    util::SystemClock,
    wallet::OvkPolicy,
};
use zcash_client_sqlite::{AccountUuid, ReceivedNoteId};
use zcash_primitives::transaction::{
    builder::{BundlePadding, cached_orchard_proving_key},
    components::orchard::bundle_version_for_branch,
};
use zcash_protocol::consensus::BranchId;

use super::{Client, Db, net::Net as Network, send::Sent};
use crate::frost::FrostSpender;

fn orchard_vk(version: OrchardCircuitVersion) -> &'static VerifyingKey {
    static PRE: OnceLock<VerifyingKey> = OnceLock::new();
    static FIXED: OnceLock<VerifyingKey> = OnceLock::new();
    static POST: OnceLock<VerifyingKey> = OnceLock::new();
    let cell = match version {
        OrchardCircuitVersion::InsecurePreNu6_2 => &PRE,
        OrchardCircuitVersion::FixedPostNu6_2 => &FIXED,
        OrchardCircuitVersion::PostNu6_3 => &POST,
    };
    cell.get_or_init(|| VerifyingKey::build(version))
}

/// Build, prove, co-sign, extract and broadcast one proposal from the escrow account.
pub async fn send_proposal(
    db: &mut Db,
    params: &Network,
    client: &mut Client,
    account_id: AccountUuid,
    frost: &FrostSpender,
    proposal: &Proposal<StandardFeeRule, ReceivedNoteId>,
) -> anyhow::Result<Sent> {
    use pczt::roles::prover::Prover;

    let pczt = create_pczt_from_proposal::<_, _, Infallible, _, Infallible, _>(
        db,
        params,
        &SystemClock,
        &mut super::rng(),
        account_id,
        OvkPolicy::Sender,
        proposal,
        None,
        BundlePadding::DEFAULT,
    )
    .map_err(|e| anyhow!("create_pczt_from_proposal: {e:?}"))?;

    let branch = BranchId::try_from(*pczt.global().consensus_branch_id())
        .map_err(|_| anyhow!("pczt carries an unknown consensus branch id"))?;
    let circuit_for = |pool: orchard::ValuePool| -> anyhow::Result<OrchardCircuitVersion> {
        bundle_version_for_branch(branch, pool)
            .map(|v| v.circuit_version())
            .ok_or_else(|| anyhow!("{pool:?} bundle is not valid under branch {branch:?}"))
    };

    // Prove first so the signers see a complete transaction (proof fields do not enter the sighash).
    let mut prover = Prover::new(pczt);
    let mut vk_version = None;
    if prover.requires_orchard_proof() {
        let v = circuit_for(orchard::ValuePool::Orchard)?;
        info!("proving orchard bundle ({v:?})");
        prover = prover
            .create_orchard_proof(super::rng(), cached_orchard_proving_key(v))
            .map_err(|e| anyhow!("orchard proof: {e:?}"))?;
        vk_version = Some(v);
    }
    if prover.requires_ironwood_proof() {
        let v = circuit_for(orchard::ValuePool::Ironwood)?;
        info!("proving ironwood bundle ({v:?})");
        prover = prover
            .create_ironwood_proof(super::rng(), cached_orchard_proving_key(v))
            .map_err(|e| anyhow!("ironwood proof: {e:?}"))?;
        vk_version = Some(v);
    }
    let pczt = prover.finish();

    let signed = frost.sign_pczt(pczt, params).await?;

    let txid = extract_and_store_transaction_from_pczt::<_, Infallible>(
        db,
        &SystemClock,
        &mut super::rng(),
        signed.pczt,
        None,
        vk_version.map(orchard_vk),
    )
    .map_err(|e| anyhow!("extract transaction: {e:?}"))?;

    let txid = super::send::broadcast_stored(db, client, txid).await?;
    Ok(Sent { txid, cosigners: signed.cosigners })
}
