use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tower_http::cors::CorsLayer;

use crate::{
    close, memo,
    state::{AppState, WalletCmd},
    store,
};

pub fn router(state: Arc<AppState>) -> Router {
    let cors = match state.cors_origin.as_deref() {
        Some(origin) => CorsLayer::new()
            .allow_origin(origin.parse::<axum::http::HeaderValue>().expect("CORS_ORIGIN must be a valid origin"))
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::HeaderName::from_static("x-admin-token"), axum::http::HeaderName::from_static("x-seller-key")]),
        None => CorsLayer::permissive(),
    };
    // Anonymous endpoints that derive addresses or run per-caller queries get a per-IP token bucket:
    // 30 requests burst, one per second sustained. Reads of public auction state are not limited.
    let limiter = Arc::new(crate::ratelimit::Limiter::new(30, 1.0, state.trust_proxy));
    let limited = Router::new()
        .route("/auctions/{id}/envelopes", post(post_envelope))
        .route("/auctions/{id}/bid-uri", get(bid_uri))
        .route("/godark/receive-address", post(godark_receive_address))
        .route("/auctions/{id}/proof", get(inclusion_proof))
        .route("/auctions/{id}/fulfilment", post(winner_fulfilment))
        .route("/auctions/{id}/standing", post(standing))
        .route("/auctions/{id}/reveals", post(reveal))
        .route("/auctions/{id}/notify", post(notify_bidder).delete(unnotify_bidder))
        .route("/sellers/demo", post(demo_seller))
        .route("/notify/confirm", post(confirm_notify))
        .route("/auctions", post(create_auction))
        .route_layer(axum::middleware::from_fn_with_state(limiter, crate::ratelimit::limit));
    Router::new()
        .merge(limited)
        .route("/health", get(health))
        .route("/rules", get(rules))
        .route("/auctions", get(list_auctions))
        .route("/auctions/{id}", get(get_auction))
        .route("/auctions/{id}/receipts", get(receipts))
        .route("/auctions/{id}/result", get(result))
        .route("/admin/payouts", post(admin_payout).get(admin_list_payouts))
        .route("/admin/scan-tx", post(admin_scan_tx))
        .route("/admin/block/{height}", get(admin_block))
        .route("/admin/sellers", post(admin_create_seller))
        .route("/admin/payouts/{id}/retry", post(admin_retry_payout))
        .route("/admin/reconcile", get(admin_reconcile))
        .route("/admin/auctions", get(admin_auctions))
        .route("/admin/envelopes", get(admin_envelopes))
        .route("/admin/reveals", get(admin_reveals))
        .route("/ledger/payouts", get(ledger_payouts))
        .route("/admin/payouts/sync", post(admin_payouts_sync))
        .route("/auctions/{id}/events", get(auction_events))
        .route("/seller/me", get(seller_me))
        .route("/seller/lots", get(seller_lots))
        .route("/seller/lots/{id}", get(seller_lot))
        .route("/seller/lots/{id}/fulfilment", post(seller_set_fulfilment))
        .route("/seller/notify", get(seller_notify_get).post(seller_notify_set))
        .layer(cors)
        .with_state(state)
}

pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!("{e:#}");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
    }
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "not found".into())
}

type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Serialize)]
struct Health {
    network: &'static str,
    chain_tip: Option<u32>,
    wallet_height: Option<u32>,
    sync_lag: Option<u32>,
    last_sync_at: Option<i64>,
    last_error: Option<String>,
    operator_address: Option<String>,
    balance_zats: Option<u64>,
    lightwalletd_version: Option<String>,
    mempool_seen: usize,
    pending_payouts_zats: u64,
    dead_payouts: i64,
    low_balance: bool,
    signing: Option<String>,
    freeze: crate::freeze::FreezeStatus,
}

/// Public health hides balances, mempool activity and raw errors (deposit timing and amounts would leak);
/// an admin token reveals them.
async fn health(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<Health> {
    let s = state.status.read().await.clone();
    let admin = check_admin(&state, &headers).is_ok();
    let (last_error, balance_zats, mempool_seen, pending_payouts_zats) = if admin {
        (s.last_error, s.balance_zats, s.mempool_seen, s.pending_payouts_zats)
    } else {
        (s.last_error.map(|_| "see admin health".to_string()), None, 0, 0)
    };
    Ok(Json(Health {
        network: state.network_name,
        chain_tip: s.chain_tip,
        wallet_height: s.wallet_height,
        sync_lag: s.chain_tip.zip(s.wallet_height).map(|(t, w)| t.saturating_sub(w)),
        last_sync_at: s.last_sync_at,
        last_error,
        operator_address: s.operator_address,
        balance_zats,
        lightwalletd_version: s.lightwalletd_version,
        mempool_seen,
        pending_payouts_zats,
        dead_payouts: s.dead_payouts,
        low_balance: s.low_balance,
        signing: s.signing,
        freeze: s.freeze,
    }))
}

/// Static rules a client needs before creating a lot.
async fn rules(State(state): State<Arc<AppState>>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::json!({
        "deposit_menu_zats": crate::denom::DEPOSIT_MENU_ZATS,
        "allow_any_deposit": state.allow_any_deposit,
        "formats": crate::close::FORMATS,
        "settlement_window_secs": state.settlement_window_secs,
    })))
}

async fn list_auctions(State(state): State<Arc<AppState>>) -> ApiResult<Vec<store::Auction>> {
    let conn = state.store.lock().await;
    Ok(Json(store::list_auctions(&conn)?))
}

#[derive(Deserialize)]
struct CreateAuction {
    title: String,
    #[serde(default)]
    description: String,
    reserve_zats: u64,
    deposit_zats: u64,
    seller_address: String,
    /// Unix seconds. Either this or `duration_secs`.
    close_at: Option<i64>,
    duration_secs: Option<i64>,
    /// Optional https image for the lot.
    image_url: Option<String>,
    /// Number of identical units (default 1). More than one implies the uniform-price format.
    units: Option<u32>,
    /// `first_price` (default for one unit), `uniform` (top-N win, all pay the lowest winning bid),
    /// `vickrey` (top bid wins, pays the second bid or the reserve), or `rfq` (a request for quotes:
    /// `reserve_zats` is the buyer's budget, `deposit_zats` the quoter's bond, the lowest quote wins
    /// and the buyer pays it).
    format: Option<String>,
    /// Sealed-reveal: seconds after close during which bidders reveal their amounts. Bids then carry a
    /// commitment instead of the amount, so not even the operator reads a bid before close. 0 or absent
    /// means bids carry the amount (readable by the operator's viewing key while the lot is open).
    reveal_secs: Option<i64>,
    /// Auditable: the lot gets a wallet account of its own, and its full viewing key is published once the
    /// lot closes, so anyone can rescan the chain and recompute the result. Reveals every bid after close.
    auditable: Option<bool>,
}

