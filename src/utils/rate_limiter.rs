use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// A RateLimiter struct that tracks the number of requests made within a given time window for a set of keys.
/// 
/// The RateLimiter maintains a HashMap that tracks the last reset time and the current count of requests for each key.
/// When `check_rate_limit` is called, it checks if the number of requests for the given key has exceeded the `max_requests` limit within the `time_window`.
/// If the limit has been exceeded, it returns `false`, otherwise it updates the count and returns `true`.
pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    max_requests: u32,
    time_window: Duration,
}

/// Checks the rate limit for the given key and updates the count if the limit has not been exceeded.
///
/// This method acquires a lock on the `limits` HashMap, checks if the given key exists, and updates the last reset time and count accordingly. If the count exceeds the `max_requests` limit within the `time_window`, it returns `false`. Otherwise, it updates the count and returns `true`.
///
/// # Arguments
/// * `key` - The key to check the rate limit for.
///
/// # Returns
/// `true` if the rate limit has not been exceeded, `false` otherwise.
impl RateLimiter {
    /// Creates a new `RateLimiter` instance with the specified maximum number of requests and time window.
    ///
    /// The `RateLimiter` maintains a HashMap that tracks the last reset time and the current count of requests for each key.
    /// When `check_rate_limit` is called, it checks if the number of requests for the given key has exceeded the `max_requests` limit within the `time_window`.
    /// If the limit has been exceeded, it returns `false`, otherwise it updates the count and returns `true`.
    ///
    /// # Arguments
    /// * `max_requests` - The maximum number of requests allowed within the time window.
    /// * `time_window` - The duration of the time window in which the requests are counted.
    ///
    /// # Returns
    /// A new `RateLimiter` instance.
    pub fn new(max_requests: u32, time_window: Duration) -> Self {
        RateLimiter {
            limits: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            time_window,
        }
    }

    /// Checks the rate limit for the given key and updates the count if the limit has not been exceeded.
    ///
    /// This method acquires a lock on the `limits` HashMap, checks if the given key exists, and updates the last reset time and count accordingly. If the count exceeds the `max_requests` limit within the `time_window`, it returns `false`. Otherwise, it updates the count and returns `true`.
    ///
    /// # Arguments
    /// * `key` - The key to check the rate limit for.
    ///
    /// # Returns
    /// `true` if the rate limit has not been exceeded, `false` otherwise.
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let mut limits = self.limits.lock().await;
        let now = Instant::now();

        if let Some((last_reset, count)) = limits.get_mut(key) {
            if now.duration_since(*last_reset) > self.time_window {
                *last_reset = now;
                *count = 1;
            } else if *count >= self.max_requests {
                return false;
            } else {
                *count += 1;
            }
        } else {
            limits.insert(key.to_string(), (now, 1));
        }

        true
    }
}
