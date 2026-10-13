//! Service-owned SQLite tables (separate file from the wallet DB).

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS auctions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            reserve_zats INTEGER NOT NULL,
            deposit_zats INTEGER NOT NULL,
            seller_address TEXT NOT NULL,
            address TEXT NOT NULL,
            diversifier_index TEXT,
            created_at INTEGER NOT NULL,
            close_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            closed_at INTEGER,
            winner_bid_id INTEGER,
            ledger_hash TEXT,
            disclosure_json TEXT,
            settlement_txid TEXT,
            payout_txid TEXT,
            settle_by INTEGER,
            seller_id INTEGER,
            image_url TEXT
         );
         CREATE TABLE IF NOT EXISTS bids (
            id INTEGER PRIMARY KEY,
            auction_id TEXT,
            txid TEXT NOT NULL,
            output_index INTEGER NOT NULL,
            pool INTEGER NOT NULL,
            mined_height INTEGER,
            value_zats INTEGER NOT NULL,
            memo TEXT,
            bid_zats INTEGER,
            refund_address TEXT,
            nonce TEXT,
            receipt TEXT,
            status TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'memo',
            received_at INTEGER NOT NULL,
            UNIQUE (txid, output_index)
         );
         CREATE INDEX IF NOT EXISTS bids_auction ON bids (auction_id, status);
         CREATE TABLE IF NOT EXISTS envelopes (
            deposit_address TEXT PRIMARY KEY,
            auction_id TEXT NOT NULL,
            rail TEXT NOT NULL,
            payload TEXT NOT NULL,
            destination_txid TEXT,
            created_at INTEGER NOT NULL,
            matched_bid_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS sellers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            key_hash TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            auction_id TEXT,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            delivered_at INTEGER,
            attempts INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS payouts (
            id INTEGER PRIMARY KEY,
            auction_id TEXT,
            bid_id INTEGER,
            kind TEXT NOT NULL,
            to_address TEXT NOT NULL,
            value_zats INTEGER NOT NULL,
            memo TEXT,
            not_before INTEGER NOT NULL,
            txid TEXT,
            status TEXT NOT NULL DEFAULT 'queued',
            attempts INTEGER NOT NULL DEFAULT 0,
            error TEXT,
            created_at INTEGER NOT NULL,
            sent_at INTEGER
         );",
    )?;
    for stmt in [
        "ALTER TABLE auctions ADD COLUMN settle_by INTEGER",
        "ALTER TABLE auctions ADD COLUMN seller_id INTEGER",
        "ALTER TABLE auctions ADD COLUMN image_url TEXT",
    ] {
        // Ignore "duplicate column" on databases that already have it.
        let _ = conn.execute(stmt, []);
    }
    Ok(())
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    init_schema(&conn)?;
    Ok(conn)
}

pub fn now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Auction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub reserve_zats: u64,
    pub deposit_zats: u64,
    #[serde(default, skip_serializing)]
    pub seller_address: String,
    pub address: String,
    pub created_at: i64,
    pub close_at: i64,
    pub status: String,
    pub closed_at: Option<i64>,
    #[serde(default, skip_serializing)]
    pub winner_bid_id: Option<i64>,
    pub ledger_hash: Option<String>,
    pub settlement_txid: Option<String>,
    pub payout_txid: Option<String>,
    #[serde(default)]
    pub settle_by: Option<i64>,
    #[serde(default, skip_serializing)]
    pub seller_id: Option<i64>,
    #[serde(default)]
    pub image_url: Option<String>,
}

const AUCTION_COLS: &str = "id, title, description, reserve_zats, deposit_zats, seller_address, address,
    created_at, close_at, status, closed_at, winner_bid_id, ledger_hash, settlement_txid, payout_txid,
    settle_by, seller_id, image_url";

