use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct CalendarStrategy;

#[async_trait]
impl Strategy for CalendarStrategy {
    fn name(&self) -> &str {
        "Calendar Spread"
    }

    fn description(&self) -> &str {
        "Finds calendar spread opportunities — selling a near-term option and buying a longer-term one at the same strike to profit from time decay differential."
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

        if chains.len() < 2 {
            return opps;
        }

        let underlying = match underlying_prices.get(&chains[0].ticker) {
            Some(p) => *p,
            None => return opps,
        };

        let mut sorted: Vec<_> = chains.iter().collect();
        sorted.sort_by_key(|c| c.expiration);

        // Detect IV term structure across sorted expirations.
        let owned_chains: Vec<OptionChain> = sorted.iter().map(|c| (*c).clone()).collect();
        let term = crate::math::detect_term_structure(&owned_chains, underlying, today);
        let in_backwardation = term.as_ref().map(|t| t.is_backwardation()).unwrap_or(false);
        let term_summary = term.as_ref().map(|t| t.summary()).unwrap_or_default();

        for (i, near_chain) in sorted.iter().enumerate() {
            let near_dte = near_chain.contracts.first()
                .map(|c| c.days_to_expiration(today))
                .unwrap_or(0);
            if near_dte < 7 || near_dte > 45 {
                continue;
            }

            for far_chain in sorted.iter().skip(i + 1) {
                let far_dte = far_chain.contracts.first()
                    .map(|c| c.days_to_expiration(today))
                    .unwrap_or(0);
                if far_dte < 30 || far_dte > 90 {
                    continue;
                }
                let dte_gap = far_dte - near_dte;
                if dte_gap < 14 {
                    continue;
                }

                let near_t = near_dte as f64 / 365.0;
                let far_t = far_dte as f64 / 365.0;

                let atm_strike = near_chain.contracts.iter()
                    .filter(|c| c.option_type == OptionType::Call)
                    .min_by(|a, b| {
                        (a.strike - underlying).abs().total_cmp(&(b.strike - underlying).abs())
                    })
                    .map(|c| c.strike);

                let strike = match atm_strike {
                    Some(s) => s,
                    None => continue,
                };

                let near_call = near_chain.contracts.iter()
                    .find(|c| c.option_type == OptionType::Call && (c.strike - strike).abs() < 0.01);
                let far_call = far_chain.contracts.iter()
                    .find(|c| c.option_type == OptionType::Call && (c.strike - strike).abs() < 0.01);

                let (short_leg, long_leg) = match (near_call, far_call) {
                    (Some(s), Some(l)) => (s, l),
                    _ => continue,
                };

                let short_mid = short_leg.mid_price();
                let long_mid = long_leg.mid_price();
                if short_mid <= 0.0 || long_mid <= 0.0 {
                    continue;
                }

                let net_debit = long_mid - short_mid;
                if net_debit <= 0.0 {
                    continue;
                }

                let iv_short = short_leg.implied_volatility.unwrap_or(0.25);
                let iv_long = long_leg.implied_volatility.unwrap_or(0.25);
                let iv_diff = iv_short - iv_long;

                let (_, short_greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: strike,
                    t: near_t,
                    r,
                    sigma: iv_short,
                    opt_type: OptionType::Call,
                });

                let (_, long_greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: strike,
                    t: far_t,
                    r,
                    sigma: iv_long,
                    opt_type: OptionType::Call,
                });

                let net_theta = short_greeks.theta.abs() - long_greeks.theta.abs();
                let net_vega = long_greeks.vega - short_greeks.vega;

                if net_theta <= 0.0 {
                    continue;
                }

                let debit_pct = net_debit / underlying * 100.0;
                if debit_pct > 3.0 {
                    continue;
                }

                let mut score = (net_theta / long_leg.mid_price() * 1000.0).min(40.0)
                    + (iv_diff * 100.0).min(30.0).max(0.0)
                    + (1.0 - debit_pct / 3.0) * 30.0;
                if in_backwardation {
                    // Backwardation (near IV > far IV) is the ideal regime for
                    // calendars — the short leg is richer than the long one.
                    score += 15.0;
                }

                let explanation = format!(
                    "Calendar spread on {}: sell the {} ${:.0} call (${:.2}), buy the {} ${:.0} call (${:.2}). \
                     Net debit: ${:.2}/share (${:.0}/contract). \
                     You profit if {} stays near ${:.0} as the short option decays faster than the long one. \
                     The {}-day gap between expirations creates a time decay advantage.",
                    chains[0].ticker,
                    near_chain.expiration,
                    strike,
                    short_mid,
                    far_chain.expiration,
                    strike,
                    long_mid,
                    net_debit,
                    net_debit * 100.0,
                    chains[0].ticker,
                    strike,
                    dte_gap,
                ) + &term_summary;

                let risk = format!(
                    "Max loss: ${:.2}/share (${:.0}/contract) if {} moves far from ${:.0}. \
                     Net theta: {:.3}/day (positive = time helps you). \
                     Net vega: {:.3} — rising IV helps the long leg more than it hurts the short.",
                    net_debit,
                    net_debit * 100.0,
                    chains[0].ticker,
                    strike,
                    net_theta,
                    net_vega,
                );

                opps.push(Opportunity {
                    contract: short_leg.clone(),
                    greeks: crate::types::Greeks {
                        delta: long_greeks.delta - short_greeks.delta,
                        gamma: long_greeks.gamma - short_greeks.gamma,
                        theta: -net_theta,
                        vega: net_vega,
                        rho: long_greeks.rho - short_greeks.rho,
                    },
                    strategy: StrategyType::CalendarSpread,
                    score,
                    explanation,
                    risk_summary: risk,
                });

                break;
            }
        }

        opps.sort_by(|a, b| b.score.total_cmp(&a.score));
        opps.truncate(50);
        opps
    }
}