/// Constant-time string equality for tokens.
pub fn token_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.len() == b.len() && a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Wait for the wallet worker's answer, but not forever: while it is mid-sync a request should fail fast
/// with 503 rather than hold the connection open.
async fn wallet_reply<T>(rx: oneshot::Receiver<T>) -> Result<T, ApiError> {
    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(_)) => Err(anyhow::anyhow!("wallet worker dropped reply").into()),
        Err(_) => Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "wallet is busy syncing; try again shortly".into())),
    }
}

/// Write endpoints refuse while the service is parked (`FREEZE_HEIGHT` reached or `PAUSED`).
async fn check_live(state: &AppState) -> Result<(), ApiError> {
    let f = state.status.read().await.freeze.clone();
    if f.active {
        return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, format!("service {}; reads still work", f.reason.unwrap_or_default())));
    }
    Ok(())
}

fn check_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let ok = !state.admin_token.is_empty()
        && headers
            .get("x-admin-token")
            .and_then(|v| v.to_str().ok())
            .map(|v| token_eq(v, &state.admin_token))
            .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(ApiError(StatusCode::UNAUTHORIZED, "bad admin token".into()))
    }
}

fn key_hash(key: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(key.trim().as_bytes()))
}

/// Either the admin token or a seller key authorises listing. Returns the seller id if a key was used.
async fn authorise_seller(state: &AppState, headers: &HeaderMap) -> Result<Option<i64>, ApiError> {
    if check_admin(state, headers).is_ok() {
        return Ok(None);
    }
    let key = headers
        .get("x-seller-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "admin token or seller key required".into()))?;
    let conn = state.store.lock().await;
    match store::seller_by_key_hash(&conn, &key_hash(key))? {
        Some((id, _)) => Ok(Some(id)),
        None => Err(ApiError(StatusCode::UNAUTHORIZED, "unknown seller key".into())),
    }
}

async fn create_auction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateAuction>,
) -> ApiResult<store::Auction> {
    let seller_id = authorise_seller(&state, &headers).await?;
    check_live(&state).await?;
    if let Some(sid) = seller_id {
        // A key can run only so many lots at once: each lot derives an address and, if auditable, a wallet account.
        let conn = state.store.lock().await;
        let (open, cap) = (store::open_lots_for_seller(&conn, sid)?, if store::seller_name(&conn, sid)?.as_deref() == Some("demo seller") { 5 } else { 20 });
        if open >= cap {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, format!("this key already has {open} open lots; close one first (limit {cap})")));
        }
    }
    if req.title.trim().is_empty() {
        return Err(bad("title required"));
    }
    if req.deposit_zats == 0 || req.reserve_zats == 0 {
        return Err(bad("reserve_zats and deposit_zats must be > 0"));
    }
    if !state.allow_any_deposit && !crate::denom::is_standard(req.deposit_zats) {
        return Err(bad(format!(
            "deposit_zats must be a standard amount so its refunds look like every other lot's: {} ZEC",
            crate::denom::menu_zec()
        )));
    }
    if !memo::looks_like_shielded_address(&req.seller_address) {
        return Err(bad("seller_address must be a unified address"));
    }
    let close_at = match (req.close_at, req.duration_secs) {
        (Some(c), _) => c,
        (None, Some(d)) if d > 0 => store::now() + d,
        _ => return Err(bad("close_at or duration_secs required")),
    };
    if close_at <= store::now() {
        return Err(bad("close_at must be in the future"));
    }

    let units = req.units.unwrap_or(1).clamp(1, 10_000);
    let format = match req.format.as_deref() {
        None => if units > 1 { "uniform" } else { "first_price" },
        Some("uniform") => "uniform",
        Some(f @ ("first_price" | "vickrey" | "rfq")) if units == 1 => f,
        Some(f @ ("first_price" | "vickrey" | "rfq")) => return Err(bad(format!("{f} needs units = 1"))),
        Some(_) => return Err(bad("format must be first_price, uniform, vickrey or rfq")),
    };
    let format: &'static str = close::FORMATS.iter().copied().find(|f| *f == format).expect("validated");
    if req.deposit_zats > req.reserve_zats {
        return Err(bad("deposit_zats must not exceed reserve_zats"));
    }
    let reveal_secs = req.reveal_secs.unwrap_or(0);
    if reveal_secs != 0 && !(60..=7 * 86_400).contains(&reveal_secs) {
        return Err(bad("reveal_secs must be between 60 and 604800"));
    }
    {
        let tip = state.status.read().await.chain_tip;
        crate::freeze::lot_fits(state.freeze, tip, store::now(), close_at, reveal_secs, state.settlement_window_secs).map_err(bad)?;
    }
    let image_url = match req.image_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) if u.starts_with("https://") && u.len() < 2048 => Some(u.to_string()),
        Some(_) => return Err(bad("image_url must be an https URL")),
        None => None,
    };

    let id = memo::random_auction_id(&mut rand::rng());
    let auditable = req.auditable.unwrap_or(false);
    let (address, di, audit) = if auditable {
        let (tx, rx) = oneshot::channel();
        state
            .wallet
            .send(WalletCmd::NewAuditAccount { name: format!("lot-{id}"), reply: tx })
            .await
            .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
        let a = match wallet_reply(rx).await? {
            Ok(a) => a,
            Err(e) => return Err(bad(format!("{e:#}"))),
        };
        (a.address, String::new(), Some((a.account, a.ufvk, a.birthday)))
    } else {
        let (tx, rx) = oneshot::channel();
        state
            .wallet
            .send(WalletCmd::NewAddress { reply: tx })
            .await
            .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
        let (address, di) = wallet_reply(rx).await??;
        (address, di, None)
    };

    let conn = state.store.lock().await;
    store::insert_auction(
        &conn,
        &store::NewAuction {
            id: &id,
            title: req.title.trim(),
            description: req.description.trim(),
            reserve_zats: req.reserve_zats,
            deposit_zats: req.deposit_zats,
            seller_address: &req.seller_address,
            address: &address,
            diversifier_index: Some(&di),
            close_at,
            seller_id,
            image_url: image_url.as_deref(),
            units,
            format,
            reveal_secs,
            audit: audit.as_ref().map(|(a, k, b)| (a.as_str(), k.as_str(), *b)),
        },
    )?;
    store::insert_event(&conn, Some(&id), "created", &serde_json::json!({ "title": req.title.trim(), "close_at": close_at }))?;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    Ok(Json(auction))
}

