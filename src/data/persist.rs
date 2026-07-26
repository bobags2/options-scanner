use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;

use crate::types::Opportunity;

pub fn init_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS scan_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scanned_at TEXT NOT NULL,
            strategy TEXT NOT NULL,
            ticker TEXT NOT NULL,
            option_type TEXT NOT NULL,
            strike REAL NOT NULL,
            expiration TEXT NOT NULL,
            score REAL NOT NULL,
            volume INTEGER NOT NULL,
            open_interest INTEGER NOT NULL,
            implied_volatility REAL,
            delta REAL,
            gamma REAL,
            theta REAL,
            vega REAL,
            explanation TEXT,
            risk_summary TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_scan_ticker ON scan_results(ticker);
        CREATE INDEX IF NOT EXISTS idx_scan_strategy ON scan_results(strategy);
        CREATE INDEX IF NOT EXISTS idx_scan_date ON scan_results(scanned_at);"
    )?;
    Ok(conn)
}

pub fn save_opportunities(conn: &Connection, opps: &[Opportunity]) -> Result<usize> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.unchecked_transaction()?;
    let mut count = 0;

    for opp in opps {
        tx.execute(
            "INSERT INTO scan_results (scanned_at, strategy, ticker, option_type, strike, expiration, score, volume, open_interest, implied_volatility, delta, gamma, theta, vega, explanation, risk_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                now,
                opp.strategy.to_string(),
                opp.contract.ticker,
                opp.contract.option_type.to_string(),
                opp.contract.strike,
                opp.contract.expiration.to_string(),
                opp.score,
                opp.contract.volume,
                opp.contract.open_interest,
                opp.contract.implied_volatility,
                opp.greeks.delta,
                opp.greeks.gamma,
                opp.greeks.theta,
                opp.greeks.vega,
                opp.explanation,
                opp.risk_summary,
            ],
        )?;
        count += 1;
    }

    tx.commit()?;
    Ok(count)
}

pub fn get_recent_scans(conn: &Connection, limit: usize) -> Result<Vec<(String, String, String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT scanned_at, strategy, ticker, score FROM scan_results ORDER BY scanned_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
