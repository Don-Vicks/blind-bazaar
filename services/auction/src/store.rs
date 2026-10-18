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
            image_url TEXT,
            units INTEGER NOT NULL DEFAULT 1,
            format TEXT NOT NULL DEFAULT 'first_price',
            clearing_zats INTEGER
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
            price_zats INTEGER,
            settle_by INTEGER,
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
         CREATE TABLE IF NOT EXISTS fulfilments (
            auction_id TEXT PRIMARY KEY,
            seller_id INTEGER NOT NULL,
            note TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
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
        "ALTER TABLE auctions ADD COLUMN units INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE auctions ADD COLUMN format TEXT NOT NULL DEFAULT 'first_price'",
        "ALTER TABLE auctions ADD COLUMN clearing_zats INTEGER",
        "ALTER TABLE bids ADD COLUMN price_zats INTEGER",
        "ALTER TABLE bids ADD COLUMN settle_by INTEGER",
        "ALTER TABLE auctions ADD COLUMN close_height INTEGER",
        "ALTER TABLE auctions ADD COLUMN reveal_secs INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE auctions ADD COLUMN reveal_until INTEGER",
        "ALTER TABLE auctions ADD COLUMN commit_hash TEXT",
        "ALTER TABLE auctions ADD COLUMN audit_account TEXT",
        "ALTER TABLE auctions ADD COLUMN audit_key TEXT",
        "ALTER TABLE auctions ADD COLUMN audit_birthday INTEGER",
        "ALTER TABLE payouts ADD COLUMN from_account TEXT",
        "ALTER TABLE bids ADD COLUMN commitment TEXT",
        "ALTER TABLE bids ADD COLUMN salt TEXT",
        "ALTER TABLE bids ADD COLUMN revealed_at INTEGER",
        "ALTER TABLE envelopes ADD COLUMN address TEXT",
        "ALTER TABLE envelopes ADD COLUMN transparent TEXT",
        "CREATE UNIQUE INDEX IF NOT EXISTS envelopes_address ON envelopes (address)",
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
    #[serde(default = "one")]
    pub units: u32,
    #[serde(default = "first_price")]
    pub format: String,
    #[serde(default)]
    pub clearing_zats: Option<u64>,
    /// Last block whose timestamp is at or before `close_at`; bids mined after it are late. Fixed by the
    /// chain, so every process that reads the chain agrees on the ledger.
    #[serde(default)]
    pub close_height: Option<u32>,
    /// Sealed-reveal: seconds after close during which bidders reveal. 0 = bids carry the amount.
    #[serde(default)]
    pub reveal_secs: i64,
    #[serde(default)]
    pub reveal_until: Option<i64>,
    /// Sealed-reveal: Merkle root over (receipt, commitment) fixed at close, before any reveal.
    #[serde(default)]
    pub commit_hash: Option<String>,
    /// Auditable lots have their own wallet account, so a viewing key for this lot alone can be published.
    #[serde(default, skip_serializing)]
    pub audit_account: Option<String>,
    /// The lot account's encoded full viewing key, published on the result once the lot has closed.
    #[serde(default, skip_serializing)]
    pub audit_key: Option<String>,
    #[serde(default)]
    pub audit_birthday: Option<u32>,
}

impl Auction {
    pub fn sealed(&self) -> bool {
        self.reveal_secs > 0
    }
    pub fn auditable(&self) -> bool {
        self.audit_key.is_some()
    }
}

fn one() -> u32 {
    1
}
fn first_price() -> String {
    "first_price".into()
}

const AUCTION_COLS: &str = "id, title, description, reserve_zats, deposit_zats, seller_address, address,
    created_at, close_at, status, closed_at, winner_bid_id, ledger_hash, settlement_txid, payout_txid,
    settle_by, seller_id, image_url, units, format, clearing_zats, close_height, reveal_secs, reveal_until, commit_hash,
    audit_account, audit_key, audit_birthday";

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
        units: r.get::<_, i64>(18)? as u32,
        format: r.get(19)?,
        clearing_zats: r.get::<_, Option<i64>>(20)?.map(|v| v as u64),
        close_height: r.get::<_, Option<i64>>(21)?.map(|v| v as u32),
        reveal_secs: r.get::<_, Option<i64>>(22)?.unwrap_or(0),
        reveal_until: r.get(23)?,
        commit_hash: r.get(24)?,
        audit_account: r.get(25)?,
        audit_key: r.get(26)?,
        audit_birthday: r.get::<_, Option<i64>>(27)?.map(|v| v as u32),
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
    pub units: u32,
    pub format: &'a str,
    pub reveal_secs: i64,
    /// Auditable lot: (wallet account uuid, encoded full viewing key, birthday height).
    pub audit: Option<(&'a str, &'a str, u32)>,
}

