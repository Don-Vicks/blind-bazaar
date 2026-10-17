//! FROST escrow: the spender's Orchard spend authority is a 2-of-3 group key. This module holds
//! the escrow description the viewer and spender share (no secrets), key generation, the signing
//! session protocol between the spender (coordinator) and signer hosts, and the signer host itself.
//!
//! Flow for one payout: the spender builds a PCZT from the wallet's proposal, proves it, and sends
//! the PCZT bytes to the signer hosts. Each signer parses the PCZT itself, recomputes the shielded
//! sighash, reads every unsigned spend's `alpha`, checks its policy against the outputs it can see,
//! and commits (round 1). The spender assembles one signing package per spend and the signers
//! return re-randomized shares (round 2). The spender aggregates each signature, the PCZT signer
//! role verifies it under the action's `rk`, and the transaction is extracted and broadcast.

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use zcash_keys::keys::UnifiedFullViewingKey;

use crate::wallet::net::Net as Network;

/// Public escrow description. Safe to share with the viewer; contains no signing material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escrow {
    pub version: u32,
    pub threshold: u16,
    pub signers: u16,
    /// Group verifying key = Orchard spend validating key `ak` (hex, 32 bytes).
    pub ak: String,
    /// Minted nullifier deriving key (hex, 32 bytes).
    pub nk: String,
    /// Minted commit-ivk randomness (hex, 32 bytes).
    pub rivk: String,
    /// Signer identifiers (hex) in share order.
    pub identifiers: Vec<String>,
    /// FROST public key package (hex): group key plus every signer's verifying share. Needed by
    /// whoever aggregates signatures; reveals nothing a signer does not already publish.
    #[serde(default)]
    pub public: String,
}

impl Escrow {
    pub fn fvk_bytes(&self) -> Result<[u8; 96]> {
        let h = |s: &str| -> Result<[u8; 32]> {
            hex::decode(s)?.try_into().map_err(|_| anyhow!("escrow field must be 32 bytes"))
        };
        Ok(bb_frost::fvk_bytes(&h(&self.ak)?, &h(&self.nk)?, &h(&self.rivk)?))
    }

    pub fn orchard_fvk(&self) -> Result<orchard::keys::FullViewingKey> {
        orchard::keys::FullViewingKey::from_bytes(&self.fvk_bytes()?)
            .ok_or_else(|| anyhow!("escrow keys do not form a valid Orchard full viewing key"))
    }

    pub fn ufvk(&self) -> Result<UnifiedFullViewingKey> {
        UnifiedFullViewingKey::from_orchard_fvk(self.orchard_fvk()?).map_err(|e| anyhow!("{e:?}"))
    }

    pub fn public_key_package(&self) -> Result<bb_frost::PublicKeyPackage> {
        if self.public.is_empty() {
            return Err(anyhow!("escrow file has no public key package; re-run `frost keygen`"));
        }
        bb_frost::wire::public_from_hex(&self.public).map_err(|e| anyhow!("{e}"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let e: Escrow = serde_json::from_str(&std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?)?;
        Ok(e)
    }
}

/// Build the escrow description for a set of freshly generated shares. Mints `nk`/`rivk` until the
/// resulting FVK validates.
pub fn escrow_from_shares(shares: &[bb_frost::SignerShare], min: u16, max: u16) -> Result<Escrow> {
    let ak = shares[0].group_ak();
    let (nk, rivk) = loop {
        let (nk, rivk) = bb_frost::mint_nk_rivk();
        if orchard::keys::FullViewingKey::from_bytes(&bb_frost::fvk_bytes(&ak, &nk, &rivk)).is_some() {
            break (nk, rivk);
        }
    };
    Ok(Escrow {
        version: 2,
        threshold: min,
        signers: max,
        ak: hex::encode(ak),
        nk: hex::encode(nk),
        rivk: hex::encode(rivk),
        identifiers: shares.iter().map(|s| bb_frost::wire::identifier_to_hex(&s.identifier)).collect(),
        public: bb_frost::wire::public_to_hex(&shares[0].public).map_err(|e| anyhow!("{e}"))?,
    })
}

/// Generate shares and the escrow file.
pub fn keygen(max: u16, min: u16, out: &Path) -> Result<Escrow> {
    std::fs::create_dir_all(out)?;
    let shares = bb_frost::generate(max, min).map_err(|e| anyhow!("{e}"))?;
    let escrow = escrow_from_shares(&shares, min, max)?;
    for (i, s) in shares.iter().enumerate() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = out.join(format!("signer-{}.json", i + 1));
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
        f.write_all(s.to_json().map_err(|e| anyhow!("{e}"))?.as_bytes())?;
    }
    std::fs::write(out.join("escrow.json"), serde_json::to_string_pretty(&escrow)?)?;
    Ok(escrow)
}

pub fn show(escrow: &Escrow, params: &Network) -> Result<(String, String)> {
    use zcash_keys::keys::UnifiedAddressRequest;
    let ufvk = escrow.ufvk()?;
    let encoded = ufvk.encode(params).map_err(|e| anyhow!("{e:?}"))?;
    let (ua, _) = ufvk
        .find_address(zip32::DiversifierIndex::new(), UnifiedAddressRequest::ORCHARD)
        .map_err(|e| anyhow!("{e:?}"))?;
    Ok((encoded, ua.encode(params)))
}

// ---------------------------------------------------------------------------------------------
// Signing plan: what a PCZT needs signed, read by both sides from the PCZT itself.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pool {
    Orchard,
    Ironwood,
}

