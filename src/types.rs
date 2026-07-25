use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionType::Call => write!(f, "Call"),
            OptionType::Put => write!(f, "Put"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionContract {
    pub ticker: String,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub option_type: OptionType,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: u64,
    pub open_interest: u64,
    pub implied_volatility: Option<f64>,
}

impl OptionContract {
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }

    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }

    pub fn spread_pct(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            self.spread() / mid * 100.0
        } else {
            f64::NAN
        }
    }

    pub fn days_to_expiration(&self, today: NaiveDate) -> u32 {
        (self.expiration - today).num_days().max(0) as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionChain {
    pub ticker: String,
    pub expiration: NaiveDate,
    pub contracts: Vec<OptionContract>,
}

impl OptionChain {
    pub fn calls(&self) -> Vec<&OptionContract> {
        self.contracts
            .iter()
            .filter(|c| c.option_type == OptionType::Call)
            .collect()
    }

    pub fn puts(&self) -> Vec<&OptionContract> {
        self.contracts
            .iter()
            .filter(|c| c.option_type == OptionType::Put)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyType {
    UnusualVolume,
    IvCrush,
    WheelSetup,
    CheapDirectional,
    CreditSpread,
    DebitSpread,
    IronCondor,
    Straddle,
    Strangle,
    CalendarSpread,
    CoveredCall,
    Butterfly,
    RatioSpread,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::UnusualVolume => write!(f, "Unusual Volume"),
            StrategyType::IvCrush => write!(f, "IV Crush"),
            StrategyType::WheelSetup => write!(f, "Wheel Setup"),
            StrategyType::CheapDirectional => write!(f, "Cheap Directional"),
            StrategyType::CreditSpread => write!(f, "Credit Spread"),
            StrategyType::DebitSpread => write!(f, "Debit Spread"),
            StrategyType::IronCondor => write!(f, "Iron Condor"),
            StrategyType::Straddle => write!(f, "Straddle"),
            StrategyType::Strangle => write!(f, "Strangle"),
            StrategyType::CalendarSpread => write!(f, "Calendar Spread"),
            StrategyType::CoveredCall => write!(f, "Covered Call"),
            StrategyType::Butterfly => write!(f, "Butterfly"),
            StrategyType::RatioSpread => write!(f, "Ratio Spread"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub contract: OptionContract,
    pub greeks: Greeks,
    pub strategy: StrategyType,
    pub score: f64,
    pub explanation: String,
    pub risk_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLegOpportunity {
    pub legs: Vec<OptionContract>,
    pub greeks: Greeks,
    pub strategy: StrategyType,
    pub score: f64,
    pub explanation: String,
    pub risk_summary: String,
    pub max_profit: f64,
    pub max_loss: f64,
    pub net_debit_credit: f64,
}

pub type UnderlyingPrices = HashMap<String, f64>;
