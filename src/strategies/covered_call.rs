use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::types::{Opportunity, OptionChain, OptionType, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct CoveredCallStrategy;

#[async_trait]
impl Strategy for CoveredCallStrategy {
    fn name(&self) -> &str {
        "Covered Call"
    }

    fn description(&self) -> &str {
        "Finds covered call setups — selling OTM calls against stock you own or could buy, generating income while holding the underlying."
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
                .filter(|c| c.option_type == OptionType::Call && c.mid_price() > 0.05)
                .filter(|c| c.strike > underlying * 1.02 && c.strike < underlying * 1.20)
                .collect();

            for call in calls {
                if call.spread_pct() > 30.0 {
                    continue;
                }

                let premium = call.mid_price();
                let otm_pct = (call.strike - underlying) / underlying * 100.0;
                let annualized_premium = (premium / underlying) * (365.0 / dte as f64) * 100.0;

                if annualized_premium < 5.0 {
                    continue;
                }

                let iv = call.implied_volatility.unwrap_or(0.25);
                let (_, greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: call.strike,
                    t,
                    r,
                    sigma: iv,
                    opt_type: OptionType::Call,
                });

                let prob_itm = greeks.delta.abs();
                let prob_profit = 1.0 - prob_itm;

                if prob_profit < 0.6 {
                    continue;
                }

                let score = annualized_premium.min(30.0) * 1.2
                    + (prob_profit - 0.6) * 50.0
                    + (1.0 - call.spread_pct() / 30.0).max(0.0) * 15.0;

                let downside_protection = premium / underlying * 100.0;

                let explanation = format!(
                    "Covered call on {}: sell the ${:.0} call ({:.1}% OTM) for ${:.2}/share (${:.0}/contract). \
                     Expiration in {} days. Annualized premium yield: {:.1}%. \
                     You keep the premium if {} stays below ${:.0} by expiration. \
                     Probability of keeping full premium: ~{:.0}%.",
                    chain.ticker,
                    call.strike,
                    otm_pct,
                    premium,
                    premium * 100.0,
                    dte,
                    annualized_premium,
                    chain.ticker,
                    call.strike,
                    prob_profit * 100.0,
                );

                let risk = format!(
                    "Downside protection: {:.1}% — breakeven at ${:.2}. \
                     Upside capped at ${:.2} (stock + premium). \
                     Delta: {:.2} — each $1 move costs ${:.2} in capped upside. \
                     If the stock runs hard past ${:.0}, you miss out on gains above that level.",
                    downside_protection,
                    underlying - premium,
                    call.strike + premium,
                    greeks.delta,
                    greeks.delta,
                    call.strike,
                );

                opps.push(Opportunity {
                    contract: call.clone(),
                    greeks,
                    strategy: StrategyType::CoveredCall,
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