pub fn insert_auction(conn: &Connection, a: &NewAuction<'_>) -> Result<()> {
    conn.execute(
        "INSERT INTO auctions (id, title, description, reserve_zats, deposit_zats, seller_address,
            address, diversifier_index, created_at, close_at, status, seller_id, image_url, units, format, reveal_secs,
            audit_account, audit_key, audit_birthday)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'open', ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
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
            a.image_url,
            a.units as i64,
            a.format,
            a.reveal_secs,
            a.audit.map(|x| x.0),
            a.audit.map(|x| x.1),
            a.audit.map(|x| x.2 as i64),
        ],
    )?;
    Ok(())
}

/// Sealed lots whose reveal window has ended.
pub fn due_revealing_auctions(conn: &Connection, now: i64) -> Result<Vec<Auction>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AUCTION_COLS} FROM auctions WHERE status = 'revealing' AND reveal_until IS NOT NULL AND reveal_until <= ?1"
    ))?;
    let rows = stmt.query_map([now], row_to_auction)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Move a sealed lot into its reveal window, fixing the commitment root and close height.
pub fn open_reveal_window(conn: &Connection, id: &str, reveal_until: i64, commit_hash: &str, close_height: Option<u32>) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET status = 'revealing', reveal_until = ?2, commit_hash = ?3, close_height = COALESCE(?4, close_height) WHERE id = ?1",
        params![id, reveal_until, commit_hash, close_height.map(|h| h as i64)],
    )?;
    Ok(())
}

/// Record a reveal: the bid becomes a valid bid with its amount and salt.
pub fn reveal_bid(conn: &Connection, bid_id: i64, bid_zats: u64, salt: &str) -> Result<()> {
    conn.execute(
        "UPDATE bids SET bid_zats = ?2, salt = ?3, revealed_at = ?4, status = 'valid' WHERE id = ?1 AND status = 'sealed'",
        params![bid_id, bid_zats as i64, salt, now()],
    )?;
    Ok(())
}

/// Reveals recorded after `after` (unix seconds), for the spender to mirror and re-verify.
pub fn reveals_after(conn: &Connection, after: i64, limit: usize) -> Result<Vec<Bid>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BID_COLS} FROM bids WHERE revealed_at IS NOT NULL AND revealed_at > ?1 ORDER BY revealed_at LIMIT ?2"
    ))?;
    let rows = stmt.query_map(params![after, limit as i64], row_to_bid)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
    pub price_zats: Option<u64>,
    pub settle_by: Option<i64>,
    /// Sealed-reveal: the commitment from the memo, and the salt once revealed.
    pub commitment: Option<String>,
    pub salt: Option<String>,
    pub revealed_at: Option<i64>,
}

const BID_COLS: &str = "id, auction_id, txid, output_index, pool, mined_height, value_zats, memo,
    bid_zats, refund_address, nonce, receipt, status, source, received_at, price_zats, settle_by, commitment, salt, revealed_at";

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
        price_zats: r.get::<_, Option<i64>>(15)?.map(|v| v as u64),
        settle_by: r.get(16)?,
        commitment: r.get(17)?,
        salt: r.get(18)?,
        revealed_at: r.get(19)?,
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
    pub commitment: Option<&'a str>,
}

