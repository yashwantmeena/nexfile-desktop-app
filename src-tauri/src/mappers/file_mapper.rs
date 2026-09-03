use std::path::Path;

use serde::Deserialize;

use crate::models::file_model::FileTypeCount;
use crate::types::file_type::FileType;

pub fn file_type_from_path(path: impl AsRef<Path>) -> FileType {
    let Some(extension) = path
        .as_ref()
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
    else {
        return FileType::Other;
    };

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "heif"
        | "avif" | "avip" | "svg" | "ico" | "raw" => FileType::Image,
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v" | "mpeg" | "mpg" | "3gp" | "flv" => {
            FileType::Video
        }
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" | "mid"
        | "midi" => FileType::Audio,
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "epub" | "xls" | "xlsx" | "ods" | "csv"
        | "tsv" | "ppt" | "pptx" | "odp" | "key" | "txt" | "md" => FileType::Document,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" => FileType::Archive,
        _ => FileType::Other,
    }
}

pub(crate) fn deserialize_file_type_counts<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<FileTypeCount>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    parse_counts(value).map_err(serde::de::Error::custom)
}

pub(crate) fn parse_counts(value: serde_json::Value) -> Result<Vec<FileTypeCount>, String> {
    let entries =
        serde_json::from_value::<Vec<FileTypeCount>>(value).map_err(|error| error.to_string())?;
    normalize_counts(entries)
}

pub(crate) fn normalize_counts(entries: Vec<FileTypeCount>) -> Result<Vec<FileTypeCount>, String> {
    let mut counts = std::collections::BTreeMap::new();
    for entry in entries {
        if entry.count < 0 {
            return Err("File-type counts cannot be negative.".into());
        }
        if counts.insert(entry.file_type, entry.count).is_some() {
            return Err("Duplicate file type in counts.".into());
        }
    }
    Ok(FileType::ALL
        .into_iter()
        .map(|file_type| FileTypeCount {
            file_type,
            count: counts.remove(&file_type).unwrap_or(0),
        })
        .collect())
}
