use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct WheelStrategy;

#[async_trait]
impl Strategy for WheelStrategy {
    fn name(&self) -> &str {
        "Wheel Setup"
    }

    fn description(&self) -> &str {
        "Finds good candidates for the wheel strategy — selling cash-secured puts on stocks you'd be happy to own, then covered calls if assigned."
    }

    async fn scan(
        &self,
        chains: &[OptionChain],
        underlying_prices: &UnderlyingPrices,
        config: &StrategiesConfig,
        risk_free_rate: f64,
    ) -> Vec<Opportunity> {
        let today = Utc::now().date_naive();
        let r = risk_free_rate;
        let target_delta = config.wheel.target_delta;
        let min_premium = config.wheel.min_annualized_premium;
        let mut opps = Vec::new();

        for chain in chains {
            let underlying = match underlying_prices.get(&chain.ticker) {
                Some(p) => *p,
                None => continue,
            };

            for c in &chain.contracts {
                if c.option_type != OptionType::Put {
                    continue;
                }

                let dte = c.days_to_expiration(today);
                if dte < 20 || dte > 60 {
                    continue;
                }

                if c.spread_pct() > 50.0 || c.volume == 0 {
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
                    opt_type: OptionType::Put,
                });

                let delta_abs = greeks.delta.abs();
                if (delta_abs - target_delta).abs() > 0.15 {
                    continue;
                }

                let premium = c.mid_price();
                if premium <= 0.0 {
                    continue;
                }
                let annualized_premium = (premium / underlying) * (365.0 / dte as f64) * 100.0;

                if annualized_premium < min_premium {
                    continue;
                }

                let score = (annualized_premium.min(100.0) / 100.0) * 50.0
                    + (1.0 - (delta_abs - target_delta).abs() / 0.3) * 50.0;

                let discount = (underlying - c.strike) / underlying * 100.0;

                let explanation = format!(
                    "This {} ${:.0} put expires in {} days with a delta of {:.2}. \
                     If assigned, you'd buy at ${:.0} — a {:.1}% discount from the current price of ${:.2}. \
                     Annualized premium is {:.1}%. The wheel strategy: sell this put, collect premium. \
                     If the stock stays above ${:.0}, you keep the premium. If assigned, you own at a discount and can sell covered calls next.",
                    c.ticker,
                    c.strike,
                    dte,
                    greeks.delta,
                    c.strike,
                    discount,
                    underlying,
                    annualized_premium,
                    c.strike,
                );

                let risk = format!(
                    "If {} drops significantly below ${:.0}, you're assigned and holding a losing position. \
                     Max loss is ${:.0} per contract (stock goes to zero). Theta is {:.3}/day.",
                    c.ticker,
                    c.strike,
                    c.strike * 100.0,
                    greeks.theta,
                );

                opps.push(Opportunity {
                    contract: c.clone(),
                    greeks,
                    strategy: StrategyType::WheelSetup,
                    score,
                    explanation,
                    risk_summary: risk,
                });
            }
        }

        opps.sort_by(|a, b| b.score.total_cmp(&a.score));
        opps.truncate(50);
        opps
    }
}