pub fn insert_bid(conn: &Connection, b: &NewBid<'_>) -> Result<i64> {
    conn.execute(
        "INSERT INTO bids (auction_id, txid, output_index, pool, mined_height, value_zats, memo,
            bid_zats, refund_address, nonce, receipt, status, source, received_at, commitment)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
            now(),
            b.commitment
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

/// The bid sealed with `nonce` on `auction_id`, whatever its status. Nonces are secrets known only to the
/// bidder, so this is the bidder's own view.
pub fn get_bid_by_nonce(conn: &Connection, auction_id: &str, nonce: &str) -> Result<Option<Bid>> {
    conn.query_row(
        &format!("SELECT {BID_COLS} FROM bids WHERE auction_id = ?1 AND nonce = ?2 ORDER BY id LIMIT 1"),
        params![auction_id, nonce],
        row_to_bid,
    )
    .optional()
    .map_err(Into::into)
}

pub fn nonce_used(conn: &Connection, auction_id: &str, nonce: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bids WHERE auction_id = ?1 AND nonce = ?2 AND status IN ('valid', 'won', 'settled', 'forfeited', 'expired', 'sealed', 'unrevealed')",
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

/// The committed ledger of a closed auction: every bid that was valid at close, as (receipt, bid).
pub fn ledger_entries(conn: &Connection, auction_id: &str) -> Result<Vec<crate::merkle::Entry>> {
    let mut stmt = conn.prepare(
        "SELECT receipt, bid_zats, nonce FROM bids WHERE auction_id = ?1
           AND status IN ('valid', 'won', 'settled', 'forfeited', 'expired', 'sealed', 'unrevealed') AND receipt IS NOT NULL AND bid_zats IS NOT NULL AND nonce IS NOT NULL",
    )?;
    let rows = stmt.query_map([auction_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64, r.get::<_, String>(2)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn receipts_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT receipt FROM bids WHERE auction_id = ?1 AND status IN ('valid', 'won', 'settled', 'forfeited', 'expired', 'sealed', 'unrevealed')
           AND receipt IS NOT NULL
         ORDER BY receipt ASC",
    )?;
    let rows = stmt.query_map([auction_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Envelope {
    /// Row key. Historically the rail's deposit address; now a random id, since matching is by `address`.
    pub deposit_address: String,
    pub auction_id: String,
    pub rail: String,
    pub payload: String,
    pub destination_txid: Option<String>,
    pub created_at: i64,
    pub matched_bid_id: Option<i64>,
    /// The fresh unified address this envelope's payment must arrive at. Nothing else can claim it.
    pub address: Option<String>,
    /// Transparent receiver of `address`, for a rail that cannot deliver shielded.
    pub transparent: Option<String>,
}

/// Record an envelope bound to a fresh receive address the service just derived.
pub fn insert_envelope(conn: &Connection, e: &Envelope) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO envelopes (deposit_address, auction_id, rail, payload, destination_txid, created_at, address, transparent)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![e.deposit_address, e.auction_id, e.rail, e.payload, e.destination_txid, e.created_at, e.address, e.transparent],
    )?;
    Ok(())
}

const ENVELOPE_COLS: &str =
    "deposit_address, auction_id, rail, payload, destination_txid, created_at, matched_bid_id, address, transparent";

fn row_to_envelope(r: &rusqlite::Row<'_>) -> rusqlite::Result<Envelope> {
    Ok(Envelope {
        deposit_address: r.get(0)?,
        auction_id: r.get(1)?,
        rail: r.get(2)?,
        payload: r.get(3)?,
        destination_txid: r.get(4)?,
        created_at: r.get(5)?,
        matched_bid_id: r.get(6)?,
        address: r.get(7)?,
        transparent: r.get(8)?,
    })
}

/// The unmatched envelope whose receive address (shielded or transparent receiver) is `to_address`.
pub fn envelope_for_address(conn: &Connection, to_address: &str) -> Result<Option<Envelope>> {
    Ok(conn
        .query_row(
            &format!("SELECT {ENVELOPE_COLS} FROM envelopes WHERE (address = ?1 OR transparent = ?1) AND matched_bid_id IS NULL"),
            [to_address],
            row_to_envelope,
        )
        .optional()?)
}

/// Envelopes created after `after` (by rowid order of creation time), for the spender to mirror.
pub fn envelopes_after(conn: &Connection, after: i64, limit: usize) -> Result<Vec<Envelope>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ENVELOPE_COLS} FROM envelopes WHERE created_at > ?1 AND address IS NOT NULL ORDER BY created_at LIMIT ?2"
    ))?;
    let rows = stmt.query_map(params![after, limit as i64], row_to_envelope)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
    /// Wallet account to spend from; None = the operator account.
    pub from_account: Option<String>,
}

