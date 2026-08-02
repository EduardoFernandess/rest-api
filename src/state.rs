use crate::middleware::RateLimiter;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<String>,
    pub rate_limiter: RateLimiter,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_secret: String, rate_limit_per_minute: u32) -> Self {
        Self {
            pool,
            jwt_secret: Arc::new(jwt_secret),
            rate_limiter: RateLimiter::new(rate_limit_per_minute),
        }
    }
}
