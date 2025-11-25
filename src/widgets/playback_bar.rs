use crate::Message;
use crate::config;
use crate::utilities::{DurationFormat, get_asset_path};
use crate::widgets;
use iced::widget::image::Handle;
use iced::{
    Color, Length,
    alignment::Vertical,
    widget::{
        Space, Svg, button, column, container, horizontal_rule, image, row, slider, svg, text,
    },
};
use std::time::Duration;

/// Renders the playback control bar with album art, track info, and controls
pub fn get_playback_bar<'a>(
    artwork: Option<Handle>,
    title: &'a str,
    user: &'a str,
    track_position: Duration,
    track_duration: Duration,
    progress_bar_value: f32,
    stream_loading: bool,
    is_playing: bool,
    current_position: Option<usize>,
    queue_length: usize,
    waveform_peaks: Option<Vec<f32>>,
    settings: &config::AppSettings,
) -> iced::Element<'a, Message> {
    let album_image = if artwork.is_some() {
        image(artwork.unwrap()).width(100).height(100)
    } else {
        image("placeholder.png").width(100).height(100)
    };

    let queue_text = if let Some(current_pos) = current_position {
        text(format!("Queue: {} of {}", current_pos + 1, queue_length))
    } else {
        text("Queue: Empty")
    };

    column![
        container(row![
            album_image,
            column![
                text("Playback").size(24),
                if stream_loading {
                    text("Loading stream...")
                } else {
                    text(format!("Now Playing: {}", title)).shaping(text::Shaping::Advanced)
                },
                text(format!("User: {}", user)).shaping(text::Shaping::Advanced),
                text(format!(
                    "{} / {}",
                    track_position.format_as_mmss(),
                    track_duration.format_as_mmss()
                )),
            ]
            .padding(5),
            Space::with_width(Length::Fill),
            container(
                column![
                    row![
                        button(
                            Svg::new(get_asset_path("assets/previous.svg"))
                                .width(22)
                                .height(22)
                                .style(|_theme, _status| svg::Style {
                                    color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                                }),
                        )
                        .on_press(Message::PreviousTrack),
                        button(
                            Svg::new(get_asset_path(
                                if is_playing {
                                    "assets/pause.svg"
                                } else {
                                    "assets/play.svg"
                                }
                            ))
                            .width(22)
                            .height(22)
                            .style(|_theme, _status| svg::Style {
                                color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                            }),
                        )
                        .on_press(Message::PlayPausePlayback),
                        button(
                            Svg::new(get_asset_path("assets/next.svg"))
                                .width(22)
                                .height(22)
                                .style(|_theme, _status| svg::Style {
                                    color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                                }),
                        )
                        .on_press(Message::NextTrack),
                        button(
                            Svg::new(get_asset_path(
                                match settings.repeat_mode {
                                    config::RepeatMode::All => "assets/repeat.svg",
                                    config::RepeatMode::One => "assets/repeat_one.svg",
                                }
                            ))
                            .width(22)
                            .height(22)
                            .style(|_theme, _status| svg::Style {
                                color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                            }),
                        )
                        .on_press(Message::ToggleRepeatMode),
                    ]
                    .spacing(5),
                    queue_text,
                    row![
                        button(
                            Svg::new(get_asset_path("assets/feed.svg"))
                                .width(22)
                                .height(22)
                                .style(|_theme, _status| svg::Style {
                                    color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                                }),
                        )
                        .on_press(Message::NavigateToFeed),
                        button(
                            Svg::new(get_asset_path("assets/heart.svg"))
                                .width(22)
                                .height(22)
                                .style(|_theme, _status| svg::Style {
                                    color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                                }),
                        )
                        .on_press(Message::NavigateToLikes),
                        button(
                            Svg::new(get_asset_path("assets/search.svg"))
                                .width(22)
                                .height(22)
                                .style(|_theme, _status| svg::Style {
                                    color: Some(Color::from_rgb(1.0, 1.0, 1.0)),
                                }),
                        )
                        .on_press(Message::NavigateToSearch),
                    ]
                    .spacing(5),
                ]
                .spacing(5)
                .padding(5)
            ),
        ],)
        .align_y(Vertical::Center),
        horizontal_rule(5.0),
        if matches!(settings.seekbar_type, config::SeekbarType::Slider) {
            row![
                slider(0.0..=100.0, progress_bar_value, Message::SeekToPosition)
                    .width(Length::Fill)
                    .step(0.1),
            ]
            .padding(5)
        } else {
            row![widgets::get_waveform_widget(
                waveform_peaks,
                progress_bar_value / 100.0,
            ),]
        },
        horizontal_rule(5.0),
    ]
    .into()
}
