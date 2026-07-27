use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub scanner: ScannerConfig,
    #[serde(default)]
    pub strategies: StrategiesConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub tradier_api_key: Option<String>,
    pub alpha_vantage_api_key: Option<String>,
    #[serde(default)]
    pub tradier_sandbox: bool,
    #[serde(default = "default_yahoo_rate_limit")]
    pub yahoo_requests_per_hour: u32,
    #[serde(default = "default_tradier_rate_limit")]
    pub tradier_requests_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_price_ttl")]
    pub price_ttl_seconds: u64,
    #[serde(default = "default_chain_ttl")]
    pub chain_ttl_seconds: u64,
    #[serde(default = "default_exp_ttl")]
    pub exp_ttl_seconds: u64,
    #[serde(default = "default_cache_dir")]
    pub sqlite_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    #[serde(default = "default_sp500_path")]
    pub sp500_csv_path: PathBuf,
    #[serde(default = "default_scan_interval")]
    pub watch_interval_seconds: u64,
    #[serde(default = "default_risk_free_rate")]
    pub risk_free_rate: f64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_max_dte")]
    pub max_dte: u32,
    #[serde(default = "default_min_dte")]
    pub min_dte: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategiesConfig {
    #[serde(default)]
    pub unusual_volume: UnusualVolumeConfig,
    #[serde(default)]
    pub iv_crush: IvCrushConfig,
    #[serde(default)]
    pub wheel: WheelConfig,
    #[serde(default)]
    pub cheap_directional: CheapDirectionalConfig,
    #[serde(default)]
    pub spreads: SpreadsConfig,
    #[serde(default)]
    pub straddles: StraddlesConfig,
    #[serde(default)]
    pub calendar: CalendarConfig,
    #[serde(default)]
    pub covered_call: CoveredCallConfig,
    #[serde(default)]
    pub butterfly: ButterflyConfig,
    #[serde(default)]
    pub iron_condor: IronCondorConfig,
    #[serde(default)]
    pub ratio_spread: RatioSpreadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnusualVolumeConfig {
    #[serde(default = "default_volume_ratio")]
    pub min_volume_oi_ratio: f64,
    #[serde(default = "default_min_volume")]
    pub min_volume: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IvCrushConfig {
    #[serde(default = "default_iv_percentile")]
    pub min_iv_percentile: f64,
    #[serde(default = "default_days_to_earnings")]
    pub max_days_to_earnings: u32,
    #[serde(default = "default_iv_z_threshold")]
    pub z_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WheelConfig {
    #[serde(default = "default_wheel_delta")]
    pub target_delta: f64,
    #[serde(default = "default_wheel_min_premium")]
    pub min_annualized_premium: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheapDirectionalConfig {
    #[serde(default = "default_cheap_max_pct")]
    pub max_pct_of_underlying: f64,
    #[serde(default = "default_cheap_max_otm")]
    pub max_otm_pct: f64,
    #[serde(default = "default_cheap_min_otm")]
    pub min_otm_pct: f64,
    #[serde(default = "default_cheap_min_delta")]
    pub min_delta: f64,
    #[serde(default = "default_cheap_max_delta")]
    pub max_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpreadsConfig {
    #[serde(default = "default_spread_min_rr")]
    pub min_risk_reward: f64,
    #[serde(default = "default_spread_max_width")]
    pub max_width_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StraddlesConfig {
    #[serde(default = "default_straddle_max_pct")]
    pub max_cost_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarConfig {
    #[serde(default = "default_calendar_max_debit")]
    pub max_debit_pct: f64,
    #[serde(default = "default_calendar_min_gap")]
    pub min_dte_gap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoveredCallConfig {
    #[serde(default = "default_cc_min_premium")]
    pub min_annualized_premium: f64,
    #[serde(default = "default_cc_min_prob")]
    pub min_prob_profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ButterflyConfig {
    #[serde(default = "default_bfly_min_rr")]
    pub min_risk_reward: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IronCondorConfig {
    #[serde(default = "default_ic_min_credit")]
    pub min_credit_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RatioSpreadConfig {
    #[serde(default = "default_rs_min_ratio")]
    pub min_ratio: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertsConfig {
    pub webhook_url: Option<String>,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
}

fn default_alert_threshold() -> f64 { 75.0 }
fn default_yahoo_rate_limit() -> u32 { 2000 }
fn default_tradier_rate_limit() -> u32 { 120 }
fn default_price_ttl() -> u64 { 300 }
fn default_chain_ttl() -> u64 { 900 }
fn default_exp_ttl() -> u64 { 3600 }
fn default_cache_dir() -> PathBuf { PathBuf::from("data/cache.db") }
fn default_sp500_path() -> PathBuf { PathBuf::from("data/sp500.csv") }
fn default_scan_interval() -> u64 { 900 }
fn default_risk_free_rate() -> f64 { 0.05 }
fn default_concurrency() -> usize { 15 }
fn default_max_dte() -> u32 { 90 }
fn default_min_dte() -> u32 { 7 }
fn default_volume_ratio() -> f64 { 2.0 }
fn default_min_volume() -> u64 { 100 }
fn default_iv_percentile() -> f64 { 80.0 }
fn default_days_to_earnings() -> u32 { 7 }
fn default_iv_z_threshold() -> f64 { 1.5 }
fn default_wheel_delta() -> f64 { 0.3 }
fn default_wheel_min_premium() -> f64 { 15.0 }
fn default_cheap_max_pct() -> f64 { 3.0 }
fn default_cheap_max_otm() -> f64 { 10.0 }
fn default_cheap_min_otm() -> f64 { 1.0 }
fn default_cheap_min_delta() -> f64 { 0.1 }
fn default_cheap_max_delta() -> f64 { 0.45 }
fn default_spread_min_rr() -> f64 { 0.2 }
fn default_spread_max_width() -> f64 { 6.0 }
fn default_straddle_max_pct() -> f64 { 8.0 }
fn default_calendar_max_debit() -> f64 { 3.0 }
fn default_calendar_min_gap() -> u32 { 14 }
fn default_cc_min_premium() -> f64 { 5.0 }
fn default_cc_min_prob() -> f64 { 0.6 }
fn default_bfly_min_rr() -> f64 { 2.0 }
fn default_ic_min_credit() -> f64 { 1.0 }
fn default_rs_min_ratio() -> f64 { 1.5 }

impl Default for Config {
    fn default() -> Self {
        Config {
            api: ApiConfig::default(),
            cache: CacheConfig::default(),
            scanner: ScannerConfig::default(),
            strategies: StrategiesConfig::default(),
            alerts: AlertsConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            tradier_api_key: None,
            alpha_vantage_api_key: None,
            yahoo_requests_per_hour: default_yahoo_rate_limit(),
            tradier_requests_per_hour: default_tradier_rate_limit(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            price_ttl_seconds: default_price_ttl(),
            chain_ttl_seconds: default_chain_ttl(),
            exp_ttl_seconds: default_exp_ttl(),
            sqlite_path: default_cache_dir(),
        }
    }
}

impl Default for ScannerConfig {
    fn default() -> Self {
        ScannerConfig {
            sp500_csv_path: default_sp500_path(),
            watch_interval_seconds: default_scan_interval(),
            risk_free_rate: default_risk_free_rate(),
            concurrency: default_concurrency(),
            max_dte: default_max_dte(),
            min_dte: default_min_dte(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default(path: &str) -> Self {
        match Self::load(path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: could not load config from '{}': {}. Using defaults.", path, e);
                Self::default()
            }
        }
    }
}
