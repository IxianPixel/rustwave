use std::time::Duration;

use crate::managers::{AudioManager, QueueManager};
use crate::pages::AuthPage;
use iced::widget::image::Handle;
use iced::{
    Event, Length, Subscription, Task,
    event::{self, Status},
    keyboard::{Event::KeyPressed, Key, key::Named},
    time,
    widget::{column, container},
    window,
};

fn main() -> iced::Result {
    // Only initialize tracing in debug builds, filtered to only rustwave logs
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter("rustwave=debug")
        .init();

    // Load the application icon
    let icon = window::icon::from_file_data(include_bytes!("../assets/icon.png"), None).ok();

    iced::application("Rustwave", MyApp::update, MyApp::view)
        .theme(|_| iced::Theme::CatppuccinMocha)
        .subscription(MyApp::subscription)
        .window(window::Settings {
            icon,
            ..Default::default()
        })
        .run_with(MyApp::new)
}

mod config;
mod constants;
mod managers;
mod models;
mod pages;
mod soundcloud;
mod utilities;
mod widgets;

#[derive(Debug, Clone)]
enum Message {
    LikesPage(pages::LikesPageMessage),
    AuthPage(pages::AuthPageMessage),
    SearchPage(pages::SearchPageMessage),
    FeedPage(pages::FeedPageMessage),
    UserPage(pages::UserPageMessage),
    PlaylistPage(pages::PlaylistPageMessage),
    PlayPausePlayback,
    SeekForwards,
    SeekBackwards,
    UiTick,
    SeekToPosition(f32),
    MediaControlEvent(souvlaki::MediaControlEvent),
    NextTrack,
    PreviousTrack,
    ToggleRepeatMode,
    TrackEnded,
    StartQueue(
        crate::models::SoundCloudTrack,
        Vec<crate::models::SoundCloudTrack>,
        crate::soundcloud::TokenManager,
    ),
    QueueStreamDownloaded(
        tokio_util::bytes::Bytes,
        Option<Handle>,
        Option<Vec<f32>>,
        crate::soundcloud::TokenManager,
    ),
    QueueStreamFailed(String, crate::soundcloud::TokenManager),
    NavigateToSearch,
    NavigateToLikes,
    NavigateToFeed,
}

trait Page {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>);
    fn view(&self) -> iced::Element<'_, Message>;
}

struct MyApp {
    page: Box<dyn Page>,
    title: String,
    user: String,
    artwork: Option<Handle>,
    waveform_peaks: Option<Vec<f32>>, // Peak data for canvas rendering
    audio_manager: AudioManager,
    queue_manager: QueueManager,
    pending_stream_download: bool, // Flag to track if we're downloading the next track
    token_manager: Option<crate::soundcloud::TokenManager>, // Store token manager for queue operations
    settings: config::AppSettings,
}

impl MyApp {
    // Helper method to start downloading and playing a track
    fn start_track_download(
        &mut self,
        track: &crate::models::SoundCloudTrack,
        token_manager: crate::soundcloud::TokenManager,
    ) -> Task<Message> {
        if track.stream_url.is_none() {
            return Task::none();
        }

        self.title = track.title.clone();
        self.user = track.user.username.clone();
        self.audio_manager.track_duration = Duration::from_millis(track.duration);
        self.audio_manager.stream_loading = true;
        self.audio_manager.sink.clear();
        self.pending_stream_download = true;

        let track_clone = track.clone();
        Task::perform(
            async move { crate::managers::download_track_stream(token_manager, &track_clone).await },
            |result| match result {
                Ok((track_data, image_handle, waveform_peaks, token_manager)) => {
                    Message::QueueStreamDownloaded(
                        track_data,
                        image_handle,
                        waveform_peaks,
                        token_manager,
                    )
                }
                Err((error, token_manager)) => Message::QueueStreamFailed(error, token_manager),
            },
        )
    }