impl Pool {
    fn value_pool(self) -> orchard::ValuePool {
        match self {
            Pool::Orchard => orchard::ValuePool::Orchard,
            Pool::Ironwood => orchard::ValuePool::Ironwood,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionToSign {
    pub pool: Pool,
    pub index: usize,
    /// The action's signature randomizer; the FROST randomizer for this spend.
    pub alpha: [u8; 32],
    /// The randomized verification key the action carries; the signature must verify under it.
    pub rk: [u8; 32],
}

/// What a signer can see about the transaction before authorising it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxSummary {
    pub spends: usize,
    /// (encoded unified address, zatoshis) for every output not owned by the escrow key.
    pub external_outputs: Vec<(String, u64)>,
    pub external_zats: u64,
    pub change_zats: u64,
}

#[derive(Clone, Debug)]
pub struct SigningPlan {
    pub sighash: [u8; 32],
    pub actions: Vec<ActionToSign>,
    pub summary: TxSummary,
}

#[derive(Debug)]
enum LlErr {
    Parse(pczt::roles::low_level_signer::OrchardParseError),
    Other(anyhow::Error),
}
impl From<pczt::roles::low_level_signer::OrchardParseError> for LlErr {
    fn from(e: pczt::roles::low_level_signer::OrchardParseError) -> Self {
        LlErr::Parse(e)
    }
}
impl From<anyhow::Error> for LlErr {
    fn from(e: anyhow::Error) -> Self {
        LlErr::Other(e)
    }
}
impl std::fmt::Display for LlErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlErr::Parse(e) => write!(f, "pczt parse: {e:?}"),
            LlErr::Other(e) => write!(f, "{e:#}"),
        }
    }
}

