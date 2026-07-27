use anyhow::Result;
use chrono::NaiveDate;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
pub struct EarningsCache {
    inner: RwLock<HashMap<String, Option<NaiveDate>>>,
}

impl EarningsCache {
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }

    pub async fn get(&self, ticker: &str) -> Option<Option<NaiveDate>> {
        self.inner.read().await.get(ticker).cloned()
    }

    pub async fn set(&self, ticker: &str, date: Option<NaiveDate>) {
        self.inner.write().await.insert(ticker.to_string(), date);
    }
}

static GLOBAL_EARNINGS_CACHE: LazyLock<EarningsCache> = LazyLock::new(EarningsCache::new);

/// Returns a reference to the process-wide earnings cache.
/// Both the prefetch in main and the per-chain lookup in strategies
/// share this same instance, so a pre-populated cache is immediately
/// visible to every caller.
pub fn global_earnings_cache() -> &'static EarningsCache {
    &GLOBAL_EARNINGS_CACHE
}

pub async fn fetch_earnings_date(ticker: &str, cache: &EarningsCache) -> Result<Option<NaiveDate>> {
    if let Some(cached) = cache.get(ticker).await {
        return Ok(cached);
    }

    let url = format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=calendarEvents",
        ticker
    );

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0")
        .build()?;

    let resp = client.get(&url).send().await;
    let date = match resp {
        Ok(r) if r.status().is_success() => {
            let text = r.text().await.unwrap_or_default();
            parse_earnings_date(&text)
        }
        _ => None,
    };

    cache.set(ticker, date).await;
    Ok(date)
}

fn parse_earnings_date(json: &str) -> Option<NaiveDate> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let dates = v.get("quoteSummary")?
        .get("result")?
        .get(0)?
        .get("calendarEvents")?
        .get("earnings")?
        .get("earningsDate")?
        .as_array()?;
    let ts = dates.first()?.as_i64()?;
    chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.date_naive())
}

pub fn is_earnings_before_expiration(earnings_date: Option<NaiveDate>, expiration: NaiveDate) -> bool {
    match earnings_date {
        Some(ed) => ed < expiration,
        None => false,
    }
}

/// Fetch earnings dates for multiple tickers in parallel, populating the cache.
/// This is called before the scan loop to warm the cache, so strategy scans
/// get instant cache hits instead of sequential API calls.
pub async fn bulk_fetch_earnings(
    tickers: &[String],
    cache: &'static EarningsCache,
    concurrency: usize,
) {
    use tokio::sync::Semaphore;
    use std::sync::Arc;

    let sem = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::new();

    for ticker in tickers {
        let ticker = ticker.clone();
        let cache: &'static EarningsCache = cache; // Copy the static reference.
        let sem = sem.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let _ = fetch_earnings_date(&ticker, cache).await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }
}
