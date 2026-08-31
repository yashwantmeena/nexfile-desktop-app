pub(crate) const APALIS_MIGRATION_TABLE: &str = "_apalis_migrations";
pub(crate) const DRIVE_METADATA_FILE: &str = "drive_metadata.json";
pub(crate) const AI_CONFIGS_DIRECTORY: &str = "ai-configs";
pub(crate) const AI_MODELS_DIRECTORY: &str = "ai-models";
pub(crate) const CLIP_MODEL_DIRECTORY: &str = "clip-vit-base-patch32";
pub(crate) const FLORENCE2_MODEL_DIRECTORY: &str = "florence-2-base-ft";
pub(crate) const CLIP_LOGIT_SCALE: f32 = 100.0;
pub(crate) const MAX_MODEL_IMAGE_DIMENSION: u32 = 2048;
pub(crate) const MODEL_IMAGE_TEMP_DIRECTORY: &str = "nexfile-image-processing";
pub(crate) const MODEL_JPEG_QUALITY: u8 = 90;
pub(crate) const IMAGE_PROCESSING_QUEUE: &str = "image-processing";
pub(crate) const IMAGE_PROCESSING_WORKER: &str = "image-processing-worker";
pub(crate) const IMAGE_PROCESSING_OUTPUT_VERSION: u32 = 5;
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
pub(crate) const IMPORT_FILE_QUEUE: &str = "import_file";
pub(crate) const IMPORT_FILE_WORKER: &str = "import_file_worker";
pub(crate) const NEXFILE_DIRECTORY: &str = "nexfile";