/// Read the shielded sighash, every unsigned spend's `alpha`/`rk`, and an output summary from a
/// PCZT. Dummy spends are already signed by the IO finalizer and are skipped.
pub fn plan_from_pczt(pczt: &pczt::Pczt, fvk: &orchard::keys::FullViewingKey, params: &Network) -> Result<SigningPlan> {
    use ff::PrimeField;
    use orchard::keys::Scope;
    use pczt::roles::low_level_signer::Signer as LowLevel;

    let sighash = pczt::roles::signer::Signer::new(pczt.clone())
        .map_err(|e| anyhow!("pczt is not signable: {e:?}"))?
        .shielded_sighash();
    let external_ivk = fvk.to_ivk(Scope::External);
    let internal_ivk = fvk.to_ivk(Scope::Internal);

    let actions = Mutex::new(Vec::new());
    let summary = Mutex::new(TxSummary::default());
    let collect = |pool: Pool, bundle: &mut orchard::pczt::Bundle| -> std::result::Result<(), LlErr> {
        let mut actions = actions.lock().unwrap();
        let mut summary = summary.lock().unwrap();
        for (index, action) in bundle.actions().iter().enumerate() {
            let spend = action.spend();
            if spend.spend_auth_sig().is_none() {
                let alpha = spend
                    .alpha()
                    .ok_or_else(|| anyhow!("spend {index} has no alpha; was the PCZT IO-finalized?"))?
                    .to_repr();
                let rk: [u8; 32] = spend.rk().clone().into();
                actions.push(ActionToSign { pool, index, alpha, rk });
                summary.spends += 1;
            }
            let output = action.output();
            if let (Some(addr), Some(value)) = (output.recipient(), output.value()) {
                let zats = value.inner();
                if zats == 0 {
                    continue; // dummy output
                }
                let ours = external_ivk.diversifier_index(addr).is_some() || internal_ivk.diversifier_index(addr).is_some();
                if ours {
                    summary.change_zats += zats;
                } else {
                    let encoded = zcash_keys::address::UnifiedAddress::from_receivers(Some(*addr), None, None, None, None)
                        .map(|ua| ua.encode(params))
                        .unwrap_or_else(|| hex::encode(addr.to_raw_address_bytes()));
                    summary.external_outputs.push((encoded, zats));
                    summary.external_zats += zats;
                }
            }
        }
        Ok(())
    };

    if !pczt.orchard().actions().is_empty() {
        LowLevel::new(pczt.clone())
            .sign_orchard_with::<LlErr, _>(|_, bundle, _| collect(Pool::Orchard, bundle))
            .map_err(|e| anyhow!("orchard bundle: {e}"))?;
    }
    if !pczt.ironwood().actions().is_empty() {
        LowLevel::new(pczt.clone())
            .sign_ironwood_with::<LlErr, _>(|_, bundle, _| collect(Pool::Ironwood, bundle))
            .map_err(|e| anyhow!("ironwood bundle: {e}"))?;
    }
    Ok(SigningPlan {
        sighash,
        actions: actions.into_inner().unwrap(),
        summary: summary.into_inner().unwrap(),
    })
}