/// Bids whose transaction should still be in a block, plus reorged ones that may have come back.
pub fn bids_to_recheck(conn: &Connection) -> Result<Vec<Bid>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BID_COLS} FROM bids WHERE mined_height IS NOT NULL AND status NOT IN ('malformed') ORDER BY id"
    ))?;
    let rows = stmt.query_map([], row_to_bid)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn mark_bid_reorged(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("UPDATE bids SET status = 'reorged' WHERE id = ?1", [id])?;
    Ok(())
}

pub fn reinstate_bid(conn: &Connection, id: i64, status: &str, mined_height: u32) -> Result<()> {
    conn.execute(
        "UPDATE bids SET status = ?2, mined_height = ?3, received_at = ?4 WHERE id = ?1",
        params![id, status, mined_height as i64, now()],
    )?;
    Ok(())
}

/// Every payout the ledger has produced after `after_id`, oldest first, whatever its send status. Sweeps are
/// internal moves and are not exported.
pub fn payouts_after_id(conn: &Connection, after_id: i64, limit: usize) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error, from_account
         FROM payouts WHERE id > ?1 AND kind != 'sweep' ORDER BY id ASC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![after_id, limit as i64], |r| {
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
            from_account: r.get(12)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Queue a payment that spends from a specific wallet account (an auditable lot's sweep).
pub fn queue_payout_from(
    conn: &Connection,
    auction_id: Option<&str>,
    kind: &str,
    from_account: &str,
    to_address: &str,
    value_zats: u64,
    not_before: i64,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO payouts (auction_id, bid_id, kind, to_address, value_zats, memo, not_before, created_at, from_account)
         VALUES (?1, NULL, ?2, ?3, ?4, NULL, ?5, ?6, ?7)",
        params![auction_id, kind, to_address, value_zats as i64, not_before, now(), from_account],
    )?;
    Ok(conn.last_insert_rowid())
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
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error, from_account
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
            from_account: r.get(12)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn all_payouts(conn: &Connection) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error, from_account
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
            from_account: r.get(12)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Seconds from close to the last refund sent, and how many refunds are sent vs pending.
pub fn refund_latency(conn: &Connection, auction_id: &str) -> Result<(Option<i64>, i64, i64)> {
    let closed_at: Option<i64> = conn.query_row("SELECT closed_at FROM auctions WHERE id = ?1", [auction_id], |r| r.get(0))?;
    let (last_sent, sent, pending): (Option<i64>, i64, i64) = conn.query_row(
        "SELECT MAX(sent_at), SUM(CASE WHEN status = 'sent' THEN 1 ELSE 0 END),
                SUM(CASE WHEN status IN ('queued','failed') THEN 1 ELSE 0 END)
         FROM payouts WHERE auction_id = ?1 AND kind = 'refund'",
        [auction_id],
        |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0), r.get::<_, Option<i64>>(2)?.unwrap_or(0))),
    )?;
    let latency = match (closed_at, last_sent) {
        (Some(c), Some(l)) if pending == 0 && sent > 0 => Some((l - c).max(0)),
        _ => None,
    };
    Ok((latency, sent, pending))
}

pub fn payouts_for_auction(conn: &Connection, auction_id: &str) -> Result<Vec<Payout>> {
    let mut stmt = conn.prepare(
        "SELECT id, auction_id, bid_id, kind, to_address, value_zats, memo, not_before, txid, status, attempts, error, from_account
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
            from_account: r.get(12)?,
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

#[allow(clippy::too_many_arguments)]
pub fn close_auction(
    conn: &Connection,
    id: &str,
    status: &str,
    winner_bid_id: Option<i64>,
    ledger_hash: &str,
    settle_by: Option<i64>,
    clearing_zats: Option<u64>,
) -> Result<()> {
    conn.execute(
        "UPDATE auctions SET status = ?2, winner_bid_id = ?3, ledger_hash = ?4, closed_at = ?5, settle_by = ?6,
            clearing_zats = ?7
         WHERE id = ?1",
        params![id, status, winner_bid_id, ledger_hash, now(), settle_by, clearing_zats.map(|v| v as i64)],
    )?;
    Ok(())
}

/// Mark a bid as a winner owing `price_zats` (total, deposit included) by `settle_by`.
pub fn set_bid_won(conn: &Connection, bid_id: i64, price_zats: u64, settle_by: i64) -> Result<()> {
    conn.execute(
        "UPDATE bids SET status = 'won', price_zats = ?2, settle_by = ?3 WHERE id = ?1",
        params![bid_id, price_zats as i64, settle_by],
    )?;
    Ok(())
}

/// Winners whose settlement deadline has passed.
pub fn overdue_winners(conn: &Connection, now: i64) -> Result<Vec<Bid>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BID_COLS} FROM bids WHERE status = 'won' AND settle_by IS NOT NULL AND settle_by <= ?1"
    ))?;
    let rows = stmt.query_map([now], row_to_bid)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn count_bids_with_status(conn: &Connection, auction_id: &str, status: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM bids WHERE auction_id = ?1 AND status = ?2",
        params![auction_id, status],
        |r| r.get(0),
    )?)
}

