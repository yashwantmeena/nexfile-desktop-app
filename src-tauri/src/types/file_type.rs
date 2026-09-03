use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum FileType {
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Other,
}

impl FileType {
    pub const ALL: [Self; 6] = [
        Self::Image,
        Self::Video,
        Self::Audio,
        Self::Document,
        Self::Archive,
        Self::Other,
    ];
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Document => "document",
            Self::Archive => "archive",
            Self::Other => "other",
        }
    }
}