#[derive(Serialize)]
struct AuctionView {
    #[serde(flatten)]
    auction: store::Auction,
    bid_count: usize,
    seconds_remaining: i64,
}

async fn get_auction(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> ApiResult<AuctionView> {
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    let bid_count = store::receipts_for_auction(&conn, &id)?.len();
    let seconds_remaining = (auction.close_at - store::now()).max(0);
    Ok(Json(AuctionView {
        auction,
        bid_count,
        seconds_remaining,
    }))
}

#[derive(Serialize)]
struct Receipts {
    auction_id: String,
    receipts: Vec<String>,
}

async fn receipts(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> ApiResult<Receipts> {
    let conn = state.store.lock().await;
    store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    Ok(Json(Receipts {
        auction_id: id.clone(),
        receipts: store::receipts_for_auction(&conn, &id)?,
    }))
}

#[derive(Deserialize)]
struct EnvelopeReq {
    /// Which rail will deliver (informational).
    #[serde(default)]
    rail: String,
    /// A `BB1` bid (open lot) or a `BB1S` settlement (closed lot, matching a winning bid).
    payload: String,
}

#[derive(Serialize)]
struct EnvelopeView {
    /// Fresh unified address this payment must be delivered to. Only money arriving here is this envelope's.
    address: String,
    /// Its transparent receiver, for a rail that cannot deliver shielded (funds are shielded on arrival).
    transparent: Option<String>,
    receipt: String,
    id: String,
}

/// Register a sealed payload for a payment that a swap rail will deliver without a memo. The service
/// derives a fresh address for it; the widget then asks the rail to deliver there. Matching is by
/// address alone, so no client-supplied txid can claim anyone else's deposit.
async fn post_envelope(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<EnvelopeReq>,
) -> ApiResult<EnvelopeView> {
    check_live(&state).await?;
    let parsed = memo::parse(&req.payload).map_err(|e| bad(format!("bad payload: {e}")))?;
    let (auction_id, nonce) = match &parsed {
        memo::Parsed::Bid(b) => (b.auction_id.clone(), b.nonce.clone()),
        memo::Parsed::Commit(c) => (c.auction_id.clone(), c.nonce.clone()),
        memo::Parsed::Settlement(s) => (s.auction_id.clone(), s.nonce.clone()),
    };
    if auction_id != id {
        return Err(bad("payload auction id does not match path"));
    }
    {
        let conn = state.store.lock().await;
        let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
        match &parsed {
            memo::Parsed::Bid(_) | memo::Parsed::Commit(_) if auction.status != "open" => return Err(bad("auction is not open")),
            memo::Parsed::Settlement(s) => {
                if auction.status != "closed" {
                    return Err(bad("nothing to settle on this lot"));
                }
                let won = store::get_bid_by_nonce(&conn, &id, &s.nonce)?.map(|b| b.status == "won").unwrap_or(false);
                if !won {
                    return Err(bad("settlement nonce does not match an unpaid winner"));
                }
            }
            _ => {}
        }
    }
    let (tx, rx) = oneshot::channel();
    state
        .wallet
        .send(WalletCmd::NewReceiveAddress { reply: tx })
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
    let (address, transparent) = wallet_reply(rx).await??;
    let env_id = hex::encode(rand::random::<[u8; 12]>());
    let conn = state.store.lock().await;
    store::insert_envelope(
        &conn,
        &store::Envelope {
            deposit_address: env_id.clone(),
            auction_id: id,
            rail: if req.rail.is_empty() { "unknown".into() } else { req.rail },
            payload: req.payload,
            destination_txid: None,
            created_at: store::now(),
            matched_bid_id: None,
            address: Some(address.clone()),
            transparent: transparent.clone(),
        },
    )?;
    Ok(Json(EnvelopeView { address, transparent, receipt: memo::receipt(&nonce), id: env_id }))
}

#[derive(Serialize)]
struct ResultView {
    auction_id: String,
    status: String,
    format: String,
    units: u32,
    units_sold: usize,
    units_settled: usize,
    clearing_zats: Option<u64>,
    winning_amount_zats: Option<u64>,
    winning_txid: Option<String>,
    winning_txids: Vec<String>,
    ledger_hash: Option<String>,
    valid_bid_count: usize,
    /// Seconds from close until the last losing deposit was refunded; null while refunds are pending.
    refund_latency_secs: Option<i64>,
    refunds_sent: i64,
    refunds_pending: i64,
    settlement_txid: Option<String>,
    payout_txid: Option<String>,
    refunds: Vec<RefundView>,
    disclosure: Option<serde_json::Value>,
    /// Where the clearing price comes from: `own_bid`, `lowest_winning`, `second_bid`, `reserve`, `lowest_quote`.
    price_source: Option<&'static str>,
    /// Sealed-reveal lots: reveal progress and the commitment root fixed at close.
    #[serde(skip_serializing_if = "Option::is_none")]
    sealed: Option<SealedCounts>,
    /// Auditable lots, once closed: the lot's own full viewing key and what to do with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    audit: Option<AuditView>,
    /// Vickrey: the runner-up's ledger leaf (receipt and amount) with its Merkle path, so anyone can check
    /// the price the winner pays is a bid committed at close. The runner-up's identity stays sealed.
    price_proof: Option<PriceProof>,
}

#[derive(Serialize)]
struct AuditView {
    /// Encoded unified full viewing key of this lot's wallet account. Scanning the chain with it from
    /// `birthday` shows every deposit and memo this lot received, and the one sweep out, and nothing else.
    ufvk: String,
    birthday: u32,
    address: String,
    /// Sealed lots: the revealed (receipt, bid, salt) triples, so an auditor can check each commitment.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reveals: Vec<RevealView>,
    command: String,
}

#[derive(Serialize)]
struct RevealView {
    receipt: String,
    bid_zats: u64,
    salt: String,
}

#[derive(Serialize)]
struct SealedCounts {
    sealed: usize,
    revealed: usize,
    unrevealed: usize,
    reveal_until: Option<i64>,
    commit_hash: Option<String>,
}

#[derive(Serialize)]
struct PriceProof {
    receipt: String,
    bid_zats: u64,
    /// The runner-up's memo nonce, needed to recompute their blinded leaf. It reveals nothing else
    /// about them: their secret, refund address and identity stay sealed.
    nonce: String,
    proof: crate::merkle::Proof,
}

#[derive(Serialize)]
struct RefundView {
    kind: String,
    status: String,
    txid: Option<String>,
}

async fn result(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> ApiResult<ResultView> {
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if auction.status == "open" {
        return Err(bad("auction is still open"));
    }
    let bids = store::bids_for_auction(&conn, &id)?;
    let audit = match (&auction.audit_key, auction.status.as_str()) {
        (Some(k), "closed" | "no_sale" | "settled" | "paid" | "unsettled") => Some(AuditView {
            ufvk: k.clone(),
            birthday: auction.audit_birthday.unwrap_or(0),
            address: auction.address.clone(),
            reveals: if auction.sealed() {
                bids.iter()
                    .filter(|b| b.revealed_at.is_some())
                    .filter_map(|b| Some(RevealView { receipt: b.receipt.clone()?, bid_zats: b.bid_zats?, salt: b.salt.clone()? }))
                    .collect()
            } else {
                vec![]
            },
            command: format!(
                "bb-auction audit --ufvk {k} --birthday {} --lot https://HOST/bb/auctions/{} --result https://HOST/bb/auctions/{}/result",
                auction.audit_birthday.unwrap_or(0),
                auction.id,
                auction.id
            ),
        }),
        _ => None,
    };
    let sealed_counts = auction.sealed().then(|| SealedCounts {
        sealed: bids.iter().filter(|b| b.status == "sealed").count(),
        revealed: bids.iter().filter(|b| b.revealed_at.is_some() && b.status != "unrevealed").count(),
        unrevealed: bids.iter().filter(|b| b.status == "unrevealed").count(),
        reveal_until: auction.reveal_until,
        commit_hash: auction.commit_hash.clone(),
    });
    let winners: Vec<&store::Bid> = bids.iter().filter(|b| matches!(b.status.as_str(), "won" | "settled")).collect();
    let winner = winners.first().copied().cloned();
    let units_settled = winners.iter().filter(|b| b.status == "settled").count();
    let valid_bid_count = store::receipts_for_auction(&conn, &id)?.len();
    let (refund_latency_secs, refunds_sent, refunds_pending) = store::refund_latency(&conn, &id)?;
    let refunds = store::payouts_for_auction(&conn, &id)?
        .into_iter()
        .map(|p| RefundView {
            kind: p.kind,
            status: p.status,
            txid: p.txid,
        })
        .collect();
    let disclosure = store::get_disclosure(&conn, &id)?
        .and_then(|s| serde_json::from_str(&s).ok());
    // Recompute the ranking the closer saw: every ledger bid, best first.
    let ledger_bids: Vec<&store::Bid> = bids
        .iter()
        .filter(|b| matches!(b.status.as_str(), "valid" | "won" | "settled" | "forfeited" | "expired"))
        .collect();
    // Forfeited and expired bids stay in the ledger but no longer take part in pricing.
    let taken: Vec<i64> = ledger_bids.iter().filter(|b| matches!(b.status.as_str(), "forfeited" | "expired")).map(|b| b.id).collect();
    let ranked = close::rank(&ledger_bids, &auction.format, auction.reserve_zats, &taken);
    let (price_source, price_proof) = if winners.is_empty() {
        (None, None)
    } else {
        let (_, source) = close::price_for(&auction.format, auction.reserve_zats, &ranked, 0);
        let source = if auction.format == "uniform" { "lowest_winning" } else { source };
        let proof = (source == "second_bid")
            .then(|| ranked.get(1).copied())
            .flatten()
            .and_then(|runner_up| {
                let receipt = runner_up.receipt.clone()?;
                let entries = store::ledger_entries(&conn, &id).ok()?;
                let proof = crate::merkle::prove(&entries, &receipt)?;
                Some(PriceProof { receipt, bid_zats: runner_up.bid_zats?, nonce: runner_up.nonce.clone()?, proof })
            });
        (Some(source), proof)
    };
    Ok(Json(ResultView {
        auction_id: id,
        status: auction.status,
        format: auction.format.clone(),
        units: auction.units,
        units_sold: winners.len(),
        units_settled,
        clearing_zats: auction.clearing_zats,
        winning_amount_zats: auction.clearing_zats,
        winning_txid: winner.as_ref().map(|w| w.txid.clone()),
        winning_txids: winners.iter().map(|w| w.txid.clone()).collect(),
        ledger_hash: auction.ledger_hash,
        valid_bid_count,
        refund_latency_secs,
        refunds_sent,
        refunds_pending,
        settlement_txid: auction.settlement_txid,
        payout_txid: auction.payout_txid,
        refunds,
        disclosure,
        price_source,
        sealed: sealed_counts,
        audit,
        price_proof,
    }))
}

#[derive(Deserialize)]
struct BidUriQuery {
    bid_zats: u64,
    refund_address: String,
    nonce: Option<String>,
}

#[derive(Serialize)]
struct BidUri {
    uri: String,
    memo: String,
    nonce: String,
    receipt: String,
    deposit_zats: u64,
    address: String,
}

/// Server-side ZIP-321 builder, mainly for testing with real wallets before the web app exists.
async fn bid_uri(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<BidUriQuery>,
) -> ApiResult<BidUri> {
    check_live(&state).await?;
    if !memo::looks_like_shielded_address(&q.refund_address) {
        return Err(bad("refund_address must be a unified address"));
    }
    let nonce = match q.nonce {
        Some(n) if memo::is_nonce(&n) => n,
        Some(_) => return Err(bad("nonce must be 16 lowercase hex chars")),
        None => memo::random_nonce(&mut rand::rng()),
    };
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    drop(conn);
    if auction.status != "open" {
        return Err(bad("auction is not open"));
    }
    let bid = memo::Bid {
        auction_id: id,
        bid_zats: q.bid_zats,
        refund_address: q.refund_address,
        nonce: nonce.clone(),
    };
    let memo_str = memo::encode_bid(&bid);
    let request = crate::wallet::send::build_request(&crate::wallet::send::Outgoing {
        address: &auction.address,
        zats: auction.deposit_zats,
        memo: Some(&memo_str),
    })?;
    Ok(Json(BidUri {
        uri: request.to_uri(),
        memo: memo_str,
        receipt: memo::receipt(&nonce),
        nonce,
        deposit_zats: auction.deposit_zats,
        address: auction.address,
    }))
}

#[derive(Deserialize)]
struct AdminPayout {
    to_address: String,
    value_zats: u64,
    memo: Option<String>,
}

/// Operator-initiated send (fund a test wallet, return funds). Goes through the payout queue.
async fn admin_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AdminPayout>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    check_live(&state).await?;
    if req.value_zats == 0 {
        return Err(bad("value_zats must be > 0"));
    }
    let conn = state.store.lock().await;
    let id = store::queue_payout(
        &conn,
        None,
        None,
        "manual",
        &req.to_address,
        req.value_zats,
        req.memo.as_deref(),
        store::now(),
    )?;
    Ok(Json(serde_json::json!({ "payout_id": id })))
}

async fn admin_list_payouts(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<Vec<store::Payout>> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    Ok(Json(store::all_payouts(&conn)?))
}

#[derive(Deserialize)]
struct ScanTx {
    txid: String,
}

/// Debug aid: fetch a transaction by id and trial-decrypt it into the wallet.
async fn admin_scan_tx(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ScanTx>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    let (tx, rx) = oneshot::channel();
    state
        .wallet
        .send(WalletCmd::ScanTx { txid: req.txid.trim().to_lowercase(), reply: tx })
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
    let detail = rx.await.map_err(|_| anyhow::anyhow!("wallet worker dropped reply"))??;
    Ok(Json(serde_json::json!({ "detail": detail })))
}

async fn admin_block(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(height): Path<u32>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    let (tx, rx) = oneshot::channel();
    state
        .wallet
        .send(WalletCmd::InspectBlock { height, reply: tx })
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
    Ok(Json(rx.await.map_err(|_| anyhow::anyhow!("wallet worker dropped reply"))??))
}

#[derive(Deserialize)]
struct CreateSeller {
    name: String,
}

/// Admin creates a seller; the key is returned once and stored hashed.
async fn admin_create_seller(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateSeller>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    if req.name.trim().is_empty() {
        return Err(bad("name required"));
    }
    let mut raw = [0u8; 24];
    rand::Rng::fill_bytes(&mut rand::rng(), &mut raw);
    let key = format!("bbs_{}", hex::encode(raw));
    let conn = state.store.lock().await;
    let id = store::insert_seller(&conn, req.name.trim(), &key_hash(&key))?;
    Ok(Json(serde_json::json!({ "seller_id": id, "name": req.name.trim(), "key": key })))
}

async fn admin_retry_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    if store::requeue_payout(&conn, id)? {
        Ok(Json(serde_json::json!({ "ok": true })))
    } else {
        Err(bad("payout is not in a retryable state"))
    }
}

#[derive(Serialize)]
struct Reconcile {
    #[serde(flatten)]
    totals: store::Totals,
    wallet_balance_zats: Option<u64>,
    /// deposits + settlements - payouts sent; fees make the wallet slightly lower than this.
    expected_balance_zats: i64,
    discrepancy_zats: Option<i64>,
}

async fn admin_reconcile(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<Reconcile> {
    check_admin(&state, &headers)?;
    let totals = {
        let conn = state.store.lock().await;
        store::totals(&conn)?
    };
    let balance = state.status.read().await.balance_zats;
    let expected = totals.deposits_received_zats as i64 + totals.settlements_received_zats as i64
        - totals.payouts_sent_zats as i64;
    Ok(Json(Reconcile {
        discrepancy_zats: balance.map(|b| b as i64 - expected),
        totals,
        wallet_balance_zats: balance,
        expected_balance_zats: expected,
    }))
}

async fn auction_events(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> ApiResult<Vec<store::Event>> {
    let conn = state.store.lock().await;
    store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    // Public view: hide receipts of rejected bids and payout ids.
    let events = store::events_for_auction(&conn, &id)?
        .into_iter()
        .filter(|e| !matches!(e.kind.as_str(), "bid_rejected" | "payout_dead"))
        .collect();
    Ok(Json(events))
}

#[derive(Serialize)]
struct AdminAuction {
    #[serde(flatten)]
    auction: store::Auction,
    seller_address: String,
}

/// Full auction definitions including the seller address, for a spender process.
async fn admin_auctions(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<Vec<AdminAuction>> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    Ok(Json(
        store::list_auctions(&conn)?
            .into_iter()
            .map(|a| AdminAuction { seller_address: a.seller_address.clone(), auction: a })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct AfterQuery {
    #[serde(default)]
    after: i64,
}

fn check_ledger(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if check_admin(state, headers).is_ok() {
        return Ok(());
    }
    let ok = match &state.ledger_token {
        Some(t) if !t.is_empty() => headers
            .get("x-ledger-token")
            .and_then(|v| v.to_str().ok())
            .map(|v| token_eq(v, t))
            .unwrap_or(false),
        _ => false,
    };
    if ok { Ok(()) } else { Err(ApiError(StatusCode::UNAUTHORIZED, "ledger token required".into())) }
}

#[derive(Deserialize)]
struct LedgerQuery {
    #[serde(default)]
    after: i64,
    kind: Option<String>,
}

/// The payouts this ledger has derived (refunds to sealed refund addresses, seller payouts, forfeits, manual
/// operator sends), oldest first after `after`. Signers match every output they are asked to sign against this
/// list; the spender mirrors manual entries. Read-only.
async fn ledger_payouts(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(q): Query<LedgerQuery>) -> ApiResult<Vec<crate::frost::ExpectedPayout>> {
    check_ledger(&state, &headers)?;
    let conn = state.store.lock().await;
    Ok(Json(
        store::payouts_after_id(&conn, q.after, 500)?
            .into_iter()
            .filter(|p| q.kind.as_deref().map(|k| k == p.kind).unwrap_or(true))
            .map(|p| crate::frost::ExpectedPayout { id: p.id, auction_id: p.auction_id, kind: p.kind, to_address: p.to_address, value_zats: p.value_zats })
            .collect(),
    ))
}

/// Envelopes created after `after` (unix seconds), for a spender process to mirror.
async fn admin_envelopes(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(q): Query<AfterQuery>) -> ApiResult<Vec<store::Envelope>> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    Ok(Json(store::envelopes_after(&conn, q.after, 500)?))
}

#[derive(Deserialize)]
struct SentPayout {
    auction_id: Option<String>,
    kind: String,
    bid_txid: Option<String>,
    to_address: String,
    value_zats: u64,
    txid: String,
}

/// A spender reports payouts it executed; the viewer marks its matching rows sent.
async fn admin_payouts_sync(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(items): Json<Vec<SentPayout>>,
) -> ApiResult<serde_json::Value> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    let mut matched = 0;
    for p in &items {
        if store::mark_payout_sent_by_match(
            &conn,
            p.auction_id.as_deref(),
            &p.kind,
            p.bid_txid.as_deref(),
            &p.to_address,
            p.value_zats,
            &p.txid,
        )? {
            matched += 1;
            if p.kind == "seller" {
                if let Some(aid) = &p.auction_id {
                    store::set_auction_payout(&conn, aid, &p.txid)?;
                }
            }
        }
    }
    Ok(Json(serde_json::json!({ "received": items.len(), "matched": matched })))
}

/// A fresh receive address for a Go Dark payment: unified address plus its transparent receiver so a
/// rail that cannot deliver shielded can still pay (the service shields on arrival).
async fn godark_receive_address(State(state): State<Arc<AppState>>) -> ApiResult<serde_json::Value> {
    check_live(&state).await?;
    let (tx, rx) = oneshot::channel();
    state
        .wallet
        .send(WalletCmd::NewReceiveAddress { reply: tx })
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
    let (ua, transparent) = wallet_reply(rx).await??;
    Ok(Json(serde_json::json!({ "ua": ua, "transparent": transparent })))
}

#[derive(Deserialize)]
struct ProofQuery {
    receipt: String,
}

#[derive(Serialize)]
struct ProofView {
    auction_id: String,
    ledger_hash: String,
    leaf_count: usize,
    proof: crate::merkle::Proof,
}

/// Inclusion proof of one receipt in the ledger committed at close. The bidder supplies the bid amount
/// and their nonce client-side to verify; the leaf is blinded by the nonce, so the server reveals no amount.
async fn inclusion_proof(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ProofQuery>,
) -> ApiResult<ProofView> {
    let receipt = q.receipt.trim().to_lowercase();
    if receipt.len() != 64 || !receipt.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad("receipt must be 64 hex characters"));
    }
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if auction.status == "open" {
        return Err(bad("the ledger is committed at close"));
    }
    let entries = store::ledger_entries(&conn, &id)?;
    let proof = crate::merkle::prove(&entries, &receipt).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "receipt not in the ledger".into()))?;
    let ledger_hash = auction.ledger_hash.clone().unwrap_or_default();
    if proof.root != ledger_hash {
        return Err(anyhow::anyhow!("ledger root drifted from the committed hash for {id}").into());
    }
    Ok(Json(ProofView { auction_id: id, ledger_hash, leaf_count: proof.leaf_count, proof }))
}

