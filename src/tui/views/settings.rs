use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(Span::styled(
        " Settings — [esc] back  [q]uit",
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let settings = Paragraph::new(vec![
        Line::from(Span::styled(
            " Configuration",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Data Source: Yahoo Finance (free tier)"),
        Line::from("  Cache: 5 min prices / 15 min chains"),
        Line::from("  Rate Limit: ~2000 req/hr"),
        Line::from("  Scan Scope: Nearest 3 expirations per ticker"),
        Line::from(""),
        Line::from(Span::styled(
            " Strategies Active",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  [x] Unusual Volume"),
        Line::from("  [x] IV Crush"),
        Line::from("  [x] Wheel Setup"),
        Line::from("  [x] Cheap Directional"),
        Line::from("  [x] Credit Spread"),
        Line::from("  [x] Straddle"),
        Line::from("  [x] Calendar Spread"),
        Line::from("  [x] Covered Call"),
        Line::from("  [x] Butterfly"),
        Line::from("  [x] Iron Condor"),
        Line::from("  [x] Ratio Spread"),
        Line::from(""),
        Line::from(Span::styled(
            " Edit config.toml to change settings.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().title("Settings").borders(Borders::ALL));
    frame.render_widget(settings, chunks[1]);

    let footer = Paragraph::new(Line::from(Span::styled(
        " Settings are read from config.toml at startup ",
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}
