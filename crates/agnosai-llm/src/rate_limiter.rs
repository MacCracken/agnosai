//! Semaphore-based rate limiting per provider.
//!
//! Caps the number of concurrent in-flight requests to a provider,
//! preventing overload and respecting API rate limits.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// A concurrency limiter backed by a tokio semaphore.
#[derive(Clone)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl RateLimiter {
    /// Create a limiter that allows `max_concurrent` in-flight requests.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Acquire a permit, waiting if the concurrency cap is reached.
    ///
    /// The permit is held until dropped, at which point the slot is released.
    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore should never be closed")
    }

    /// Number of currently available permits.
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// The configured maximum concurrency.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_and_release() {
        let limiter = RateLimiter::new(2);
        assert_eq!(limiter.available(), 2);

        let p1 = limiter.acquire().await;
        assert_eq!(limiter.available(), 1);

        let p2 = limiter.acquire().await;
        assert_eq!(limiter.available(), 0);

        drop(p1);
        assert_eq!(limiter.available(), 1);

        drop(p2);
        assert_eq!(limiter.available(), 2);
    }

    #[tokio::test]
    async fn clone_shares_semaphore() {
        let limiter = RateLimiter::new(3);
        let limiter2 = limiter.clone();

        let _p = limiter.acquire().await;
        assert_eq!(limiter2.available(), 2);
    }

    #[tokio::test]
    async fn max_concurrent_accessor() {
        let limiter = RateLimiter::new(10);
        assert_eq!(limiter.max_concurrent(), 10);
    }

    #[tokio::test]
    async fn waits_when_full() {
        let limiter = RateLimiter::new(1);
        let p1 = limiter.acquire().await;

        // Spawn a task that will wait for the permit
        let limiter2 = limiter.clone();
        let handle = tokio::spawn(async move {
            let _p = limiter2.acquire().await;
            42
        });

        // Give the spawned task a moment to reach the semaphore
        tokio::task::yield_now().await;
        assert_eq!(limiter.available(), 0);

        // Release p1 — the spawned task should now complete
        drop(p1);
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }
}
