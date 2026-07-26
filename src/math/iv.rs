use statrs::distribution::{Continuous, Normal};
use std::sync::LazyLock;
use crate::types::OptionType;
use super::black_scholes::{bs_price, BsInputs};

static NORM: LazyLock<Normal> = LazyLock::new(|| Normal::new(0.0, 1.0).unwrap());
pub fn implied_volatility(
    market_price: f64,
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    opt_type: OptionType,
) -> Option<f64> {
    if t <= 0.0 || market_price <= 0.0 || s <= 0.0 || k <= 0.0 {
        return None;
    }
    let intrinsic = match opt_type {
        OptionType::Call => (s - k).max(0.0),
        OptionType::Put => (k - s).max(0.0),
    };
    if market_price < intrinsic {
        return None;
    }
    let mut sigma = 0.5_f64;
    let n = &*NORM;
    for _ in 0..100 {
        let price = bs_price(&BsInputs { s, k, t, r, sigma, opt_type });
        let diff = price - market_price;
        if diff.abs() < 1e-6 {
            return Some(sigma);
        }
        let sq = t.sqrt();
        let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sq);
        let vega = s * n.pdf(d1) * sq;
        if vega < 1e-10 {
            break;
        }
        sigma = (sigma - diff / vega).max(0.001).min(10.0);
    }
    None
}
