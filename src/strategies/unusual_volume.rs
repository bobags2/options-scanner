use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct UnusualVolumeStrategy;

#[async_trait]
impl Strategy for UnusualVolumeStrategy {
    fn name(&self) -> &str {
        "Unusual Volume"
    }

    fn description(&self) -> &str {
        "Finds contracts where today's volume is significantly higher than open interest, suggesting big institutional positioning."
    }

    async fn scan(
        &self,
        chains: &[OptionChain],
        underlying_prices: &UnderlyingPrices,
        config: &StrategiesConfig,
        risk_free_rate: f64,
    ) -> Vec<Opportunity> {
        let min_ratio = config.unusual_volume.min_volume_oi_ratio;
        let min_volume = config.unusual_volume.min_volume;
        let today = Utc::now().date_naive();
        let r = risk_free_rate;

        let mut opps = Vec::new();

        for chain in chains {
            let underlying = match underlying_prices.get(&chain.ticker) {
                Some(p) => *p,
                None => continue,
            };

            for c in &chain.contracts {
                if c.volume < min_volume || c.open_interest == 0 {
                    continue;
                }

                let ratio = c.volume as f64 / c.open_interest as f64;
                if ratio < min_ratio {
                    continue;
                }

                // Skip illiquid contracts
                if c.spread_pct() > 50.0 {
                    continue;
                }

                let dte = c.days_to_expiration(today);
                if dte < 1 {
                    continue;
                }
                let t = dte as f64 / 365.0;

                let iv = c.implied_volatility.unwrap_or(0.25);
                let (_, greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: c.strike,
                    t,
                    r,
                    sigma: iv,
                    opt_type: c.option_type,
                });

                let score = ratio.min(20.0) / 20.0 * 100.0;

                let direction = match c.option_type {
                    OptionType::Call => "bullish",
                    OptionType::Put => "bearish",
                };

                let moneyness = if (c.strike - underlying).abs() / underlying < 0.02 {
                    "at-the-money"
                } else if (c.option_type == OptionType::Call && c.strike > underlying)
                    || (c.option_type == OptionType::Put && c.strike < underlying)
                {
                    "out-of-the-money"
                } else {
                    "in-the-money"
                };

                let explanation = format!(
                    "Volume is {:.1}x the open interest on this {} {} {} option ({} strike, {} DTE). \
                     This suggests aggressive new positioning by large traders. The contract is {} \
                     and has a delta of {:.2}, meaning it will move roughly ${:.2} for every $1 move in the stock.",
                    ratio,
                    moneyness,
                    c.ticker,
                    direction,
                    c.strike,
                    dte,
                    moneyness,
                    greeks.delta,
                    greeks.delta.abs()
                );

                let risk = if dte < 14 {
                    format!("High time decay risk — expires in {} days. Theta is {:.3} per day.", dte, greeks.theta)
                } else if moneyness == "out-of-the-money" {
                    "OTM options can expire worthless if the stock doesn't move enough.".to_string()
                } else {
                    format!("Delta of {:.2} means moderate directional exposure.", greeks.delta)
                };

                opps.push(Opportunity {
                    contract: c.clone(),
                    greeks,
                    strategy: StrategyType::UnusualVolume,
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
