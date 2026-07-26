use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct ButterflyStrategy;

#[async_trait]
impl Strategy for ButterflyStrategy {
    fn name(&self) -> &str {
        "Butterfly"
    }

    fn description(&self) -> &str {
        "Finds long call butterfly setups — buy 1 ITM call, sell 2 ATM calls, buy 1 OTM call for a low-cost, high-reward pin play."
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

            let calls: Vec<_> = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call && c.mid_price() > 0.05 && c.spread_pct() < 40.0)
                .collect();

            if calls.len() < 5 {
                continue;
            }

            let atm_strike = calls.iter()
                .min_by(|a, b| {
                    (a.strike - underlying).abs().total_cmp(&(b.strike - underlying).abs())
                })
                .map(|c| c.strike);
            let atm = match atm_strike {
                Some(s) => s,
                None => continue,
            };

            let strikes: Vec<f64> = calls.iter().map(|c| c.strike).collect();
            let step = if strikes.len() >= 2 { (strikes[1] - strikes[0]).abs() } else { 0.0 };
            if step < 0.5 {
                continue;
            }

            let lower_strike = atm - step;
            let upper_strike = atm + step;

            let lower = calls.iter().find(|c| (c.strike - lower_strike).abs() < 0.01);
            let mid = calls.iter().find(|c| (c.strike - atm).abs() < 0.01);
            let upper = calls.iter().find(|c| (c.strike - upper_strike).abs() < 0.01);

            let (low, mid, high) = match (lower, mid, upper) {
                (Some(l), Some(m), Some(h)) => (l, m, h),
                _ => continue,
            };

            let cost = low.mid_price() - 2.0 * mid.mid_price() + high.mid_price();
            if cost <= 0.0 {
                continue;
            }

            let max_profit = step - cost;
            if max_profit <= 0.0 {
                continue;
            }

            let risk_reward = max_profit / cost;
            if risk_reward < 2.0 {
                continue;
            }

            let iv = mid.implied_volatility.unwrap_or(0.25);
            let (_, greeks) = bs_price_with_greeks(&BsInputs {
                s: underlying,
                k: atm,
                t,
                r,
                sigma: iv,
                opt_type: OptionType::Call,
            });

            let score = (risk_reward.min(10.0) / 10.0) * 50.0
                + (1.0 - (atm - underlying).abs() / underlying).max(0.0) * 30.0
                + (dte as f64 / 60.0) * 20.0;

            let explanation = format!(
                "Long call butterfly on {}: buy ${:.0} call, sell 2x ${:.0} calls, buy ${:.0} call. \
                 Net debit: ${:.2}/share (${:.0}/contract). \
                 Max profit ${:.2}/share if {} pins exactly ${:.0} at expiration ({} days). \
                 Risk/reward: {:.1}:1.",
                chain.ticker,
                lower_strike,
                atm,
                upper_strike,
                cost,
                cost * 100.0,
                max_profit,
                chain.ticker,
                atm,
                dte,
                risk_reward,
            );

            let risk = format!(
                "Max loss: ${:.2}/share (${:.0}/contract) if {} is below ${:.0} or above ${:.0} at expiration. \
                 Profit zone: ${:.0} to ${:.0}. Best case: pin at ${:.0}.",
                cost,
                cost * 100.0,
                chain.ticker,
                lower_strike,
                upper_strike,
                lower_strike,
                upper_strike,
                atm,
            );

            opps.push(Opportunity {
                contract: (*mid).clone(),
                greeks,
                strategy: StrategyType::Butterfly,
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