/// Seller-key auth only (no admin fallback): returns (seller_id, name).
/// A seller key identifies one seller. The admin token also opens the seller pages, as seller 0 ("operator"),
/// which sees every lot; so the person running the instance never needs to mint a key just to look.
async fn require_seller(state: &AppState, headers: &HeaderMap) -> Result<(i64, String), ApiError> {
    if check_admin(state, headers).is_ok() {
        return Ok((0, "operator".into()));
    }
    let key = headers
        .get("x-seller-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "seller key required".into()))?;
    let conn = state.store.lock().await;
    store::seller_by_key_hash(&conn, &key_hash(key))?
        .ok_or_else(|| ApiError(StatusCode::UNAUTHORIZED, "unknown seller key".into()))
}

/// On a test network anyone can mint a demo seller key, so a visitor can list a lot without the operator.
async fn demo_seller(State(state): State<Arc<AppState>>) -> ApiResult<serde_json::Value> {
    if state.network_name == "main" {
        return Err(ApiError(StatusCode::FORBIDDEN, "demo keys are only issued on test networks".into()));
    }
    let raw: [u8; 24] = rand::random();
    let key = format!("bbs_{}", hex::encode(raw));
    let conn = state.store.lock().await;
    let id = store::insert_seller(&conn, "demo seller", &key_hash(&key))?;
    Ok(Json(serde_json::json!({ "seller_id": id, "name": "demo seller", "key": key })))
}

async fn seller_me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<serde_json::Value> {
    let (id, name) = require_seller(&state, &headers).await?;
    Ok(Json(serde_json::json!({ "seller_id": id, "name": name })))
}

#[derive(Serialize)]
struct SellerLot {
    #[serde(flatten)]
    auction: store::Auction,
    seller_address: String,
    money: store::LotMoney,
    fulfilment: Option<String>,
}

async fn seller_lots(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<Vec<SellerLot>> {
    let (sid, _) = require_seller(&state, &headers).await?;
    let conn = state.store.lock().await;
    let mut out = Vec::new();
    let lots = if sid == 0 { store::list_auctions(&conn)? } else { store::auctions_for_seller(&conn, sid)? };
    for a in lots {
        let money = store::lot_money(&conn, &a.id)?;
        let fulfilment = store::get_fulfilment(&conn, &a.id)?.map(|(n, _)| n);
        out.push(SellerLot { seller_address: a.seller_address.clone(), auction: a, money, fulfilment });
    }
    Ok(Json(out))
}

/// A payout as the seller may see it: no destination address, since losers' refund addresses are theirs alone.
#[derive(Serialize)]
struct SellerPayout {
    id: i64,
    kind: String,
    status: String,
    value_zats: u64,
    txid: Option<String>,
    not_before: i64,
}

#[derive(Serialize)]
struct SellerLotDetail {
    #[serde(flatten)]
    lot: SellerLot,
    events: Vec<store::Event>,
    payouts: Vec<SellerPayout>,
    /// RFQ only, once closed: how the buyer pays the winning quote. The memo carries the winning
    /// quote's nonce, which only the buyer (here) and the quoter know.
    #[serde(skip_serializing_if = "Option::is_none")]
    settlement: Option<SettlementView>,
}

#[derive(Serialize)]
struct SettlementView {
    uri: String,
    memo: String,
    amount_zats: u64,
    address: String,
    settle_by: Option<i64>,
}

async fn seller_lot(State(state): State<Arc<AppState>>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<SellerLotDetail> {
    let (sid, _) = require_seller(&state, &headers).await?;
    let conn = state.store.lock().await;
    let a = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if sid != 0 && a.seller_id != Some(sid) {
        return Err(not_found());
    }
    let money = store::lot_money(&conn, &a.id)?;
    let fulfilment = store::get_fulfilment(&conn, &a.id)?.map(|(n, _)| n);
    // Receipts identify bidders to themselves; the seller does not need them.
    let events = store::events_for_auction(&conn, &id)?
        .into_iter()
        .map(|mut e| {
            if let Some(obj) = e.payload.as_object_mut() {
                obj.remove("receipt");
                obj.remove("bid_receipt");
            }
            e
        })
        .collect();
    let payouts = store::payouts_for_auction(&conn, &id)?
        .into_iter()
        .map(|p| SellerPayout { id: p.id, kind: p.kind, status: p.status, value_zats: p.value_zats, txid: p.txid, not_before: p.not_before })
        .collect();
    let settlement = if close::is_rfq(&a.format) && a.status == "closed" {
        store::bids_for_auction(&conn, &id)?
            .into_iter()
            .find(|b| b.status == "won")
            .and_then(|w| {
                let nonce = w.nonce.clone()?;
                let memo_str = memo::encode_settlement(&memo::Settlement { auction_id: id.clone(), nonce });
                let amount_zats = w.price_zats?;
                let request = crate::wallet::send::build_request(&crate::wallet::send::Outgoing {
                    address: &a.address,
                    zats: amount_zats,
                    memo: Some(&memo_str),
                })
                .ok()?;
                Some(SettlementView { uri: request.to_uri(), memo: memo_str, amount_zats, address: a.address.clone(), settle_by: w.settle_by })
            })
    } else {
        None
    };
    Ok(Json(SellerLotDetail {
        lot: SellerLot { seller_address: a.seller_address.clone(), auction: a, money, fulfilment },
        events,
        payouts,
        settlement,
    }))
}

#[derive(Deserialize)]
struct FulfilmentReq {
    /// Delivery instructions for the winner: a download link, a shipping handshake, a contact. Shown only to
    /// a bidder who can present a winning receipt. Keep it under 2000 characters.
    note: String,
}

async fn seller_set_fulfilment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<FulfilmentReq>,
) -> ApiResult<serde_json::Value> {
    let (sid, _) = require_seller(&state, &headers).await?;
    let note = req.note.trim();
    if note.is_empty() || note.len() > 2000 {
        return Err(bad("note must be 1 to 2000 characters"));
    }
    let conn = state.store.lock().await;
    let a = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if sid != 0 && a.seller_id != Some(sid) {
        return Err(not_found());
    }
    store::set_fulfilment(&conn, &id, a.seller_id.unwrap_or(sid), note)?;
    store::insert_event(&conn, Some(&id), "fulfilment_posted", &serde_json::json!({}))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct SecretQuery {
    /// The bidder secret the browser generated with the bid. The memo nonce is sha256(secret)[..16],
    /// and the receipt is sha256(nonce). Receipts are public and the nonce may become public if the
    /// winning memo is disclosed, so only the secret authenticates the bidder.
    secret: String,
}

/// Derive the memo nonce from a bidder secret, rejecting malformed input.
fn nonce_param(raw: &str) -> Result<String, ApiError> {
    let s = raw.trim().to_lowercase();
    if !memo::is_secret(&s) {
        return Err(bad("secret must be 32 lowercase hex chars"));
    }
    Ok(memo::nonce_from_secret(&s))
}

/// The seller's delivery note, visible only to a winner who presents their bidder secret.
async fn winner_fulfilment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(q): Json<SecretQuery>,
) -> ApiResult<serde_json::Value> {
    let nonce = nonce_param(&q.secret)?;
    let conn = state.store.lock().await;
    store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    let receipt = memo::receipt(&nonce);
    if !store::receipt_is_winner(&conn, &id, &receipt)? {
        return Err(ApiError(StatusCode::FORBIDDEN, "that bid did not win this lot".into()));
    }
    let f = store::get_fulfilment(&conn, &id)?;
    Ok(Json(serde_json::json!({ "note": f.as_ref().map(|x| x.0.clone()), "updated_at": f.map(|x| x.1) })))
}

#[derive(Serialize)]
struct Standing {
    receipt: String,
    /// Raw bid status: valid, won, settled, forfeited, expired, late, wrong_deposit, duplicate_nonce, malformed.
    status: String,
    /// What it means for the bidder: pending, won, settled, lost, forfeited, expired, rejected.
    outcome: &'static str,
    format: String,
    deposit_zats: u64,
    /// Total the bidder owes if they won (deposit included). Null until the lot closes.
    price_zats: Option<u64>,
    /// What is still to pay: price minus deposit. Zero once settled or if the deposit covered it.
    owed_zats: u64,
    settle_by: Option<i64>,
    /// How to pay the balance: a ZIP-321 URI with the settlement memo. Only for a winner who still owes.
    #[serde(skip_serializing_if = "Option::is_none")]
    settlement: Option<SettlementView>,
    /// Refund or payout state for this bid, if any payout exists.
    payouts: Vec<store::Payout>,
    /// The seller's delivery note, for winners only.
    #[serde(skip_serializing_if = "Option::is_none")]
    fulfilment: Option<String>,
    /// Sealed-reveal: when the reveal window ends (while the lot is revealing).
    #[serde(skip_serializing_if = "Option::is_none")]
    reveal_until: Option<i64>,
}

/// Everything one bidder needs to know about their own bid, authenticated by the bidder secret.
/// POST, not GET: the secret must not land in proxy or browser history logs as a query string.
async fn standing(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(q): Json<SecretQuery>) -> ApiResult<Standing> {
    let nonce = nonce_param(&q.secret)?;
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    let bid = store::get_bid_by_nonce(&conn, &id, &nonce)?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "no bid with that nonce has arrived yet".into()))?;
    let rfq = close::is_rfq(&auction.format);
    let outcome: &'static str = match bid.status.as_str() {
        "sealed" if auction.status == "revealing" => "reveal_now",
        "sealed" => "sealed",
        "unrevealed" => "unrevealed",
        "valid" if auction.status == "open" => "pending",
        "valid" if auction.status == "revealing" => "revealed",
        "valid" => "lost",
        "won" => "won",
        "settled" => "settled",
        "forfeited" => "forfeited",
        "expired" => "expired",
        _ => "rejected",
    };
    let owed_zats = match (bid.status.as_str(), bid.price_zats) {
        ("won", Some(p)) if !rfq => p.saturating_sub(bid.value_zats),
        _ => 0,
    };
    let settlement = if owed_zats > 0 {
        let memo_str = memo::encode_settlement(&memo::Settlement { auction_id: id.clone(), nonce: nonce.clone() });
        crate::wallet::send::build_request(&crate::wallet::send::Outgoing { address: &auction.address, zats: owed_zats, memo: Some(&memo_str) })
            .ok()
            .map(|r| SettlementView { uri: r.to_uri(), memo: memo_str, amount_zats: owed_zats, address: auction.address.clone(), settle_by: bid.settle_by })
    } else {
        None
    };
    let payouts = store::payouts_for_auction(&conn, &id)?.into_iter().filter(|p| p.bid_id == Some(bid.id)).collect();
    let fulfilment = if matches!(bid.status.as_str(), "won" | "settled") {
        store::get_fulfilment(&conn, &id)?.map(|(n, _)| n)
    } else {
        None
    };
    Ok(Json(Standing {
        receipt: bid.receipt.clone().unwrap_or_else(|| memo::receipt(&nonce)),
        status: bid.status.clone(),
        outcome,
        format: auction.format.clone(),
        deposit_zats: bid.value_zats,
        price_zats: bid.price_zats,
        owed_zats,
        settle_by: bid.settle_by,
        settlement,
        payouts,
        fulfilment,
        reveal_until: if auction.status == "revealing" { auction.reveal_until } else { None },
    }))
}