// ---------------------------------------------------------------------------------------------
// Wire protocol between the spender and signer hosts.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round1Request {
    pub session: String,
    /// The whole PCZT (hex). The signer trusts nothing else about the transaction.
    pub pczt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commitment {
    pub pool: Pool,
    pub index: usize,
    pub commitments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round1Response {
    pub identifier: String,
    pub commitments: Vec<Commitment>,
    pub summary: TxSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageItem {
    pub pool: Pool,
    pub index: usize,
    pub package: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round2Request {
    pub session: String,
    pub packages: Vec<PackageItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Share {
    pub pool: Pool,
    pub index: usize,
    pub share: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round2Response {
    pub identifier: String,
    pub shares: Vec<Share>,
}

// ---------------------------------------------------------------------------------------------
// Signer host: holds one share, inspects every PCZT it is asked to sign.

#[derive(Clone, Debug, Default)]
pub struct SignerPolicy {
    /// Refuse to sign a transaction paying more than this to addresses the escrow key does not own.
    pub max_external_zats: Option<u64>,
}

struct Session {
    plan: SigningPlan,
    nonces: HashMap<(Pool, usize), bb_frost::SigningNonces>,
    created: Instant,
}

pub struct SignerHost {
    share: bb_frost::SignerShare,
    fvk: orchard::keys::FullViewingKey,
    params: Network,
    policy: SignerPolicy,
    sessions: Mutex<HashMap<String, Session>>,
    pub signed: AtomicU64,
}

const SESSION_TTL: Duration = Duration::from_secs(600);

impl SignerHost {
    pub fn new(share: bb_frost::SignerShare, escrow: &Escrow, params: Network, policy: SignerPolicy) -> Result<Self> {
        if hex::encode(share.group_ak()) != escrow.ak {
            return Err(anyhow!("this share belongs to a different group key than the escrow file"));
        }
        Ok(Self {
            fvk: escrow.orchard_fvk()?,
            share,
            params,
            policy,
            sessions: Mutex::new(HashMap::new()),
            signed: AtomicU64::new(0),
        })
    }

    pub fn load(share_path: &Path, escrow: &Escrow, params: Network, policy: SignerPolicy) -> Result<Self> {
        let share = bb_frost::SignerShare::from_json(
            &std::fs::read_to_string(share_path).with_context(|| format!("read {}", share_path.display()))?,
        )
        .map_err(|e| anyhow!("{e}"))?;
        Self::new(share, escrow, params, policy)
    }

    pub fn identifier(&self) -> String {
        bb_frost::wire::identifier_to_hex(&self.share.identifier)
    }

    /// Round 1 over PCZT bytes: parse, plan, apply policy, commit.
    pub fn round1(&self, req: &Round1Request) -> Result<Round1Response> {
        let bytes = hex::decode(&req.pczt).map_err(|_| anyhow!("pczt is not hex"))?;
        let pczt = pczt::Pczt::parse(&bytes).map_err(|e| anyhow!("pczt parse: {e:?}"))?;
        let plan = plan_from_pczt(&pczt, &self.fvk, &self.params)?;
        self.round1_plan(&req.session, plan)
    }

    /// Round 1 over an already-derived plan (the PCZT path above, and tests).
    pub fn round1_plan(&self, session: &str, plan: SigningPlan) -> Result<Round1Response> {
        if session.is_empty() || session.len() > 64 {
            return Err(anyhow!("bad session id"));
        }
        if plan.actions.is_empty() {
            return Err(anyhow!("nothing to sign"));
        }
        if let Some(max) = self.policy.max_external_zats {
            if plan.summary.external_zats > max {
                warn!(
                    "refusing session {session}: {} zats to external recipients exceeds policy {max}",
                    plan.summary.external_zats
                );
                return Err(anyhow!("policy: external value {} exceeds {max}", plan.summary.external_zats));
            }
        }
        info!(
            "session {session}: {} spend(s), {} external output(s) totalling {} zats, change {} zats",
            plan.actions.len(),
            plan.summary.external_outputs.len(),
            plan.summary.external_zats,
            plan.summary.change_zats
        );
        let mut nonces = HashMap::new();
        let mut commitments = Vec::with_capacity(plan.actions.len());
        for a in &plan.actions {
            let (n, c) = bb_frost::round1(&self.share);
            commitments.push(Commitment {
                pool: a.pool,
                index: a.index,
                commitments: bb_frost::wire::commitments_to_hex(&c).map_err(|e| anyhow!("{e}"))?,
            });
            nonces.insert((a.pool, a.index), n);
        }
        let summary = plan.summary.clone();
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, s| s.created.elapsed() < SESSION_TTL);
        sessions.insert(session.to_string(), Session { plan, nonces, created: Instant::now() });
        Ok(Round1Response { identifier: self.identifier(), commitments, summary })
    }

    /// Round 2: produce one share per spend. The session is consumed whatever the outcome, so
    /// nonces are never reused.
    pub fn round2(&self, req: &Round2Request) -> Result<Round2Response> {
        let session = self
            .sessions
            .lock()
            .unwrap()
            .remove(&req.session)
            .ok_or_else(|| anyhow!("unknown or expired session"))?;
        let mut shares = Vec::with_capacity(req.packages.len());
        for item in &req.packages {
            let package = bb_frost::wire::package_from_hex(&item.package).map_err(|e| anyhow!("{e}"))?;
            if package.message().as_slice() != session.plan.sighash {
                return Err(anyhow!("signing package message is not this session's sighash"));
            }
            let action = session
                .plan
                .actions
                .iter()
                .find(|a| a.pool == item.pool && a.index == item.index)
                .ok_or_else(|| anyhow!("package for an action not in the plan"))?;
            let nonces = session
                .nonces
                .get(&(item.pool, item.index))
                .ok_or_else(|| anyhow!("no nonces for action"))?;
            let randomizer = bb_frost::randomizer_from_alpha(&action.alpha).map_err(|e| anyhow!("{e}"))?;
            let share = bb_frost::round2(&self.share, nonces, &package, &randomizer).map_err(|e| anyhow!("{e}"))?;
            shares.push(Share { pool: item.pool, index: item.index, share: bb_frost::wire::share_to_hex(&share) });
        }
        self.signed.fetch_add(1, Ordering::Relaxed);
        info!("session {}: signed {} spend(s)", req.session, shares.len());
        Ok(Round2Response { identifier: self.identifier(), shares })
    }
}

/// HTTP server for a signer host: `POST /frost/round1`, `POST /frost/round2`, `GET /health`.
pub async fn run_signer(host: Arc<SignerHost>, bind: &str, token: Option<String>) -> Result<()> {
    use axum::{Json, Router, extract::State, http::{HeaderMap, StatusCode}, routing::{get, post}};

    #[derive(Clone)]
    struct St {
        host: Arc<SignerHost>,
        token: Option<String>,
    }
    fn authed(st: &St, headers: &HeaderMap) -> std::result::Result<(), (StatusCode, String)> {
        match &st.token {
            None => Ok(()),
            Some(t) => {
                let got = headers.get("x-frost-token").and_then(|v| v.to_str().ok()).unwrap_or("");
                if got == t { Ok(()) } else { Err((StatusCode::UNAUTHORIZED, "bad token".into())) }
            }
        }
    }
    async fn health(State(st): State<St>) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "identifier": st.host.identifier(),
            "signed_sessions": st.host.signed.load(Ordering::Relaxed),
            "policy_max_external_zats": st.host.policy.max_external_zats,
        }))
    }
    async fn round1(
        State(st): State<St>,
        headers: HeaderMap,
        Json(req): Json<Round1Request>,
    ) -> std::result::Result<Json<Round1Response>, (StatusCode, String)> {
        authed(&st, &headers)?;
        st.host.round1(&req).map(Json).map_err(|e| (StatusCode::BAD_REQUEST, format!("{e:#}")))
    }
    async fn round2(
        State(st): State<St>,
        headers: HeaderMap,
        Json(req): Json<Round2Request>,
    ) -> std::result::Result<Json<Round2Response>, (StatusCode, String)> {
        authed(&st, &headers)?;
        st.host.round2(&req).map(Json).map_err(|e| (StatusCode::BAD_REQUEST, format!("{e:#}")))
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/frost/round1", post(round1))
        .route("/frost/round2", post(round2))
        .with_state(St { host, token });
    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("frost signer listening on {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Coordinator (spender side).

#[async_trait::async_trait]
pub trait SignerTransport: Send + Sync {
    fn name(&self) -> String;
    async fn round1(&self, req: &Round1Request) -> Result<Round1Response>;
    async fn round2(&self, req: &Round2Request) -> Result<Round2Response>;
}

pub struct HttpSigner {
    url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl HttpSigner {
    pub fn new(url: &str, token: Option<String>) -> Result<Self> {
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            token,
            client: reqwest::Client::builder().timeout(Duration::from_secs(60)).build()?,
        })
    }
    async fn post<T: Serialize, R: serde::de::DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let mut req = self.client.post(format!("{}{path}", self.url)).json(body);
        if let Some(t) = &self.token {
            req = req.header("x-frost-token", t);
        }
        let resp = req.send().await.with_context(|| format!("signer {} unreachable", self.url))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("signer {} refused ({status}): {text}", self.url));
        }
        Ok(resp.json().await?)
    }
}

