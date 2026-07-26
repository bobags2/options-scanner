use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct StraddleStrategy;

#[async_trait]
impl Strategy for StraddleStrategy {
    fn name(&self) -> &str {
        "Straddle"
    }

    fn description(&self) -> &str {
        "Finds straddle opportunities — buying both a call and put at the same strike when you expect a big move but don't know which direction."
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
            if dte < 7 || dte > 60 {
                continue;
            }
            let t = dte as f64 / 365.0;

            // Find the ATM strike
            let atm_strike = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call)
                .min_by(|a, b| {
                    (a.strike - underlying).abs().total_cmp(&(b.strike - underlying).abs())
                })
                .map(|c| c.strike);

            let atm = match atm_strike {
                Some(s) => s,
                None => continue,
            };

            // Find the ATM call and put
            let atm_call = chain.contracts.iter()
                .find(|c| c.option_type == OptionType::Call && (c.strike - atm).abs() < 0.01);
            let atm_put = chain.contracts.iter()
                .find(|c| c.option_type == OptionType::Put && (c.strike - atm).abs() < 0.01);

            let (call, put) = match (atm_call, atm_put) {
                (Some(c), Some(p)) => (c, p),
                _ => continue,
            };

            let call_mid = call.mid_price();
            let put_mid = put.mid_price();
            if call_mid <= 0.0 || put_mid <= 0.0 {
                continue;
            }

            let straddle_cost = call_mid + put_mid;
            let straddle_pct = straddle_cost / underlying * 100.0;

            // Look for cheap straddles — market not pricing in much movement
            if straddle_pct > 8.0 {
                continue;
            }

            // Calculate combined Greeks
            let iv_call = call.implied_volatility.unwrap_or(0.25);
            let (_, call_greeks) = bs_price_with_greeks(&BsInputs {
                s: underlying,
                k: atm,
                t,
                r,
                sigma: iv_call,
                opt_type: OptionType::Call,
            });

            let iv_put = put.implied_volatility.unwrap_or(0.25);
            let (_, put_greeks) = bs_price_with_greeks(&BsInputs {
                s: underlying,
                k: atm,
                t,
                r,
                sigma: iv_put,
                opt_type: OptionType::Put,
            });

            let combined_vega = call_greeks.vega + put_greeks.vega;
            let combined_theta = call_greeks.theta + put_greeks.theta;

            let score = (1.0 - straddle_pct / 8.0) * 50.0
                + (combined_vega.min(1.0)) * 30.0
                + (dte as f64 / 60.0) * 20.0;

            let breakeven_up = atm + straddle_cost;
            let breakeven_down = atm - straddle_cost;
            let move_needed = straddle_cost / underlying * 100.0;

            let explanation = format!(
                "Straddle on {}: buy the ${:.0} call and ${:.0} put for a combined ${:.2} per share (${:.0} per contract). \
                 You profit if {} moves more than {:.1}% in either direction by expiration ({} days). \
                 Breakeven prices: ${:.2} (upside) and ${:.2} (downside). \
                 This is a 'something big will happen' bet — earnings, FDA decisions, or macro events. \
                 The straddle is relatively cheap at {:.1}% of the stock price, meaning the market isn't pricing in a huge move.",
                chain.ticker,
                atm,
                atm,
                straddle_cost,
                straddle_cost * 100.0,
                chain.ticker,
                move_needed,
                dte,
                breakeven_up,
                breakeven_down,
                straddle_pct,
            );

            let risk = format!(
                "If {} stays between ${:.2} and ${:.2}, you lose money. \
                 Max loss is ${:.2}/share (${:.0}/contract) if the stock closes exactly at ${:.0}.\
                 Time decay costs ${:.2}/day (combined theta). \
                 Combined vega is {:.3} — a 1% IV increase adds ${:.2} per contract.",
                chain.ticker,
                breakeven_down,
                breakeven_up,
                straddle_cost,
                straddle_cost * 100.0,
                atm,
                combined_theta.abs() * 100.0,
                combined_vega,
                combined_vega * 100.0,
            );

            opps.push(Opportunity {
                contract: call.clone(),
                greeks: crate::types::Greeks {
                    delta: call_greeks.delta + put_greeks.delta,
                    gamma: call_greeks.gamma + put_greeks.gamma,
                    theta: combined_theta,
                    vega: combined_vega,
                    rho: call_greeks.rho + put_greeks.rho,
                },
                strategy: StrategyType::Straddle,
                score,
                explanation,
                risk_summary: risk,
            });
        }

        opps.sort_by(|a, b| b.score.total_cmp(&a.score));
        opps.truncate(50);
        opps
    }
}
