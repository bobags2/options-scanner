use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct CheapDirectionalStrategy;

#[async_trait]
impl Strategy for CheapDirectionalStrategy {
    fn name(&self) -> &str {
        "Cheap Directional"
    }

    fn description(&self) -> &str {
        "Finds cheap, slightly OTM options with reasonable time left — low-cost bets on a moderate move."
    }

    async fn scan(
        &self,
        chains: &[OptionChain],
        underlying_prices: &UnderlyingPrices,
        _config: &StrategiesConfig,
        risk_free_rate: f64,
    ) -> Vec<Opportunity> {
        let today = Utc::now().date_naive();
        let r = risk_free_rate;
        let mut opps = Vec::new();

        for chain in chains {
            let underlying = match underlying_prices.get(&chain.ticker) {
                Some(p) => *p,
                None => continue,
            };

            for c in &chain.contracts {
                let dte = c.days_to_expiration(today);
                if dte < 14 || dte > 90 {
                    continue;
                }

                if c.spread_pct() > 30.0 || c.volume < 10 {
                    continue;
                }

                let mid = c.mid_price();
                if mid <= 0.0 {
                    continue;
                }

                // Only consider OTM options
                let is_otm = match c.option_type {
                    OptionType::Call => c.strike > underlying,
                    OptionType::Put => c.strike < underlying,
                };
                if !is_otm {
                    continue;
                }

                // Option should be cheap — less than 3% of underlying
                let pct_of_underlying = mid / underlying * 100.0;
                if pct_of_underlying > 3.0 {
                    continue;
                }

                // Strike should be within 10% of underlying
                let otm_pct = (c.strike - underlying).abs() / underlying * 100.0;
                if otm_pct > 10.0 || otm_pct < 1.0 {
                    continue;
                }

                let iv = c.implied_volatility.unwrap_or(0.25);
                let t = dte as f64 / 365.0;
                let (_, greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: c.strike,
                    t,
                    r,
                    sigma: iv,
                    opt_type: c.option_type,
                });

                let delta_abs = greeks.delta.abs();
                if delta_abs < 0.1 || delta_abs > 0.45 {
                    continue;
                }

                // Score: cheap + decent delta + reasonable DTE
                let score = (1.0 - pct_of_underlying / 3.0) * 30.0
                    + (delta_abs / 0.45) * 40.0
                    + (dte as f64 / 90.0) * 30.0;

                let direction = match c.option_type {
                    OptionType::Call => "bullish",
                    OptionType::Put => "bearish",
                };

                let explanation = format!(
                    "This {} {} ${:.0} option costs ${:.2} per share (${:.0} per contract) — just {:.1}% of the stock price. \
                     It's {:.1}% OTM with {} DTE and a delta of {:.2}. \
                     This is a low-cost directional bet: if {} moves {} toward ${:.0}, this option gains value. \
                     Low entry cost means limited downside if the trade doesn't work out.",
                    c.ticker,
                    direction,
                    c.strike,
                    mid,
                    mid * 100.0,
                    pct_of_underlying,
                    otm_pct,
                    dte,
                    greeks.delta,
                    c.ticker,
                    direction,
                    c.strike,
                );

                let risk = format!(
                    "This is OTM — if {} doesn't move enough by expiration, it expires worthless. \
                     Theta is {:.3}/day (losing ${:.0} per contract daily). \
                     Breakeven at expiration: ${:.2}.",
                    c.ticker,
                    greeks.theta,
                    greeks.theta.abs() * 100.0,
                    match c.option_type {
                        OptionType::Call => c.strike + mid,
                        OptionType::Put => c.strike - mid,
                    },
                );

                opps.push(Opportunity {
                    contract: c.clone(),
                    greeks,
                    strategy: StrategyType::CheapDirectional,
                    score,
                    explanation,
                    risk_summary: risk,
                });
            }
        }

        opps.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        opps.truncate(50);
        opps
    }
}