#[async_trait::async_trait]
impl SignerTransport for HttpSigner {
    fn name(&self) -> String {
        self.url.clone()
    }
    async fn round1(&self, req: &Round1Request) -> Result<Round1Response> {
        self.post("/frost/round1", req).await
    }
    async fn round2(&self, req: &Round2Request) -> Result<Round2Response> {
        self.post("/frost/round2", req).await
    }
}

/// An in-process signer. For development on one machine and for tests; it gives no separation.
pub struct LocalSigner {
    pub host: Arc<SignerHost>,
    /// When set, round 1 uses this plan instead of parsing the request's PCZT (tests only).
    pub plan: Option<SigningPlan>,
}

#[async_trait::async_trait]
impl SignerTransport for LocalSigner {
    fn name(&self) -> String {
        format!("local:{}", self.host.identifier())
    }
    async fn round1(&self, req: &Round1Request) -> Result<Round1Response> {
        match &self.plan {
            Some(p) => self.host.round1_plan(&req.session, p.clone()),
            None => self.host.round1(req),
        }
    }
    async fn round2(&self, req: &Round2Request) -> Result<Round2Response> {
        self.host.round2(req)
    }
}

pub struct FrostSpender {
    pub escrow: Escrow,
    public: bb_frost::PublicKeyPackage,
    fvk: orchard::keys::FullViewingKey,
    signers: Vec<Box<dyn SignerTransport>>,
}

