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