pub fn set_auction_close_height(conn: &Connection, id: &str, height: u32) -> Result<()> {
    conn.execute("UPDATE auctions SET close_height = ?2 WHERE id = ?1", params![id, height as i64])?;
    Ok(())
}

/// Update the published clearing price (after a second-chance offer reprices the lot).
pub fn set_auction_clearing(conn: &Connection, id: &str, clearing_zats: u64) -> Result<()> {
    conn.execute("UPDATE auctions SET clearing_zats = ?2 WHERE id = ?1", params![id, clearing_zats as i64])?;
    Ok(())
}

pub fn set_auction_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute("UPDATE auctions SET status = ?2 WHERE id = ?1", params![id, status])?;
    Ok(())
}

pub fn pending_seller_payouts(conn: &Connection, auction_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM payouts WHERE auction_id = ?1 AND kind = 'seller' AND status != 'sent'",
        [auction_id],
        |r| r.get(0),
    )?)
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
         WHERE id = ?1 AND status IN ('dead', 'failed')",
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

pub fn auctions_for_seller(conn: &Connection, seller_id: i64) -> Result<Vec<Auction>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {AUCTION_COLS} FROM auctions WHERE seller_id = ?1 ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map([seller_id], row_to_auction)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LotMoney {
    pub bids_valid: i64,
    pub bids_rejected: i64,
    pub deposits_held_zats: u64,
    pub winners: i64,
    pub winners_settled: i64,
    pub seller_paid_zats: u64,
    pub seller_pending_zats: u64,
    pub refunds_sent: i64,
    pub refunds_pending: i64,
}

pub fn lot_money(conn: &Connection, auction_id: &str) -> Result<LotMoney> {
    let mut m = LotMoney::default();
    let (v, r, held): (i64, i64, i64) = conn.query_row(
        "SELECT SUM(CASE WHEN status IN ('valid','won','settled','forfeited','expired','sealed','unrevealed') THEN 1 ELSE 0 END),
                SUM(CASE WHEN status IN ('malformed','wrong_deposit','duplicate_nonce','late','unknown_auction') THEN 1 ELSE 0 END),
                COALESCE(SUM(CASE WHEN status IN ('valid','won') THEN value_zats ELSE 0 END), 0)
         FROM bids WHERE auction_id = ?1",
        [auction_id],
        |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0), row.get::<_, Option<i64>>(1)?.unwrap_or(0), row.get(2)?)),
    )?;
    m.bids_valid = v;
    m.bids_rejected = r;
    m.deposits_held_zats = held as u64;
    let (w, ws): (i64, i64) = conn.query_row(
        "SELECT SUM(CASE WHEN status IN ('won','settled') THEN 1 ELSE 0 END), SUM(CASE WHEN status = 'settled' THEN 1 ELSE 0 END)
         FROM bids WHERE auction_id = ?1",
        [auction_id],
        |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0), row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
    )?;
    m.winners = w;
    m.winners_settled = ws;
    let (paid, pending, rs, rp): (i64, i64, i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN kind IN ('seller','forfeit') AND status = 'sent' THEN value_zats ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN kind IN ('seller','forfeit') AND status IN ('queued','failed') THEN value_zats ELSE 0 END), 0),
                SUM(CASE WHEN kind = 'refund' AND status = 'sent' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'refund' AND status IN ('queued','failed') THEN 1 ELSE 0 END)
         FROM payouts WHERE auction_id = ?1",
        [auction_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, Option<i64>>(2)?.unwrap_or(0), row.get::<_, Option<i64>>(3)?.unwrap_or(0))),
    )?;
    m.seller_paid_zats = paid as u64;
    m.seller_pending_zats = pending as u64;
    m.refunds_sent = rs;
    m.refunds_pending = rp;
    Ok(m)
}