/// A signed PCZT plus which signers took part.
pub struct Signed {
    pub pczt: pczt::Pczt,
    pub cosigners: Vec<String>,
}

impl FrostSpender {
    pub fn new(escrow: Escrow, signers: Vec<Box<dyn SignerTransport>>) -> Result<Self> {
        if signers.len() < escrow.threshold as usize {
            return Err(anyhow!(
                "escrow needs {} signers but only {} configured",
                escrow.threshold,
                signers.len()
            ));
        }
        Ok(Self {
            public: escrow.public_key_package()?,
            fvk: escrow.orchard_fvk()?,
            escrow,
            signers,
        })
    }

    pub fn describe(&self) -> String {
        format!(
            "frost {}-of-{} ({})",
            self.escrow.threshold,
            self.escrow.signers,
            self.signers.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
        )
    }

    /// Run one signing session for `pczt` and return it with every spend authorised.
    pub async fn sign_pczt(&self, pczt: pczt::Pczt, params: &Network) -> Result<Signed> {
        let plan = plan_from_pczt(&pczt, &self.fvk, params)?;
        if plan.actions.is_empty() {
            return Ok(Signed { pczt, cosigners: vec![] });
        }
        let pczt_hex = hex::encode(pczt.clone().serialize().map_err(|e| anyhow!("pczt serialize: {e:?}"))?);
        let sigs = self.run_session(&plan, pczt_hex).await?;

        let mut signer = pczt::roles::signer::Signer::new(pczt).map_err(|e| anyhow!("pczt signer: {e:?}"))?;
        for (action, sig) in &sigs.signatures {
            let s = pczt::roles::signer::SpendAuthSignature::from_parts(action.pool.value_pool(), action.index, *sig);
            signer
                .apply_orchard_spend_auth_signature(&s)
                .map_err(|e| anyhow!("signature for {:?} action {} rejected: {e:?}", action.pool, action.index))?;
        }
        Ok(Signed { pczt: signer.finish(), cosigners: sigs.cosigners })
    }

