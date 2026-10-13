//! Thin wrappers over zcash_client_sqlite / zcash_client_backend, lifted from zcash-devtool.

pub mod cache;
pub mod net;
pub mod send;
pub mod sync;

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use bip0039::{Count, English, Mnemonic};
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng as OsRng;
use secrecy::{ExposeSecret, SecretVec};
use tonic::transport::{Channel, ClientTlsConfig, Uri};
use tracing::info;
use zcash_client_backend::{
    data_api::{Account, AccountBirthday, WalletRead, WalletWrite, chain::ChainState},
    proto::service::{self, compact_tx_streamer_client::CompactTxStreamerClient},
};
use zcash_client_sqlite::{AccountUuid, WalletDb, util::SystemClock, wallet::init::init_wallet_db};
use zcash_keys::keys::{UnifiedAddressRequest, UnifiedSpendingKey};
use zcash_protocol::consensus::{BlockHeight, NetworkUpgrade, Parameters};
use net::Net as Network;
use zip32::AccountId;

pub type Client = CompactTxStreamerClient<Channel>;
pub type Db = WalletDb<rusqlite::Connection, Network, SystemClock, UnwrapErr<OsRng>>;

pub fn rng() -> UnwrapErr<OsRng> {
    UnwrapErr(OsRng)
}

pub const DATA_DB: &str = "data.sqlite";

pub fn db_paths(wallet_dir: &Path) -> (PathBuf, PathBuf) {
    (wallet_dir.to_path_buf(), wallet_dir.join(DATA_DB))
}

pub fn open_db(params: Network, wallet_dir: &Path) -> anyhow::Result<Db> {
    std::fs::create_dir_all(wallet_dir)?;
    let (_, data_path) = db_paths(wallet_dir);
    let mut db_data = WalletDb::for_path(data_path, params, SystemClock, rng())?;
    init_wallet_db(&mut db_data, None).map_err(|e| anyhow!("{e:?}"))?;
    Ok(db_data)
}

pub async fn connect(url: &str) -> anyhow::Result<Client> {
    let uri: Uri = url.parse().context("invalid LIGHTWALLETD_URL")?;
    let host = uri.host().ok_or_else(|| anyhow!("missing host"))?.to_string();
    let use_tls = uri.scheme_str() == Some("https");
    info!("connecting to lightwalletd at {url}");
    let mut endpoint = Channel::builder(uri);
    if use_tls {
        endpoint = endpoint.tls_config(
            ClientTlsConfig::new()
                .domain_name(host)
                .assume_http2(true)
                .with_webpki_roots(),
        )?;
    }
    Ok(CompactTxStreamerClient::new(endpoint.connect().await?))
}

pub async fn chain_tip(client: &mut Client) -> anyhow::Result<u32> {
    Ok(client
        .get_latest_block(service::ChainSpec::default())
        .await?
        .into_inner()
        .height
        .try_into()
        .expect("block heights fit in u32"))
}

pub async fn get_wallet_birthday(
    client: &mut Client,
    params: &Network,
    birthday_height: BlockHeight,
    recover_until: Option<BlockHeight>,
) -> anyhow::Result<AccountBirthday> {
    let sapling_activation = params
        .activation_height(NetworkUpgrade::Sapling)
        .ok_or_else(|| anyhow!("Sapling activation height is not set for this network"))?;
    let birthday_height = birthday_height.max(sapling_activation);

    let birthday = if birthday_height == sapling_activation {
        let birthday_block = client
            .get_block(service::BlockId {
                height: u64::from(birthday_height),
                ..Default::default()
            })
            .await?
            .into_inner();
        AccountBirthday::from_parts(
            ChainState::empty(birthday_height.saturating_sub(1), birthday_block.prev_hash()),
            recover_until,
        )
    } else {
        let request = service::BlockId {
            height: u64::from(birthday_height) - 1,
            ..Default::default()
        };
        let treestate = client.get_tree_state(request).await?.into_inner();
        AccountBirthday::from_treestate(treestate, recover_until).map_err(|e| anyhow!("{e:?}"))?
    };
    Ok(birthday)
}

/// Load the operator mnemonic from `path`, generating one if the file does not exist.
pub fn load_or_create_seed(path: &Path) -> anyhow::Result<(SecretVec<u8>, bool)> {
    if path.exists() {
        let phrase = std::fs::read_to_string(path)?;
        let mnemonic = <Mnemonic<English>>::from_phrase(phrase.trim())
            .map_err(|e| anyhow!("invalid mnemonic in {}: {e}", path.display()))?;
        Ok((SecretVec::new(mnemonic.to_seed("").to_vec()), false))
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mnemonic = <Mnemonic<English>>::generate(Count::Words24);
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?;
            writeln!(f, "{}", mnemonic.phrase())?;
        }
        info!("generated new operator mnemonic at {}", path.display());
        Ok((SecretVec::new(mnemonic.to_seed("").to_vec()), true))
    }
}

