use crate::error::AppError;
use crate::state::AppState;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    hits: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            limit: limit_per_minute,
            window: Duration::from_secs(60),
            hits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn check(&self, key: &str) -> Result<(), AppError> {
        let mut guard = self.hits.lock().expect("rate limiter poisoned");
        let now = Instant::now();
        let entries = guard.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < self.window);
        if entries.len() as u32 >= self.limit {
            return Err(AppError::RateLimited);
        }
        entries.push(now);
        Ok(())
    }
}

pub async fn rate_limit(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let key = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("local")
        .to_string();
    state.rate_limiter.check(&key)?;
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(matches!(limiter.check("a"), Err(AppError::RateLimited)));
        assert!(limiter.check("b").is_ok());
    }
}
