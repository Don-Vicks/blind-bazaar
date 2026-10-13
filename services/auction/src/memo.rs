//! BB1 memo format: `BB1|<auction_id>|<bid_zatoshis>|<refund_address>|<nonce>`.
//! See docs/MEMO_SPEC.md.

use sha2::{Digest, Sha256};

pub const MAX_MEMO_BYTES: usize = 300;
const CROCKFORD: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bid {
    pub auction_id: String,
    pub bid_zats: u64,
    pub refund_address: String,
    pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub auction_id: String,
    pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Bid(Bid),
    Settlement(Settlement),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MemoError {
    #[error("memo does not start with BB1")]
    NotBb1,
    #[error("wrong field count")]
    FieldCount,
    #[error("invalid auction id")]
    AuctionId,
    #[error("invalid bid amount")]
    Amount,
    #[error("invalid refund address")]
    RefundAddress,
    #[error("invalid nonce")]
    Nonce,
    #[error("memo too long")]
    TooLong,
}

pub fn is_auction_id(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| CROCKFORD.contains(&b))
}

pub fn is_nonce(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Loose shape check for a shielded-capable address string. Full validation
/// happens when the refund is built (`ZcashAddress::from_str`).
pub fn looks_like_shielded_address(s: &str) -> bool {
    (s.starts_with("u1") || s.starts_with("utest1") || s.starts_with("uregtest1"))
        && s.len() > 20
        && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

pub fn parse(memo: &str) -> Result<Parsed, MemoError> {
    if memo.len() > MAX_MEMO_BYTES {
        return Err(MemoError::TooLong);
    }
    let memo = memo.trim_end_matches('\0').trim();
    let mut parts = memo.split('|');
    match parts.next() {
        Some("BB1") => {
            let fields: Vec<&str> = parts.collect();
            if fields.len() != 4 {
                return Err(MemoError::FieldCount);
            }
            let auction_id = fields[0];
            if !is_auction_id(auction_id) {
                return Err(MemoError::AuctionId);
            }
            let bid_zats: u64 = fields[1].parse().map_err(|_| MemoError::Amount)?;
            if bid_zats == 0 {
                return Err(MemoError::Amount);
            }
            let refund_address = fields[2];
            if !looks_like_shielded_address(refund_address) {
                return Err(MemoError::RefundAddress);
            }
            let nonce = fields[3];
            if !is_nonce(nonce) {
                return Err(MemoError::Nonce);
            }
            Ok(Parsed::Bid(Bid {
                auction_id: auction_id.to_string(),
                bid_zats,
                refund_address: refund_address.to_string(),
                nonce: nonce.to_string(),
            }))
        }
        Some("BB1S") => {
            let fields: Vec<&str> = parts.collect();
            if fields.len() != 2 {
                return Err(MemoError::FieldCount);
            }
            if !is_auction_id(fields[0]) {
                return Err(MemoError::AuctionId);
            }
            if !is_nonce(fields[1]) {
                return Err(MemoError::Nonce);
            }
            Ok(Parsed::Settlement(Settlement {
                auction_id: fields[0].to_string(),
                nonce: fields[1].to_string(),
            }))
        }
        _ => Err(MemoError::NotBb1),
    }
}

pub fn encode_bid(bid: &Bid) -> String {
    format!(
        "BB1|{}|{}|{}|{}",
        bid.auction_id, bid.bid_zats, bid.refund_address, bid.nonce
    )
}

pub fn encode_settlement(s: &Settlement) -> String {
    format!("BB1S|{}|{}", s.auction_id, s.nonce)
}

/// Receipt a bidder can check publicly without revealing the nonce.
pub fn receipt(nonce: &str) -> String {
    hex::encode(Sha256::digest(nonce.as_bytes()))
}

pub fn random_auction_id<R: rand::Rng>(rng: &mut R) -> String {
    (0..8)
        .map(|_| CROCKFORD[rng.gen_range(0..CROCKFORD.len())] as char)
        .collect()
}

pub fn random_nonce<R: rand::Rng>(rng: &mut R) -> String {
    let mut b = [0u8; 8];
    rng.fill(&mut b);
    hex::encode(b)
}

/// Hash of the full bid ledger at close: sha256 over sorted `receipt:bid_zats` lines.
pub fn ledger_hash(entries: &[(String, u64)]) -> String {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|(r, v)| format!("{r}:{v}"))
        .collect();
    lines.sort();
    hex::encode(Sha256::digest(lines.join("\n").as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA: &str = "utest1zkkkjfxkamagznjr6ayemffj2d2gacdwpzcyw669pvg06xevzqslpmm27zjsctlkstl2vsw62xrjktmzqcu4yu9zdhdxqz3kafa4j2q85y6mv74rzjcgjg8c0ytrg7dwyzwtgnuc76h";

    #[test]
    fn round_trip_bid() {
        let bid = Bid {
            auction_id: "7k2m9q4e".into(),
            bid_zats: 150_000_000,
            refund_address: UA.into(),
            nonce: "9f3a0c7b12de44aa".into(),
        };
        let memo = encode_bid(&bid);
        assert!(memo.len() < MAX_MEMO_BYTES);
        assert_eq!(parse(&memo), Ok(Parsed::Bid(bid)));
    }

    #[test]
    fn round_trip_settlement() {
        let s = Settlement {
            auction_id: "7k2m9q4e".into(),
            nonce: "9f3a0c7b12de44aa".into(),
        };
        assert_eq!(parse(&encode_settlement(&s)), Ok(Parsed::Settlement(s)));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse("hello"), Err(MemoError::NotBb1));
        assert_eq!(parse("BB1|7k2m9q4e|1"), Err(MemoError::FieldCount));
        assert_eq!(
            parse(&format!("BB1|7K2M9Q4E|1|{UA}|9f3a0c7b12de44aa")),
            Err(MemoError::AuctionId)
        );
        assert_eq!(
            parse(&format!("BB1|7k2m9q4e|0|{UA}|9f3a0c7b12de44aa")),
            Err(MemoError::Amount)
        );
        assert_eq!(
            parse("BB1|7k2m9q4e|1|t1L2vcgaUr5YVczW7sPsAi8mJSWTLbkLyeW|9f3a0c7b12de44aa"),
            Err(MemoError::RefundAddress)
        );
        assert_eq!(
            parse(&format!("BB1|7k2m9q4e|1|{UA}|xyz")),
            Err(MemoError::Nonce)
        );
    }

    #[test]
    fn tolerates_null_padding() {
        let memo = format!("BB1|7k2m9q4e|5|{UA}|9f3a0c7b12de44aa\0\0\0");
        assert!(matches!(parse(&memo), Ok(Parsed::Bid(_))));
    }

    #[test]
    fn receipt_is_sha256_hex() {
        assert_eq!(receipt("9f3a0c7b12de44aa").len(), 64);
    }

    #[test]
    fn ledger_hash_is_order_independent() {
        let a = ledger_hash(&[("r1".into(), 5), ("r2".into(), 7)]);
        let b = ledger_hash(&[("r2".into(), 7), ("r1".into(), 5)]);
        assert_eq!(a, b);
    }

    #[test]
    fn random_ids_have_right_shape() {
        let mut rng = rand::thread_rng();
        assert!(is_auction_id(&random_auction_id(&mut rng)));
        assert!(is_nonce(&random_nonce(&mut rng)));
    }
}
