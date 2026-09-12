use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

use futures::FutureExt;

use crate::error::AppResult;
use crate::utils::operation_logger::log_event;

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
            log_event(
                scope,
                "ACK-FAILED",
                format!("subject={subject} exhausted three retries | {failure}"),
            );
            return Ok(JobOutcome::Failed(failure));
        } else {
            log_event(
                scope,
                &format!("RETRY {}/3", attempt + 1),
                format!("subject={subject} | {failure}"),
            );
            tokio::time::sleep(Duration::from_millis(100 * (attempt + 1))).await;
        }
    }
    unreachable!("the retry loop returns after success or the final attempt")
}

#[cfg(test)]
#[path = "../../tests/workers/retry.rs"]
mod tests;