    /// The protocol proper, independent of the PCZT: commitments, packages, shares, aggregation.
    pub async fn run_session(&self, plan: &SigningPlan, pczt_hex: String) -> Result<SessionResult> {
        let threshold = self.escrow.threshold as usize;
        let session: String = hex::encode(rand::random::<[u8; 16]>());
        let req1 = Round1Request { session: session.clone(), pczt: pczt_hex };

        let mut round1: Vec<(usize, Round1Response)> = Vec::new();
        for (i, s) in self.signers.iter().enumerate() {
            if round1.len() >= threshold {
                break;
            }
            match s.round1(&req1).await {
                Ok(r) => round1.push((i, r)),
                Err(e) => warn!("signer {} did not commit: {e:#}", s.name()),
            }
        }
        if round1.len() < threshold {
            return Err(anyhow!("only {} of {threshold} required signers committed", round1.len()));
        }

        let mut packages = Vec::with_capacity(plan.actions.len());
        let mut package_map: HashMap<(Pool, usize), bb_frost::SigningPackage> = HashMap::new();
        for a in &plan.actions {
            let mut commitments = BTreeMap::new();
            for (_, r) in &round1 {
                let c = r
                    .commitments
                    .iter()
                    .find(|c| c.pool == a.pool && c.index == a.index)
                    .ok_or_else(|| anyhow!("signer {} sent no commitment for action {}", r.identifier, a.index))?;
                commitments.insert(
                    bb_frost::wire::identifier_from_hex(&r.identifier).map_err(|e| anyhow!("{e}"))?,
                    bb_frost::wire::commitments_from_hex(&c.commitments).map_err(|e| anyhow!("{e}"))?,
                );
            }
            let package = bb_frost::signing_package(&plan.sighash, commitments);
            packages.push(PackageItem {
                pool: a.pool,
                index: a.index,
                package: bb_frost::wire::package_to_hex(&package).map_err(|e| anyhow!("{e}"))?,
            });
            package_map.insert((a.pool, a.index), package);
        }

        let req2 = Round2Request { session, packages };
        let mut round2 = Vec::with_capacity(round1.len());
        let mut cosigners = Vec::with_capacity(round1.len());
        for (i, _) in &round1 {
            let s = &self.signers[*i];
            round2.push(s.round2(&req2).await.with_context(|| format!("signer {} failed round 2", s.name()))?);
            cosigners.push(s.name());
        }

        let mut signatures = Vec::with_capacity(plan.actions.len());
        for a in &plan.actions {
            let mut shares = BTreeMap::new();
            for r in &round2 {
                let sh = r
                    .shares
                    .iter()
                    .find(|s| s.pool == a.pool && s.index == a.index)
                    .ok_or_else(|| anyhow!("signer {} sent no share for action {}", r.identifier, a.index))?;
                shares.insert(
                    bb_frost::wire::identifier_from_hex(&r.identifier).map_err(|e| anyhow!("{e}"))?,
                    bb_frost::wire::share_from_hex(&sh.share).map_err(|e| anyhow!("{e}"))?,
                );
            }
            let randomizer = bb_frost::randomizer_from_alpha(&a.alpha).map_err(|e| anyhow!("{e}"))?;
            if bb_frost::randomized_rk(&self.public, &randomizer) != a.rk {
                return Err(anyhow!("action {} rk does not match ak + alpha·G; refusing to sign", a.index));
            }
            let package = &package_map[&(a.pool, a.index)];
            let sig = bb_frost::aggregate(&self.public, package, &shares, &randomizer)
                .map_err(|e| anyhow!("aggregate action {}: {e}", a.index))?;
            signatures.push((a.clone(), sig));
        }
        info!("frost session complete: {} spend(s) co-signed by {}", signatures.len(), cosigners.join(", "));
        Ok(SessionResult { signatures, cosigners })
    }
}

#[derive(Debug)]
pub struct SessionResult {
    pub signatures: Vec<(ActionToSign, [u8; 64])>,
    pub cosigners: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(policy: SignerPolicy) -> (Escrow, Vec<Arc<SignerHost>>) {
        let shares = bb_frost::generate(3, 2).unwrap();
        let escrow = escrow_from_shares(&shares, 2, 3).unwrap();
        let params = Network::regtest(1, "nu6_1");
        let hosts = shares
            .into_iter()
            .map(|s| Arc::new(SignerHost::new(s, &escrow, params, policy.clone()).unwrap()))
            .collect();
        (escrow, hosts)
    }

    fn plan(public: &bb_frost::PublicKeyPackage, n: usize, external_zats: u64) -> SigningPlan {
        let sighash: [u8; 32] = rand::random();
        let actions = (0..n)
            .map(|index| {
                let (_, alpha) = bb_frost::mint_nk_rivk();
                let r = bb_frost::randomizer_from_alpha(&alpha).unwrap();
                ActionToSign { pool: Pool::Orchard, index, alpha, rk: bb_frost::randomized_rk(public, &r) }
            })
            .collect();
        SigningPlan {
            sighash,
            actions,
            summary: TxSummary { spends: n, external_outputs: vec![("u1test".into(), external_zats)], external_zats, change_zats: 0 },
        }
    }

    fn verify(rk: &[u8; 32], msg: &[u8; 32], sig: &[u8; 64]) -> bool {
        use orchard::primitives::redpallas::{Signature, SpendAuth, VerificationKey};
        let vk = match VerificationKey::<SpendAuth>::try_from(*rk) {
            Ok(v) => v,
            Err(_) => return false,
        };
        vk.verify(msg, &Signature::from(*sig)).is_ok()
    }

