pub(crate) const APALIS_MIGRATION_TABLE: &str = "_apalis_migrations";
pub(crate) const DRIVE_METADATA_FILE: &str = "drive_metadata.json";
pub(crate) const IMAGE_PROCESSING_QUEUE: &str = "image-processing";
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
