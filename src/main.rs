use clap::{Parser, Subcommand};
use options_scanner::config::Config;
use options_scanner::data::{DataProvider, YahooProvider, CachedProvider};
use options_scanner::strategies::all_strategies;
use options_scanner::tui::app::{App, View};
use options_scanner::types::Opportunity;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

fn load_ticker_universe(csv_path: &std::path::Path) -> Vec<String> {
    let fallback = vec![
        "AAPL".into(), "MSFT".into(), "TSLA".into(), "NVDA".into(),
        "AMZN".into(), "META".into(), "GOOGL".into(), "SPY".into(),
        "QQQ".into(), "AMD".into(), "PLTR".into(), "IWM".into(),
    ];
    let content = match std::fs::read_to_string(csv_path) {
        Ok(c) => c,
        Err(_) => return fallback,
    };
    let tickers: Vec<String> = content
        .lines()
        .skip(1)
        .filter_map(|line| line.split(',').next().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'))
        .collect();
    if tickers.is_empty() { fallback } else { tickers }
}

async fn refresh_sp500_tickers(csv_path: &std::path::Path) -> anyhow::Result<()> {
    use std::io::Write;
    println!("Fetching S&P 500 list from Wikipedia...");

    let client = reqwest::Client::builder()
        .user_agent("options-scanner/0.1 (Rust)")
        .build()?;

    let html = client
        .get("https://en.wikipedia.org/wiki/List_of_S%26P_500_companies")
        .send()
        .await?
        .text()
        .await?;

    let mut tickers: Vec<String> = Vec::new();
    let table_marker = "<table id=\"constituents\"";
    if let Some(table_start) = html.find(table_marker) {
        let table_section = &html[table_start..];
        let mut in_row = false;
        let mut first_cell = false;
        let mut buf = String::new();
        let mut in_td = false;

        for token in table_section.split_inclusive('>') {
            if token.contains("</table>") {
                break;
            }
            if token.contains("<tr") {
                in_row = true;
                first_cell = true;
                continue;
            }
            if token.contains("</tr>") {
                in_row = false;
                continue;
            }
            if in_row && first_cell && token.contains("<td") {
                in_td = true;
                buf.clear();
                continue;
            }
            if in_td && token.contains("</td>") {
                let ticker = buf.trim().to_string();
                if !ticker.is_empty()
                    && ticker.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
                {
                    tickers.push(ticker);
                }
                in_td = false;
                first_cell = false;
                continue;
            }
            if in_td {
                let clean = token.split('<').next().unwrap_or("");
                buf.push_str(clean);
            }
        }
    }

    if tickers.is_empty() {
        anyhow::bail!("Failed to parse any tickers from Wikipedia");
    }

    tickers.sort();
    tickers.dedup();

    let mut f = std::fs::File::create(csv_path)?;
    writeln!(f, "Symbol,Name")?;
    for t in &tickers {
        writeln!(f, "{},", t)?;
    }

    println!("Saved {} S&P 500 tickers to {}", tickers.len(), csv_path.display());
    Ok(())
}

async fn scan_tickers(
    tickers: &[String],
    cfg: &Config,
) -> Vec<Opportunity> {
    scan_tickers_with_progress(tickers, cfg, None).await
}

async fn scan_tickers_with_progress(
    tickers: &[String],
    cfg: &Config,
    progress: Option<Arc<AtomicUsize>>,
) -> Vec<Opportunity> {
    let provider = Arc::new(CachedProvider::new(
        YahooProvider::with_rate_limit(cfg.api.yahoo_requests_per_hour),
        cfg.cache.price_ttl_seconds,
        cfg.cache.chain_ttl_seconds,
    ));
    let strats = Arc::new(all_strategies());
    let rfr = cfg.scanner.risk_free_rate;
    let strat_cfg = cfg.strategies.clone();
    let sem = Arc::new(Semaphore::new(15));
    let mut handles = Vec::new();

    for t in tickers {
        let t = t.clone();
        let provider = provider.clone();
        let strats = strats.clone();
        let strat_cfg = strat_cfg.clone();
        let sem = sem.clone();
        let progress = progress.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let mut opps = Vec::new();

            let (price, exps) = match provider.get_price_and_expirations(&t).await {
                Ok(pa) => pa,
                Err(_) => {
                    if let Some(p) = &progress { p.fetch_add(1, Ordering::Relaxed); }
                    return opps;
                }
            };

            let today = chrono::Utc::now().date_naive();
            let useful: Vec<_> = exps.iter()
                .filter(|e| {
                    let dte = (**e - today).num_days();
                    dte >= 7 && dte <= 90
                })
                .take(6)
                .collect();

            let mut chains = Vec::new();
            for exp in useful {
                if let Ok(chain) = provider.get_option_chain(&t, *exp).await {
                    chains.push(chain);
                }
            }

            let mut prices = std::collections::HashMap::new();
            prices.insert(t, price);

            for strat in strats.iter() {
                let found = strat.scan(&chains, &prices, &strat_cfg, rfr).await;
                opps.extend(found);
            }

            if let Some(p) = &progress { p.fetch_add(1, Ordering::Relaxed); }
            opps
        }));
    }

    let mut all_opps = Vec::new();
    for h in handles {
        if let Ok(opps) = h.await {
            all_opps.extend(opps);
        }
    }

    all_opps.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen = std::collections::HashSet::new();
    all_opps.retain(|o| {
        let key = (
            o.contract.ticker.clone(),
            o.contract.strike.to_bits(),
            o.contract.expiration,
            o.contract.option_type,
        );
        seen.insert(key)
    });

    all_opps
}