pub struct Operator {
    pub account_id: AccountUuid,
    /// None for a viewer process (viewing key only).
    pub usk: Option<UnifiedSpendingKey>,
    /// The account's full viewing key (always available).
    pub ufvk: Option<zcash_keys::keys::UnifiedFullViewingKey>,
}

/// Ensure a view-only account exists for `ufvk_str` and return its id (viewer role).
pub async fn ensure_viewer_account(
    db: &mut Db,
    client: &mut Client,
    params: &Network,
    ufvk_str: &str,
    birthday_height: Option<u32>,
) -> anyhow::Result<Operator> {
    use zcash_address::unified::{Encoding, Ufvk};
    use zcash_client_backend::data_api::AccountPurpose;
    use zcash_keys::keys::UnifiedFullViewingKey;
    let existing = db.get_account_ids()?;
    let account_id = if let Some(id) = existing.first() {
        *id
    } else {
        let (_, _, ufvk) = Ufvk::decode(ufvk_str).map_err(|e| anyhow!("bad OPERATOR_UFVK: {e}"))?;
        let ufvk = UnifiedFullViewingKey::parse(&ufvk).map_err(|e| anyhow!("bad OPERATOR_UFVK: {e:?}"))?;
        let tip = chain_tip(client).await?;
        let birthday = get_wallet_birthday(
            client,
            params,
            birthday_height.unwrap_or(tip.saturating_sub(100)).into(),
            None,
        )
        .await?;
        let account = db
            .import_account_ufvk("viewer", &ufvk, &birthday, AccountPurpose::ViewOnly, None)
            .map_err(|e| anyhow!("{e:?}"))?;
        let id = account.id();
        info!("imported view-only account {id:?} with birthday {}", birthday.height());
        id
    };
    let ufvk = db.get_account(account_id)?.and_then(|a| a.ufvk().cloned());
    Ok(Operator { account_id, usk: None, ufvk })
}

/// Ensure the operator account exists in the wallet DB and return its id and spending key.
pub async fn ensure_operator_account(
    db: &mut Db,
    client: &mut Client,
    params: &Network,
    seed: &SecretVec<u8>,
    birthday_height: Option<u32>,
) -> anyhow::Result<Operator> {
    let existing = db.get_account_ids()?;
    let account_id = if let Some(id) = existing.first() {
        *id
    } else {
        let tip = chain_tip(client).await?;
        let birthday = get_wallet_birthday(
            client,
            params,
            birthday_height.unwrap_or(tip.saturating_sub(100)).into(),
            None,
        )
        .await?;
        let (id, _usk) = db
            .create_account("operator", seed, &birthday, None)
            .map_err(|e| anyhow!("{e:?}"))?;
        info!("created operator account {id:?} with birthday {}", birthday.height());
        id
    };
    let account = db
        .get_account(account_id)?
        .ok_or_else(|| anyhow!("operator account missing"))?;
    let derivation = account
        .source()
        .key_derivation()
        .ok_or_else(|| anyhow!("operator account is view-only"))?;
    let usk = UnifiedSpendingKey::from_seed(params, seed.expose_secret(), derivation.account_index())
        .map_err(|e| anyhow!("{e:?}"))?;
    let ufvk = Some(usk.to_unified_full_viewing_key());
    Ok(Operator { account_id, usk: Some(usk), ufvk })
}

/// Orchard-only unified address request so bids never land in Sapling or transparent receivers.
pub fn orchard_only_request() -> UnifiedAddressRequest {
    UnifiedAddressRequest::ORCHARD
}

/// A fresh receive address for Go Dark payments: a unified address that also carries a transparent
/// receiver, so a rail that cannot deliver shielded can still pay and the funds get shielded on arrival.
/// Returns (encoded UA, encoded transparent address).
pub fn next_receive_address(db: &mut Db, params: &Network, account_id: AccountUuid) -> anyhow::Result<(String, Option<String>)> {
    use zcash_keys::encoding::AddressCodec;
    let (ua, _) = db
        .get_next_available_address(account_id, UnifiedAddressRequest::ALLOW_ALL)
        .map_err(|e| anyhow!("{e:?}"))?
        .ok_or_else(|| anyhow!("wallet could not derive a new address"))?;
    let transparent = ua.transparent().map(|t| t.encode(params));
    Ok((ua.encode(params), transparent))
}

/// A fresh diversified address for a new auction. Returns (encoded UA, diversifier index hex).
pub fn next_auction_address(
    db: &mut Db,
    params: &Network,
    account_id: AccountUuid,
) -> anyhow::Result<(String, String)> {
    let (ua, di) = db
        .get_next_available_address(account_id, orchard_only_request())
        .map_err(|e| anyhow!("{e:?}"))?
        .ok_or_else(|| anyhow!("wallet could not derive a new address"))?;
    Ok((ua.encode(params), hex::encode(di.as_bytes())))
}

pub fn wallet_height(db: &Db) -> anyhow::Result<Option<u32>> {
    Ok(db.chain_height()?.map(u32::from))
}

#[allow(dead_code)]
pub fn account_zero() -> AccountId {
    AccountId::ZERO
}
