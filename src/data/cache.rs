use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use moka::future::Cache;
use std::time::Duration;

use crate::types::OptionChain;
use super::DataProvider;

pub struct CachedProvider<P: DataProvider> {
    inner: P,
    price_cache: Cache<String, f64>,
    chain_cache: Cache<(String, NaiveDate), OptionChain>,
    exp_cache: Cache<String, Vec<NaiveDate>>,
}

impl<P: DataProvider> CachedProvider<P> {
    pub fn new(inner: P, price_ttl_secs: u64, chain_ttl_secs: u64) -> Self {
        Self {
            inner,
            price_cache: Cache::builder()
                .time_to_live(Duration::from_secs(price_ttl_secs))
                .max_capacity(10_000)
                .build(),
            chain_cache: Cache::builder()
                .time_to_live(Duration::from_secs(chain_ttl_secs))
                .max_capacity(5_000)
                .build(),
            exp_cache: Cache::builder()
                .time_to_live(Duration::from_secs(3600))
                .max_capacity(1_000)
                .build(),
        }
    }
}

#[async_trait]
impl<P: DataProvider> DataProvider for CachedProvider<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn get_option_chain(&self, ticker: &str, expiration: NaiveDate) -> Result<OptionChain> {
        let key = (ticker.to_string(), expiration);
        if let Some(cached) = self.chain_cache.get(&key).await {
            return Ok(cached);
        }
        let chain = self.inner.get_option_chain(ticker, expiration).await?;
        self.chain_cache.insert(key, chain.clone()).await;
        Ok(chain)
    }

    async fn get_underlying_price(&self, ticker: &str) -> Result<f64> {
        if let Some(cached) = self.price_cache.get(&ticker.to_string()).await {
            return Ok(cached);
        }
        let price = self.inner.get_underlying_price(ticker).await?;
        self.price_cache.insert(ticker.to_string(), price).await;
        Ok(price)
    }

    async fn get_expirations(&self, ticker: &str) -> Result<Vec<NaiveDate>> {
        if let Some(cached) = self.exp_cache.get(&ticker.to_string()).await {
            return Ok(cached);
        }
        let exps = self.inner.get_expirations(ticker).await?;
        self.exp_cache.insert(ticker.to_string(), exps.clone()).await;
        Ok(exps)
    }

    async fn get_price_and_expirations(&self, ticker: &str) -> Result<(f64, Vec<NaiveDate>)> {
        let price_cached = self.price_cache.get(&ticker.to_string()).await;
        let exps_cached = self.exp_cache.get(&ticker.to_string()).await;
        if let (Some(p), Some(e)) = (price_cached, exps_cached) {
            return Ok((p, e));
        }
        let (price, exps) = self.inner.get_price_and_expirations(ticker).await?;
        self.price_cache.insert(ticker.to_string(), price).await;
        self.exp_cache.insert(ticker.to_string(), exps.clone()).await;
        Ok((price, exps))
    }
}
