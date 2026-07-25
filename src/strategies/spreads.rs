use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct SpreadStrategy;

#[async_trait]
impl Strategy for SpreadStrategy {
    fn name(&self) -> &str {
        "Credit Spread"
    }

    fn description(&self) -> &str {
        "Finds vertical credit spread opportunities — selling a near-the-money option and buying a further OTM one to collect premium with defined risk."
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
            if dte < 14 || dte > 60 {
                continue;
            }
            let t = dte as f64 / 365.0;

            // Look for put credit spreads (bullish) below the stock
            let puts: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Put && c.mid_price() > 0.05)
                .filter(|c| c.strike < underlying && c.strike > underlying * 0.85)
                .collect();

            for (i, short_put) in puts.iter().enumerate() {
                if short_put.spread_pct() > 40.0 {
                    continue;
                }

                // Find a long put 2-5% further OTM
                for long_put in puts.iter().skip(i + 1) {
                    let width = short_put.strike - long_put.strike;
                    let width_pct = width / underlying * 100.0;
                    if width_pct < 1.5 || width_pct > 6.0 {
                        continue;
                    }

                    let credit = short_put.mid_price() - long_put.mid_price();
                    if credit <= 0.0 {
                        continue;
                    }

                    let max_loss = width - credit;
                    let risk_reward = credit / max_loss;
                    if risk_reward < 0.2 {
                        continue;
                    }

                    let iv = short_put.implied_volatility.unwrap_or(0.25);
                    let (_, greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: short_put.strike,
                        t,
                        r,
                        sigma: iv,
                        opt_type: OptionType::Put,
                    });

                    let score = (risk_reward.min(1.0)) * 60.0
                        + (credit / underlying * 100.0).min(2.0) / 2.0 * 40.0;

                    let explanation = format!(
                        "Put credit spread on {}: sell the ${:.0} put, buy the ${:.0} put. \
                         You collect ${:.2} per share (${:.0} per contract) upfront. \
                         If {} stays above ${:.0} by expiration ({} days), you keep the full credit. \
                         Your max loss is ${:.2} per share — the spread width minus what you collected. \
                         This is a bullish strategy with defined risk: you know exactly what you can lose.",
                        chain.ticker,
                        short_put.strike,
                        long_put.strike,
                        credit,
                        credit * 100.0,
                        chain.ticker,
                        short_put.strike,
                        dte,
                        max_loss,
                    );

                    let risk = format!(
                        "Max loss: ${:.2}/share (${:.0}/contract). \
                         Breakeven at expiration: ${:.2}. \
                         Probability of profit is roughly {:.0}% based on the short put's delta of {:.2}.",
                        max_loss,
                        max_loss * 100.0,
                        short_put.strike - credit,
                        (1.0 - greeks.delta.abs()) * 100.0,
                        greeks.delta,
                    );

                    opps.push(Opportunity {
                        contract: (*short_put).clone(),
                        greeks,
                        strategy: StrategyType::CreditSpread,
                        score,
                        explanation,
                        risk_summary: risk,
                    });

                    break; // one spread per short leg
                }
            }

            // Look for call credit spreads (bearish) above the stock
            let calls: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call && c.mid_price() > 0.05)
                .filter(|c| c.strike > underlying && c.strike < underlying * 1.15)
                .collect();

            for (i, short_call) in calls.iter().enumerate() {
                if short_call.spread_pct() > 40.0 {
                    continue;
                }

                for long_call in calls.iter().skip(i + 1) {
                    let width = long_call.strike - short_call.strike;
                    let width_pct = width / underlying * 100.0;
                    if width_pct < 1.5 || width_pct > 6.0 {
                        continue;
                    }

                    let credit = short_call.mid_price() - long_call.mid_price();
                    if credit <= 0.0 {
                        continue;
                    }

                    let max_loss = width - credit;
                    let risk_reward = credit / max_loss;
                    if risk_reward < 0.2 {
                        continue;
                    }

                    let iv = short_call.implied_volatility.unwrap_or(0.25);
                    let (_, greeks) = bs_price_with_greeks(&BsInputs {
                        s: underlying,
                        k: short_call.strike,
                        t,
                        r,
                        sigma: iv,
                        opt_type: OptionType::Call,
                    });

                    let score = (risk_reward.min(1.0)) * 60.0
                        + (credit / underlying * 100.0).min(2.0) / 2.0 * 40.0;

                    let explanation = format!(
                        "Call credit spread on {}: sell the ${:.0} call, buy the ${:.0} call. \
                         You collect ${:.2} per share (${:.0} per contract) upfront. \
                         If {} stays below ${:.0} by expiration ({} days), you keep the full credit. \
                         This is a bearish/neutral strategy — you profit if the stock doesn't go up.",
                        chain.ticker,
                        short_call.strike,
                        long_call.strike,
                        credit,
                        credit * 100.0,
                        chain.ticker,
                        short_call.strike,
                        dte,
                    );

                    let risk = format!(
                        "Max loss: ${:.2}/share (${:.0}/contract). \
                         Breakeven at expiration: ${:.2}.",
                        max_loss,
                        max_loss * 100.0,
                        short_call.strike + credit,
                    );

                    opps.push(Opportunity {
                        contract: (*short_call).clone(),
                        greeks,
                        strategy: StrategyType::CreditSpread,
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