#[derive(Deserialize)]
struct RevealReq {
    secret: String,
    bid_zats: u64,
    salt: String,
}

/// Reveal a sealed bid during the reveal window. The commitment in the memo must equal
/// sha256("<auction>|<bid>|<salt>"); the secret proves the reveal comes from the bidder.
async fn reveal(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<RevealReq>) -> ApiResult<serde_json::Value> {
    let nonce = nonce_param(&req.secret)?;
    let salt = req.salt.trim().to_lowercase();
    if !memo::is_salt(&salt) {
        return Err(bad("salt must be 32 lowercase hex chars"));
    }
    if req.bid_zats == 0 {
        return Err(bad("bid_zats must be above zero"));
    }
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if !auction.sealed() {
        return Err(bad("this lot does not use sealed bids"));
    }
    if auction.status != "revealing" {
        return Err(bad(if auction.status == "open" { "the lot is still open; reveal after it closes" } else { "the reveal window has ended" }));
    }
    let bid = store::get_bid_by_nonce(&conn, &id, &nonce)?.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "no sealed bid with that key".into()))?;
    if bid.status != "sealed" {
        return Err(bad("this bid is already revealed or was not accepted"));
    }
    let expected = bid.commitment.clone().unwrap_or_default();
    if memo::commitment(&id, req.bid_zats, &salt) != expected {
        return Err(bad("bid and salt do not match the commitment in your memo"));
    }
    store::reveal_bid(&conn, bid.id, req.bid_zats, &salt)?;
    store::insert_event(&conn, Some(&id), "bid_revealed", &serde_json::json!({ "receipt": bid.receipt }))?;
    Ok(Json(serde_json::json!({ "ok": true, "receipt": bid.receipt })))
}

