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
    memo,
    state::{AppState, WalletCmd},
    store,
};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auctions", get(list_auctions).post(create_auction))
        .route("/auctions/{id}", get(get_auction))
        .route("/auctions/{id}/receipts", get(receipts))
        .route("/auctions/{id}/envelopes", post(post_envelope))
        .route("/auctions/{id}/result", get(result))
        .route("/auctions/{id}/bid-uri", get(bid_uri))
        .layer(CorsLayer::permissive())
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
}

async fn health(State(state): State<Arc<AppState>>) -> ApiResult<Health> {
    let s = state.status.read().await.clone();
    Ok(Json(Health {
        network: state.network_name,
        chain_tip: s.chain_tip,
        wallet_height: s.wallet_height,
        sync_lag: s.chain_tip.zip(s.wallet_height).map(|(t, w)| t.saturating_sub(w)),
        last_sync_at: s.last_sync_at,
        last_error: s.last_error,
        operator_address: s.operator_address,
        balance_zats: s.balance_zats,
    }))
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
}

fn check_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let ok = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == state.admin_token)
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(ApiError(StatusCode::UNAUTHORIZED, "bad admin token".into()))
    }
}

async fn create_auction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateAuction>,
) -> ApiResult<store::Auction> {
    check_admin(&state, &headers)?;
    if req.title.trim().is_empty() {
        return Err(bad("title required"));
    }
    if req.deposit_zats == 0 || req.reserve_zats == 0 {
        return Err(bad("reserve_zats and deposit_zats must be > 0"));
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

    let (tx, rx) = oneshot::channel();
    state
        .wallet
        .send(WalletCmd::NewAddress { reply: tx })
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker gone"))?;
    let (address, di) = rx
        .await
        .map_err(|_| anyhow::anyhow!("wallet worker dropped reply"))??;

    let id = memo::random_auction_id(&mut rand::thread_rng());
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
        },
    )?;
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
    deposit_address: String,
    rail: String,
    /// The full `BB1|...` memo string the bidder would have put on chain.
    payload: String,
    destination_txid: Option<String>,
}

async fn post_envelope(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<EnvelopeReq>,
) -> ApiResult<serde_json::Value> {
    let parsed = memo::parse(&req.payload).map_err(|e| bad(format!("bad payload: {e}")))?;
    let memo::Parsed::Bid(bid) = parsed else {
        return Err(bad("payload must be a BB1 bid"));
    };
    if bid.auction_id != id {
        return Err(bad("payload auction id does not match path"));
    }
    if req.deposit_address.len() < 8 || req.rail.is_empty() {
        return Err(bad("deposit_address and rail required"));
    }
    let conn = state.store.lock().await;
    let auction = store::get_auction(&conn, &id)?.ok_or_else(not_found)?;
    if auction.status != "open" {
        return Err(bad("auction is not open"));
    }
    store::upsert_envelope(
        &conn,
        &req.deposit_address,
        &id,
        &req.rail,
        &req.payload,
        req.destination_txid.as_deref(),
    )?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "receipt": memo::receipt(&bid.nonce),
    })))
}

#[derive(Serialize)]
struct ResultView {
    auction_id: String,
    status: String,
    winning_amount_zats: Option<u64>,
    winning_txid: Option<String>,
    ledger_hash: Option<String>,
    valid_bid_count: usize,
    settlement_txid: Option<String>,
    payout_txid: Option<String>,
    refunds: Vec<RefundView>,
    disclosure: Option<serde_json::Value>,
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
    let winner = match auction.winner_bid_id {
        Some(wid) => store::get_bid(&conn, wid)?,
        None => None,
    };
    let valid_bid_count = store::receipts_for_auction(&conn, &id)?.len();
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
    Ok(Json(ResultView {
        auction_id: id,
        status: auction.status,
        winning_amount_zats: winner.as_ref().and_then(|w| w.bid_zats),
        winning_txid: winner.as_ref().map(|w| w.txid.clone()),
        ledger_hash: auction.ledger_hash,
        valid_bid_count,
        settlement_txid: auction.settlement_txid,
        payout_txid: auction.payout_txid,
        refunds,
        disclosure,
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
    if !memo::looks_like_shielded_address(&q.refund_address) {
        return Err(bad("refund_address must be a unified address"));
    }
    let nonce = match q.nonce {
        Some(n) if memo::is_nonce(&n) => n,
        Some(_) => return Err(bad("nonce must be 16 lowercase hex chars")),
        None => memo::random_nonce(&mut rand::thread_rng()),
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