fn row_to_auction(r: &rusqlite::Row<'_>) -> rusqlite::Result<Auction> {
    Ok(Auction {
        id: r.get(0)?,
        title: r.get(1)?,
        description: r.get(2)?,
        reserve_zats: r.get::<_, i64>(3)? as u64,
        deposit_zats: r.get::<_, i64>(4)? as u64,
        seller_address: r.get(5)?,
        address: r.get(6)?,
        created_at: r.get(7)?,
        close_at: r.get(8)?,
        status: r.get(9)?,
        closed_at: r.get(10)?,
        winner_bid_id: r.get(11)?,
        ledger_hash: r.get(12)?,
        settlement_txid: r.get(13)?,
        payout_txid: r.get(14)?,
        settle_by: r.get(15)?,
        seller_id: r.get(16)?,
        image_url: r.get(17)?,
    })
}

pub struct NewAuction<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub reserve_zats: u64,
    pub deposit_zats: u64,
    pub seller_address: &'a str,
    pub address: &'a str,
    pub diversifier_index: Option<&'a str>,
    pub close_at: i64,
    pub seller_id: Option<i64>,
    pub image_url: Option<&'a str>,
}

pub fn insert_auction(conn: &Connection, a: &NewAuction<'_>) -> Result<()> {
    conn.execute(
        "INSERT INTO auctions (id, title, description, reserve_zats, deposit_zats, seller_address,
            address, diversifier_index, created_at, close_at, status, seller_id, image_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'open', ?11, ?12)",
        params![
            a.id,
            a.title,
            a.description,
            a.reserve_zats as i64,
            a.deposit_zats as i64,
            a.seller_address,
            a.address,
            a.diversifier_index,
            now(),
            a.close_at,
            a.seller_id,
            a.image_url
        ],
    )?;
    Ok(())
}

pub fn get_auction(conn: &Connection, id: &str) -> Result<Option<Auction>> {
    Ok(conn
        .query_row(
            &format!("SELECT {AUCTION_COLS} FROM auctions WHERE id = ?1"),
            [id],
            row_to_auction,
        )
        .optional()?)
}

pub fn list_auctions(conn: &Connection) -> Result<Vec<Auction>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AUCTION_COLS} FROM auctions ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map([], row_to_auction)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn due_open_auctions(conn: &Connection, now: i64) -> Result<Vec<Auction>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AUCTION_COLS} FROM auctions WHERE status = 'open' AND close_at <= ?1"
    ))?;
    let rows = stmt.query_map([now], row_to_auction)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct Bid {
    pub id: i64,
    pub auction_id: Option<String>,
    pub txid: String,
    pub output_index: u32,
    pub pool: i64,
    pub mined_height: Option<u32>,
    pub value_zats: u64,
    pub memo: Option<String>,
    pub bid_zats: Option<u64>,
    pub refund_address: Option<String>,
    pub nonce: Option<String>,
    pub receipt: Option<String>,
    pub status: String,
    pub source: String,
    pub received_at: i64,
}

const BID_COLS: &str = "id, auction_id, txid, output_index, pool, mined_height, value_zats, memo,
    bid_zats, refund_address, nonce, receipt, status, source, received_at";

fn row_to_bid(r: &rusqlite::Row<'_>) -> rusqlite::Result<Bid> {
    Ok(Bid {
        id: r.get(0)?,
        auction_id: r.get(1)?,
        txid: r.get(2)?,
        output_index: r.get::<_, i64>(3)? as u32,
        pool: r.get(4)?,
        mined_height: r.get::<_, Option<i64>>(5)?.map(|h| h as u32),
        value_zats: r.get::<_, i64>(6)? as u64,
        memo: r.get(7)?,
        bid_zats: r.get::<_, Option<i64>>(8)?.map(|v| v as u64),
        refund_address: r.get(9)?,
        nonce: r.get(10)?,
        receipt: r.get(11)?,
        status: r.get(12)?,
        source: r.get(13)?,
        received_at: r.get(14)?,
    })
}

pub struct NewBid<'a> {
    pub auction_id: Option<&'a str>,
    pub txid: &'a str,
    pub output_index: u32,
    pub pool: i64,
    pub mined_height: Option<u32>,
    pub value_zats: u64,
    pub memo: Option<&'a str>,
    pub bid_zats: Option<u64>,
    pub refund_address: Option<&'a str>,
    pub nonce: Option<&'a str>,
    pub receipt: Option<&'a str>,
    pub status: &'a str,
    pub source: &'a str,
}

