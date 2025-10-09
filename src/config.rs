use std::{env, fs, path::PathBuf};

use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

pub fn get_data_dir() -> PathBuf {
    if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    }
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "malgra", env!("CARGO_PKG_NAME"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeekbarType {
    Waveform,
    Slider,
}

impl Default for SeekbarType {
    fn default() -> Self {
        Self::Waveform
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub seekbar_type: SeekbarType,
}

pub fn get_settings_path() -> PathBuf {
    get_data_dir().join("app.toml")
}

pub fn load_settings() -> AppSettings {
    let settings_path = get_settings_path();

    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(settings) => settings,
                Err(e) => {
                    eprintln!("Failed to parse settings file: {}. Using defaults.", e);
                    AppSettings::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to read settings file: {}. Using defaults.", e);
                AppSettings::default()
            }
        }
    } else {
        let settings = AppSettings::default();
        let _ = save_settings(&settings);
        settings
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = get_settings_path();

    // Ensure the data directory exists
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml_string = toml::to_string_pretty(settings)?;
    fs::write(&settings_path, toml_string)?;

    Ok(())
}
