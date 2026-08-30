use std::path::PathBuf;

use tauri::Manager;

use crate::error::{AppError, AppResult};

pub struct AppConfig {
    pub app_data_dir: PathBuf,
    pub database_path: PathBuf,
    pub resources_dir: PathBuf,
}

impl AppConfig {
    pub fn resolve<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<Self> {
        let app_data_dir = app.path().app_data_dir().map_err(AppError::internal)?;

        let database_path = app_data_dir.join("nexfile.sqlite3");
        let bundled_resources = app
            .path()
            .resource_dir()
            .map_err(AppError::internal)?
            .join("resources");
        let development_resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        let resources_dir = if cfg!(debug_assertions) {
            development_resources
        } else if bundled_resources.is_dir() {
            bundled_resources
        } else {
            development_resources
        };

        Ok(Self {
            app_data_dir,
            database_path,
            resources_dir,
        })
    }
}
