pub struct IvStats {
    pub current_iv: f64,
    pub iv_rank: f64,
    pub iv_percentile: f64,
    pub iv_high: f64,
    pub iv_low: f64,
    pub iv_mean: f64,
}

pub fn compute_iv_stats(current_iv: f64, historical_ivs: &[f64]) -> Option<IvStats> {
    if historical_ivs.is_empty() {
        return None;
    }

    let mut sorted: Vec<f64> = historical_ivs.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));

    let iv_low = sorted[0];
    let iv_high = sorted[sorted.len() - 1];
    let iv_mean = sorted.iter().sum::<f64>() / sorted.len() as f64;

    let range = iv_high - iv_low;
    let iv_rank = if range > 1e-6 {
        ((current_iv - iv_low) / range).max(0.0).min(1.0)
    } else {
        0.5
    };

    let below_count = sorted.iter().filter(|&&iv| iv < current_iv).count();
    let iv_percentile = below_count as f64 / sorted.len() as f64;

    Some(IvStats {
        current_iv,
        iv_rank,
        iv_percentile,
        iv_high,
        iv_low,
        iv_mean,
    })
}
