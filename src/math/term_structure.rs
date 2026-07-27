use crate::types::{OptionChain, OptionType};

/// Describes how implied volatility changes with time-to-expiration for a
/// given underlying, computed from ATM calls across available expirations.
#[derive(Debug, Clone, Copy)]
pub struct TermStructure {
    /// ATM IV of the nearest usable expiration.
    pub near_iv: f64,
    /// ATM IV of the farthest usable expiration.
    pub far_iv: f64,
    /// Slope of IV vs DTE (IV units per day). Positive = contango (normal),
    /// negative = backwardation (near-term vol elevated, often event-driven).
    pub slope_per_day: f64,
    /// Near DTE in days (for reference).
    pub near_dte: u32,
    /// Far DTE in days (for reference).
    pub far_dte: u32,
}

impl TermStructure {
    /// True when near-term IV exceeds far-term IV — typically signals an
    /// upcoming event (earnings, FDA, etc.) and favors short-near/long-far
    /// trades like calendar spreads.
    pub fn is_backwardation(&self) -> bool {
        self.near_iv > self.far_iv
    }

    /// Human-readable summary for strategy explanations.
    pub fn summary(&self) -> String {
        if self.is_backwardation() {
            format!(
                " IV term structure is in backwardation ({:.1}% near vs {:.1}% far, {:.3}%/day) — near-term vol is elevated, a classic setup for calendar spreads.",
                self.near_iv * 100.0,
                self.far_iv * 100.0,
                self.slope_per_day * 100.0,
            )
        } else {
            format!(
                " IV term structure is in contango ({:.1}% near vs {:.1}% far, {:.3}%/day).",
                self.near_iv * 100.0,
                self.far_iv * 100.0,
                self.slope_per_day * 100.0,
            )
        }
    }
}

/// Detect the ATM IV term structure across a set of option chains for the
/// same underlying. Returns `None` if fewer than two chains have usable
/// ATM IV data.
pub fn detect_term_structure(
    chains: &[OptionChain],
    underlying: f64,
    today: chrono::NaiveDate,
) -> Option<TermStructure> {
    if chains.len() < 2 {
        return None;
    }

    // Collect (dte, atm_iv) pairs — one per expiration.
    let mut points: Vec<(u32, f64)> = chains
        .iter()
        .filter_map(|chain| {
            let dte = chain.contracts.first()
                .map(|c| c.days_to_expiration(today))
                .unwrap_or(0);
            if dte == 0 {
                return None;
            }

            // Find the ATM call (strike closest to underlying).
            let atm_call = chain.contracts.iter()
                .filter(|c| c.option_type == OptionType::Call)
                .min_by(|a, b| {
                    (a.strike - underlying).abs()
                        .total_cmp(&(b.strike - underlying).abs())
                })?;

            // Require the ATM strike to be reasonably close (within 5%).
            if (atm_call.strike - underlying).abs() / underlying > 0.05 {
                return None;
            }

            let iv = atm_call.implied_volatility?;
            if iv > 0.01 && iv < 5.0 {
                Some((dte, iv))
            } else {
                None
            }
        })
        .collect();

    if points.len() < 2 {
        return None;
    }

    points.sort_by_key(|&(dte, _)| dte);

    let near = points[0];
    let far = points[points.len() - 1];

    let dte_gap = (far.0 - near.0) as f64;
    if dte_gap < 1.0 {
        return None;
    }

    let slope = (far.1 - near.1) / dte_gap;

    Some(TermStructure {
        near_iv: near.1,
        far_iv: far.1,
        slope_per_day: slope,
        near_dte: near.0,
        far_dte: far.0,
    })
}
