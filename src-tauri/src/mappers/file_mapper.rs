use std::path::Path;



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


