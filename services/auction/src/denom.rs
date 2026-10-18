//! Standard deposit amounts.
//!
//! A refund is the lot's deposit sent back, so if two lots use different unusual deposits, anyone who later
//! learns a refund's amount (a recipient who unshields, a wallet export, a disclosed note) can tell which lot
//! it came from. Deposits drawn from one short menu make a refund of 0.05 ZEC look like every other 0.05 ZEC
//! refund. Splitting refunds into denominations was considered instead and rejected: each extra output costs a
//! ZIP-317 action fee and the number of pieces leaks what the single amount would have.

/// 0.001 to 5 ZEC in a 1-2-5 series.
pub const DEPOSIT_MENU_ZATS: [u64; 12] =
    [100_000, 200_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000, 20_000_000, 50_000_000, 100_000_000, 200_000_000, 500_000_000];

pub fn is_standard(zats: u64) -> bool {
    DEPOSIT_MENU_ZATS.contains(&zats)
}

/// The menu as ZEC amounts for an error message: "0.001, 0.002, … 5".
pub fn menu_zec() -> String {
    DEPOSIT_MENU_ZATS.iter().map(|z| zec(*z)).collect::<Vec<_>>().join(", ")
}

fn zec(zats: u64) -> String {
    let whole = zats / 100_000_000;
    let frac = zats % 100_000_000;
    if frac == 0 {
        return whole.to_string();
    }
    let s = format!("{whole}.{frac:08}");
    s.trim_end_matches('0').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_is_ascending_one_two_five() {
        assert!(DEPOSIT_MENU_ZATS.windows(2).all(|w| w[0] < w[1]));
        for z in DEPOSIT_MENU_ZATS {
            let lead = z.to_string().chars().next().unwrap();
            assert!(matches!(lead, '1' | '2' | '5'), "{z}");
            assert!(is_standard(z));
        }
        assert!(!is_standard(123_456));
        assert!(!is_standard(0));
        assert!(!is_standard(1_500_000));
    }

    #[test]
    fn menu_prints_in_zec() {
        assert_eq!(menu_zec(), "0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5");
    }
}
