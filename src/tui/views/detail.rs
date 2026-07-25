use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        " Opportunity Detail — [up/down] navigate  [esc] back  [q]uit",
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    if app.opportunities.is_empty() {
        let empty = Paragraph::new("No opportunities to display. Run a scan first.")
            .block(Block::default().title("Detail").borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let idx = app.selected_opportunity.min(app.opportunities.len() - 1);
    let opp = &app.opportunities[idx];

    // Contract summary
    let contract_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", opp.contract.ticker),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", opp.contract.option_type),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(format!("  Strike: ${:.2}", opp.contract.strike)),
            Span::raw(format!("  Exp: {}", opp.contract.expiration)),
            Span::raw(format!("  Score: {:.0}/100", opp.score)),
        ]),
        Line::from(vec![
            Span::raw(format!("  Bid: ${:.2}", opp.contract.bid)),
            Span::raw(format!("  Ask: ${:.2}", opp.contract.ask)),
            Span::raw(format!("  Mid: ${:.2}", opp.contract.mid_price())),
            Span::raw(format!("  Vol: {}", opp.contract.volume)),
            Span::raw(format!("  OI: {}", opp.contract.open_interest)),
        ]),
        Line::from(vec![
            Span::raw("  Greeks: "),
            Span::styled(format!("D:{:.2}", opp.greeks.delta), Style::default().fg(Color::Green)),
            Span::raw(format!("  G:{:.4}", opp.greeks.gamma)),
            Span::styled(format!("  T:{:.3}", opp.greeks.theta), Style::default().fg(Color::Red)),
            Span::raw(format!("  V:{:.3}", opp.greeks.vega)),
        ]),
        Line::from(vec![
            Span::raw(format!("  IV: {:.1}%", opp.contract.implied_volatility.unwrap_or(0.0) * 100.0)),
            Span::raw(format!("  Spread: {:.1}%", opp.contract.spread_pct())),
        ]),
    ])
    .block(
        Block::default()
            .title(format!(
                " {} [{}/{}] ",
                opp.strategy,
                idx + 1,
                app.opportunities.len()
            ))
            .borders(Borders::ALL),
    );
    frame.render_widget(contract_info, chunks[1]);

    // Why this matters
    let why = Paragraph::new(opp.explanation.clone())
        .block(
            Block::default()
                .title(" Why This Matters ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(why, chunks[2]);

    // Risk summary
    let risk = Paragraph::new(opp.risk_summary.clone())
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .title(" Risk ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(risk, chunks[3]);

    // Navigation hint
    let nav = Paragraph::new(Line::from(Span::styled(
        format!(" Showing {}/{} — use up/down arrows to browse ", idx + 1, app.opportunities.len()),
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(nav, chunks[4]);
}