pub fn insert_bid(conn: &Connection, b: &NewBid<'_>) -> Result<i64> {
    conn.execute(
        "INSERT INTO bids (auction_id, txid, output_index, pool, mined_height, value_zats, memo,
            bid_zats, refund_address, nonce, receipt, status, source, received_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            b.auction_id,
            b.txid,
            b.output_index as i64,
            b.pool,
            b.mined_height.map(|h| h as i64),
            b.value_zats as i64,
            b.memo,
            b.bid_zats.map(|v| v as i64),
            b.refund_address,
            b.nonce,
            b.receipt,
            b.status,
            b.source,
            now()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn bid_exists(conn: &Connection, txid: &str, output_index: u32) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bids WHERE txid = ?1 AND output_index = ?2",
        params![txid, output_index as i64],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn nonce_used(conn: &Connection, auction_id: &str, nonce: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bids WHERE auction_id = ?1 AND nonce = ?2 AND status = 'valid'",
        params![auction_id, nonce],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn bids_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<Bid>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BID_COLS} FROM bids WHERE auction_id = ?1 ORDER BY received_at ASC, id ASC"
    ))?;
    let rows = stmt.query_map([auction_id], row_to_bid)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_bid(conn: &Connection, id: i64) -> Result<Option<Bid>> {
    Ok(conn
        .query_row(&format!("SELECT {BID_COLS} FROM bids WHERE id = ?1"), [id], row_to_bid)
        .optional()?)
}

