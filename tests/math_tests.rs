use options_scanner::math::black_scholes::{bs_price, bs_price_with_greeks, BsInputs};
use options_scanner::math::iv::implied_volatility;
use options_scanner::types::OptionType;

#[test]
fn test_call_price() {
    let p = bs_price(&BsInputs {
        s: 150.0, k: 150.0, t: 30.0 / 365.0, r: 0.05, sigma: 0.25,
        opt_type: OptionType::Call,
    });
    assert!(p > 2.0 && p < 8.0, "Call price {} unreasonable", p);
}

#[test]
fn test_put_call_parity() {
    let (s, k, t, r, sig) = (100.0, 100.0, 0.5, 0.05, 0.2);
    let c = bs_price(&BsInputs { s, k, t, r, sigma: sig, opt_type: OptionType::Call });
    let p = bs_price(&BsInputs { s, k, t, r, sigma: sig, opt_type: OptionType::Put });
    assert!((c - p - s + k * (-r * t).exp()).abs() < 1e-6);
}

#[test]
fn test_greeks_reasonable() {
    let (_, g) = bs_price_with_greeks(&BsInputs {
        s: 100.0, k: 100.0, t: 0.25, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Call,
    });
    assert!(g.delta > 0.4 && g.delta < 0.7);
    assert!(g.gamma > 0.0);
    assert!(g.theta < 0.0);
    assert!(g.vega > 0.0);
}

#[test]
fn test_iv_roundtrip() {
    let sigma = 0.3;
    let p = bs_price(&BsInputs {
        s: 100.0, k: 100.0, t: 0.25, r: 0.05, sigma,
        opt_type: OptionType::Call,
    });
    let iv = implied_volatility(p, 100.0, 100.0, 0.25, 0.05, OptionType::Call).unwrap();
    assert!((iv - sigma).abs() < 1e-4);
}

use options_scanner::math::iv_rank::compute_iv_stats;

// --- Black-Scholes boundary conditions ---

#[test]
fn test_bs_zero_time_to_expiry() {
    // At expiry, call = max(S-K, 0), put = max(K-S, 0)
    let c = bs_price(&BsInputs {
        s: 110.0, k: 100.0, t: 0.0, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Call,
    });
    assert!((c - 10.0).abs() < 1e-6, "ITM call at expiry should be intrinsic");

    let p = bs_price(&BsInputs {
        s: 90.0, k: 100.0, t: 0.0, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Put,
    });
    assert!((p - 10.0).abs() < 1e-6, "ITM put at expiry should be intrinsic");

    let otm = bs_price(&BsInputs {
        s: 90.0, k: 100.0, t: 0.0, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Call,
    });
    assert!(otm.abs() < 1e-6, "OTM call at expiry should be zero");
}

#[test]
fn test_bs_zero_volatility() {
    // The implementation returns intrinsic value when sigma <= 0.
    // For an ITM call (S=110, K=100), intrinsic = 10.
    let c = bs_price(&BsInputs {
        s: 110.0, k: 100.0, t: 1.0, r: 0.05, sigma: 0.0,
        opt_type: OptionType::Call,
    });
    assert!((c - 10.0).abs() < 1e-6, "Zero-vol ITM call should return intrinsic, got {}", c);
}

#[test]
fn test_bs_deep_itm_call() {
    let p = bs_price(&BsInputs {
        s: 200.0, k: 50.0, t: 0.25, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Call,
    });
    // Deep ITM call ≈ S - K*exp(-rT)
    let intrinsic = 200.0_f64 - 50.0_f64 * (-0.05_f64 * 0.25_f64).exp();
    assert!((p - intrinsic).abs() < 1.0, "Deep ITM call {} far from intrinsic {}", p, intrinsic);
}

#[test]
fn test_bs_deep_otm_put() {
    let p = bs_price(&BsInputs {
        s: 200.0, k: 50.0, t: 0.25, r: 0.05, sigma: 0.2,
        opt_type: OptionType::Put,
    });
    assert!(p < 0.01, "Deep OTM put should be near zero, got {}", p);
}

