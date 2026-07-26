use chrono::NaiveDate;
use options_scanner::data::persist::{init_db, save_opportunities, get_recent_scans};
use options_scanner::types::{Greeks, Opportunity, OptionContract, OptionType, StrategyType};

fn make_opp(ticker: &str, score: f64) -> Opportunity {
    Opportunity {
        contract: OptionContract {
            ticker: ticker.to_string(),
            strike: 100.0,
            expiration: NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            option_type: OptionType::Call,
            bid: 2.0,
            ask: 2.5,
            last: 2.25,
            volume: 500,
            open_interest: 1200,
            implied_volatility: Some(0.35),
        },
        greeks: Greeks {
            delta: 0.45,
            gamma: 0.03,
            theta: -0.05,
            vega: 0.18,
            rho: 0.01,
        },
        strategy: StrategyType::IvCrush,
        score,
        explanation: "Test opportunity".to_string(),
        risk_summary: "Test risk".to_string(),
    }
}

#[test]
fn test_persist_roundtrip() {
    // Use in-memory SQLite
    let conn = init_db(std::path::Path::new(":memory:")).unwrap();
    let opps = vec![
        make_opp("AAPL", 85.0),
        make_opp("TSLA", 92.0),
        make_opp("NVDA", 78.0),
    ];

    let count = save_opportunities(&conn, &opps).unwrap();
    assert_eq!(count, 3);

    let scans = get_recent_scans(&conn, 10).unwrap();
    assert_eq!(scans.len(), 3);

    // Most recent first — all inserted at the same time, so order may vary
    let tickers: Vec<&str> = scans.iter().map(|(_, _, t, _)| t.as_str()).collect();
    assert!(tickers.contains(&"AAPL"));
    assert!(tickers.contains(&"TSLA"));
    assert!(tickers.contains(&"NVDA"));
}

#[test]
fn test_persist_empty_save() {
    let conn = init_db(std::path::Path::new(":memory:")).unwrap();
    let count = save_opportunities(&conn, &[]).unwrap();
    assert_eq!(count, 0);

    let scans = get_recent_scans(&conn, 10).unwrap();
    assert!(scans.is_empty());
}

#[test]
fn test_persist_limit() {
    let conn = init_db(std::path::Path::new(":memory:")).unwrap();
    let opps: Vec<_> = (0..20).map(|i| make_opp("SPY", i as f64)).collect();
    save_opportunities(&conn, &opps).unwrap();

    let scans = get_recent_scans(&conn, 5).unwrap();
    assert_eq!(scans.len(), 5);
}

#[test]
fn test_persist_creates_parent_dir() {
    let dir = std::env::temp_dir().join("options_scanner_test_persist");
    let db_path = dir.join("subdir").join("test.db");

    // Clean up from any prior run
    let _ = std::fs::remove_dir_all(&dir);

    let conn = init_db(&db_path);
    assert!(conn.is_ok(), "init_db should create parent directories");
    assert!(db_path.exists(), "DB file should exist after init");

    // Clean up
    let _ = std::fs::remove_dir_all(&dir);
}
