use reqwest::Client;
use serde_json::json;

use crate::types::Opportunity;

/// Fire-and-forget webhook alert for high-scoring opportunities.
/// Supports Discord and Telegram (auto-detected from URL).
pub async fn send_alerts(webhook_url: &str, opps: &[Opportunity], threshold: f64) {
    let alerts: Vec<&Opportunity> = opps.iter().filter(|o| o.score >= threshold).collect();
    if alerts.is_empty() {
        return;
    }

    let client = match Client::builder()
        .user_agent("options-scanner/0.1 (Rust)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Alert: failed to build HTTP client: {}", e);
            return;
        }
    };

    let body = format_alert_body(&alerts, threshold);

    let result = if webhook_url.contains("discord") {
        send_discord(&client, webhook_url, &body).await
    } else if webhook_url.contains("api.telegram.org") {
        send_telegram(&client, webhook_url, &body).await
    } else {
        // Generic webhook — POST JSON with a "text" field
        send_generic(&client, webhook_url, &body).await
    };

    if let Err(e) = result {
        eprintln!("Alert: failed to send webhook: {}", e);
    }
}

fn format_alert_body(opps: &[&Opportunity], threshold: f64) -> String {
    let mut lines = vec![format!(
        "Options Scanner Alert: {} opportunities scored >= {:.0}\n",
        opps.len(),
        threshold
    )];

    for (i, opp) in opps.iter().take(10).enumerate() {
        lines.push(format!(
            "{}. [{}] {} {} ${:.0} {} (score: {:.0}) | IV: {:.1}% | Delta: {:.2}",
            i + 1,
            opp.strategy,
            opp.contract.ticker,
            opp.contract.option_type,
            opp.contract.strike,
            opp.contract.expiration,
            opp.score,
            opp.contract.implied_volatility.unwrap_or(0.0) * 100.0,
            opp.greeks.delta,
        ));
    }

    if opps.len() > 10 {
        lines.push(format!("... and {} more", opps.len() - 10));
    }

    lines.join("\n")
}

async fn send_discord(client: &Client, url: &str, body: &str) -> Result<(), anyhow::Error> {
    let payload = json!({ "content": body });
    let resp = client.post(url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Discord returned {}", resp.status());
    }
    Ok(())
}

async fn send_telegram(client: &Client, url: &str, body: &str) -> Result<(), anyhow::Error> {
    let payload = json!({ "text": body, "parse_mode": "Markdown" });
    let resp = client.post(url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Telegram returned {}", resp.status());
    }
    Ok(())
}

async fn send_generic(client: &Client, url: &str, body: &str) -> Result<(), anyhow::Error> {
    let payload = json!({ "text": body });
    let resp = client.post(url).json(&payload).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Webhook returned {}", resp.status());
    }
    Ok(())
}
