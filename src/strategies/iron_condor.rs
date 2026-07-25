use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct IronCondorStrategy;

#[async_trait]
impl Strategy for IronCondorStrategy {
    fn name(&self) -> &str {
        "Iron Condor"
    }

    fn description(&self) -> &str {
        "Finds iron condor setups — selling a put spread and call spread around the current price to collect premium when expecting low volatility."
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

            let puts: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Put && c.mid_price() > 0.05 && c.spread_pct() < 40.0)
                .filter(|c| c.strike < underlying * 0.97 && c.strike > underlying * 0.80)
                .collect();

            let calls: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call && c.mid_price() > 0.05 && c.spread_pct() < 40.0)
                .filter(|c| c.strike > underlying * 1.03 && c.strike < underlying * 1.20)
                .collect();

            if puts.len() < 2 || calls.len() < 2 {
                continue;
            }

            for (i, short_put) in puts.iter().enumerate() {
                let long_put = match puts.iter().skip(i + 1).next() {
                    Some(p) => p,
                    None => continue,
                };
                let put_width = short_put.strike - long_put.strike;
                if put_width <= 0.0 {
                    continue;
                }
                let put_credit = short_put.mid_price() - long_put.mid_price();
                if put_credit <= 0.0 {
                    continue;
                }

                for (j, short_call) in calls.iter().enumerate() {
                    let long_call = match calls.iter().skip(j + 1).next() {
                        Some(c) => c,
                        None => continue,
                    };
                    let call_width = long_call.strike - short_call.strike;
                    if call_width <= 0.0 {
                        continue;
                    }
                    let call_credit = short_call.mid_price() - long_call.mid_price();
                    if call_credit <= 0.0 {
                        continue;
                    }

                    let total_credit = put_credit + call_credit;
                    let max_width = put_width.max(call_width);
                    let max_loss = max_width - total_credit;
                    if max_loss <= 0.0 {
                        continue;
                    }

                    let risk_reward = total_credit / max_loss;
                    if risk_reward < 0.25 {
                        continue;
                    }

                    let put_iv = short_put.implied_volatility.unwrap_or(0.25);
                    let (_, put_greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: short_put.strike,
                        t,
                        r,
                        sigma: put_iv,
                        opt_type: OptionType::Put,
                    });

                    let call_iv = short_call.implied_volatility.unwrap_or(0.25);
                    let (_, call_greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: short_call.strike,
                        t,
                        r,
                        sigma: call_iv,
                        opt_type: OptionType::Call,
                    });

                    let prob_put = 1.0 - put_greeks.delta.abs();
                    let prob_call = 1.0 - call_greeks.delta;
                    let prob_profit = prob_put * prob_call;

                    let score = (risk_reward.min(1.0)) * 40.0
                        + (prob_profit.min(1.0)) * 40.0
                        + (total_credit / underlying * 100.0).min(3.0) / 3.0 * 20.0;

                    let explanation = format!(
                        "Iron condor on {}: sell ${:.0} put / buy ${:.0} put, sell ${:.0} call / buy ${:.0} call. \
                         Total credit: ${:.2}/share (${:.0}/contract). Expiration in {} days. \
                         Profit if {} stays between ${:.0} and ${:.0} at expiration. \
                         Estimated probability of profit: {:.0}%.",
                        chain.ticker,
                        short_put.strike,
                        long_put.strike,
                        short_call.strike,
                        long_call.strike,
                        total_credit,
                        total_credit * 100.0,
                        dte,
                        chain.ticker,
                        short_put.strike - put_credit,
                        short_call.strike + call_credit,
                        prob_profit * 100.0,
                    );

                    let risk = format!(
                        "Max loss: ${:.2}/share (${:.0}/contract) if {} moves past either wing. \
                         Risk/reward: {:.2}:1. Net theta is positive — time decay helps. \
                         Combined delta: {:.2}.",
                        max_loss,
                        max_loss * 100.0,
                        chain.ticker,
                        risk_reward,
                        put_greeks.delta + call_greeks.delta,
                    );

                    opps.push(Opportunity {
                        contract: (*short_put).clone(),
                        greeks: crate::types::Greeks {
                            delta: put_greeks.delta + call_greeks.delta,
                            gamma: put_greeks.gamma + call_greeks.gamma,
                            theta: put_greeks.theta + call_greeks.theta,
                            vega: put_greeks.vega + call_greeks.vega,
                            rho: put_greeks.rho + call_greeks.rho,
                        },
                        strategy: StrategyType::IronCondor,
                        score,
                        explanation,
                        risk_summary: risk,
                    });

                    break;
                }
                break;
            }
        }

        opps.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        opps.truncate(50);
        opps
    }
}
