use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct RatioSpreadStrategy;

#[async_trait]
impl Strategy for RatioSpreadStrategy {
    fn name(&self) -> &str {
        "Ratio Spread"
    }

    fn description(&self) -> &str {
        "Finds ratio call spreads — buy 1 ITM call, sell 2 OTM calls for near-zero cost with big upside if the stock rises modestly."
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

            let dte = chain.contracts.first()
                .map(|c| c.days_to_expiration(today))
                .unwrap_or(0);
            if dte < 20 || dte > 60 {
                continue;
            }
            let t = dte as f64 / 365.0;

            let calls: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call && c.mid_price() > 0.05 && c.spread_pct() < 40.0)
                .collect();

            if calls.len() < 4 {
                continue;
            }

            for long_call in &calls {
                if long_call.strike < underlying * 0.95 || long_call.strike > underlying * 1.05 {
                    continue;
                }

                for short_call in &calls {
                    let otm_pct = (short_call.strike - underlying) / underlying * 100.0;
                    if otm_pct < 3.0 || otm_pct > 12.0 {
                        continue;
                    }

                    let long_mid = long_call.mid_price();
                    let short_mid = short_call.mid_price();
                    let net_cost = long_mid - 2.0 * short_mid;
                    let width = short_call.strike - long_call.strike;
                    if width <= 0.0 {
                        continue;
                    }

                    if net_cost.abs() > long_mid * 0.25 {
                        continue;
                    }

                    let max_profit = width - net_cost.max(0.0);
                    if max_profit <= 0.0 {
                        continue;
                    }

                    let long_iv = long_call.implied_volatility.unwrap_or(0.25);
                    let (_, long_greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: long_call.strike,
                        t,
                        r,
                        sigma: long_iv,
                        opt_type: OptionType::Call,
                    });

                    let short_iv = short_call.implied_volatility.unwrap_or(0.25);
                    let (_, short_greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: short_call.strike,
                        t,
                        r,
                        sigma: short_iv,
                        opt_type: OptionType::Call,
                    });

                    let net_delta = long_greeks.delta - 2.0 * short_greeks.delta;
                    let net_theta = long_greeks.theta - 2.0 * short_greeks.theta;
                    let net_vega = long_greeks.vega - 2.0 * short_greeks.vega;

                    let cost_pct = net_cost.max(0.0) / underlying * 100.0;
                    let score = (max_profit / underlying * 100.0).min(5.0) / 5.0 * 40.0
                        + (1.0 - cost_pct / 2.0).max(0.0) * 40.0
                        + net_delta.abs().min(0.5) / 0.5 * 20.0;

                    let explanation = format!(
                        "Ratio call spread on {}: buy 1x ${:.0} call (${:.2}), sell 2x ${:.0} calls (${:.2} each). \
                         Net cost: ${:.2}/share (${:.0}/contract). Expiration in {} days. \
                         Max profit ${:.2}/share if {} pins ${:.0} at expiration. \
                         Near-zero cost means asymmetric upside with defined risk below.",
                        chain.ticker,
                        long_call.strike,
                        long_mid,
                        short_call.strike,
                        short_mid,
                        net_cost,
                        net_cost * 100.0,
                        dte,
                        max_profit,
                        chain.ticker,
                        short_call.strike,
                    );

                    let risk = format!(
                        "If {} rallies past ${:.0}, the 2x short calls create uncapped upside risk. \
                         Max loss: ${:.2}/share if stock stays below ${:.0}. \
                         Net delta: {:.2}, net theta: {:.3}/day, net vega: {:.3}.",
                        chain.ticker,
                        short_call.strike,
                        net_cost.max(0.0),
                        long_call.strike,
                        net_delta,
                        net_theta,
                        net_vega,
                    );

                    opps.push(Opportunity {
                        contract: (*long_call).clone(),
                        greeks: crate::types::Greeks {
                            delta: net_delta,
                            gamma: long_greeks.gamma - 2.0 * short_greeks.gamma,
                            theta: net_theta,
                            vega: net_vega,
                            rho: long_greeks.rho - 2.0 * short_greeks.rho,
                        },
                        strategy: StrategyType::RatioSpread,
                        score,
                        explanation,
                        risk_summary: risk,
                    });

                    break;
                }
            }
        }

        opps.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        opps.truncate(50);
        opps
    }
}