    fn new() -> (Self, Task<Message>) {
        (
            Self {
                page: Box::new(AuthPage::new()),
                title: "Nothing".to_string(),
                user: "Nothing".to_string(),
                artwork: None,
                waveform_peaks: None,
                audio_manager: AudioManager::new(),
                queue_manager: QueueManager::new(),
                pending_stream_download: false,
                token_manager: None,
                settings: config::load_settings(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let (maybe_page, page_task) = self.page.update(message.clone());
        if let Some(page) = maybe_page {
            self.page = page;
        }

        // Handle the main app messages
        let app_task = match message {
            Message::StartQueue(track, tracks, token_manager) => {
                // Store the token manager for future queue operations
                self.token_manager = Some(token_manager.clone());

                // Initialize the queue starting from the selected track
                self.queue_manager.start_queue_from_track(track.id, tracks);

                // Start playing the first track in the queue
                if let Some(current_track) = self.queue_manager.current_track().cloned() {
                    self.start_track_download(&current_track, token_manager)
                } else {
                    Task::none()
                }
            }
            Message::QueueStreamDownloaded(
                track_data,
                image_handle,
                waveform_peaks,
                token_manager,
            ) => {
                // Update stored token manager
                self.token_manager = Some(token_manager);
                // Store waveform peak data
                self.waveform_peaks = waveform_peaks;

                // Load the track using AudioManager
                if let Err(e) = self.audio_manager.load_track(track_data) {
                    eprintln!("Failed to load track: {}", e);
                    self.pending_stream_download = false;
                    return Task::none();
                }

                self.pending_stream_download = false;
                self.artwork = image_handle;

                // Update media controls metadata
                self.audio_manager.update_metadata(
                    &self.title,
                    &self.user,
                    self.audio_manager.track_duration,
                );
                Task::none()
            }
            Message::QueueStreamFailed(error, token_manager) => {
                eprintln!("Failed to download stream: {}", error);
                self.audio_manager.stream_loading = false;
                self.pending_stream_download = false;
                // Update stored token manager
                self.token_manager = Some(token_manager);
                Task::none()
            }
            Message::PlayPausePlayback => {
                self.audio_manager.toggle_play_pause();
                Task::none()
            }
            Message::SeekForwards => {
                self.audio_manager.seek_forward(Duration::from_secs(10));
                Task::none()
            }
            Message::SeekBackwards => {
                self.audio_manager.seek_backward(Duration::from_secs(10));
                Task::none()
            }
            Message::UiTick => {
                // Check for media control events
                if let Ok(event) = self.audio_manager.media_event_receiver.try_recv() {
                    // Process the media control event
                    return Task::done(Message::MediaControlEvent(event));
                }

                // Update playback position
                self.audio_manager.update_position();

                // Check if track has ended
                if self.audio_manager.has_track_ended() && !self.pending_stream_download {
                    return Task::done(Message::TrackEnded);
                }

                Task::none()
            }
            Message::SeekToPosition(percent) => {
                self.audio_manager.seek_to_position(percent);
                Task::none()
            }
            Message::MediaControlEvent(event) => {
                match event {
                    souvlaki::MediaControlEvent::Play => {
                        self.audio_manager.play();
                    }
                    souvlaki::MediaControlEvent::Pause => {
                        self.audio_manager.pause();
                    }
                    souvlaki::MediaControlEvent::Toggle => {
                        self.audio_manager.toggle_play_pause();
                    }
                    souvlaki::MediaControlEvent::Next => {
                        return self.update(Message::NextTrack);
                    }
                    souvlaki::MediaControlEvent::Previous => {
                        return self.update(Message::PreviousTrack);
                    }
                    souvlaki::MediaControlEvent::SeekBy(direction, offset) => match direction {
                        souvlaki::SeekDirection::Forward => {
                            self.audio_manager.seek_forward(offset);
                        }
                        souvlaki::SeekDirection::Backward => {
                            self.audio_manager.seek_backward(offset);
                        }
                    },
                    souvlaki::MediaControlEvent::SetPosition(position) => {
                        if !self.audio_manager.is_empty() {
                            let _ = self.audio_manager.sink.try_seek(position.0);
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::NextTrack => {
                if let Some(next_track) = self.queue_manager.next_track().cloned() {
                    if let Some(token_manager) = self.token_manager.clone() {
                        self.start_track_download(&next_track, token_manager)
                    } else {
                        eprintln!("No token manager available for next track");
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            Message::PreviousTrack => {
                if let Some(prev_track) = self.queue_manager.previous_track().cloned() {
                    if let Some(token_manager) = self.token_manager.clone() {
                        self.start_track_download(&prev_track, token_manager)
                    } else {
                        eprintln!("No token manager available for previous track");
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            Message::ToggleRepeatMode => {
                self.settings.repeat_mode = self.settings.repeat_mode.toggle();

                if let Err(e) = config::save_settings(&self.settings) {
                    eprintln!("Failed to save settings: {}", e);
                }

                Task::none()
            }
            Message::TrackEnded => {
                match self.settings.repeat_mode {
                    config::RepeatMode::One => {
                        // Repeat current track - reload from stored data
                        if let Some(track_data) = self.audio_manager.current_track_data.clone() {
                            // Reload the track using the stored data
                            if let Err(e) = self.audio_manager.load_track(tokio_util::bytes::Bytes::from(track_data)) {
                                eprintln!("Failed to reload track for repeat: {}", e);
                            }
                        }
                        Task::none()
                    }
                    config::RepeatMode::All => {
                        // Try to play next track, or restart queue
                        if self.queue_manager.has_next() {
                            Task::done(Message::NextTrack)
                        } else if self.queue_manager.queue_length() > 0 {
                            // Queue finished - restart from beginning
                            if let Some(token_manager) = self.token_manager.clone() {
                                self.queue_manager.reset_to_beginning();
                                if let Some(first_track) = self.queue_manager.current_track().cloned() {
                                    self.start_track_download(&first_track, token_manager)
                                } else {
                                    Task::none()
                                }
                            } else {
                                eprintln!("No token manager available for queue restart");
                                self.audio_manager.clear();
                                Task::none()
                            }
                        } else {
                            // Empty queue, stop playback
                            self.audio_manager.clear();
                            Task::none()
                        }
                    }
                }
            }
            _ => Task::none(),
        };

        // Combine both tasks
        Task::batch([page_task, app_task])
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let keyboard_listerer = event::listen_with(|event, status, _| match (event, status) {
            (
                Event::Keyboard(KeyPressed {
                    key: Key::Named(Named::Space),
                    ..
                }),
                Status::Ignored,
            ) => Some(Message::PlayPausePlayback),
            (
                Event::Keyboard(KeyPressed {
                    key: Key::Named(Named::ArrowRight),
                    ..
                }),
                Status::Ignored,
            ) => Some(Message::SeekForwards),
            (
                Event::Keyboard(KeyPressed {
                    key: Key::Named(Named::ArrowLeft),
                    ..
                }),
                Status::Ignored,
            ) => Some(Message::SeekBackwards),
            _ => None,
        });

        Subscription::batch(vec![
            keyboard_listerer,
            time::every(Duration::from_millis(100)).map(|_| Message::UiTick), // More frequent for media control responsiveness
        ])
    }

    fn view(&self) -> iced::Element<'_, Message> {
        column![
            widgets::get_playback_bar(
                self.artwork.clone(),
                &self.title,
                &self.user,
                self.audio_manager.track_position,
                self.audio_manager.track_duration,
                self.audio_manager.progress_bar_value,
                self.audio_manager.stream_loading,
                !self.audio_manager.is_empty() && !self.audio_manager.is_paused(),
                self.queue_manager.current_position(),
                self.queue_manager.queue_length(),
                self.waveform_peaks.clone(),
                &self.settings,
            ),
            container(self.page.view())
                .padding(5)
                .width(Length::Fill)
                .height(Length::FillPortion(1)),
        ]
        .into()
    }
}
