use std::path::Path;

use tantivy::directory::MmapDirectory;
use tantivy::schema::{Field, Schema, FAST, INDEXED, STORED, STRING, TEXT};
use tantivy::Index;

use crate::error::{AppError, AppResult};

// Tantivy cannot open an existing index with a different schema. Keep the
// schema generation in the directory name so upgrades create a fresh derived
// index instead of preventing the application from starting.
pub const SEARCH_INDEX_DIRECTORY: &str = "search-index-v2";

#[derive(Clone, Copy, Debug)]
pub struct SearchFields {
    pub file_id: Field,
    pub drive_id: Field,
    pub name: Field,
    pub extension: Field,
    pub tags: Field,
    pub caption: Field,
    pub ocr: Field,
    pub modified_at_ms: Field,
    pub size_bytes: Field,
}

pub struct SearchIndex {
    index: Index,
    fields: SearchFields,
}

impl SearchIndex {
    pub fn open(app_data_dir: impl AsRef<Path>) -> AppResult<Self> {
        let index_path = app_data_dir.as_ref().join(SEARCH_INDEX_DIRECTORY);
        std::fs::create_dir_all(&index_path)?;

        let (schema, fields) = build_schema();
        let directory = MmapDirectory::open(&index_path).map_err(AppError::internal)?;
        let index = Index::open_or_create(directory, schema).map_err(AppError::internal)?;

        Ok(Self { index, fields })
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub const fn fields(&self) -> SearchFields {
        self.fields
    }
}

fn build_schema() -> (Schema, SearchFields) {
    let mut builder = Schema::builder();

    let fields = SearchFields {
        file_id: builder.add_text_field("file_id", STRING | STORED),
        drive_id: builder.add_text_field("drive_id", STRING | STORED),
        name: builder.add_text_field("name", TEXT | STORED),
        extension: builder.add_text_field("extension", STRING | STORED),
        tags: builder.add_text_field("tags", STRING | STORED),
        caption: builder.add_text_field("caption", TEXT | STORED),
        ocr: builder.add_text_field("ocr", TEXT | STORED),
        modified_at_ms: builder.add_i64_field("modified_at_ms", INDEXED | FAST | STORED),
        size_bytes: builder.add_u64_field("size_bytes", INDEXED | FAST | STORED),
    };

    (builder.build(), fields)
}

#[cfg(test)]
#[path = "../../tests/search/search_index.rs"]
mod tests;
