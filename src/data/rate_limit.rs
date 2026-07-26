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
    // Poll check() with short tokio sleep — yields to the executor without
    // relying on governor's DefaultClock (which uses std::thread::sleep internally
    // and would block the tokio runtime thread if used via until_ready()).
    while limiter.check().is_err() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
