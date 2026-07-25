use crate::types::Opportunity;

pub const NUM_STRATEGIES: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    StrategyList,
    Detail,
    Settings,
}

pub struct App {
    pub current_view: View,
    pub opportunities: Vec<Opportunity>,
    pub selected_strategy: usize,
    pub selected_opportunity: usize,
    pub should_quit: bool,
    pub status_message: String,
    pub scan_in_progress: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_view: View::Dashboard,
            opportunities: Vec::new(),
            selected_strategy: 0,
            selected_opportunity: 0,
            should_quit: false,
            status_message: "Ready. Press 's' to scan or 'q' to quit.".to_string(),
            scan_in_progress: false,
        }
    }

    pub fn next_view(&mut self) {
        self.current_view = match self.current_view {
            View::Dashboard => View::StrategyList,
            View::StrategyList => View::Detail,
            View::Detail => View::Settings,
            View::Settings => View::Dashboard,
        };
    }

    pub fn prev_view(&mut self) {
        self.current_view = match self.current_view {
            View::Dashboard => View::Settings,
            View::StrategyList => View::Dashboard,
            View::Detail => View::StrategyList,
            View::Settings => View::Detail,
        };
    }

    pub fn select_next(&mut self) {
        match self.current_view {
            View::StrategyList => {
                if self.selected_strategy < NUM_STRATEGIES.saturating_sub(1) {
                    self.selected_strategy += 1;
                }
            }

            View::Detail => {
                if !self.opportunities.is_empty()
                    && self.selected_opportunity < self.opportunities.len() - 1
                {
                    self.selected_opportunity += 1;
                }
            }
            _ => {}
        }
    }

    pub fn select_prev(&mut self) {
        match self.current_view {
            View::StrategyList => {
                if self.selected_strategy > 0 {
                    self.selected_strategy -= 1;
                }
            }
            View::Detail => {
                if self.selected_opportunity > 0 {
                    self.selected_opportunity -= 1;
                }
            }
            _ => {}
        }
    }
}
