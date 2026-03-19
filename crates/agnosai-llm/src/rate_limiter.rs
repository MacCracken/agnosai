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

    #[tokio::test]
    async fn concurrent_acquire_limits_to_m_permits() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let max_permits = 3;
        let total_tasks = 10;
        let limiter = RateLimiter::new(max_permits);
        let concurrent_count = StdArc::new(AtomicUsize::new(0));
        let max_observed = StdArc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..total_tasks {
            let lim = limiter.clone();
            let cc = concurrent_count.clone();
            let mo = max_observed.clone();
            handles.push(tokio::spawn(async move {
                let _permit = lim.acquire().await;
                let current = cc.fetch_add(1, Ordering::SeqCst) + 1;
                // Update max observed concurrency
                mo.fetch_max(current, Ordering::SeqCst);
                // Simulate some work
                tokio::task::yield_now().await;
                cc.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let observed = max_observed.load(Ordering::SeqCst);
        assert!(
            observed <= max_permits,
            "max concurrent was {observed}, expected <= {max_permits}"
        );
        assert_eq!(
            concurrent_count.load(Ordering::SeqCst),
            0,
            "all tasks completed"
        );
        assert_eq!(limiter.available(), max_permits, "all permits returned");
    }

    #[tokio::test]
    async fn release_allows_next_waiter() {
        let limiter = RateLimiter::new(1);
        let p1 = limiter.acquire().await;
        assert_eq!(limiter.available(), 0);

        let limiter2 = limiter.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _p = limiter2.acquire().await;
            tx.send(()).unwrap();
            // hold briefly
            tokio::task::yield_now().await;
        });

        // Yield to let the spawned task block on acquire
        tokio::task::yield_now().await;

        // The waiter should be blocked — channel should not have a value yet
        // (we can't easily assert this without a timeout, but we can check permits)
        assert_eq!(limiter.available(), 0);

        // Release — waiter should proceed
        drop(p1);
        rx.await
            .expect("waiter should have acquired and sent signal");
        handle.await.unwrap();
        assert_eq!(limiter.available(), 1);
    }

    #[tokio::test]
    async fn many_acquire_release_cycles_no_leak() {
        let limiter = RateLimiter::new(5);

        for _ in 0..1000 {
            let permit = limiter.acquire().await;
            drop(permit);
        }

        assert_eq!(
            limiter.available(),
            5,
            "all permits should be returned after 1000 cycles"
        );
    }

    #[tokio::test]
    async fn acquire_release_interleaved_many_tasks() {
        let limiter = RateLimiter::new(4);
        let mut handles = Vec::new();

        for i in 0..50 {
            let lim = limiter.clone();
            handles.push(tokio::spawn(async move {
                let _permit = lim.acquire().await;
                // Vary work slightly
                if i % 3 == 0 {
                    tokio::task::yield_now().await;
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(
            limiter.available(),
            4,
            "all permits returned after 50 tasks"
        );
    }
}
