use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};

pub fn current_time_millis() -> AppResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(AppError::system_time)?
        .as_millis()
        .try_into()
        .map_err(AppError::system_time)
}
