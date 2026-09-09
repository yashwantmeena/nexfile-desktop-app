use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

use futures::FutureExt;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum JobOutcome {
    Completed,
    Failed(String),
}

/// One initial attempt and three retries. Returning Ok acknowledges exhausted jobs.
pub(super) async fn retry_and_ack<F, Fut>(
    scope: &str,
    subject: &str,
    mut process: F,
) -> AppResult<JobOutcome>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = AppResult<()>>,
{
    for attempt in 0..=3 {
        let failure = match AssertUnwindSafe(async { process().await })
            .catch_unwind()
            .await
        {
            Ok(Ok(())) => return Ok(JobOutcome::Completed),
            Ok(Err(error)) => format!("{error:?}"),
            Err(_) => "job handler panicked".to_owned(),
        };
        if attempt == 3 {
            eprintln!("[{scope}][ACK-FAILED] {subject} | exhausted three retries | {failure}");
            return Ok(JobOutcome::Failed(failure));
        } else {
            eprintln!("[{scope}][RETRY {}/3] {subject} | {failure}", attempt + 1);
            tokio::time::sleep(Duration::from_millis(100 * (attempt + 1))).await;
        }
    }
    unreachable!("the retry loop returns after success or the final attempt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    #[tokio::test]
    async fn acknowledges_after_three_retries_and_accepts_next_job() {
        let mut attempts = 0;
        retry_and_ack("test", "bad job", || {
            attempts += 1;
            async { Err(AppError::validation("invalid job")) }
        })
        .await
        .unwrap();
        assert_eq!(attempts, 4);
        let mut next_attempts = 0;
        retry_and_ack("test", "next job", || {
            next_attempts += 1;
            async { Ok(()) }
        })
        .await
        .unwrap();
        assert_eq!(next_attempts, 1);
    }

    #[tokio::test]
    async fn stops_retrying_after_success() {
        let mut attempts = 0;
        retry_and_ack("test", "transient failure", || {
            attempts += 1;
            let attempt = attempts;
            async move {
                if attempt < 3 {
                    Err(AppError::validation("temporary"))
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn acknowledges_panicking_jobs_after_retry_limit() {
        let mut attempts = 0;
        retry_and_ack("test", "panic", || {
            attempts += 1;
            async { panic!("bad handler") }
        })
        .await
        .unwrap();
        assert_eq!(attempts, 4);
    }
}
