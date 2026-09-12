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
