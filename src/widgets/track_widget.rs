use crate::models::{SoundCloudTrack};
use iced::widget::{image, image::Handle, container, column, row};
use iced::widget::{button, mouse_area, svg, text, MouseArea, Row, Svg};
use iced::Color;
use crate::page_b;
use crate::Message;
use crate::utilities::{DurationFormat, format_compact_number};
use std::time::Duration;

pub fn get_track_widget<F>(track: &'_ SoundCloudTrack, image_handle: Option<Handle>, on_play: F) -> MouseArea<'_, Message>
where
    F: Fn(SoundCloudTrack) -> Message + 'static,
{
    let mut row = Row::new();

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
                    text(format_compact_number(track.favoritings_count.unwrap_or(0).into())).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(Message::PageB(page_b::PageBMessage::LikeTrack(track.clone()))),
                button(row![
                    Svg::new("assets/repost.svg").width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(format_compact_number(track.reposts_count.unwrap_or(0).into())).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(on_play(track.clone())),
                button(row![
                    Svg::new("assets/play.svg").width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(format_compact_number(track.playback_count.unwrap_or(0).into())).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(on_play(track.clone())),
            ].spacing(5)
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    ).on_press(on_play(track.clone()))
}