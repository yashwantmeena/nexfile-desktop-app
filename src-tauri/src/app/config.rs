use std::path::PathBuf;

use tauri::Manager;

use crate::error::{AppError, AppResult};

pub struct AppConfig {
    pub app_data_dir: PathBuf,
    pub database_path: PathBuf,
}

impl AppConfig {
    pub fn resolve<R: tauri::Runtime>(app: &tauri::App<R>) -> AppResult<Self> {
        let app_data_dir = app.path().app_data_dir().map_err(AppError::internal)?;

        let database_path = app_data_dir.join("nexfile.redb");

        Ok(Self {
            app_data_dir,
            database_path,
        })
    }
}
