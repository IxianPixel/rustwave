use crate::models::{SoundCloudTrack};
use iced::widget::{image, image::Handle, container, column, row};
use iced::widget::{button, mouse_area, svg, text, MouseArea, Row, Svg};
use iced::Color;
use crate::page_b;
use crate::Message;
use crate::utilities::{DurationFormat, NumberFormat, get_asset_path};
use std::time::Duration;

pub fn get_track_widget<F, U>(track: &'_ SoundCloudTrack, image_handle: Option<Handle>, on_play: F, on_user: U) -> MouseArea<'_, Message>
where
    F: Fn(SoundCloudTrack) -> Message + 'static,
    U: Fn(String) -> Message + 'static,
{
    let mut row = Row::new();

    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(image(get_asset_path("assets/icon.png")).width(100).height(100));
    }

    let duration = Duration::from_millis(track.duration);

    let title_text = if track.stream_url.is_some() {
        text(track.title.clone()).shaping(text::Shaping::Advanced)
    } else {
        text(format!("{} (Unavailable)", track.title.clone())).shaping(text::Shaping::Advanced).color(Color::from_rgb(1.0, 0.0, 0.0))
    };

    row = row.push(
        column![
            mouse_area(
                text(track.user.username.clone()).shaping(text::Shaping::Advanced).size(20)
            ).on_press(on_user(track.user.urn.clone())),
            title_text,
            text(duration.format_as_mmss()),
            row![
                button(row![
                    Svg::new(get_asset_path("assets/heart.svg"))
                        .width(20)
                        .height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.favoritings_count.unwrap_or(0).format_compact_number()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(Message::PageB(page_b::PageBMessage::LikeTrack(track.clone()))),
                button(row![
                    Svg::new(get_asset_path("assets/repost.svg")).width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.reposts_count.unwrap_or(0).format_compact_number()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(on_play(track.clone())),
                button(row![
                    Svg::new(get_asset_path("assets/play.svg")).width(20).height(20)
                        .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                    text(track.playback_count.unwrap_or(0).format_compact_number()).color(Color::from_rgb(1.0, 1.0, 1.0)),
                ]).on_press(on_play(track.clone())),
            ].spacing(5)
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    ).on_press(on_play(track.clone()))
}