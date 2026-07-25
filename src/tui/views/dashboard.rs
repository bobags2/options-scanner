use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Options Scanner ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "[s]can  [enter] detail  [tab] view  [q]uit",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    if app.scan_in_progress {
        let scanning = Paragraph::new("Scanning... this may take a few minutes.")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().title("Status").borders(Borders::ALL));
        frame.render_widget(scanning, chunks[1]);
    } else if app.opportunities.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from("No opportunities found yet."),
            Line::from(""),
            Line::from("Press 's' to scan tickers, or use the CLI:"),
            Line::from("  options-scanner scan --ticker AAPL,TSLA,MSFT"),
        ])
        .block(Block::default().title("Dashboard").borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app.opportunities.iter().take(50).enumerate().map(|(i, opp)| {
            let style = if i == app.selected_opportunity {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let line = Line::from(vec![
                Span::styled(format!(" {:>2}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("[{:^14}]", opp.strategy), Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {} ", opp.contract.ticker)),
                Span::styled(format!("{}", opp.contract.option_type), Style::default().fg(Color::Green)),
                Span::raw(format!(" ${:<6.0}", opp.contract.strike)),
                Span::raw(format!(" {} ", opp.contract.expiration)),
                Span::styled(format!("score:{:>3.0}", opp.score), Style::default().fg(Color::Magenta)),
                Span::raw(format!(" IV:{:.0}%", opp.contract.implied_volatility.unwrap_or(0.0) * 100.0)),
            ]);
            ListItem::new(line).style(style)
        }).collect();

        let list = List::new(items).block(
            Block::default()
                .title(format!(" Top Opportunities ({}) ", app.opportunities.len()))
                .borders(Borders::ALL),
        );
        frame.render_widget(list, chunks[1]);
    }

    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default().bg(Color::DarkGray)),
        Span::styled(&app.status_message, Style::default().fg(Color::White)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, chunks[2]);
}