#[derive(Parser)]
#[command(name = "options-scanner", version, about = "Options chain scanner for finding trading opportunities")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan the full ticker universe and rank the best opportunities
    Scan {
        #[arg(short, long, value_delimiter = ',', help = "Specific tickers (default: scan full universe)")]
        ticker: Vec<String>,
        #[arg(short, long, default_value = "20", help = "Number of top results to show")]
        top: usize,
        #[arg(short, long, help = "Export results to file (.json or .csv)")]
        output: Option<String>,
    },
    /// Launch the interactive TUI
    Tui {
        #[arg(short, long, value_delimiter = ',', help = "Specific tickers (default: scan full universe)")]
        ticker: Vec<String>,
    },
    /// Refresh S&P 500 ticker list from Wikipedia
    RefreshTickers,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let cfg = Config::load_or_default(&cli.config);

    match cli.command {
        Commands::Scan { ticker, top, output } => {
            let tickers = if ticker.is_empty() {
                load_ticker_universe(&cfg.scanner.sp500_csv_path)
            } else {
                ticker
            };
            run_scan(&tickers, top, &cfg, output.as_deref()).await?;
        }
        Commands::Tui { ticker } => {
            let tickers = if ticker.is_empty() {
                load_ticker_universe(&cfg.scanner.sp500_csv_path)
            } else {
                ticker
            };
            run_tui(&tickers, &cfg).await?;
        }
        Commands::RefreshTickers => {
            refresh_sp500_tickers(&cfg.scanner.sp500_csv_path).await?;
        }
    }

    Ok(())
}

