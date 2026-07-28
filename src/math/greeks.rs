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

use crate::types::Opportunity;
use std::collections::HashMap;

/// Sum delta/theta/vega/rho across opportunities. Gamma is deliberately
/// excluded — it measures sensitivity to a *specific* underlying's move and
/// summing it across AAPL + MSFT + TSLA has no coherent meaning. Use
/// `gamma_by_ticker` for the per-underlying breakdown.
pub fn aggregate_greeks(opps: &[Opportunity]) -> Greeks {
    let mut delta = 0.0;
    let gamma = 0.0; // kept at 0 — see doc comment
    let mut theta = 0.0;
    let mut vega = 0.0;
    let mut rho = 0.0;
    for o in opps {
        delta += o.greeks.delta;
        theta += o.greeks.theta;
        vega += o.greeks.vega;
        rho += o.greeks.rho;
    }
    Greeks { delta, gamma, theta, vega, rho }
}

/// Sum gamma per underlying ticker. Gamma only makes sense within a single
/// underlying, so we return a map keyed by ticker.
pub fn gamma_by_ticker(opps: &[Opportunity]) -> HashMap<String, f64> {
    let mut map: HashMap<String, f64> = HashMap::new();
    for o in opps {
        *map.entry(o.contract.ticker.clone()).or_insert(0.0) += o.greeks.gamma;
    }
    map
}

/// Compact CLI summary of the cross-ticker rollup. Gamma is omitted here;
/// pair with `format_gamma_breakdown` for the per-ticker view.
pub fn format_greeks_summary(g: &Greeks) -> String {
    format!(
        "Δ {:>+.2}  Θ {:>+.3}/day  V {:>.3}  ρ {:>+.3}",
        g.delta, g.theta, g.vega, g.rho,
    )
}

/// Format the per-ticker gamma breakdown, sorted by ticker.
pub fn format_gamma_breakdown(by_ticker: &HashMap<String, f64>) -> String {
    let mut entries: Vec<_> = by_ticker.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
        .into_iter()
        .filter(|(_, g)| (*g).abs() > 1e-6)
        .map(|(t, g)| format!("{}:{:.4}", t, g))
        .collect::<Vec<_>>()
        .join("  ")

}
