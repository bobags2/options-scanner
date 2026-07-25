use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{OptionChain, OptionContract, OptionType};
use super::rate_limit::{self, Limiter};
use super::DataProvider;

pub struct YahooProvider {
    client: Client,
    crumb: Arc<RwLock<Option<String>>>,
    limiter: Arc<Limiter>,
}

impl YahooProvider {
    pub fn new() -> Self {
        Self::with_rate_limit(2000)
    }

    pub fn with_rate_limit(requests_per_hour: u32) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .cookie_store(true)
                .build()
                .expect("Failed to build HTTP client"),
            crumb: Arc::new(RwLock::new(None)),
            limiter: rate_limit::create_limiter(requests_per_hour),
        }
    }

    async fn ensure_auth(&self) -> Result<String> {
        // Check if we already have a crumb
        {
            let crumb = self.crumb.read().await;
            if let Some(c) = crumb.as_ref() {
                return Ok(c.clone());
            }
        }

        // Step 1: Hit the consent page to get cookies
        self.client
            .get("https://guce.yahoo.com/consent?brandType=finance")
            .send()
            .await
            .context("Failed to fetch Yahoo consent page")?;

        // Step 2: Get the crumb
        let crumb_text = self.client
            .get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .send()
            .await
            .context("Failed to fetch Yahoo crumb")?
            .text()
            .await
            .context("Failed to read crumb response")?;

        let crumb = crumb_text.trim().to_string();
        if crumb.is_empty() || crumb.contains("error") {
            anyhow::bail!("Failed to obtain valid Yahoo crumb");
        }

        // Store it
        {
            let mut c = self.crumb.write().await;
            *c = Some(crumb.clone());
        }

        Ok(crumb)
    }

    async fn api_get(&self, url: &str) -> Result<reqwest::Response> {
        rate_limit::wait_for_permit(&self.limiter).await;
        let crumb = self.ensure_auth().await?;
        let separator = if url.contains('?') { "&" } else { "?" };
        let full_url = format!("{}{}crumb={}", url, separator, crumb);

        let mut last_err = None;
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
            match self.client.get(&full_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        let mut c = self.crumb.write().await;
                        *c = None;
                        drop(c);
                        let new_crumb = self.ensure_auth().await?;
                        let new_url = format!("{}{}crumb={}", url, separator, new_crumb);
                        let retry = self.client.get(&new_url).send().await.context("Yahoo retry failed")?;
                        if retry.status().is_success() {
                            return Ok(retry);
                        }
                        last_err = Some(anyhow::anyhow!("Yahoo returned {} after auth retry", retry.status()));
                        continue;
                    }
                    if status.as_u16() == 429 || status.is_server_error() {
                        last_err = Some(anyhow::anyhow!("Yahoo returned {}", status));
                        continue;
                    }
                    return Err(anyhow::anyhow!("Yahoo returned {}", status));
                }
                Err(e) => {
                    last_err = Some(e.into());
                    continue;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Yahoo request failed after 3 attempts")))
    }
}

#[derive(Deserialize)]
struct YahooOptionResponse {
    #[serde(rename = "optionChain")]
    option_chain: YahooOptionChainWrapper,
}

#[derive(Deserialize)]
struct YahooOptionChainWrapper {
    result: Vec<YahooOptionResult>,
}

#[derive(Deserialize)]
struct YahooOptionResult {
    #[serde(default)]
    quote: Option<YahooQuote>,
    #[serde(default)]
    options: Vec<YahooExpirationData>,
    #[serde(default, rename = "expirationDates")]
    expiration_dates: Vec<i64>,
}

#[derive(Deserialize)]
struct YahooQuote {
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
}

#[derive(Deserialize)]
struct YahooExpirationData {
    #[serde(default)]
    calls: Vec<YahooContractData>,
    #[serde(default)]
    puts: Vec<YahooContractData>,
}