fn valid_email(e: &str) -> bool {
    let e = e.trim();
    e.len() >= 6 && e.len() <= 254 && e.contains('@') && !e.contains(' ') && e.rsplit('@').next().map(|d| d.contains('.')).unwrap_or(false)
}

#[derive(Deserialize)]
struct NotifyReq {
    secret: String,
    email: String,
}

#[derive(Deserialize)]
struct NotifyKey {
    secret: String,
}

/// Opt in to email for one bid: received, reveal window, won or lost, refund sent. The address is tied to the
/// bid's nonce, never returned by any endpoint, and purged a week after the lot ends.
async fn notify_bidder(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<NotifyReq>) -> ApiResult<serde_json::Value> {
    let nonce = nonce_param(&req.secret)?;
    if !valid_email(&req.email) {
        return Err(bad("that does not look like an email address"));
    }
    let conn = state.store.lock().await;
    store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    let email = req.email.trim();
    if store::unconfirmed_count(&conn, email)? >= 3 {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "that address has unconfirmed requests already; confirm one from your inbox first".into()));
    }
    let sub = store::subscribe_bidder(&conn, &id, &nonce, email)?;
    let pending = sub.confirm_token.is_some();
    if let Some(token) = &sub.confirm_token {
        crate::notify::queue_confirmation(&conn, sub.id, token, &state.site_url, "your bid")?;
    }
    Ok(Json(serde_json::json!({ "ok": true, "confirm_pending": pending })))
}

