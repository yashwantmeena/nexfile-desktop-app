#[derive(Debug, thiserror::Error)]
#[error("the import metadata counter overflowed")]
pub(crate) struct CounterOverflow;
