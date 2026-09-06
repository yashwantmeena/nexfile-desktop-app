use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::indexer::{IndexWriter, IndexWriterOptions};
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, Query, RegexQuery, TermQuery};
use tantivy::schema::{Field, Schema, FAST, INDEXED, STORED, STRING};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{
    DocSet, Document, Index, IndexReader, ReloadPolicy, TantivyDocument, Term, TERMINATED,
};

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
    name: Option<Field>,
}

#[derive(Clone)]
pub struct TantivyIndexingRepository {
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    fields: IndexFields,
    reader: IndexReader,
}

impl TantivyIndexingRepository {
    pub fn open(app_data_dir: impl AsRef<Path>) -> AppResult<Self> {
        let index_path = app_data_dir.as_ref().join(SEARCH_INDEX_DIRECTORY);
        std::fs::create_dir_all(&index_path)?;

        let (schema, mut fields) = build_schema(true);
        let directory = MmapDirectory::open(&index_path).map_err(AppError::internal)?;
        let index = if Index::exists(&directory).map_err(AppError::internal)? {
            let index = Index::open(directory).map_err(AppError::internal)?;
            if index.schema() != schema {
                let (legacy_schema, legacy_fields) = build_schema(false);
                if index.schema() != legacy_schema {
                    return Err(AppError::validation(
                        "The search index has an unsupported schema.",
                    ));
                }
                fields = legacy_fields;
            }
            index
        } else {
            Index::open_or_create(directory, schema).map_err(AppError::internal)?
        };
        let options = IndexWriterOptions::builder()
            .num_worker_threads(1)
            .num_merge_threads(1)
            .memory_budget_per_thread(INDEX_WRITER_MEMORY_BUDGET_BYTES)
            .build();
        let writer = index
            .writer_with_options::<TantivyDocument>(options)
            .map_err(AppError::internal)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(AppError::internal)?;
        Ok(Self {
            index,
            writer: Arc::new(Mutex::new(writer)),
            fields,
            reader,
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
        if let Some(field) = self.fields.name {
            let name = if source.name.is_empty() {
                self.existing_document(&source.drive_id, &source.file_id)?
                    .and_then(|doc| {
                        doc.get_first(field)
                            .and_then(|value| value.as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or_default()
            } else {
                source.name.trim().to_lowercase()
            };
            if !name.is_empty() {
                document.add_text(field, name);
            }
        }
        writer
            .delete_query(Box::new(
                self.identity_query(&source.drive_id, &source.file_id),
            ))
            .map_err(AppError::internal)?;
        writer.add_document(document).map_err(AppError::internal)?;
        writer.commit().map_err(AppError::internal)?;
        self.reader.reload().map_err(AppError::internal)?;
        Ok(())
    }

    /// Seek directly into tag dictionaries, with a shared work budget across segments.
    /// Results are alphabetical within the bounded candidate set, not frequency-ranked.
    pub fn suggest_tags(&self, prefix: &str) -> AppResult<Vec<String>> {
        if prefix.len() > 256 {
            return Ok(Vec::new());
        }
        let prefix = prefix
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if prefix.chars().count() < 2 {
            return Ok(Vec::new());
        }
        let searcher = self.reader.searcher();
        let mut suggestions = BTreeSet::new();
        let mut budget = 512usize;
        'segments: for segment in searcher.segment_readers() {
            if segment.num_docs() == 0 {
                continue;
            }
            for field in [
                self.fields.search_keywords,
                self.fields.object_labels,
                self.fields.secondary_labels,
                self.fields.categories,
            ] {
                let inverted = segment.inverted_index(field).map_err(AppError::internal)?;
                let mut stream = inverted
                    .terms()
                    .range()
                    .ge(prefix.as_bytes())
                    .into_stream()
                    .map_err(AppError::internal)?;
                while stream.advance() {
                    if budget == 0 {
                        break 'segments;
                    }
                    budget -= 1;
                    if !stream.key().starts_with(prefix.as_bytes()) {
                        break;
                    }
                    let Ok(value) = std::str::from_utf8(stream.key()) else {
                        continue;
                    };
                    if suggestions.contains(value) {
                        continue;
                    }
                    let term = Term::from_field_text(field, value);
                    if let Some(mut postings) = inverted
                        .read_postings(&term, tantivy::schema::IndexRecordOption::Basic)
                        .map_err(AppError::internal)?
                    {
                        while postings.doc() != TERMINATED {
                            if budget == 0 {
                                break 'segments;
                            }
                            budget -= 1;
                            if !segment.is_deleted(postings.doc()) {
                                suggestions.insert(value.to_owned());
                                break;
                            }
                            postings.advance();
                        }
                    }
                }
            }
        }
        Ok(suggestions.into_iter().take(8).collect())
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn index_filename(&self, drive: &str, file: &str, name: &str) -> AppResult<()> {
        // Keep legacy indexes untouched until the user explicitly recreates them.
        let Some(name_field) = self.fields.name else {
            return Ok(());
        };
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Search index writer lock is poisoned.",
            ))
        })?;
        let mut document = TantivyDocument::default();
        if let Some(existing) = self.existing_document(drive, file)? {
            for (field, value) in existing.iter_fields_and_values() {
                if field != name_field {
                    document.add_field_value(field, value);
                }
            }
        } else {
            document.add_text(self.fields.drive_id, drive);
            document.add_text(self.fields.file_id, file);
        }
        document.add_text(name_field, name.trim().to_lowercase());
        writer
            .delete_query(Box::new(self.identity_query(drive, file)))
            .map_err(AppError::internal)?;
        writer.add_document(document).map_err(AppError::internal)?;
        writer.commit().map_err(AppError::internal)?;
        self.reader.reload().map_err(AppError::internal)
    }

    fn identity_query(&self, drive: &str, file: &str) -> BooleanQuery {
        BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.drive_id, drive),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.file_id, file),
                    IndexRecordOption::Basic,
                )),
            ),
        ])
    }

    fn existing_document(&self, drive: &str, file: &str) -> AppResult<Option<TantivyDocument>> {
        let searcher = self.reader.searcher();
        let hits = searcher
            .search(
                &self.identity_query(drive, file),
                &TopDocs::with_limit(1).order_by_score(),
            )
            .map_err(AppError::internal)?;
        hits.first()
            .map(|(_, address)| searcher.doc(*address).map_err(AppError::internal))
            .transpose()
    }

    fn search_names(&self, query: &str) -> AppResult<HashSet<(String, String)>> {
        if query.len() > 256 {
            return Err(AppError::validation("Search text is too long."));
        }
        let field = self.fields.name.ok_or_else(|| AppError::validation(
            "Name search requires a search index created with the name field. The existing index has not been rebuilt."))?;
        let mut pattern = String::from(".*");
        for character in query.trim().to_lowercase().chars() {
            if ".+*?()|[]{}^$\\".contains(character) {
                pattern.push('\\');
            }
            pattern.push(character);
        }
        pattern.push_str(".*");
        self.matching_files(&RegexQuery::from_pattern(&pattern, field).map_err(AppError::internal)?)
    }

    pub fn search_files(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
    ) -> AppResult<HashSet<(String, String)>> {
        match mode {
            "tags" => self.search_tags(query, tags),
            "name" => {
                if query.trim().is_empty() {
                    return self.search_tags("", tags);
                }
                let mut files = self.search_names(query)?;
                if !tags.is_empty() {
                    let tagged = self.search_tags("", tags)?;
                    files.retain(|file| tagged.contains(file));
                }
                Ok(files)
            }
            _ => Err(AppError::validation("Unknown search mode.")),
        }
    }

    pub fn search_tags(
        &self,
        query: &str,
        tags: &[String],
    ) -> AppResult<HashSet<(String, String)>> {
        if query.len() > 256 || tags.len() > 32 || tags.iter().any(|tag| tag.len() > 256) {
            return Err(AppError::validation(
                "Search text or tag filters exceed the supported limit.",
            ));
        }
        let fields = [
            self.fields.search_keywords,
            self.fields.object_labels,
            self.fields.secondary_labels,
            self.fields.categories,
        ];
        let normalize = |value: &str| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        let query = normalize(query);
        if !query.is_empty() {
            clauses.push((
                Occur::Must,
                Box::new(BooleanQuery::new(
                    fields
                        .iter()
                        .map(|&field| {
                            (
                                Occur::Should,
                                Box::new(FuzzyTermQuery::new_prefix(
                                    Term::from_field_text(field, &query),
                                    0,
                                    false,
                                )) as Box<dyn Query>,
                            )
                        })
                        .collect(),
                )),
            ));
        }
        for tag in tags {
            let tag = normalize(tag);
            if tag.is_empty() {
                continue;
            }
            clauses.push((
                Occur::Must,
                Box::new(BooleanQuery::new(
                    fields
                        .iter()
                        .map(|&field| {
                            (
                                Occur::Should,
                                Box::new(TermQuery::new(
                                    Term::from_field_text(field, &tag),
                                    IndexRecordOption::Basic,
                                )) as Box<dyn Query>,
                            )
                        })
                        .collect(),
                )),
            ));
        }
        if clauses.is_empty() {
            return Ok(HashSet::new());
        }
        self.matching_files(&BooleanQuery::new(clauses))
    }

    fn matching_files(&self, query: &dyn Query) -> AppResult<HashSet<(String, String)>> {
        let searcher = self.reader.searcher();
        let matches = searcher
            .search(query, &DocSetCollector)
            .map_err(AppError::internal)?;
        let mut files = HashSet::with_capacity(matches.len());
        for address in matches {
            let doc: TantivyDocument = searcher.doc(address).map_err(AppError::internal)?;
            if let (Some(drive), Some(file)) = (
                doc.get_first(self.fields.drive_id)
                    .and_then(|value| value.as_str()),
                doc.get_first(self.fields.file_id)
                    .and_then(|value| value.as_str()),
            ) {
                files.insert((drive.to_owned(), file.to_owned()));
            }
        }
        Ok(files)
    }
}

fn build_schema(include_name: bool) -> (Schema, IndexFields) {
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
        name: include_name.then(|| builder.add_text_field("name", STRING | STORED)),
    };
    (builder.build(), fields)
}
