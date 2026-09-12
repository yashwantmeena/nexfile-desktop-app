pub(crate) const APALIS_MIGRATION_TABLE: &str = "_apalis_migrations";
pub(crate) const CATEGORY_TAG_PREFIX: &str = "ct::";
pub(crate) const DRIVE_METADATA_FILE: &str = "drive_metadata.json";
pub(crate) const AI_CONFIGS_DIRECTORY: &str = "ai-configs";
pub(crate) const AI_MODELS_DIRECTORY: &str = "ai-models";
pub(crate) const CLIP_MODEL_DIRECTORY: &str = "clip-vit-base-patch32";
pub(crate) const FLORENCE2_MODEL_DIRECTORY: &str = "florence-2-base-ft";
pub(crate) const CLIP_LOGIT_SCALE: f32 = 100.0;
pub(crate) const MAX_MODEL_IMAGE_DIMENSION: u32 = 2048;
pub(crate) const MODEL_IMAGE_TEMP_DIRECTORY: &str = "nexfile-image-processing";
pub(crate) const MODEL_JPEG_QUALITY: u8 = 90;
pub(crate) const AI_PROCESSING_QUEUE: &str = "ai_processing";
pub(crate) const IMAGE_PROCESSING_PROCESS_TYPE: &str = "image_processing";
pub(crate) const AI_PROCESSING_WORKER: &str = "ai_processing_worker";
pub(crate) const IMAGE_PROCESSING_OUTPUT_VERSION: u32 = 25;
pub(crate) const INDEXING_QUEUE: &str = "indexing";
pub(crate) const INDEXING_PROCESS_TYPE: &str = "indexing";
pub(crate) const DELETE_INDEX_PROCESS_TYPE: &str = "delete_index";
pub(crate) const INDEXING_WORKER: &str = "indexing-worker";
pub(crate) const INDEX_BATCH_SIZE: usize = 100;
pub(crate) const INDEX_WRITER_MEMORY_BUDGET_BYTES: usize = 32_000_000;
pub(crate) const OCR_LABEL: &str = "ocr";
pub(crate) const OBJECT_TAG_PREFIX: &str = "ob::";
pub(crate) const SEARCH_TAG_PREFIX: &str = "tg::";
pub(crate) const SEARCH_INDEX_DIRECTORY: &str = "search-index";
pub(crate) const VISUAL_LABEL: &str = "visual";
pub(crate) const MAX_KEYWORD_CANDIDATES: usize = 64;
pub(crate) const MAX_SEARCH_TAGS: usize = 10;
pub(crate) const MAX_TAG_WORDS: usize = 1;
pub(crate) const IMPORTED_FILES_DIRECTORY: &str = "files";
pub(crate) const IMPORT_FOLDER_PROCESS_TYPE: &str = "import_folder";
pub(crate) const IMPORT_FILE_ID_ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];
pub(crate) const IMPORT_FILE_ID_LENGTH: usize = 14;
pub(crate) const IMPORT_FILE_PROCESS_TYPE: &str = "import_file";
pub(crate) const FILE_PROCESSING_QUEUE: &str = "file_processing";
pub(crate) const DELETE_FILE_PROCESS_TYPE: &str = "delete_file";
pub(crate) const FILE_PROCESSING_WORKER: &str = "file_processing_worker";
pub(crate) const NEXFILE_DIRECTORY: &str = "nexfile";
pub(crate) const COLLECTION_ID_LENGTH: usize = 14;
