pub mod unusual_volume;
pub mod iv_crush;
pub mod wheel;
pub mod cheap_directional;
pub mod spreads;
pub mod straddles;
pub mod calendar;
pub mod covered_call;
pub mod butterfly;
pub mod iron_condor;
pub mod ratio;

use async_trait::async_trait;
use crate::types::{Opportunity, OptionChain, UnderlyingPrices};
use crate::config::StrategiesConfig;

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn scan(
        &self,
        chains: &[OptionChain],
        underlying_prices: &UnderlyingPrices,
        config: &StrategiesConfig,
        risk_free_rate: f64,
    ) -> Vec<Opportunity>;
}

pub use unusual_volume::UnusualVolumeStrategy;
pub use iv_crush::IvCrushStrategy;
pub use wheel::WheelStrategy;
pub use cheap_directional::CheapDirectionalStrategy;
pub use spreads::SpreadStrategy;
pub use straddles::StraddleStrategy;
pub use calendar::CalendarStrategy;
pub use covered_call::CoveredCallStrategy;
pub use butterfly::ButterflyStrategy;
pub use iron_condor::IronCondorStrategy;
pub use ratio::RatioSpreadStrategy;

pub fn all_strategies() -> Vec<Box<dyn Strategy>> {
    vec![
        Box::new(UnusualVolumeStrategy),
        Box::new(IvCrushStrategy),
        Box::new(WheelStrategy),
        Box::new(CheapDirectionalStrategy),
        Box::new(SpreadStrategy),
        Box::new(StraddleStrategy),
        Box::new(CalendarStrategy),
        Box::new(CoveredCallStrategy),
        Box::new(ButterflyStrategy),
        Box::new(IronCondorStrategy),
        Box::new(RatioSpreadStrategy),
    ]
}