pub fn set_fulfilment(conn: &Connection, auction_id: &str, seller_id: i64, note: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO fulfilments (auction_id, seller_id, note, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT (auction_id) DO UPDATE SET note = excluded.note, updated_at = excluded.updated_at",
        params![auction_id, seller_id, note, now()],
    )?;
    Ok(())
}

pub fn get_fulfilment(conn: &Connection, auction_id: &str) -> Result<Option<(String, i64)>> {
    Ok(conn
        .query_row("SELECT note, updated_at FROM fulfilments WHERE auction_id = ?1", [auction_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?)
}

pub fn receipt_is_winner(conn: &Connection, auction_id: &str, receipt: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bids WHERE auction_id = ?1 AND receipt = ?2 AND status IN ('won','settled')",
        params![auction_id, receipt],
        |r| r.get(0),
    )?;
    Ok(n > 0)
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

pub fn set_auction_settled_txid(conn: &Connection, id: &str, settlement_txid: &str) -> Result<()> {
    conn.execute("UPDATE auctions SET settlement_txid = ?2 WHERE id = ?1", params![id, settlement_txid])?;
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
            address, created_at, close_at, status, image_url, units, format, reveal_secs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'open', ?10, ?11, ?12, ?13)",
        params![
            a.id, a.title, a.description, a.reserve_zats as i64, a.deposit_zats as i64,
            a.seller_address, a.address, a.created_at, a.close_at, a.image_url, a.units as i64, a.format, a.reveal_secs
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
                from_account: None,
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

#[cfg(test)]
mod money_tests {
    use super::*;

    #[test]
    fn lot_money_sums_bids_and_payouts() {
        let c = open_in_memory().unwrap();
        insert_auction(&c, &NewAuction { id: "mmmmmmmm", title: "t", description: "", reserve_zats: 2, deposit_zats: 1, seller_address: "u1x", address: "u1y", diversifier_index: None, close_at: 0, seller_id: Some(7), image_url: None, units: 2, format: "uniform", reveal_secs: 0, audit: None }).unwrap();
        fn mk<'a>(txid: &'a str, status: &'a str, val: u64) -> NewBid<'a> {
            NewBid { auction_id: Some("mmmmmmmm"), txid, output_index: 0, pool: 3, mined_height: Some(1), value_zats: val, memo: None, bid_zats: Some(5), refund_address: Some("u1r"), nonce: Some("n"), receipt: Some("r"), status, source: "memo", commitment: None }
        }
        let a = insert_bid(&c, &mk("a", "won", 100)).unwrap();
        let _b = insert_bid(&c, &mk("b", "settled", 100)).unwrap();
        let _c2 = insert_bid(&c, &mk("c", "valid", 100)).unwrap();
        let _d = insert_bid(&c, &mk("d", "wrong_deposit", 50)).unwrap();
        queue_payout(&c, Some("mmmmmmmm"), Some(a), "seller", "u1x", 400, None, 0).unwrap();
        let p = queue_payout(&c, Some("mmmmmmmm"), None, "seller", "u1x", 300, None, 0).unwrap();
        mark_payout_sent(&c, p, "tx").unwrap();
        let r = queue_payout(&c, Some("mmmmmmmm"), None, "refund", "u1r", 100, None, 0).unwrap();
        mark_payout_sent(&c, r, "tx2").unwrap();
        queue_payout(&c, Some("mmmmmmmm"), None, "refund", "u1r", 100, None, 0).unwrap();
        let m = lot_money(&c, "mmmmmmmm").unwrap();
        assert_eq!(m.bids_valid, 3);
        assert_eq!(m.bids_rejected, 1);
        assert_eq!(m.deposits_held_zats, 200, "valid + won deposits are still held; settled is not");
        assert_eq!((m.winners, m.winners_settled), (2, 1));
        assert_eq!((m.seller_paid_zats, m.seller_pending_zats), (300, 400));
        assert_eq!((m.refunds_sent, m.refunds_pending), (1, 1));
        assert_eq!(auctions_for_seller(&c, 7).unwrap().len(), 1);
        assert!(auctions_for_seller(&c, 8).unwrap().is_empty());
    }
}
