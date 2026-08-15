//! Rendering an atomic token amount as a decimal string.

use alloy_primitives::U256;

/// Render `value` scaled down by `decimals` places.
///
/// A whole amount keeps a single trailing fraction digit (`0` renders as
/// `"0.0"`, one whole USDC as `"1.0"`); any further trailing zeros are dropped.
/// With `decimals == 0` there is no fraction and no point.
pub fn format(value: U256, decimals: u8) -> String {
    let mut digits = value.to_string();
    if decimals == 0 {
        return digits;
    }
    let decimals = decimals as usize;
    // Guarantee at least one whole digit ahead of the point.
    while digits.len() <= decimals {
        digits.insert(0, '0');
    }
    digits.insert(digits.len() - decimals, '.');
    while digits.ends_with('0') && !digits[..digits.len() - 1].ends_with('.') {
        digits.pop();
    }
    digits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_amounts_keep_one_fraction_digit() {
        assert_eq!(format(U256::ZERO, 18), "0.0");
        assert_eq!(format(U256::from(1_000_000u64), 6), "1.0");
        assert_eq!(format(U256::from(10u64).pow(U256::from(18u64)), 18), "1.0");
    }

    #[test]
    fn fractions_keep_their_significant_digits() {
        assert_eq!(format(U256::from(1_500_000u64), 6), "1.5");
        assert_eq!(format(U256::from(1u64), 6), "0.000001");
        assert_eq!(format(U256::from(1u64), 18), "0.000000000000000001");
        assert_eq!(format(U256::from(123_456u64), 6), "0.123456");
        assert_eq!(format(U256::from(1_000_001u64), 6), "1.000001");
    }

    #[test]
    fn no_point_without_decimals() {
        assert_eq!(format(U256::from(5u64), 0), "5");
        assert_eq!(format(U256::ZERO, 0), "0");
    }

    #[test]
    fn large_amounts_do_not_lose_precision() {
        // 165 tokens at 18 decimals, the shape staked balances take on chain.
        let value = U256::from_str_radix("165000000000000000000", 10).unwrap();
        assert_eq!(format(value, 18), "165.0");
        // U256::MAX must render exactly, which a float path could not do.
        assert_eq!(
            format(U256::MAX, 18),
            "115792089237316195423570985008687907853269984665640564039457.584007913129639935"
        );
    }
}
