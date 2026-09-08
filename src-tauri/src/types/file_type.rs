use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
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
}


