use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::{header, Client};
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{OptionChain, OptionContract, OptionType};
use super::rate_limit::{self, Limiter};
use super::DataProvider;

pub struct TradierProvider {
    client: Client,
    base_url: String,
    limiter: Arc<Limiter>,
}

impl TradierProvider {
    pub fn new(api_key: &str, sandbox: bool, requests_per_hour: u32) -> Self {
        let base_url = if sandbox {
            "https://sandbox.tradier.com"
        } else {
            "https://api.tradier.com"
        };

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid API key format"),
        );

        Self {
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to build Tradier HTTP client"),
            base_url: base_url.to_string(),
            limiter: rate_limit::create_limiter(requests_per_hour),
        }
    }

    async fn api_get(&self, path: &str) -> Result<reqwest::Response> {
        rate_limit::wait_for_permit(&self.limiter).await;
        let url = format!("{}{}", self.base_url, path);

        let mut last_err = None;
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (attempt as u64 + 1))).await;
            }
            match self.client.get(&url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }
                    if status.as_u16() == 429 || status.is_server_error() {
                        last_err = Some(anyhow::anyhow!("Tradier returned {}", status));
                        continue;
                    }
                    return Err(anyhow::anyhow!("Tradier returned {}", status));
                }
                Err(e) => {
                    last_err = Some(e.into());
                    continue;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Tradier request failed after 3 attempts")))
    }
}

// --- Serde types for Tradier JSON responses ---

/// Tradier wraps single-or-array values inconsistently.
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(item) => vec![item],
            OneOrMany::Many(items) => items,
        }
    }
}

#[derive(Deserialize)]
struct TradierExpirationsResponse {
    expirations: Option<TradierExpirations>,
}

#[derive(Deserialize)]
struct TradierExpirations {
    date: Option<OneOrMany<String>>,
}

#[derive(Deserialize)]
struct TradierChainsResponse {
    options: Option<TradierOptions>,
}

#[derive(Deserialize)]
struct TradierOptions {
    option: Option<OneOrMany<TradierContract>>,
}

#[derive(Deserialize)]
struct TradierContract {
    #[serde(default)]
    strike: f64,
    #[serde(default)]
    last: f64,
    #[serde(default)]
    bid: f64,
    #[serde(default)]
    ask: f64,
    #[serde(default)]
    volume: Option<u64>,
    #[serde(default)]
    open_interest: Option<u64>,
    #[serde(default)]
    greeks: Option<TradierGreeks>,
    #[serde(default, rename = "option_type")]
    opt_type: String,
    #[serde(default, rename = "expiration_date")]
    expiration_date: String,
}

#[derive(Deserialize)]
struct TradierGreeks {
    #[serde(default)]
    mid_iv: Option<f64>,
}

#[derive(Deserialize)]
struct TradierQuotesResponse {
    quotes: Option<TradierQuotes>,
}

#[derive(Deserialize)]
struct TradierQuotes {
    quote: Option<OneOrMany<TradierQuote>>,
}

#[derive(Deserialize)]
struct TradierQuote {
    #[serde(default)]
    last: f64,
}

#[async_trait]
impl DataProvider for TradierProvider {
    fn name(&self) -> &str {
        "Tradier"
    }

    async fn get_option_chain(&self, ticker: &str, expiration: NaiveDate) -> Result<OptionChain> {
        let path = format!(
            "/v1/markets/options/chains?symbol={}&expiration={}&greeks=true",
            ticker, expiration
        );
        let resp: TradierChainsResponse = self
            .api_get(&path)
            .await?
            .json()
            .await
            .context("Failed to parse Tradier chain response")?;

        let options = resp
            .options
            .and_then(|o| o.option)
            .map(|o| o.into_vec())
            .unwrap_or_default();

        let contracts: Vec<OptionContract> = options
            .into_iter()
            .filter_map(|c| {
                let opt_type = match c.opt_type.as_str() {
                    "call" => OptionType::Call,
                    "put" => OptionType::Put,
                    _ => return None,
                };
                let iv = c.greeks.and_then(|g| g.mid_iv);
                Some(OptionContract {
                    ticker: ticker.to_string(),
                    strike: c.strike,
                    expiration,
                    option_type: opt_type,
                    bid: c.bid,
                    ask: c.ask,
                    last: c.last,
                    volume: c.volume.unwrap_or(0),
                    open_interest: c.open_interest.unwrap_or(0),
                    implied_volatility: iv,
                })
            })
            .collect();

        Ok(OptionChain {
            ticker: ticker.to_string(),
            expiration,
            contracts,
        })
    }

    async fn get_underlying_price(&self, ticker: &str) -> Result<f64> {
        let path = format!("/v1/markets/quotes?symbols={}", ticker);
        let resp: TradierQuotesResponse = self
            .api_get(&path)
            .await?
            .json()
            .await
            .context("Failed to parse Tradier quote response")?;

        resp.quotes
            .and_then(|q| q.quote)
            .map(|q| q.into_vec())
            .and_then(|mut v| v.pop())
            .map(|q| q.last)
            .filter(|&p| p > 0.0)
            .context("No price available from Tradier")
    }

    async fn get_expirations(&self, ticker: &str) -> Result<Vec<NaiveDate>> {
        let path = format!(
            "/v1/markets/options/expirations?symbol={}&includeAllRoots=false",
            ticker
        );
        let resp: TradierExpirationsResponse = self
            .api_get(&path)
            .await?
            .json()
            .await
            .context("Failed to parse Tradier expirations response")?;

        let dates = resp
            .expirations
            .and_then(|e| e.date)
            .map(|d| d.into_vec())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            .collect();

        Ok(dates)
    }

    async fn get_price_and_expirations(&self, ticker: &str) -> Result<(f64, Vec<NaiveDate>)> {
        // Tradier has no combined endpoint — fetch both in parallel.
        let price_fut = self.get_underlying_price(ticker);
        let exps_fut = self.get_expirations(ticker);
        let (price, exps) = tokio::try_join!(price_fut, exps_fut)?;
        Ok((price, exps))
    }
}
