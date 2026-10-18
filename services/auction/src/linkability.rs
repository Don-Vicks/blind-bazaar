//! How much a chain observer can learn about refunds from timing and transaction shape, measured by
//! simulation. This is the number behind the "refunds are private" claim; it is only as good as the
//! observer model below, which is written out so a reader can disagree with it.
//!
//! Observer model. The observer knows, for every lot, its close time and its receipt count (both are
//! public), and sees every shielded transaction the operator broadcasts: its time and its action count
//! (an Orchard bundle's action count is public; outputs are not). Refunds leave between `delay_min`
//! and `delay_max` seconds after close. For each refund transaction the observer lists the lots whose
//! refund window contains that transaction. A refund output is *linked* when its transaction has
//! exactly one candidate lot, so the observer can say "this payment is a refund from lot L". With
//! batching across lots, a transaction carrying refunds from two lots has two candidates and links
//! nothing; and one window-sized transaction replaces several per-bidder ones.
//!
//! What the model leaves out, on purpose: the action count of a batched transaction tells the observer
//! how many refunds it carries, which can be matched against receipt counts when candidate lots are
//! few; and amounts are hidden only because deposits are fixed per lot. Both favour mixing more lots.

use rand::RngExt;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Lots closing per hour, as a Poisson rate.
    pub lots_per_hour: f64,
    /// Losing bidders per lot (refund outputs).
    pub losers_per_lot: u32,
    pub delay_min: i64,
    pub delay_max: i64,
    /// 0 = one transaction per refund at its own random time.
    pub window_secs: i64,
    pub batch_max: usize,
    pub hours: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Outcome {
    pub lots: usize,
    pub refunds: usize,
    pub transactions: usize,
    pub linked_refunds: usize,
}

impl Outcome {
    pub fn linked_fraction(&self) -> f64 {
        if self.refunds == 0 { 0.0 } else { self.linked_refunds as f64 / self.refunds as f64 }
    }
}

pub fn simulate<R: rand::Rng>(rng: &mut R, p: &Params) -> Outcome {
    // Lot close times.
    let mut closes: Vec<i64> = Vec::new();
    let mut t = 0.0f64;
    let total = p.hours * 3600.0;
    while t < total {
        let u: f64 = rng.random_range(0.0..1.0);
        t += -u.ln() / (p.lots_per_hour / 3600.0);
        if t < total {
            closes.push(t as i64);
        }
    }
    // Refund send times.
    let mut refunds: Vec<(usize, i64)> = Vec::new(); // (lot index, send time)
    for (i, c) in closes.iter().enumerate() {
        for _ in 0..p.losers_per_lot {
            let delay = rng.random_range(p.delay_min..=p.delay_max.max(p.delay_min));
            let mut st = c + delay;
            if p.window_secs > 0 {
                st = ((st + p.window_secs - 1) / p.window_secs) * p.window_secs;
            }
            refunds.push((i, st));
        }
    }
    refunds.sort_by_key(|r| r.1);
    // Group into transactions: same send time, up to batch_max, when batching is on.
    let mut txs: Vec<(i64, Vec<usize>)> = Vec::new();
    for (lot, st) in refunds {
        match txs.last_mut() {
            Some((t0, lots)) if p.window_secs > 0 && *t0 == st && lots.len() < p.batch_max => lots.push(lot),
            _ => txs.push((st, vec![lot])),
        }
    }
    // Attribution.
    let mut out = Outcome { lots: closes.len(), transactions: txs.len(), ..Default::default() };
    for (t, lots) in &txs {
        out.refunds += lots.len();
        let candidates = closes.iter().filter(|c| *c + p.delay_min <= *t && *t <= *c + p.delay_max + p.window_secs).count();
        if candidates == 1 {
            out.linked_refunds += lots.len();
        }
    }
    out
}

/// A table of linked fractions for a few lot densities, naive versus batched.
pub fn table(seed: u64, losers_per_lot: u32, delay_min: i64, delay_max: i64, window_secs: i64, batch_max: usize) -> String {
    use rand::SeedableRng;
    let mut s = String::new();
    s.push_str(&format!(
        "| lots/hour | refunds | naive: txs | naive: linked | batched: txs | batched: linked |\n|---|---|---|---|---|---|\n"
    ));
    for rate in [0.5, 2.0, 6.0, 20.0, 60.0] {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let naive = simulate(&mut rng, &Params { lots_per_hour: rate, losers_per_lot, delay_min, delay_max, window_secs: 0, batch_max: 1, hours: 48.0 });
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let batched = simulate(&mut rng, &Params { lots_per_hour: rate, losers_per_lot, delay_min, delay_max, window_secs, batch_max, hours: 48.0 });
        s.push_str(&format!(
            "| {rate} | {} | {} | {:.0}% | {} | {:.0}% |\n",
            naive.refunds,
            naive.transactions,
            naive.linked_fraction() * 100.0,
            batched.transactions,
            batched.linked_fraction() * 100.0
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn batching_never_links_more_than_naive_and_uses_fewer_transactions() {
        for rate in [1.0, 10.0, 60.0] {
            let mut a = rand::rngs::StdRng::seed_from_u64(7);
            let mut b = rand::rngs::StdRng::seed_from_u64(7);
            let base = Params { lots_per_hour: rate, losers_per_lot: 4, delay_min: 600, delay_max: 14_400, window_secs: 0, batch_max: 1, hours: 48.0 };
            let naive = simulate(&mut a, &base);
            let batched = simulate(&mut b, &Params { window_secs: 600, batch_max: 8, ..base });
            assert_eq!(naive.refunds, batched.refunds);
            assert!(batched.transactions <= naive.transactions, "rate {rate}");
            assert!(batched.linked_fraction() <= naive.linked_fraction() + 0.02, "rate {rate}: {} vs {}", batched.linked_fraction(), naive.linked_fraction());
        }
    }

    #[test]
    fn an_isolated_lot_is_always_linkable_by_timing() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let o = simulate(&mut rng, &Params { lots_per_hour: 0.05, losers_per_lot: 3, delay_min: 30, delay_max: 300, window_secs: 60, batch_max: 8, hours: 200.0 });
        assert!(o.linked_fraction() > 0.9, "sparse lots cannot hide behind each other: {}", o.linked_fraction());
    }
}
