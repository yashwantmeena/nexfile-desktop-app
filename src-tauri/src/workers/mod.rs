pub mod ai_processing_worker;
pub mod bulk_operation_worker;
pub mod file_processing_worker;
pub mod indexing_worker;
mod retry;

fn queue_config(queue: &str) -> apalis_sqlite::Config {
    use std::time::Duration;

    use apalis::prelude::{BackoffConfig, IntervalStrategy, StrategyBuilder};

    let poll_strategy = StrategyBuilder::new()
        .apply(
            IntervalStrategy::new(Duration::from_millis(100))
                .with_backoff(BackoffConfig::new(Duration::from_secs(1))),
        )
        .build();
    apalis_sqlite::Config::new(queue).with_poll_interval(poll_strategy)
}