    #[tokio::test]
    async fn two_of_three_session_signs_every_action_under_its_rk() {
        let (escrow, hosts) = setup(SignerPolicy::default());
        let public = escrow.public_key_package().unwrap();
        let plan = plan(&public, 3, 50_000);
        // Signer 1 is offline; 2 and 3 carry the session.
        let signers: Vec<Box<dyn SignerTransport>> = vec![
            Box::new(Offline),
            Box::new(LocalSigner { host: hosts[1].clone(), plan: Some(plan.clone()) }),
            Box::new(LocalSigner { host: hosts[2].clone(), plan: Some(plan.clone()) }),
        ];
        let spender = FrostSpender::new(escrow, signers).unwrap();
        let out = spender.run_session(&plan, String::new()).await.unwrap();
        assert_eq!(out.signatures.len(), 3);
        assert_eq!(out.cosigners.len(), 2);
        for (a, sig) in &out.signatures {
            assert!(verify(&a.rk, &plan.sighash, sig), "action {} must verify under its rk", a.index);
            let (_, other) = bb_frost::mint_nk_rivk();
            assert!(!verify(&other, &plan.sighash, sig));
        }
        assert_eq!(hosts[1].signed.load(Ordering::Relaxed), 1);
        assert_eq!(hosts[0].signed.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn one_signer_is_not_enough() {
        let (escrow, hosts) = setup(SignerPolicy::default());
        let public = escrow.public_key_package().unwrap();
        let plan = plan(&public, 1, 1);
        let signers: Vec<Box<dyn SignerTransport>> = vec![
            Box::new(LocalSigner { host: hosts[0].clone(), plan: Some(plan.clone()) }),
            Box::new(Offline),
            Box::new(Offline),
        ];
        let spender = FrostSpender::new(escrow, signers).unwrap();
        let err = spender.run_session(&plan, String::new()).await.unwrap_err();
        assert!(err.to_string().contains("only 1 of 2"), "{err}");
    }

    #[tokio::test]
    async fn signer_policy_refuses_large_external_value() {
        let (escrow, hosts) = setup(SignerPolicy { max_external_zats: Some(10_000) });
        let public = escrow.public_key_package().unwrap();
        let plan = plan(&public, 1, 10_001);
        let err = hosts[0].round1_plan("s1", plan).unwrap_err();
        assert!(err.to_string().contains("policy"), "{err}");
        let ok = plan_ok(&public);
        hosts[0].round1_plan("s2", ok).unwrap();
    }

    fn plan_ok(public: &bb_frost::PublicKeyPackage) -> SigningPlan {
        plan(public, 1, 10_000)
    }

    #[tokio::test]
    async fn session_is_single_use_and_checks_the_message() {
        let (escrow, hosts) = setup(SignerPolicy::default());
        let public = escrow.public_key_package().unwrap();
        let plan = plan(&public, 1, 1);
        let r1 = hosts[0].round1_plan("s", plan.clone()).unwrap();
        let mut commitments = BTreeMap::new();
        commitments.insert(
            bb_frost::wire::identifier_from_hex(&r1.identifier).unwrap(),
            bb_frost::wire::commitments_from_hex(&r1.commitments[0].commitments).unwrap(),
        );
        let wrong_msg = bb_frost::signing_package(&[9u8; 32], commitments);
        let req = Round2Request {
            session: "s".into(),
            packages: vec![PackageItem { pool: Pool::Orchard, index: 0, package: bb_frost::wire::package_to_hex(&wrong_msg).unwrap() }],
        };
        let err = hosts[0].round2(&req).unwrap_err();
        assert!(err.to_string().contains("sighash"), "{err}");
        // The session was consumed by the failed attempt.
        let err = hosts[0].round2(&req).unwrap_err();
        assert!(err.to_string().contains("unknown"), "{err}");
    }

    struct Offline;
    #[async_trait::async_trait]
    impl SignerTransport for Offline {
        fn name(&self) -> String {
            "offline".into()
        }
        async fn round1(&self, _: &Round1Request) -> Result<Round1Response> {
            Err(anyhow!("connection refused"))
        }
        async fn round2(&self, _: &Round2Request) -> Result<Round2Response> {
            Err(anyhow!("connection refused"))
        }
    }
}