#[derive(Deserialize)]
struct ConfirmReq {
    token: String,
}

/// The link in a confirmation mail lands here (through the site's /notify/confirm page).
async fn confirm_notify(State(state): State<Arc<AppState>>, Json(req): Json<ConfirmReq>) -> ApiResult<serde_json::Value> {
    if req.token.len() != 32 || !req.token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad("bad token"));
    }
    let conn = state.store.lock().await;
    let ok = store::confirm_subscription(&conn, &req.token)?;
    if !ok {
        return Err(ApiError(StatusCode::NOT_FOUND, "that link is not valid any more".into()));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn unnotify_bidder(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<NotifyKey>) -> ApiResult<serde_json::Value> {
    let nonce = nonce_param(&req.secret)?;
    let conn = state.store.lock().await;
    let removed = store::unsubscribe_bidder(&conn, &id, &nonce)?;
    Ok(Json(serde_json::json!({ "ok": true, "removed": removed })))
}

#[derive(Deserialize)]
struct SellerNotifyReq {
    /// Empty or absent clears it.
    email: Option<String>,
}

async fn seller_notify_get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult<serde_json::Value> {
    let (sid, _) = require_seller(&state, &headers).await?;
    let conn = state.store.lock().await;
    Ok(Json(serde_json::json!({ "subscribed": sid != 0 && store::seller_has_subscription(&conn, sid)? })))
}

async fn seller_notify_set(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(req): Json<SellerNotifyReq>) -> ApiResult<serde_json::Value> {
    let (sid, _) = require_seller(&state, &headers).await?;
    if sid == 0 {
        return Err(bad("the operator account has no seller subscription; use WEBHOOK_URL"));
    }
    let email = req.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    if let Some(e) = email {
        if !valid_email(e) {
            return Err(bad("that does not look like an email address"));
        }
    }
    let conn = state.store.lock().await;
    if let Some(e) = email {
        if store::unconfirmed_count(&conn, e)? >= 3 {
            return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "that address has unconfirmed requests already; confirm one from your inbox first".into()));
        }
    }
    let sub = store::subscribe_seller(&conn, sid, email)?;
    let pending = sub.as_ref().and_then(|s| s.confirm_token.as_ref()).is_some();
    if let Some(s) = &sub {
        if let Some(token) = &s.confirm_token {
            crate::notify::queue_confirmation(&conn, s.id, token, &state.site_url, "your lots")?;
        }
    }
    Ok(Json(serde_json::json!({ "ok": true, "subscribed": email.is_some(), "confirm_pending": pending })))
}

/// Reveals since `after`, for the spender to mirror (it re-checks every commitment itself).
async fn admin_reveals(State(state): State<Arc<AppState>>, headers: HeaderMap, Query(q): Query<AfterQuery>) -> ApiResult<Vec<serde_json::Value>> {
    check_admin(&state, &headers)?;
    let conn = state.store.lock().await;
    Ok(Json(
        store::reveals_after(&conn, q.after, 500)?
            .into_iter()
            .map(|b| serde_json::json!({ "auction_id": b.auction_id, "nonce": b.nonce, "bid_zats": b.bid_zats, "salt": b.salt, "revealed_at": b.revealed_at }))
            .collect(),
    ))
}