pub fn receipts_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT receipt FROM bids WHERE auction_id = ?1 AND status = 'valid' AND receipt IS NOT NULL
         ORDER BY receipt ASC",
    )?;
    let rows = stmt.query_map([auction_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    pub deposit_address: String,
    pub auction_id: String,
    pub rail: String,
    pub payload: String,
    pub destination_txid: Option<String>,
    pub created_at: i64,
    pub matched_bid_id: Option<i64>,
}

pub fn upsert_envelope(
    conn: &Connection,
    deposit_address: &str,
    auction_id: &str,
    rail: &str,
    payload: &str,
    destination_txid: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO envelopes (deposit_address, auction_id, rail, payload, destination_txid, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (deposit_address) DO UPDATE SET
            destination_txid = COALESCE(excluded.destination_txid, envelopes.destination_txid)",
        params![deposit_address, auction_id, rail, payload, destination_txid, now()],
    )?;
    Ok(())
}

pub fn envelope_for_txid(conn: &Connection, txid: &str) -> Result<Option<Envelope>> {
    Ok(conn
        .query_row(
            "SELECT deposit_address, auction_id, rail, payload, destination_txid, created_at, matched_bid_id
             FROM envelopes WHERE destination_txid = ?1 AND matched_bid_id IS NULL",
            [txid],
            |r| {
                Ok(Envelope {
                    deposit_address: r.get(0)?,
                    auction_id: r.get(1)?,
                    rail: r.get(2)?,
                    payload: r.get(3)?,
                    destination_txid: r.get(4)?,
                    created_at: r.get(5)?,
                    matched_bid_id: r.get(6)?,
                })
            },
        )
        .optional()?)
}

pub fn mark_envelope_matched(conn: &Connection, deposit_address: &str, bid_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE envelopes SET matched_bid_id = ?2 WHERE deposit_address = ?1",
        params![deposit_address, bid_id],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct Payout {
    pub id: i64,
    pub auction_id: Option<String>,
    pub bid_id: Option<i64>,
    pub kind: String,
    pub to_address: String,
    pub value_zats: u64,
    pub memo: Option<String>,
    pub not_before: i64,
    pub txid: Option<String>,
    pub status: String,
    pub attempts: i64,
    pub error: Option<String>,
}

pub fn queue_payout(
    conn: &Connection,
    auction_id: Option<&str>,
    bid_id: Option<i64>,
    kind: &str,
    to_address: &str,
    value_zats: u64,
    memo: Option<&str>,
    not_before: i64,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO payouts (auction_id, bid_id, kind, to_address, value_zats, memo, not_before, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![auction_id, bid_id, kind, to_address, value_zats as i64, memo, not_before, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn due_payouts(conn: &Connection, now: i64, max_attempts: i64) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error
         FROM payouts WHERE status IN ('queued', 'failed') AND not_before <= ?1 AND attempts < ?2
         ORDER BY not_before ASC",
    )?;
    let rows = stmt.query_map(params![now, max_attempts], |r| {
        Ok(Payout {
            id: r.get(0)?,
            auction_id: r.get(1)?,
            bid_id: r.get(2)?,
            kind: r.get(3)?,
            to_address: r.get(4)?,
            value_zats: r.get::<_, i64>(5)? as u64,
            memo: r.get(6)?,
            not_before: r.get(7)?,
            txid: r.get(8)?,
            status: r.get(9)?,
            attempts: r.get(10)?,
            error: r.get(11)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn all_payouts(conn: &Connection) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error
         FROM payouts ORDER BY id DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Payout {
            id: r.get(0)?,
            auction_id: r.get(1)?,
            bid_id: r.get(2)?,
            kind: r.get(3)?,
            to_address: r.get(4)?,
            value_zats: r.get::<_, i64>(5)? as u64,
            memo: r.get(6)?,
            not_before: r.get(7)?,
            txid: r.get(8)?,
            status: r.get(9)?,
            attempts: r.get(10)?,
            error: r.get(11)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn payouts_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error
         FROM payouts WHERE auction_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([auction_id], |r| {
        Ok(Payout {
            id: r.get(0)?,
            auction_id: r.get(1)?,
            bid_id: r.get(2)?,
            kind: r.get(3)?,
            to_address: r.get(4)?,
            value_zats: r.get::<_, i64>(5)? as u64,
            memo: r.get(6)?,
            not_before: r.get(7)?,
            txid: r.get(8)?,
            status: r.get(9)?,
            attempts: r.get(10)?,
            error: r.get(11)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn mark_payout_sent(conn: &Connection, id: i64, txid: &str) -> Result<()> {
    conn.execute(
        "UPDATE payouts SET status = 'sent', txid = ?2, sent_at = ?3, error = NULL,
            attempts = attempts + 1 WHERE id = ?1",
        params![id, txid, now()],
    )?;
    Ok(())
}

pub fn mark_payout_failed(conn: &Connection, id: i64, error: &str, retry_at: i64) -> Result<()> {
    conn.execute(
        "UPDATE payouts SET status = 'failed', error = ?2, attempts = attempts + 1, not_before = ?3
         WHERE id = ?1",
        params![id, error, retry_at],
    )?;
    Ok(())
}

pub fn close_auction(
    conn: &Connection,
    id: &str,
    status: &str,
    winner_bid_id: Option<i64>,
    ledger_hash: &str,
    settle_by: Option<i64>,
) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET status = ?2, winner_bid_id = ?3, ledger_hash = ?4, closed_at = ?5, settle_by = ?6
         WHERE id = ?1",
        params![id, status, winner_bid_id, ledger_hash, now(), settle_by],
    )?;
    Ok(())
}

/// Auctions awaiting settlement whose deadline has passed.
pub fn settlement_overdue(conn: &Connection, now: i64) -> Result<Vec<Auction>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AUCTION_COLS} FROM auctions WHERE status = 'closed' AND settle_by IS NOT NULL AND settle_by <= ?1"
    ))?;
    let rows = stmt.query_map([now], row_to_auction)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn set_winner(conn: &Connection, id: &str, winner_bid_id: Option<i64>, status: &str, settle_by: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET winner_bid_id = ?2, status = ?3, settle_by = ?4 WHERE id = ?1",
        params![id, winner_bid_id, status, settle_by],
    )?;
    Ok(())
}

pub fn set_bid_status(conn: &Connection, bid_id: i64, status: &str) -> Result<()> {
    conn.execute("UPDATE bids SET status = ?2 WHERE id = ?1", params![bid_id, status])?;
    Ok(())
}

/// Cancel a queued (not yet sent) payout for a bid. Returns true if one was cancelled.
pub fn cancel_queued_payout_for_bid(conn: &Connection, bid_id: i64) -> Result<bool> {
    let n = conn.execute(
        "UPDATE payouts SET status = 'cancelled' WHERE bid_id = ?1 AND kind = 'refund' AND status IN ('queued', 'failed')",
        [bid_id],
    )?;
    Ok(n > 0)
}

pub fn mark_payout_dead(conn: &Connection, id: i64, error: &str) -> Result<()> {
    conn.execute(
        "UPDATE payouts SET status = 'dead', error = ?2, attempts = attempts + 1 WHERE id = ?1",
        params![id, error],
    )?;
    Ok(())
}

pub fn requeue_payout(conn: &Connection, id: i64) -> Result<bool> {
    let n = conn.execute(
        "UPDATE payouts SET status = 'queued', attempts = 0, error = NULL, not_before = ?2
         WHERE id = ?1 AND status IN ('dead', 'failed', 'cancelled')",
        params![id, now()],
    )?;
    Ok(n > 0)
}

// ---- sellers ----

pub fn insert_seller(conn: &Connection, name: &str, key_hash: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO sellers (name, key_hash, created_at) VALUES (?1, ?2, ?3)",
        params![name, key_hash, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn seller_by_key_hash(conn: &Connection, key_hash: &str) -> Result<Option<(i64, String)>> {
    Ok(conn
        .query_row(
            "SELECT id, name FROM sellers WHERE key_hash = ?1",
            [key_hash],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

// ---- events ----

pub fn insert_event(conn: &Connection, auction_id: Option<&str>, kind: &str, payload: &serde_json::Value) -> Result<i64> {
    conn.execute(
        "INSERT INTO events (auction_id, kind, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![auction_id, kind, payload.to_string(), now()],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i64,
    pub auction_id: Option<String>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

pub fn undelivered_events(conn: &Connection, max_attempts: i64) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, kind, payload, created_at FROM events
         WHERE delivered_at IS NULL AND attempts < ?1 ORDER BY id ASC LIMIT 50",
    )?;
    let rows = stmt.query_map([max_attempts], |r| {
        Ok(Event {
            id: r.get(0)?,
            auction_id: r.get(1)?,
            kind: r.get(2)?,
            payload: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(serde_json::Value::Null),
            created_at: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn mark_event_delivered(conn: &Connection, id: i64, ok: bool) -> Result<()> {
    if ok {
        conn.execute("UPDATE events SET delivered_at = ?2, attempts = attempts + 1 WHERE id = ?1", params![id, now()])?;
    } else {
        conn.execute("UPDATE events SET attempts = attempts + 1 WHERE id = ?1", [id])?;
    }
    Ok(())
}

pub fn events_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, kind, payload, created_at FROM events WHERE auction_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([auction_id], |r| {
        Ok(Event {
            id: r.get(0)?,
            auction_id: r.get(1)?,
            kind: r.get(2)?,
            payload: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(serde_json::Value::Null),
            created_at: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---- reconciliation ----

#[derive(Debug, Clone, Serialize, Default)]
pub struct Totals {
    pub deposits_received_zats: u64,
    pub settlements_received_zats: u64,
    pub payouts_sent_zats: u64,
    pub payouts_pending_zats: u64,
    pub payouts_dead: i64,
}

pub fn totals(conn: &Connection) -> Result<Totals> {
    let mut t = Totals::default();
    t.deposits_received_zats = conn.query_row(
        "SELECT COALESCE(SUM(value_zats), 0) FROM bids WHERE status NOT IN ('settlement', 'unexpected_settlement')",
        [],
        |r| r.get::<_, i64>(0),
    )? as u64;
    t.settlements_received_zats = conn.query_row(
        "SELECT COALESCE(SUM(value_zats), 0) FROM bids WHERE status = 'settlement'",
        [],
        |r| r.get::<_, i64>(0),
    )? as u64;
    t.payouts_sent_zats = conn.query_row(
        "SELECT COALESCE(SUM(value_zats), 0) FROM payouts WHERE status = 'sent'",
        [],
        |r| r.get::<_, i64>(0),
    )? as u64;
    t.payouts_pending_zats = conn.query_row(
        "SELECT COALESCE(SUM(value_zats), 0) FROM payouts WHERE status IN ('queued', 'failed')",
        [],
        |r| r.get::<_, i64>(0),
    )? as u64;
    t.payouts_dead = conn.query_row("SELECT COUNT(*) FROM payouts WHERE status = 'dead'", [], |r| r.get(0))?;
    Ok(t)
}

pub fn set_auction_settled(conn: &Connection, id: &str, settlement_txid: &str) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET status = 'settled', settlement_txid = ?2 WHERE id = ?1",
        params![id, settlement_txid],
    )?;
    Ok(())
}

pub fn set_auction_payout(conn: &Connection, id: &str, payout_txid: &str) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET status = 'paid', payout_txid = ?2 WHERE id = ?1",
        params![id, payout_txid],
    )?;
    Ok(())
}

pub fn set_disclosure(conn: &Connection, id: &str, disclosure_json: &str) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET disclosure_json = ?2 WHERE id = ?1",
        params![id, disclosure_json],
    )?;
    Ok(())
}

pub fn get_disclosure(conn: &Connection, id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT disclosure_json FROM auctions WHERE id = ?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

/// Spender: adopt an auction definition from the viewer exactly once; later changes are ignored.
pub fn mirror_auction(conn: &Connection, a: &Auction) -> Result<bool> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO auctions (id, title, description, reserve_zats, deposit_zats, seller_address,
            address, created_at, close_at, status, image_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'open', ?10)",
        params![
            a.id, a.title, a.description, a.reserve_zats as i64, a.deposit_zats as i64,
            a.seller_address, a.address, a.created_at, a.close_at, a.image_url
        ],
    )?;
    Ok(n > 0)
}

/// Payouts the spender has sent since `after_id`, with the bid txid so the viewer can match them.
pub fn sent_payouts_after(conn: &Connection, after_id: i64) -> Result<Vec<(Payout, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.auction_id, p.bid_id, p.kind, p.to_address, p.value_zats, p.memo, p.not_before, p.txid,
                p.status, p.attempts, p.error, b.txid
         FROM payouts p LEFT JOIN bids b ON b.id = p.bid_id
         WHERE p.status = 'sent' AND p.id > ?1 ORDER BY p.id ASC LIMIT 100",
    )?;
    let rows = stmt.query_map([after_id], |r| {
        Ok((
            Payout {
                id: r.get(0)?,
                auction_id: r.get(1)?,
                bid_id: r.get(2)?,
                kind: r.get(3)?,
                to_address: r.get(4)?,
                value_zats: r.get::<_, i64>(5)? as u64,
                memo: r.get(6)?,
                not_before: r.get(7)?,
                txid: r.get(8)?,
                status: r.get(9)?,
                attempts: r.get(10)?,
                error: r.get(11)?,
            },
            r.get::<_, Option<String>>(12)?,
        ))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Viewer: mark its own matching payout row as sent by an external spender.
pub fn mark_payout_sent_by_match(
    conn: &Connection,
    auction_id: Option<&str>,
    kind: &str,
    bid_txid: Option<&str>,
    to_address: &str,
    value_zats: u64,
    txid: &str,
) -> Result<bool> {
    let n = conn.execute(
        "UPDATE payouts SET status = 'sent', txid = ?1, sent_at = ?2, error = NULL
         WHERE status IN ('queued', 'failed', 'dead') AND kind = ?3 AND to_address = ?4 AND value_zats = ?5
           AND (?6 IS NULL OR auction_id = ?6)
           AND (?7 IS NULL OR bid_id IN (SELECT id FROM bids WHERE txid = ?7))",
        params![txid, now(), kind, to_address, value_zats as i64, auction_id, bid_txid],
    )?;
    Ok(n > 0)
}
