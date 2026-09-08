use crate::error::{AppError, AppResult};

pub(crate) fn timestamp_ms() -> AppResult<i64> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(AppError::system_time)?;
    i64::try_from(elapsed.as_millis()).map_err(AppError::internal)
}