// --- IV solver edge cases ---

#[test]
fn test_iv_negative_price_returns_none() {
    let iv = implied_volatility(-1.0, 100.0, 100.0, 0.25, 0.05, OptionType::Call);
    assert!(iv.is_none(), "Negative price should return None");
}

#[test]
fn test_iv_zero_time_returns_none() {
    let iv = implied_volatility(5.0, 100.0, 100.0, 0.0, 0.05, OptionType::Call);
    assert!(iv.is_none(), "Zero time should return None");
}

#[test]
fn test_iv_below_intrinsic_returns_none() {
    // Call with S=110, K=100 → intrinsic = 10. Price of 5 is impossible.
    let iv = implied_volatility(5.0, 110.0, 100.0, 0.25, 0.05, OptionType::Call);
    assert!(iv.is_none(), "Price below intrinsic should return None");
}

#[test]
fn test_iv_put_roundtrip() {
    let sigma = 0.4;
    let p = bs_price(&BsInputs {
        s: 95.0, k: 100.0, t: 0.5, r: 0.03, sigma,
        opt_type: OptionType::Put,
    });
    let iv = implied_volatility(p, 95.0, 100.0, 0.5, 0.03, OptionType::Put).unwrap();
    assert!((iv - sigma).abs() < 1e-4, "Put IV roundtrip failed: {} != {}", iv, sigma);
}

#[test]
fn test_iv_high_vol() {
    let sigma = 2.0;
    let p = bs_price(&BsInputs {
        s: 100.0, k: 100.0, t: 0.25, r: 0.05, sigma,
        opt_type: OptionType::Call,
    });
    let iv = implied_volatility(p, 100.0, 100.0, 0.25, 0.05, OptionType::Call).unwrap();
    assert!((iv - sigma).abs() < 0.01, "High-vol IV roundtrip failed: {} != {}", iv, sigma);
}

// --- NaN safety ---
#[test]
fn test_nan_sort_does_not_panic() {
    let mut scores = vec![85.0, f64::NAN, 92.0, f64::NAN, 78.0];
    // total_cmp should handle NaN without panicking
    scores.sort_by(|a, b| b.total_cmp(a));
    // NaN is treated as largest in total_cmp, so descending sort puts NaN first.
    // Verify non-NaN values are in correct descending order.
    let non_nan: Vec<f64> = scores.iter().copied().filter(|x| !x.is_nan()).collect();
    assert_eq!(non_nan, vec![92.0, 85.0, 78.0]);
}
// --- IV rank ---

#[test]
fn test_iv_rank_basic() {
    let ivs = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let stats = compute_iv_stats(0.3, &ivs).unwrap();
    assert!((stats.iv_rank - 0.5).abs() < 1e-6, "IV rank of median should be 0.5");
    assert!((stats.iv_low - 0.1).abs() < 1e-6);
    assert!((stats.iv_high - 0.5).abs() < 1e-6);
}

#[test]
fn test_iv_rank_empty_returns_none() {
    let stats = compute_iv_stats(0.3, &[]);
    assert!(stats.is_none());
}

#[test]
fn test_iv_rank_all_same() {
    let ivs = vec![0.25, 0.25, 0.25, 0.25];
    let stats = compute_iv_stats(0.25, &ivs).unwrap();
    // When range is zero, iv_rank defaults to 0.5
    assert!((stats.iv_rank - 0.5).abs() < 1e-6);
}

#[test]
fn test_iv_rank_with_nan_values() {
    // NaN values should not cause a panic thanks to total_cmp
    let ivs = vec![0.1, f64::NAN, 0.3, 0.5];
    let stats = compute_iv_stats(0.3, &ivs);
    assert!(stats.is_some(), "iv_rank should handle NaN without panicking");
}
