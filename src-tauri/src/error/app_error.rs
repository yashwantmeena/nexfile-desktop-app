use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::ser::{Serialize, SerializeStruct, Serializer};

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// The application's error boundary.
///
/// User-actionable variants retain their message. Infrastructure variants
/// retain their original error as a source, but serialize only a safe message
/// across the Tauri IPC boundary.
#[derive(Debug)]
pub enum AppError {
    Validation { message: String },
    StorageUnavailable { message: String },
    Database { source: BoxError },
    Filesystem { source: BoxError },
    Serialization { source: BoxError },
    SystemTime { source: BoxError },
    Internal { source: BoxError },
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn storage_unavailable(message: impl Into<String>) -> Self {
        Self::StorageUnavailable {
            message: message.into(),
        }
    }

    pub fn database(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Database {
            source: Box::new(error),
        }
    }

    pub fn serialization(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Serialization {
            source: Box::new(error),
        }
    }

    pub fn system_time(error: impl Error + Send + Sync + 'static) -> Self {
        Self::SystemTime {
            source: Box::new(error),
        }
    }

    pub fn internal(error: impl Error + Send + Sync + 'static) -> Self {
        Self::Internal {
            source: Box::new(error),
        }
    }

    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::StorageUnavailable { .. } => "STORAGE_UNAVAILABLE",
            Self::Database { .. } => "DATABASE_ERROR",
            Self::Filesystem { .. } => "FILESYSTEM_ERROR",
            Self::Serialization { .. } => "SERIALIZATION_ERROR",
            Self::SystemTime { .. } => "SYSTEM_TIME_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    pub fn user_message(&self) -> &str {
        match self {
            Self::Validation { message } | Self::StorageUnavailable { message } => message,
            Self::Database { .. } => "The application could not access its database.",
            Self::Filesystem { .. } => "The application could not access the filesystem.",
            Self::Serialization { .. } => "The application encountered invalid stored data.",
            Self::SystemTime { .. } => "The system clock could not be read.",
            Self::Internal { .. } => "An unexpected application error occurred.",
        }
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.user_message())
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database { source }
            | Self::Filesystem { source }
            | Self::Serialization { source }
            | Self::SystemTime { source }
            | Self::Internal { source } => Some(source.as_ref()),
            Self::Validation { .. } | Self::StorageUnavailable { .. } => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", self.user_message())?;
        state.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(source: std::io::Error) -> Self {
        Self::Filesystem {
            source: Box::new(source),
        }
    }
}
