pub mod yahoo;
pub mod cache;
pub mod rate_limit;
pub mod persist;
pub mod earnings;

use crate::types::OptionChain;
use async_trait::async_trait;
use chrono::NaiveDate;
use anyhow::Result;

#[async_trait]
pub trait DataProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn get_option_chain(&self, ticker: &str, expiration: NaiveDate) -> Result<OptionChain>;
    async fn get_underlying_price(&self, ticker: &str) -> Result<f64>;
    async fn get_expirations(&self, ticker: &str) -> Result<Vec<NaiveDate>>;
    async fn get_price_and_expirations(&self, ticker: &str) -> Result<(f64, Vec<NaiveDate>)>;
}

pub use yahoo::YahooProvider;
pub use cache::CachedProvider;
