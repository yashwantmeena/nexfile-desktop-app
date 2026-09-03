use std::path::Path;
use std::sync::{Arc, Mutex};

use tantivy::directory::MmapDirectory;
use tantivy::indexer::{IndexWriter, IndexWriterOptions};
use tantivy::schema::{Field, Schema, FAST, INDEXED, STORED, STRING};
use tantivy::{Index, TantivyDocument, Term};

use crate::error::{AppError, AppResult};
use crate::models::indexing_model::IndexDocument;
use crate::utils::constants::{INDEX_WRITER_MEMORY_BUDGET_BYTES, SEARCH_INDEX_DIRECTORY};

#[derive(Clone, Copy, Debug)]
struct IndexFields {
    file_id: Field,
    drive_id: Field,
    created_at_ms: Field,
    updated_at_ms: Field,
    media_type: Field,
    size_bytes: Field,
    latitude: Field,
    longitude: Field,
    object_labels: Field,
    search_keywords: Field,
    secondary_labels: Field,
    categories: Field,
}

#[derive(Clone)]
pub struct TantivyIndexingRepository {
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    fields: IndexFields,
}

impl TantivyIndexingRepository {
    pub fn open(app_data_dir: impl AsRef<Path>) -> AppResult<Self> {
        let index_path = app_data_dir.as_ref().join(SEARCH_INDEX_DIRECTORY);
        std::fs::create_dir_all(&index_path)?;

        let (schema, fields) = build_schema();
        let directory = MmapDirectory::open(&index_path).map_err(AppError::internal)?;
        let index = Index::open_or_create(directory, schema).map_err(AppError::internal)?;
        let options = IndexWriterOptions::builder()
            .num_worker_threads(1)
            .num_merge_threads(1)
            .memory_budget_per_thread(INDEX_WRITER_MEMORY_BUDGET_BYTES)
            .build();
        let writer = index
            .writer_with_options::<TantivyDocument>(options)
            .map_err(AppError::internal)?;

        Ok(Self {
            index,
            writer: Arc::new(Mutex::new(writer)),
            fields,
        })
    }

    pub fn upsert(&self, source: IndexDocument) -> AppResult<()> {
        let mut document = TantivyDocument::default();
        document.add_text(self.fields.file_id, &source.file_id);
        document.add_text(self.fields.drive_id, &source.drive_id);
        document.add_u64(self.fields.created_at_ms, source.created_at_ms);
        document.add_u64(self.fields.updated_at_ms, source.updated_at_ms);
        if let Some(media_type) = source.media_type {
            document.add_text(self.fields.media_type, media_type);
        }
        document.add_u64(self.fields.size_bytes, source.size_bytes);
        if let Some(latitude) = source.latitude {
            document.add_f64(self.fields.latitude, latitude);
        }
        if let Some(longitude) = source.longitude {
            document.add_f64(self.fields.longitude, longitude);
        }
        for label in source.object_labels {
            document.add_text(self.fields.object_labels, label);
        }
        for keyword in source.search_keywords {
            document.add_text(self.fields.search_keywords, keyword);
        }
        for label in source.secondary_labels {
            document.add_text(self.fields.secondary_labels, label);
        }
        for category in source.categories {
            document.add_text(self.fields.categories, category);
        }

        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Tantivy index writer lock is poisoned.",
            ))
        })?;
        writer.delete_term(Term::from_field_text(self.fields.file_id, &source.file_id));
        writer.add_document(document).map_err(AppError::internal)?;
        writer.commit().map_err(AppError::internal)?;
        Ok(())
    }

    pub fn index(&self) -> &Index {
        &self.index
    }
}

fn build_schema() -> (Schema, IndexFields) {
    let mut builder = Schema::builder();
    let fields = IndexFields {
        file_id: builder.add_text_field("file_id", STRING | STORED),
        drive_id: builder.add_text_field("drive_id", STRING | STORED),
        created_at_ms: builder.add_u64_field("created_at_ms", INDEXED | FAST | STORED),
        updated_at_ms: builder.add_u64_field("updated_at_ms", INDEXED | FAST | STORED),
        media_type: builder.add_text_field("media_type", STRING | STORED),
        size_bytes: builder.add_u64_field("size_bytes", INDEXED | FAST | STORED),
        latitude: builder.add_f64_field("latitude", INDEXED | FAST | STORED),
        longitude: builder.add_f64_field("longitude", INDEXED | FAST | STORED),
        object_labels: builder.add_text_field("object_labels", STRING | STORED),
        search_keywords: builder.add_text_field("search_keywords", STRING | STORED),
        secondary_labels: builder.add_text_field("secondary_labels", STRING | STORED),
        categories: builder.add_text_field("categories", STRING | STORED),
    };
    (builder.build(), fields)
}