async fn run_scan(tickers: &[String], top_n: usize, cfg: &Config, output: Option<&str>) -> anyhow::Result<()> {
    println!("Scanning {} tickers with all strategies...", tickers.len());

    let progress = Arc::new(AtomicUsize::new(0));
    let total = tickers.len();
    let progress_clone = progress.clone();

    let scan_handle = {
        let tickers = tickers.to_vec();
        let cfg = cfg.clone();
        let progress = progress.clone();
        tokio::spawn(async move {
            scan_tickers_with_progress(&tickers, &cfg, Some(progress)).await
        })
    };

    let poll = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let done = progress_clone.load(Ordering::Relaxed);
            print!("\r  {}/{} tickers scanned...", done, total);
            use std::io::Write;
            let _ = std::io::stdout().flush();
            if done >= total { break; }
        }
    });

    let all_opps = scan_handle.await.unwrap_or_default();
    poll.abort();
    println!("\r  {}/{} tickers scanned.    ", total, total);

    if all_opps.is_empty() {
        println!("No opportunities found.");
        return Ok(());
    }

    println!("Found {} total opportunities. Top {}:\n", all_opps.len(), top_n.min(all_opps.len()));

    for (i, opp) in all_opps.iter().take(top_n).enumerate() {
        println!("{}. [{}] {} {} ${:.0} {} (score: {:.0})",
            i + 1,
            opp.strategy,
            opp.contract.ticker,
            opp.contract.option_type,
            opp.contract.strike,
            opp.contract.expiration,
            opp.score,
        );
        println!("   Vol/OI: {}/{} | IV: {:.1}% | Delta: {:.2} | Spread: {:.1}%",
            opp.contract.volume,
            opp.contract.open_interest,
            opp.contract.implied_volatility.unwrap_or(0.0) * 100.0,
            opp.greeks.delta,
            opp.contract.spread_pct(),
        );
        println!("   Why: {}", opp.explanation);
        println!("   Risk: {}", opp.risk_summary);
        println!();
    }

    if let Ok(conn) = options_scanner::data::persist::init_db(&cfg.cache.sqlite_path) {
        if let Ok(n) = options_scanner::data::persist::save_opportunities(&conn, &all_opps) {
            println!("Saved {} results to {}", n, cfg.cache.sqlite_path.display());
        }
    }

    if let Some(path) = output {
        use std::io::Write;
        if path.ends_with(".json") {
            let json = serde_json::to_string_pretty(&all_opps)?;
            std::fs::write(path, json)?;
            println!("Exported {} opportunities to {}", all_opps.len(), path);
        } else if path.ends_with(".csv") {
            let mut f = std::fs::File::create(path)?;
            writeln!(f, "rank,strategy,ticker,option_type,strike,expiration,score,volume,open_interest,iv,delta,gamma,theta,vega")?;
            for (i, opp) in all_opps.iter().enumerate() {
                writeln!(f, "{},{},{},{},{:.2},{},{:.1},{},{},{:.4},{:.3},{:.4},{:.4},{:.4}",
                    i + 1, opp.strategy, opp.contract.ticker, opp.contract.option_type,
                    opp.contract.strike, opp.contract.expiration, opp.score,
                    opp.contract.volume, opp.contract.open_interest,
                    opp.contract.implied_volatility.unwrap_or(0.0),
                    opp.greeks.delta, opp.greeks.gamma, opp.greeks.theta, opp.greeks.vega,
                )?;
            }
            println!("Exported {} opportunities to {}", all_opps.len(), path);
        } else {
            println!("Unsupported output format. Use .json or .csv");
        }
    }

    Ok(())
}

async fn run_tui(tickers: &[String], cfg: &Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let tickers_owned: Vec<String> = tickers.to_vec();
    let cfg_clone = cfg.clone();

    let result = run_tui_loop(&mut terminal, &mut app, &tickers_owned, &cfg_clone).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tickers: &[String],
    cfg: &Config,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<Vec<Opportunity>>(1);

    loop {
        if let Ok(opps) = rx.try_recv() {
            let count = opps.len();
            app.opportunities = opps;
            app.scan_in_progress = false;
            app.status_message = format!("Found {} opportunities. Press 's' to rescan.", count);
        }

        terminal.draw(|frame| {
            let area = frame.area();
            match app.current_view {
                View::Dashboard => options_scanner::tui::views::dashboard::render(frame, area, app),
                View::StrategyList => options_scanner::tui::views::strategy_view::render(frame, area, app),
                View::Detail => options_scanner::tui::views::detail::render(frame, area, app),
                View::Settings => options_scanner::tui::views::settings::render(frame, area, app),
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.next_view();
                    }
                    KeyCode::BackTab => {
                        app.prev_view();
                    }
                    KeyCode::Esc => {
                        app.prev_view();
                    }
                    KeyCode::Enter => {
                        if app.current_view == View::StrategyList {
                            app.current_view = View::Detail;
                            app.selected_opportunity = 0;
                        } else if app.current_view == View::Dashboard && !app.opportunities.is_empty() {
                            app.current_view = View::Detail;
                            app.selected_opportunity = 0;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.select_next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.select_prev();
                    }
                    KeyCode::Char('s') => {
                        if !app.scan_in_progress {
                            app.scan_in_progress = true;
                            app.status_message = format!("Scanning {} tickers...", tickers.len());
                            let tickers_clone = tickers.to_vec();
                            let cfg_clone = cfg.clone();
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                let opps = scan_tickers(&tickers_clone, &cfg_clone).await;
                                let _ = tx.send(opps).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
