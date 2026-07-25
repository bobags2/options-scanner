use async_trait::async_trait;
use chrono::Utc;

use crate::config::StrategiesConfig;
use crate::math::black_scholes::{bs_price_with_greeks, BsInputs};
use crate::math::iv_rank::compute_iv_stats;
use crate::types::{Opportunity, OptionChain, StrategyType, UnderlyingPrices};
use super::Strategy;

pub struct IvCrushStrategy;

#[async_trait]
impl Strategy for IvCrushStrategy {
    fn name(&self) -> &str {
        "IV Crush"
    }

    fn description(&self) -> &str {
        "Finds options with unusually high implied volatility that are likely to drop after an event (earnings, FDA, etc)."
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
        let _min_iv_pct = config.iv_crush.min_iv_percentile;
        let z_threshold = config.iv_crush.z_threshold;
        let mut opps = Vec::new();

        let all_ivs: Vec<f64> = chains.iter()
            .flat_map(|ch| ch.contracts.iter())
            .filter_map(|c| c.implied_volatility)
            .filter(|iv| *iv > 0.01 && *iv < 5.0)
            .collect();

        for chain in chains {
            let underlying = match underlying_prices.get(&chain.ticker) {
                Some(p) => *p,
                None => continue,
            };

            if all_ivs.is_empty() {
                continue;
            }

            let avg_iv = all_ivs.iter().sum::<f64>() / all_ivs.len() as f64;
            let iv_stddev = (all_ivs.iter().map(|x| (x - avg_iv).powi(2)).sum::<f64>() / all_ivs.len() as f64).sqrt();

            for c in &chain.contracts {
                let iv = match c.implied_volatility {
                    Some(iv) if iv > 0.01 => iv,
                    _ => continue,
                };

                let dte = c.days_to_expiration(today);
                if dte < 1 || dte > 60 {
                    continue;
                }

                // Look for IV significantly above the chain average
                let z_score = if iv_stddev > 0.0 { (iv - avg_iv) / iv_stddev } else { 0.0 };
                if z_score < z_threshold {
                    continue;
                }

                // Skip illiquid contracts
                if c.spread_pct() > 50.0 || c.volume == 0 {
                    continue;
                }

                let t = dte as f64 / 365.0;
                let (_, greeks) = bs_price_with_greeks(&BsInputs {
                    s: underlying,
                    k: c.strike,
                    t,
                    r,
                    sigma: iv,
                    opt_type: c.option_type,
                });

                let iv_stats = compute_iv_stats(iv, &all_ivs);
                let score = (z_score.min(5.0) / 5.0) * 100.0;

                let iv_rank_info = match &iv_stats {
                    Some(stats) => format!(
                        " IV rank: {:.0}% (range {:.1}%-{:.1}%).",
                        stats.iv_rank * 100.0,
                        stats.iv_low * 100.0,
                        stats.iv_high * 100.0,
                    ),
                    None => String::new(),
                };

                let explanation = format!(
                    "This {} {} option has an IV of {:.1}%, which is {:.1} standard deviations above the chain average of {:.1}%.{} \
                     High IV inflates option premiums. After an event (earnings, FDA decision), IV typically 'crushes' back toward normal, \
                     deflating the option price. This makes it attractive for selling (credit spreads, strangles) if you expect the stock to stay range-bound.",
                    c.ticker,
                    c.option_type,
                    iv * 100.0,
                    z_score,
                    avg_iv * 100.0,
                    iv_rank_info,
                );

                let risk = format!(
                    "If IV doesn't crush or the stock moves significantly, short options can lose money. Theta is {:.3}/day (time decay helps). \
                     Vega is {:.3} — a 1% drop in IV would increase value by ${:.2} per contract.",
                    greeks.theta,
                    greeks.vega,
                    -greeks.vega,
                );

                opps.push(Opportunity {
                    contract: c.clone(),
                    greeks,
                    strategy: StrategyType::IvCrush,
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
