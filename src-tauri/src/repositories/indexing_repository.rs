use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::indexer::{IndexWriter, IndexWriterOptions};
use tantivy::query::{
    AllQuery, BooleanQuery, EmptyQuery, FuzzyTermQuery, Occur, Query, RegexQuery, TermQuery,
};
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
    collection_ids: Field,
    favorite: Field,
    name: Field,
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
        let previous_index_path = app_data_dir.as_ref().join("search-index-v3");
        std::fs::create_dir_all(&index_path)?;

        let (schema, fields) = build_schema();
        let directory = MmapDirectory::open(&index_path).map_err(AppError::internal)?;
        let index_existed = Index::exists(&directory).map_err(AppError::internal)?;
        let index = if index_existed {
            let index = Index::open(directory).map_err(AppError::internal)?;
            if index.schema() != schema {
                return Err(AppError::validation(
                    "The search index has an unsupported schema.",
                ));
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
        let repository = Self {
            index,
            writer: Arc::new(Mutex::new(writer)),
            fields,
            reader,
        };
        if !index_existed && previous_index_path != index_path && previous_index_path.is_dir() {
            repository.migrate_previous_index(&previous_index_path)?;
        }
        Ok(repository)
    }

    fn migrate_previous_index(&self, path: &Path) -> AppResult<()> {
        let directory = MmapDirectory::open(path).map_err(AppError::internal)?;
        if !Index::exists(&directory).map_err(AppError::internal)? {
            return Ok(());
        }
        let previous = Index::open(directory).map_err(AppError::internal)?;
        let previous_schema = previous.schema();
        let reader = previous.reader().map_err(AppError::internal)?;
        let searcher = reader.searcher();
        let addresses = searcher
            .search(&AllQuery, &DocSetCollector)
            .map_err(AppError::internal)?;
        if addresses.is_empty() {
            return Ok(());
        }
        let current_schema = self.index.schema();
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Search index writer lock is poisoned.",
            ))
        })?;
        for address in addresses {
            let previous_document: TantivyDocument =
                searcher.doc(address).map_err(AppError::internal)?;
            let mut document = TantivyDocument::default();
            for (field, value) in previous_document.iter_fields_and_values() {
                let name = previous_schema.get_field_name(field);
                if name == "favorite" {
                    continue;
                }
                if let Ok(target) = current_schema.get_field(name) {
                    document.add_field_value(target, value);
                }
            }
            document.add_bool(self.fields.favorite, false);
            writer.add_document(document).map_err(AppError::internal)?;
        }
        writer.commit().map_err(AppError::internal)?;
        self.reader.reload().map_err(AppError::internal)
    }

    pub fn upsert(&self, source: IndexDocument) -> AppResult<()> {
        self.upsert_batch(vec![source])
    }

    /// Replaces each file's existing identity entry before adding its current document, then
    /// commits the bounded batch once. This keeps indexing idempotent across drive mounts.
    pub fn upsert_batch(&self, sources: Vec<IndexDocument>) -> AppResult<()> {
        if sources.is_empty() {
            return Ok(());
        }
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Tantivy index writer lock is poisoned.",
            ))
        })?;
        for source in sources {
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
            for collection_id in source.collection_ids {
                if !collection_id.is_empty() {
                    document.add_text(self.fields.collection_ids, collection_id);
                }
            }
            document.add_bool(self.fields.favorite, source.favorite);
            let name = if source.name.is_empty() {
                self.existing_document(&source.drive_id, &source.file_id)?
                    .and_then(|doc| {
                        doc.get_first(self.fields.name)
                            .and_then(|value| value.as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or_default()
            } else {
                source.name.trim().to_lowercase()
            };
            if !name.is_empty() {
                document.add_text(self.fields.name, name);
            }
            writer
                .delete_query(Box::new(
                    self.identity_query(&source.drive_id, &source.file_id),
                ))
                .map_err(AppError::internal)?;
            writer.add_document(document).map_err(AppError::internal)?;
        }
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

    pub fn index_filename(
        &self,
        drive: &str,
        file: &str,
        name: &str,
        collection_ids: &[String],
        favorite: bool,
    ) -> AppResult<()> {
        self.index_metadata(drive, file, name, collection_ids, None, favorite)
    }

    pub fn index_file_metadata(
        &self,
        drive: &str,
        file: &str,
        name: &str,
        collection_ids: &[String],
        search_keywords: &[String],
        favorite: bool,
    ) -> AppResult<()> {
        self.index_metadata(
            drive,
            file,
            name,
            collection_ids,
            Some(search_keywords),
            favorite,
        )
    }

    fn index_metadata(
        &self,
        drive: &str,
        file: &str,
        name: &str,
        collection_ids: &[String],
        search_keywords: Option<&[String]>,
        favorite: bool,
    ) -> AppResult<()> {
        let name_field = self.fields.name;
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Search index writer lock is poisoned.",
            ))
        })?;
        let mut document = TantivyDocument::default();
        if let Some(existing) = self.existing_document(drive, file)? {
            for (field, value) in existing.iter_fields_and_values() {
                if field != name_field
                    && field != self.fields.collection_ids
                    && field != self.fields.favorite
                    && (search_keywords.is_none() || field != self.fields.search_keywords)
                {
                    document.add_field_value(field, value);
                }
            }
        } else {
            document.add_text(self.fields.drive_id, drive);
            document.add_text(self.fields.file_id, file);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(AppError::system_time)?
                .as_millis() as u64;
            document.add_u64(self.fields.created_at_ms, now);
            document.add_u64(self.fields.updated_at_ms, now);
        }
        document.add_text(name_field, name.trim().to_lowercase());
        for collection_id in collection_ids {
            if !collection_id.is_empty() {
                document.add_text(self.fields.collection_ids, collection_id);
            }
        }
        if let Some(search_keywords) = search_keywords {
            for keyword in search_keywords {
                let keyword = keyword.trim().to_lowercase();
                if !keyword.is_empty() {
                    document.add_text(self.fields.search_keywords, keyword);
                }
            }
        }
        document.add_bool(self.fields.favorite, favorite);
        writer
            .delete_query(Box::new(self.identity_query(drive, file)))
            .map_err(AppError::internal)?;
        writer.add_document(document).map_err(AppError::internal)?;
        writer.commit().map_err(AppError::internal)?;
        self.reader.reload().map_err(AppError::internal)
    }

    pub fn delete_file(&self, drive: &str, file: &str) -> AppResult<()> {
        let file_ids = [file.to_owned()];
        self.delete_files(drive, &file_ids)
    }

    /// Deletes a bounded group of documents for one drive with a single commit and reload.
    pub fn delete_files(&self, drive: &str, file_ids: &[String]) -> AppResult<()> {
        if file_ids.is_empty() {
            return Ok(());
        }
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Search index writer lock is poisoned.",
            ))
        })?;
        for file_id in file_ids {
            writer
                .delete_query(Box::new(self.identity_query(drive, file_id)))
                .map_err(AppError::internal)?;
        }
        writer.commit().map_err(AppError::internal)?;
        self.reader.reload().map_err(AppError::internal)
    }

    /// Returns stable file IDs that can be split into durable deletion batches.
    pub fn file_ids_for_drive(&self, drive: &str) -> AppResult<Vec<String>> {
        let query = TermQuery::new(
            Term::from_field_text(self.fields.drive_id, drive),
            IndexRecordOption::Basic,
        );
        let searcher = self.reader.searcher();
        let hits = searcher
            .search(&query, &DocSetCollector)
            .map_err(AppError::internal)?;
        let mut file_ids = HashSet::new();
        for address in hits {
            let document: TantivyDocument = searcher.doc(address).map_err(AppError::internal)?;
            if let Some(file_id) = document
                .get_first(self.fields.file_id)
                .and_then(|value| value.as_str())
            {
                file_ids.insert(file_id.to_owned());
            }
        }
        let mut file_ids = file_ids.into_iter().collect::<Vec<_>>();
        file_ids.sort();
        Ok(file_ids)
    }

    /// Removes every indexed document belonging to a drive that was removed from NexFile.
    pub fn delete_drive(&self, drive: &str) -> AppResult<()> {
        let mut writer = self.writer.lock().map_err(|_| {
            AppError::internal(std::io::Error::other(
                "Search index writer lock is poisoned.",
            ))
        })?;
        writer
            .delete_query(Box::new(TermQuery::new(
                Term::from_field_text(self.fields.drive_id, drive),
                IndexRecordOption::Basic,
            )))
            .map_err(AppError::internal)?;
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

    fn name_query(&self, query: &str) -> AppResult<Box<dyn Query>> {
        if query.len() > 256 {
            return Err(AppError::validation("Search text is too long."));
        }
        let field = self.fields.name;
        let mut pattern = String::from(".*");
        for character in query.trim().to_lowercase().chars() {
            if ".+*?()|[]{}^$\\".contains(character) {
                pattern.push('\\');
            }
            pattern.push(character);
        }
        pattern.push_str(".*");
        Ok(Box::new(
            RegexQuery::from_pattern(&pattern, field).map_err(AppError::internal)?,
        ))
    }

    pub fn search_files(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
        collection_ids: Option<&[(String, String)]>,
    ) -> AppResult<HashSet<(String, String)>> {
        self.matching_files(
            self.filtered_query(query, mode, tags, collection_ids, false, None)?
                .as_ref(),
        )
    }

    fn filtered_query(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
        collection_ids: Option<&[(String, String)]>,
        favorite_only: bool,
        model_category: Option<&str>,
    ) -> AppResult<Box<dyn Query>> {
        let base = match mode {
            "tags" => self.tag_query(query, tags)?,
            "name" if query.trim().is_empty() => self.tag_query("", tags)?,
            "name" => Box::new(BooleanQuery::new(vec![
                (Occur::Must, self.name_query(query)?),
                (Occur::Must, self.tag_query("", tags)?),
            ])),
            _ => return Err(AppError::validation("Unknown search mode.")),
        };
        let collection = self.collection_query(collection_ids)?;
        let model_category = model_category
            .map(str::trim)
            .filter(|category| !category.is_empty())
            .map(|category| category.to_lowercase());
        if collection.is_none() && !favorite_only && model_category.is_none() {
            return Ok(base);
        }
        let mut filters = vec![(Occur::Must, base)];
        if let Some(collection) = collection {
            filters.push((Occur::Must, collection));
        }
        if favorite_only {
            filters.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_bool(self.fields.favorite, true),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        if let Some(category) = model_category {
            filters.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.categories, &category),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        Ok(Box::new(BooleanQuery::new(filters)))
    }

    fn collection_query(
        &self,
        collection_ids: Option<&[(String, String)]>,
    ) -> AppResult<Option<Box<dyn Query>>> {
        let Some(collection_ids) = collection_ids else {
            return Ok(None);
        };
        let field = self.fields.collection_ids;
        let matches = collection_ids
            .iter()
            .filter(|(drive_id, collection_id)| !drive_id.is_empty() && !collection_id.is_empty())
            .map(|(drive_id, collection_id)| {
                (
                    Occur::Should,
                    Box::new(BooleanQuery::new(vec![
                        (
                            Occur::Must,
                            Box::new(TermQuery::new(
                                Term::from_field_text(self.fields.drive_id, drive_id),
                                IndexRecordOption::Basic,
                            )) as Box<dyn Query>,
                        ),
                        (
                            Occur::Must,
                            Box::new(TermQuery::new(
                                Term::from_field_text(field, collection_id),
                                IndexRecordOption::Basic,
                            )) as Box<dyn Query>,
                        ),
                    ])) as Box<dyn Query>,
                )
            })
            .collect::<Vec<_>>();
        Ok(Some(if matches.is_empty() {
            Box::new(EmptyQuery)
        } else {
            Box::new(BooleanQuery::new(matches))
        }))
    }

    pub fn indexed_results(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
        collection_ids: Option<&[(String, String)]>,
    ) -> AppResult<(
        HashSet<(String, String)>,
        crate::models::file_model::FileCountSummary,
    )> {
        self.indexed_results_filtered(query, mode, tags, collection_ids, false)
    }

    pub fn indexed_results_filtered(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
        collection_ids: Option<&[(String, String)]>,
        favorite_only: bool,
    ) -> AppResult<(
        HashSet<(String, String)>,
        crate::models::file_model::FileCountSummary,
    )> {
        self.indexed_results_filtered_with_category(
            query,
            mode,
            tags,
            collection_ids,
            favorite_only,
            None,
        )
    }

    pub fn indexed_results_filtered_with_category(
        &self,
        query: &str,
        mode: &str,
        tags: &[String],
        collection_ids: Option<&[(String, String)]>,
        favorite_only: bool,
        model_category: Option<&str>,
    ) -> AppResult<(
        HashSet<(String, String)>,
        crate::models::file_model::FileCountSummary,
    )> {
        use crate::models::file_model::{FileCountSummary, FileTypeCount};
        use crate::types::file_type::FileType;
        let query = self.filtered_query(
            query,
            mode,
            tags,
            collection_ids,
            favorite_only,
            model_category,
        )?;
        let searcher = self.reader.searcher();
        let hits = searcher
            .search(query.as_ref(), &DocSetCollector)
            .map_err(AppError::internal)?;
        let mut files = HashSet::new();
        let mut counts = FileType::ALL.map(|file_type| FileTypeCount {
            file_type,
            count: 0,
        });
        for address in hits {
            let doc: TantivyDocument = searcher.doc(address).map_err(AppError::internal)?;
            let text = |field| doc.get_first(field).and_then(|value| value.as_str());
            let (Some(drive), Some(file)) = (text(self.fields.drive_id), text(self.fields.file_id))
            else {
                continue;
            };
            if !files.insert((drive.to_owned(), file.to_owned())) {
                continue;
            }
            let media = text(self.fields.media_type).unwrap_or_default();
            let file_type = match media.split('/').next().unwrap_or_default() {
                "image" => FileType::Image,
                "video" => FileType::Video,
                "audio" => FileType::Audio,
                "document" => FileType::Document,
                "archive" => FileType::Archive,
                _ => text(self.fields.name)
                    .map(crate::mappers::file_mapper::file_type_from_path)
                    .unwrap_or(FileType::Other),
            };
            counts
                .iter_mut()
                .find(|count| count.file_type == file_type)
                .unwrap()
                .count += 1;
        }
        let total_count = Some(counts.iter().map(|entry| entry.count).sum());
        Ok((
            files,
            FileCountSummary {
                counts: Some(counts.to_vec()),
                total_count,
                issues: Vec::new(),
            },
        ))
    }

    pub fn search_tags(
        &self,
        query: &str,
        tags: &[String],
    ) -> AppResult<HashSet<(String, String)>> {
        self.matching_files(self.tag_query(query, tags)?.as_ref())
    }

    fn tag_query(&self, query: &str, tags: &[String]) -> AppResult<Box<dyn Query>> {
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
            return Ok(Box::new(AllQuery));
        }
        Ok(Box::new(BooleanQuery::new(clauses)))
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
        collection_ids: builder.add_text_field("collection_ids", STRING | STORED),
        favorite: builder.add_bool_field("favorite", INDEXED | FAST | STORED),
        name: builder.add_text_field("name", STRING | STORED),
    };
    (builder.build(), fields)
}
