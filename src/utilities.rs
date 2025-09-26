use std::time::Duration;

use iced::widget::{image, image::Handle, container, column, row};
use iced::widget::{button, mouse_area, svg, text, MouseArea, Row, Svg};
use iced::Color;
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

pub fn get_track_widget(track: &'_ SoundCloudTrack, image_handle: Option<Handle>) -> MouseArea<'_, Message> {
    let mut row = Row::new();

    // Add image if handle is available, otherwise show placeholder text
    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(text("Loading image..."));
    }

    let duration = Duration::from_millis(track.duration);

    let title_text = if track.stream_url.is_some() {
        text(track.title.clone()).shaping(text::Shaping::Advanced)
    } else {
        text(format!("{} (Unavailable)", track.title.clone())).shaping(text::Shaping::Advanced).color(Color::from_rgb(1.0, 0.0, 0.0))
    };

    row = row.push(
        column![
            text(track.user.username.clone()).shaping(text::Shaping::Advanced).size(20),
            title_text,
            text(duration.format_as_mmss()),
            row![
                button(row![
                    Svg::new("assets/heart.svg")
                        .width(20)
                        .height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.favoritings_count.unwrap_or(0).to_string()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(Message::PageB(page_b::PageBMessage::LikeTrack(track.clone()))),
                button(row![
                    Svg::new("assets/repost.svg").width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.reposts_count.unwrap_or(0).to_string()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(Message::PageB(page_b::PageBMessage::PlayTrack(track.clone()))),
                button(row![
                    Svg::new("assets/play.svg").width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.playback_count.unwrap_or(0).to_string()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(Message::PageB(page_b::PageBMessage::PlayTrack(track.clone()))),
            ].spacing(5)
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    ).on_press(Message::PageB(page_b::PageBMessage::PlayTrack(track.clone())))
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