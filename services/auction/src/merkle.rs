//! Bid-ledger commitment as a Merkle tree so each bidder can check their own inclusion without
//! learning anyone else's bid. Leaves are `sha256(0x00 || receipt || bid_zats_be8 || nonce)`, sorted
//! by receipt; internal nodes are `sha256(0x01 || left || right)`; an odd node is paired with itself.
//! The nonce blinds the leaf: receipts are public and bid amounts are small, so without the nonce
//! (known only to the bidder, and sealed in their memo) a leaf cannot be brute-forced to an amount.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Proof {
    pub leaf: String,
    pub index: usize,
    pub leaf_count: usize,
    /// Sibling hashes from the leaf up, each tagged with which side it sits on.
    pub path: Vec<Sibling>,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sibling {
    pub hash: String,
    pub side: Side,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

/// One ledger entry: (receipt, bid in zatoshis, nonce).
pub type Entry = (String, u64, String);

pub fn leaf_hash(receipt: &str, bid_zats: u64, nonce: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0u8]);
    h.update(receipt.as_bytes());
    h.update(bid_zats.to_be_bytes());
    h.update(nonce.as_bytes());
    h.finalize().into()
}

fn node_hash(l: &[u8; 32], r: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([1u8]);
    h.update(l);
    h.update(r);
    h.finalize().into()
}

/// Sorted leaves for a ledger of `(receipt, bid_zats, nonce)`.
pub fn leaves(entries: &[Entry]) -> Vec<Entry> {
    let mut v = entries.to_vec();
    v.sort();
    v
}

fn levels(leaf_hashes: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    let mut lv = vec![leaf_hashes.to_vec()];
    while lv.last().map(|l| l.len()).unwrap_or(0) > 1 {
        let cur = lv.last().unwrap();
        let mut next = Vec::with_capacity(cur.len().div_ceil(2));
        for pair in cur.chunks(2) {
            let r = pair.get(1).unwrap_or(&pair[0]);
            next.push(node_hash(&pair[0], r));
        }
        lv.push(next);
    }
    lv
}

/// Root of the ledger, hex. Empty ledger commits to sha256 of the empty string, matching the old
/// plain hash so an empty auction is unchanged.
pub fn root(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return hex::encode(Sha256::digest(b""));
    }
    let lh: Vec<[u8; 32]> = leaves(entries).iter().map(|(r, v, n)| leaf_hash(r, *v, n)).collect();
    hex::encode(levels(&lh).last().unwrap()[0])
}

/// Inclusion proof for `receipt` in the ledger, if present.
pub fn prove(entries: &[Entry], receipt: &str) -> Option<Proof> {
    let sorted = leaves(entries);
    let index = sorted.iter().position(|(r, _, _)| r == receipt)?;
    let lh: Vec<[u8; 32]> = sorted.iter().map(|(r, v, n)| leaf_hash(r, *v, n)).collect();
    let lv = levels(&lh);
    let mut path = Vec::new();
    let mut i = index;
    for level in &lv[..lv.len() - 1] {
        let (sib, side) = if i % 2 == 0 {
            (level.get(i + 1).unwrap_or(&level[i]), Side::Right)
        } else {
            (&level[i - 1], Side::Left)
        };
        path.push(Sibling { hash: hex::encode(sib), side });
        i /= 2;
    }
    Some(Proof {
        leaf: hex::encode(lh[index]),
        index,
        leaf_count: sorted.len(),
        path,
        root: hex::encode(lv.last().unwrap()[0]),
    })
}

/// Recompute the root from a leaf and its path.
pub fn verify(receipt: &str, bid_zats: u64, nonce: &str, proof: &Proof) -> bool {
    let mut acc = leaf_hash(receipt, bid_zats, nonce);
    if hex::encode(acc) != proof.leaf {
        return false;
    }
    for s in &proof.path {
        let Ok(sib) = hex::decode(&s.hash) else { return false };
        let Ok(sib): Result<[u8; 32], _> = sib.try_into() else { return false };
        acc = match s.side {
            Side::Right => node_hash(&acc, &sib),
            Side::Left => node_hash(&sib, &acc),
        };
    }
    hex::encode(acc) == proof.root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger(n: usize) -> Vec<Entry> {
        (0..n)
            .map(|i| (format!("{:064x}", (i as u64 + 1) * 0x9e37), (i as u64 + 1) * 1_000_000, format!("{:016x}", i as u64 + 7)))
            .collect()
    }

    #[test]
    fn every_leaf_proves_against_the_root() {
        for n in [1usize, 2, 3, 4, 5, 8, 13] {
            let l = ledger(n);
            let r = root(&l);
            for (receipt, bid, nonce) in &l {
                let p = prove(&l, receipt).expect("present");
                assert_eq!(p.root, r);
                assert_eq!(p.leaf_count, n);
                assert!(verify(receipt, *bid, nonce, &p), "n={n} receipt={receipt}");
                assert!(!verify(receipt, bid + 1, nonce, &p), "wrong bid must fail");
                assert!(!verify(receipt, *bid, "0000000000000000", &p), "wrong nonce must fail");
            }
        }
    }

    #[test]
    fn missing_receipt_has_no_proof() {
        assert!(prove(&ledger(4), "nope").is_none());
    }

    #[test]
    fn root_is_order_independent_and_empty_matches_plain_hash() {
        let mut l = ledger(6);
        let a = root(&l);
        l.reverse();
        assert_eq!(root(&l), a);
        assert_eq!(root(&[]), hex::encode(Sha256::digest(b"")));
    }

    #[test]
    fn tampered_path_fails() {
        let l = ledger(5);
        let mut p = prove(&l, &l[2].0).unwrap();
        p.path[0].hash = "00".repeat(32);
        assert!(!verify(&l[2].0, l[2].1, &l[2].2, &p));
    }
}

#[cfg(test)]
mod vectors {
    /// Printed by `cargo test -- --nocapture vectors` and pinned in packages/bb-sdk/test/sdk.test.ts.
    #[test]
    fn print_leaf_vector() {
        println!("LEAF_VECTOR {}", hex::encode(super::leaf_hash("ab", 1, "00000000000000ab")));
    }
}
