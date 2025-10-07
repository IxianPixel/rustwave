use std::time::Duration;
use std::path::PathBuf;

use iced::widget::{image, image::Handle, container, column, row};
use iced::widget::{button, mouse_area, svg, text, MouseArea, Row, Svg};
use iced::Color;
use crate::models::SoundCloudUser;
use crate::{models::SoundCloudTrack, page_b, Message};
use ::image::load_from_memory;

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

/// Downloads waveform image and returns raw bytes for peak extraction
pub async fn download_waveform_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

/// Extract peak data from waveform PNG for canvas rendering
pub fn extract_waveform_peaks(waveform_bytes: &[u8], target_width: usize) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    // Load the waveform image
    let img = load_from_memory(waveform_bytes)?;
    let rgba_img = img.to_rgba8();

    let width = rgba_img.width() as usize;
    let height = rgba_img.height() as usize;

    if width == 0 || height == 0 {
        return Ok(vec![0.0; target_width]);
    }

    let samples_per_peak = width / target_width.max(1);
    let mut peaks = Vec::with_capacity(target_width);

    // For each vertical slice, find the maximum height of non-transparent pixels
    for chunk_idx in 0..target_width {
        let start_x = chunk_idx * samples_per_peak;
        let end_x = (start_x + samples_per_peak).min(width);

        let mut max_extent = 0.0f32;

        // For this horizontal range, check all columns
        for x in start_x..end_x {
            // Find the extent of transparent pixels in this column (waveform shape)
            let mut top = height;
            let mut bottom = 0;

            for y in 0..height {
                let pixel = rgba_img.get_pixel(x as u32, y as u32);
                // SoundCloud waveforms have transparent shapes on white/opaque background
                // Check if pixel is transparent (low alpha - is part of waveform)
                if pixel[3] < 50 {
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
            }

            if bottom > top {
                let extent = (bottom - top) as f32 / height as f32;
                max_extent = max_extent.max(extent);
            }
        }

        peaks.push(max_extent);
    }

    Ok(peaks)
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