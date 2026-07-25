use crate::types::{Greeks, OptionType};
use super::black_scholes::{bs_price_with_greeks, BsInputs};

pub fn compute_greeks(
    s: f64,
    k: f64,
    t: f64,
    r: f64,
    sigma: f64,
    opt_type: OptionType,
) -> Greeks {
    let (_, g) = bs_price_with_greeks(&BsInputs { s, k, t, r, sigma, opt_type });
    g
}
