use std::num::NonZeroU32;
use governor::{Quota, RateLimiter, clock::DefaultClock, state::{InMemoryState, NotKeyed}};
use std::sync::Arc;

pub type Limiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub fn create_limiter(requests_per_hour: u32) -> Arc<Limiter> {
    let per_second = (requests_per_hour as f64 / 3600.0).ceil() as u32;
    let quota = Quota::per_second(NonZeroU32::new(per_second.max(1)).unwrap());
    Arc::new(RateLimiter::direct(quota))
}

pub async fn wait_for_permit(limiter: &Limiter) {
    while limiter.check().is_err() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
