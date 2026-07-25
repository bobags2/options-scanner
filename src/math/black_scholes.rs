use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use crate::types::{Greeks, OptionType};

fn norm() -> Normal {
    Normal::new(0.0, 1.0).unwrap()
}

pub struct BsInputs {
    pub s: f64,
    pub k: f64,
    pub t: f64,
    pub r: f64,
    pub sigma: f64,
    pub opt_type: OptionType,
}

pub fn bs_price(i: &BsInputs) -> f64 {
    let &BsInputs { s, k, t, r, sigma, opt_type } = i;
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        return match opt_type {
            OptionType::Call => (s - k).max(0.0),
            OptionType::Put => (k - s).max(0.0),
        };
    }
    let sq = t.sqrt();
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sq);
    let d2 = d1 - sigma * sq;
    let n = norm();
    match opt_type {
        OptionType::Call => s * n.cdf(d1) - k * (-r * t).exp() * n.cdf(d2),
        OptionType::Put => k * (-r * t).exp() * n.cdf(-d2) - s * n.cdf(-d1),
    }
}

pub fn bs_price_with_greeks(i: &BsInputs) -> (f64, Greeks) {
    let &BsInputs { s, k, t, r, sigma, opt_type } = i;
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        let p = match opt_type {
            OptionType::Call => (s - k).max(0.0),
            OptionType::Put => (k - s).max(0.0),
        };
        let d = match opt_type {
            OptionType::Call => if s > k { 1.0 } else { 0.0 },
            OptionType::Put => if s < k { -1.0 } else { 0.0 },
        };
        return (
            p,
            Greeks { delta: d, gamma: 0.0, theta: 0.0, vega: 0.0, rho: 0.0 },
        );
    }
    let sq = t.sqrt();
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sq);
    let d2 = d1 - sigma * sq;
    let n = norm();
    let nd1 = n.pdf(d1);
    let price = match opt_type {
        OptionType::Call => s * n.cdf(d1) - k * (-r * t).exp() * n.cdf(d2),
        OptionType::Put => k * (-r * t).exp() * n.cdf(-d2) - s * n.cdf(-d1),
    };
    let delta = match opt_type {
        OptionType::Call => n.cdf(d1),
        OptionType::Put => n.cdf(d1) - 1.0,
    };
    let gamma = nd1 / (s * sigma * sq);
    let theta = (match opt_type {
        OptionType::Call => {
            -(s * nd1 * sigma) / (2.0 * sq) - r * k * (-r * t).exp() * n.cdf(d2)
        }
        OptionType::Put => {
            -(s * nd1 * sigma) / (2.0 * sq) + r * k * (-r * t).exp() * n.cdf(-d2)
        }
    }) / 365.0;
    let vega = s * nd1 * sq / 100.0;
    let rho = match opt_type {
        OptionType::Call => k * t * (-r * t).exp() * n.cdf(d2) / 100.0,
        OptionType::Put => -k * t * (-r * t).exp() * n.cdf(-d2) / 100.0,
    };
    (price, Greeks { delta, gamma, theta, vega, rho })
}
