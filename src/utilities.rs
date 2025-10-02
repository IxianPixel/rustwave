use std::time::Duration;

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

pub fn get_user_widget(user: &'_ SoundCloudUser, image_handle: Option<Handle>) -> MouseArea<'_, Message> {
    let mut row = Row::new();

    // Add image if handle is available, otherwise show placeholder text
    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(text("Loading image..."));
    }

    row = row.push(
        column![
            text(user.username.clone()).shaping(text::Shaping::Advanced).size(20),
            text(format!("{} followers", format_compact_number(user.followers_count.unwrap_or(0)))).size(20),
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    )
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

pub fn format_compact_number(num: u64) -> String {
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