//! Park the service at a chain height it cannot follow.
//!
//! The wallet crates this build uses carry NU7's consensus branch id and its testnet activation height, but no
//! mainnet height: after mainnet NU7 activates (expected around Nov 5, 2026) this binary could neither parse new
//! transactions nor sign one the network would accept. Rather than fail mid-lot, the operator sets
//! `FREEZE_HEIGHT` to the announced activation height. From that block the service stops syncing, closing
//! lots and spending, every write endpoint answers 503, and `/health` says why. Reads keep working so published
//! results and proofs stay available. `PAUSED=true` does the same immediately, for a manual stop.
//!
//! Lots are refused if they could not finish before the freeze: close time plus the settlement window must fall
//! before the estimated freeze time, so the winner can pay and losers can be refunded on a chain this binary
//! still understands.

use serde::Serialize;

/// Zcash target block spacing, used for the freeze time estimate.
pub const BLOCK_SECS: i64 = 75;

#[derive(Debug, Clone, Copy, Default)]
pub struct FreezePlan {
    pub height: Option<u32>,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct FreezeStatus {
    /// True when the service is parked: nothing moves on chain, write endpoints answer 503.
    pub active: bool,
    pub reason: Option<String>,
    /// The planned freeze height, if one is set.
    pub height: Option<u32>,
    /// Blocks until the freeze, from the last known tip.
    pub blocks_left: Option<u32>,
    /// Estimated unix time of the freeze.
    pub eta_at: Option<i64>,
}

/// What the service should do now, given the plan and the last known chain tip.
pub fn status(plan: FreezePlan, tip: Option<u32>, now: i64) -> FreezeStatus {
    if plan.paused {
        return FreezeStatus { active: true, reason: Some("paused by the operator".into()), height: plan.height, ..Default::default() };
    }
    let Some(h) = plan.height else { return FreezeStatus::default() };
    match tip {
        Some(t) if t >= h => FreezeStatus {
            active: true,
            reason: Some(format!("frozen at block {h}: this build does not follow the chain past the next network upgrade")),
            height: Some(h),
            blocks_left: Some(0),
            eta_at: None,
        },
        Some(t) => {
            let left = h - t;
            FreezeStatus { active: false, reason: None, height: Some(h), blocks_left: Some(left), eta_at: Some(now + i64::from(left) * BLOCK_SECS) }
        }
        // No tip yet: the plan is known but not whether it has arrived. Treat as live; the worker re-checks every pass.
        None => FreezeStatus { active: false, reason: None, height: Some(h), blocks_left: None, eta_at: None },
    }
}

/// Whether a lot closing at `close_at` can settle before the freeze.
/// `reveal_secs` is the sealed lot's reveal window (0 for classic lots): settlement starts only after it.
pub fn lot_fits(plan: FreezePlan, tip: Option<u32>, now: i64, close_at: i64, reveal_secs: i64, settlement_window_secs: i64) -> Result<(), String> {
    let st = status(plan, tip, now);
    if st.active {
        return Err(format!("the service is {}; no new lots", st.reason.unwrap_or_default()));
    }
    if let Some(eta) = st.eta_at {
        let needs = close_at + reveal_secs + settlement_window_secs;
        if needs > eta {
            return Err(format!(
                "this lot would still be settling when the service freezes at block {} (about {} unix); close it at least {} seconds earlier",
                st.height.unwrap_or(0),
                eta,
                needs - eta
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_plan_means_live() {
        assert_eq!(status(FreezePlan::default(), Some(100), 0), FreezeStatus::default());
        assert!(lot_fits(FreezePlan::default(), Some(100), 0, 10_000_000, 0, 86_400).is_ok());
    }

    #[test]
    fn paused_wins_over_everything() {
        let s = status(FreezePlan { height: Some(1_000), paused: true }, Some(10), 0);
        assert!(s.active);
        assert!(s.reason.as_deref().unwrap().contains("paused"));
        assert!(lot_fits(FreezePlan { height: None, paused: true }, None, 0, 100, 0, 0).is_err());
    }

    #[test]
    fn freezes_at_the_height_and_not_before() {
        let plan = FreezePlan { height: Some(3_000_000), paused: false };
        let before = status(plan, Some(2_999_990), 1_000);
        assert!(!before.active);
        assert_eq!(before.blocks_left, Some(10));
        assert_eq!(before.eta_at, Some(1_000 + 10 * BLOCK_SECS));
        let at = status(plan, Some(3_000_000), 1_000);
        assert!(at.active);
        assert_eq!(at.blocks_left, Some(0));
        assert!(status(plan, Some(3_000_500), 0).active);
        assert!(!status(plan, None, 0).active, "unknown tip does not freeze on its own");
    }

    #[test]
    fn a_lot_must_settle_before_the_freeze() {
        let plan = FreezePlan { height: Some(1_100), paused: false };
        let now = 10_000;
        // 100 blocks away: 7500 s. A lot closing in 1000 s with a 3600 s settlement window fits.
        assert!(lot_fits(plan, Some(1_000), now, now + 1_000, 0, 3_600).is_ok());
        // One closing in 5000 s with the same window does not (needs 8600 > 7500).
        let err = lot_fits(plan, Some(1_000), now, now + 5_000, 0, 3_600).unwrap_err();
        assert!(err.contains("1100 seconds earlier"), "{err}");
        // A sealed lot's reveal window comes before settlement: 1000 + 3000 reveal + 3600 = 7600 > 7500.
        assert!(lot_fits(plan, Some(1_000), now, now + 1_000, 3_000, 3_600).is_err());
        // Once frozen, nothing fits.
        assert!(lot_fits(plan, Some(1_100), now, now + 10, 0, 0).is_err());
    }
}
