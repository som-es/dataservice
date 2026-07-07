use std::{future::Future, sync::Arc};

use tokio::sync::Semaphore;

pub const API_MAX_CONCURRENT_TASKS_ENV: &str = "PARLIAMENT_API_MAX_CONCURRENT_TASKS";
pub const DEFAULT_PARLIAMENT_API_MAX_CONCURRENT_TASKS: usize = 24;

#[derive(Clone, Debug)]
pub struct ApiLimiter {
    semaphore: Arc<Semaphore>,
}

impl ApiLimiter {
    pub fn new(max_concurrent_tasks: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent_tasks.max(1))),
        }
    }

    pub fn from_env() -> Self {
        let max_concurrent_tasks = api_max_concurrent_tasks_from_env();
        log::info!("concurrent scraper tasks: {max_concurrent_tasks}");
        Self::new(max_concurrent_tasks)
    }

    pub async fn run<F, T>(&self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("parliament API limiter semaphore must not be closed");
        future.await
    }
}

pub fn api_max_concurrent_tasks_from_env() -> usize {
    std::env::var(API_MAX_CONCURRENT_TASKS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_PARLIAMENT_API_MAX_CONCURRENT_TASKS)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex, OnceLock,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::*;

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn max_concurrent_tasks_env_defaults_when_unset() {
        let _guard = env_lock().lock().unwrap();
        unsafe { std::env::remove_var(API_MAX_CONCURRENT_TASKS_ENV) };

        assert_eq!(
            api_max_concurrent_tasks_from_env(),
            DEFAULT_PARLIAMENT_API_MAX_CONCURRENT_TASKS
        );
    }

    #[test]
    fn max_concurrent_tasks_env_uses_valid_value() {
        let _guard = env_lock().lock().unwrap();
        unsafe { std::env::set_var(API_MAX_CONCURRENT_TASKS_ENV, "13") };

        assert_eq!(api_max_concurrent_tasks_from_env(), 12);

        unsafe { std::env::remove_var(API_MAX_CONCURRENT_TASKS_ENV) };
    }

    #[test]
    fn max_concurrent_tasks_env_defaults_for_zero_negative_and_invalid_values() {
        let _guard = env_lock().lock().unwrap();
        for value in ["0", "-1", "nope"] {
            unsafe { std::env::set_var(API_MAX_CONCURRENT_TASKS_ENV, value) };

            assert_eq!(
                api_max_concurrent_tasks_from_env(),
                DEFAULT_PARLIAMENT_API_MAX_CONCURRENT_TASKS
            );
        }

        unsafe { std::env::remove_var(API_MAX_CONCURRENT_TASKS_ENV) };
    }

    #[tokio::test]
    async fn limiter_caps_concurrent_futures() {
        let limiter = ApiLimiter::new(2);
        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..8 {
            let limiter = limiter.clone();
            let active = active.clone();
            let max_seen = max_seen.clone();

            handles.push(tokio::spawn(async move {
                limiter
                    .run(async {
                        let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_seen.fetch_max(now_active, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        active.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(max_seen.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn limiter_releases_permits_after_errors() {
        let limiter = ApiLimiter::new(1);

        let first = limiter.run(async { Result::<(), ()>::Err(()) }).await;
        let second = limiter.run(async { Result::<usize, ()>::Ok(1) }).await;

        assert!(first.is_err());
        assert_eq!(second, Ok(1));
    }
}
