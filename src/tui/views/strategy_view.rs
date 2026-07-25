use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::types::StrategyType;

const STRATEGIES: &[(StrategyType, &str)] = &[
    (StrategyType::UnusualVolume, "Unusual Volume — big institutional positioning"),
    (StrategyType::IvCrush, "IV Crush — high IV likely to drop after event"),
    (StrategyType::WheelSetup, "Wheel Setup — sell puts on stocks you'd own"),
    (StrategyType::CheapDirectional, "Cheap Directional — low-cost directional bets"),
    (StrategyType::CreditSpread, "Credit Spread — collect premium with defined risk"),
    (StrategyType::Straddle, "Straddle — bet on a big move either direction"),
    (StrategyType::CalendarSpread, "Calendar Spread — profit from time decay differential"),
    (StrategyType::CoveredCall, "Covered Call — sell calls against stock you own"),
    (StrategyType::Butterfly, "Butterfly — low-cost pin play with defined risk"),
    (StrategyType::IronCondor, "Iron Condor — collect premium with defined risk range"),
    (StrategyType::RatioSpread, "Ratio Spread — near-zero cost asymmetric upside"),
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        " Strategies — [up/down] select  [enter] view  [esc] back  [q]uit",
        Style::default().fg(Color::DarkGray),
    )))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Strategy list
    let items: Vec<ListItem> = STRATEGIES
        .iter()
        .enumerate()
        .map(|(i, (_, desc))| {
            let style = if i == app.selected_strategy {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(format!("  {} ", desc), style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Available Strategies")
            .borders(Borders::ALL),
    );
    frame.render_widget(list, chunks[1]);

    // Count for selected strategy
    if let Some((strat_type, _)) = STRATEGIES.get(app.selected_strategy) {
        let count = app
            .opportunities
            .iter()
            .filter(|o| o.strategy == *strat_type)
            .count();
        let info = Paragraph::new(format!("{} opportunities found for this strategy", count))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(info, chunks[2]);
    }
}
