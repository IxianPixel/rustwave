use std::time::Duration;
use std::path::PathBuf;

use iced::widget::{image, image::Handle, container, column, row};
use iced::widget::{button, mouse_area, svg, text, MouseArea, Row, Svg};
use iced::Color;
use crate::models::SoundCloudUser;
use crate::{models::SoundCloudTrack, page_b, Message};

pub trait DurationFormat {
    fn format_as_mmss(&self) -> String;
}

impl DurationFormat for Duration {
    fn format_as_mmss(&self) -> String {
        let total_seconds = self.as_secs();
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;

        format!("{:02}:{:02}", minutes, seconds)
    }
}

pub async fn download_image(url: &str) -> Result<Handle, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    Ok(Handle::from_bytes(bytes))
}

pub fn get_track_queue(track_id: u64, tracks: Vec<SoundCloudTrack>) -> Vec<SoundCloudTrack> {
    // We own `tracks`, so we can split it efficiently without extra allocations.
    let mut tracks = tracks;
    if let Some(pos) = tracks.iter().position(|t| t.id == track_id) {
        // Keep from `pos` to the end (inclusive of the found track)
        tracks.split_off(pos)
    } else {
        // If the track is not found, return an empty queue
        Vec::new()
    }
}

pub trait NumberFormat {
    fn format_compact_number(&self) -> String;
}

macro_rules! impl_number_format {
    ($($t:ty),*) => {
        $(
            impl NumberFormat for $t {
                fn format_compact_number(&self) -> String {
                    let num = *self as u64;
                    match num {
                        n if n < 1_000 => n.to_string(),
                        n if n < 1_000_000 => {
                            let val = n as f64 / 1_000.0;
                            if val.fract() == 0.0 {
                                format!("{}K", val as u64)
                            } else {
                                let formatted = format!("{:.1}", val).trim_end_matches('0').trim_end_matches('.').to_string();
                                format!("{}K", formatted)
                            }
                        }
                        n if n < 1_000_000_000 => {
                            let val = n as f64 / 1_000_000.0;
                            if val.fract() == 0.0 {
                                format!("{}M", val as u64)
                            } else {
                                let formatted = format!("{:.1}", val).trim_end_matches('0').trim_end_matches('.').to_string();
                                format!("{}M", formatted)
                            }
                        }
                        n => {
                            let val = n as f64 / 1_000_000_000.0;
                            if val.fract() == 0.0 {
                                format!("{}B", val as u64)
                            } else {
                                let formatted = format!("{:.1}", val).trim_end_matches('0').trim_end_matches('.').to_string();
                                format!("{}B", formatted)
                            }
                        }
                    }
                }
            }
        )*
    };
}

impl_number_format!(u32, u64);

/// Get the path to an asset file relative to the executable location
/// This works both in development (cargo run) and in the app bundle
pub fn get_asset_path(relative_path: &str) -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let asset_path = exe_dir.join(relative_path);
            if asset_path.exists() {
                return asset_path.to_string_lossy().to_string();
            }
        }
    }

    // Fallback to relative path for development
    relative_path.to_string()
}