#[derive(Deserialize)]
struct YahooContractData {
    #[serde(default)]
    strike: f64,
    #[serde(default, rename = "lastPrice")]
    last_price: f64,
    #[serde(default)]
    bid: f64,
    #[serde(default)]
    ask: f64,
    #[serde(default)]
    volume: Option<u64>,
    #[serde(rename = "openInterest")]
    #[serde(default)]
    open_interest: Option<u64>,
    #[serde(rename = "impliedVolatility")]
    #[serde(default)]
    implied_volatility: Option<f64>,
    #[serde(default)]
    expiration: i64,
}

impl YahooContractData {
    fn to_contract(&self, ticker: &str, opt_type: OptionType) -> OptionContract {
        let expiration = chrono::DateTime::from_timestamp(self.expiration, 0)
            .map(|dt| dt.date_naive())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        OptionContract {
            ticker: ticker.to_string(),
            strike: self.strike,
            expiration,
            option_type: opt_type,
            bid: self.bid,
            ask: self.ask,
            last: self.last_price,
            volume: self.volume.unwrap_or(0),
            open_interest: self.open_interest.unwrap_or(0),
            implied_volatility: self.implied_volatility,
        }
    }
}

#[async_trait]
impl DataProvider for YahooProvider {
    fn name(&self) -> &str {
        "Yahoo Finance"
    }

    async fn get_option_chain(&self, ticker: &str, expiration: NaiveDate) -> Result<OptionChain> {
        let date_ts = expiration.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}?date={}",
            ticker, date_ts
        );
        let resp: YahooOptionResponse = self
            .api_get(&url)
            .await?
            .json()
            .await
            .context("Failed to parse Yahoo response")?;

        let result = resp
            .option_chain
            .result
            .into_iter()
            .next()
            .context("No option chain data")?;

        let exp_data = result
            .options
            .into_iter()
            .next()
            .context("No expiration data")?;

        let mut contracts = Vec::new();
        for c in exp_data.calls {
            contracts.push(c.to_contract(ticker, OptionType::Call));
        }
        for c in exp_data.puts {
            contracts.push(c.to_contract(ticker, OptionType::Put));
        }

        Ok(OptionChain {
            ticker: ticker.to_string(),
            expiration,
            contracts,
        })
    }

    async fn get_underlying_price(&self, ticker: &str) -> Result<f64> {
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}",
            ticker
        );
        let resp: YahooOptionResponse = self
            .api_get(&url)
            .await?
            .json()
            .await
            .context("Failed to parse Yahoo response")?;

        let result = resp
            .option_chain
            .result
            .into_iter()
            .next()
            .context("No quote data")?;

        result
            .quote
            .and_then(|q| q.regular_market_price)
            .context("No price available")
    }

    async fn get_expirations(&self, ticker: &str) -> Result<Vec<NaiveDate>> {
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}",
            ticker
        );
        let resp: YahooOptionResponse = self
            .api_get(&url)
            .await?
            .json()
            .await
            .context("Failed to parse Yahoo response")?;

        let result = resp
            .option_chain
            .result
            .into_iter()
            .next()
            .context("No expiration data")?;

        let dates = result
            .expiration_dates
            .iter()
            .filter_map(|ts| chrono::DateTime::from_timestamp(*ts, 0).map(|dt| dt.date_naive()))
            .collect();

        Ok(dates)
    }

    async fn get_price_and_expirations(&self, ticker: &str) -> Result<(f64, Vec<NaiveDate>)> {
        let url = format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{}",
            ticker
        );
        let resp: YahooOptionResponse = self
            .api_get(&url)
            .await?
            .json()
            .await
            .context("Failed to parse Yahoo response")?;

        let result = resp
            .option_chain
            .result
            .into_iter()
            .next()
            .context("No data")?;

        let price = result
            .quote
            .and_then(|q| q.regular_market_price)
            .context("No price available")?;

        let dates = result
            .expiration_dates
            .iter()
            .filter_map(|ts| chrono::DateTime::from_timestamp(*ts, 0).map(|dt| dt.date_naive()))
            .collect();

        Ok((price, dates))
    }
}
