use std::{io::Cursor, time::Duration, sync::mpsc};

use iced::{
    alignment::Vertical, event::{self, Status}, keyboard::{key::Named, Event::KeyPressed, Key}, time, widget::{button, column, container, horizontal_rule, row, slider, svg, text, Space, Svg}, window, Color, Event, Length, Subscription, Task
};
use iced::widget::{image, image::Handle};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

use crate::{pages::SearchPage, utilities::DurationFormat};
use crate::queue_manager::QueueManager;

fn main() -> iced::Result {
    // Only initialize tracing in debug builds, filtered to only rustwave logs
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_env_filter("rustwave=debug")
        .init();

    // Load the application icon
    let icon = window::icon::from_file_data(
        include_bytes!("../assets/icon.png"),
        None,
    ).ok();

    iced::application("Rustwave", MyApp::update, MyApp::view)
        .theme(|_| iced::Theme::CatppuccinMocha)
        .subscription(MyApp::subscription)
        .window(window::Settings {
            icon,
            ..Default::default()
        })
        .run_with(MyApp::new)
}

mod page_b;
mod auth_page;
mod auth;
mod api_helpers;
mod constants;
mod config;
mod models;
mod soundcloud;
mod utilities;
mod queue_manager;
mod stream_manager;
mod pages;
mod widgets;

#[derive(Debug, Clone)]
enum Message {
    PageB(page_b::PageBMessage),
    AuthPage(auth_page::AuthPageMessage),
    SearchPage(pages::SearchPageMessage),
    FeedPage(pages::FeedPageMessage),
    UserPage(pages::UserPageMessage),
    PlayPausePlayback,
    SeekForwards,
    SeekBackwards,
    UiTick,
    ProgressBarClicked,
    ProgressBarReleased,
    SeekToPosition(f32),
    MediaControlEvent(souvlaki::MediaControlEvent),
    NextTrack,
    PreviousTrack,
    TrackEnded,
    StartQueue(crate::models::SoundCloudTrack, Vec<crate::models::SoundCloudTrack>, crate::auth::TokenManager),
    QueueStreamDownloaded(tokio_util::bytes::Bytes, Option<Handle>, crate::auth::TokenManager),
    QueueStreamFailed(String, crate::auth::TokenManager),
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
    stream_loading: bool,
    stream: OutputStream,
    sink: Sink,
    track_duration: Duration,
    track_position: Duration,
    progress_bar_value: f32,
    media_controls: MediaControls,
    media_event_receiver: mpsc::Receiver<souvlaki::MediaControlEvent>,
    current_track_data: Option<Vec<u8>>, // Store the current track data for backward seeking
    queue_manager: QueueManager,
    pending_stream_download: bool, // Flag to track if we're downloading the next track
    token_manager: Option<crate::auth::TokenManager>, // Store token manager for queue operations
}

impl MyApp {
    // Helper method to start downloading and playing a track
    fn start_track_download(&mut self, track: &crate::models::SoundCloudTrack, token_manager: crate::auth::TokenManager) -> Task<Message> {
        if track.stream_url.is_none() {
            return Task::none();
        }

        self.title = track.title.clone();
        self.user = track.user.username.clone();
        self.track_duration = Duration::from_millis(track.duration);
        self.stream_loading = true;
        self.sink.clear();
        self.pending_stream_download = true;

        let track_clone = track.clone();
        Task::perform(
            async move { crate::stream_manager::download_track_stream(token_manager, &track_clone).await },
            |result| match result {
                Ok((track_data, image_handle, token_manager)) => {
                    Message::QueueStreamDownloaded(track_data, image_handle, token_manager)
                }
                Err((error, token_manager)) => {
                    Message::QueueStreamFailed(error, token_manager)
                }
            },
        )
    }

    // Unified backward seeking function that handles the workaround
    fn seek_backward(&mut self, seek_amount: Duration) -> bool {
        if self.sink.empty() {
            return false;
        }

        let cur_pos = self.sink.get_pos();
        let new_position = cur_pos.saturating_sub(seek_amount);

        // Try direct backward seek first
        match self.sink.try_seek(new_position) {
            Ok(_) => {
                self.track_position = new_position;
                true
            },
            Err(_) => {
                // Advanced workaround: recreate the audio source and seek forward
                if let Some(ref track_data) = self.current_track_data {
                    // Remember if we were paused
                    let was_paused = self.sink.is_paused();
                    
                    // Recreate the sink and source
                    self.sink = Sink::connect_new(self.stream.mixer());
                    
                    match Decoder::new(Cursor::new(track_data.clone())) {
                        Ok(source) => {
                            self.sink.clear();
                            self.sink.append(source);
                            
                            // If we want to seek to a position > 0, do forward seek
                            if new_position > Duration::from_secs(0) {
                                match self.sink.try_seek(new_position) {
                                    Ok(_) => {
                                        self.track_position = new_position;
                                        
                                        // Restore play/pause state
                                        if was_paused {
                                            self.sink.pause();
                                        } else {
                                            self.sink.play();
                                        }
                                        true
                                    },
                                    Err(_) => false,
                                }
                            } else {
                                self.track_position = Duration::from_secs(0);
                                
                                // Restore play/pause state
                                if was_paused {
                                    self.sink.pause();
                                } else {
                                    self.sink.play();
                                }
                                true
                            }
                        },
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
        }
    }

    fn new() -> (Self, Task<Message>) {
        let stream = OutputStreamBuilder::open_default_stream()
            .expect("Failed to open default audio output stream");
        let sink = Sink::connect_new(stream.mixer());
        
        // Initialize media controls with channel
        let (sender, receiver) = mpsc::channel();
        let hwnd = None; // For Windows, you might need to get the window handle
        let config = PlatformConfig {
            dbus_name: "rustwave",
            display_name: "Rustwave",
            hwnd,
        };
        
        let mut media_controls = MediaControls::new(config)
            .expect("Failed to initialize media controls");
            
        // Attach the event handler
        media_controls.attach(move |event| {
            let _ = sender.send(event);
        }).expect("Failed to attach media controls event handler");
        
        (
            Self {
                page: Box::new(auth_page::AuthPage::new()),
                title: "Nothing".to_string(),
                user: "Nothing".to_string(),
                artwork: None,
                stream_loading: false,
                stream,
                sink,
                track_duration: Duration::from_secs(0),
                track_position: Duration::from_secs(0),
                progress_bar_value: 0.0,
                media_controls,
                media_event_receiver: receiver,
                current_track_data: None,
                queue_manager: QueueManager::new(),
                pending_stream_download: false,
                token_manager: None,
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
            },
            Message::PageB(page_b::PageBMessage::PlayTrack(_track)) => {
                // This will be handled by the page to convert to StartQueue message
                Task::none()
            },
            Message::QueueStreamDownloaded(track_data, image_handle, token_manager) => {
                // Update stored token manager
                self.token_manager = Some(token_manager);
                // Store the track data for potential backward seeking workaround
                self.current_track_data = Some(track_data.to_vec());
                
                // Recreate a fresh Sink on our existing, long-lived stream's mixer
                self.sink = Sink::connect_new(self.stream.mixer());

                let source = match Decoder::new(Cursor::new(track_data)) {
                    Ok(source) => source,
                    Err(e) => {
                        eprintln!("Failed to create decoder: {e}");
                        self.pending_stream_download = false;
                        return Task::none()
                    }
                };
                self.sink.clear();
                self.sink.append(source);
                self.sink.play();
                self.stream_loading = false;
                self.pending_stream_download = false;
                self.artwork = image_handle;
                
                // Update media controls metadata
                let metadata = MediaMetadata {
                    title: Some(&self.title),
                    artist: Some(&self.user),
                    album: None,
                    cover_url: None, // You could add artwork URL here if available
                    duration: Some(self.track_duration),
                };
                let _ = self.media_controls.set_metadata(metadata);
                let _ = self.media_controls.set_playback(MediaPlayback::Playing { 
                    progress: Some(souvlaki::MediaPosition(Duration::from_secs(0))) 
                });
                Task::none()
            },
            Message::QueueStreamFailed(error, token_manager) => {
                eprintln!("Failed to download stream: {}", error);
                self.stream_loading = false;
                self.pending_stream_download = false;
                // Update stored token manager
                self.token_manager = Some(token_manager);
                Task::none()
            },
            Message::PageB(page_b::PageBMessage::StreamDownloadedWithToken(track_data, image_handle, token_manager)) => {
                // Legacy handler - redirect to new queue system
                Task::done(Message::QueueStreamDownloaded(track_data, image_handle, token_manager))
            },
            Message::PlayPausePlayback => {
                if !self.sink.empty() {
                    if self.sink.is_paused() {
                        self.sink.play();
                        let _ = self.media_controls.set_playback(MediaPlayback::Playing { 
                            progress: Some(souvlaki::MediaPosition(self.track_position)) 
                        });
                    } else {
                        self.sink.pause();
                        let _ = self.media_controls.set_playback(MediaPlayback::Paused { 
                            progress: Some(souvlaki::MediaPosition(self.track_position)) 
                        });
                    }
                }
                Task::none()
            },
            Message::SeekForwards => {
                if !self.sink.empty() {
                    let seek_limit = Duration::from_secs(10);
                    let cur_pos = self.sink.get_pos();
                    let new_position = cur_pos + seek_limit;

                    let _ = self.sink.try_seek(new_position);
                }
                Task::none()
            },
            Message::SeekBackwards => {
                let seek_limit = Duration::from_secs(10);
                self.seek_backward(seek_limit);
                Task::none()
            },
            Message::UiTick => {
                // Check for media control events
                if let Ok(event) = self.media_event_receiver.try_recv() {
                    // Process the media control event
                    return Task::done(Message::MediaControlEvent(event));
                }
                
                if !self.sink.empty() {
                    let new_position = self.sink.get_pos();
                    self.track_position = new_position;

                    self.progress_bar_value = (new_position.as_secs_f32() / self.track_duration.as_secs_f32()) * 100.0;
                    
                    // Check if track has ended (reached the end or very close to it)
                    if new_position >= self.track_duration.saturating_sub(Duration::from_millis(500)) && !self.pending_stream_download {
                        return Task::done(Message::TrackEnded);
                    }
                    
                    // Update media controls with current position
                    let playback_state = if self.sink.is_paused() {
                        MediaPlayback::Paused { progress: Some(souvlaki::MediaPosition(self.track_position)) }
                    } else {
                        MediaPlayback::Playing { progress: Some(souvlaki::MediaPosition(self.track_position)) }
                    };
                    let _ = self.media_controls.set_playback(playback_state);
                }
                Task::none()
            },
            Message::SeekToPosition(percent) => {
                if !self.sink.empty() {
                    let new_position = self.track_duration.mul_f32(percent / 100.0);
                    let current_position = self.sink.get_pos();
                    
                    // Determine if this is forward or backward seeking
                    if new_position < current_position {
                        // Backward seeking - use our unified function
                        let seek_amount = current_position - new_position;
                        if self.seek_backward(seek_amount) {
                            self.progress_bar_value = percent;
                        }
                    } else {
                        // Forward seeking - use direct seek
                        match self.sink.try_seek(new_position) {
                            Ok(_) => {
                                self.track_position = new_position;
                                self.progress_bar_value = percent;
                            },
                            Err(_) => {
                                // Forward seek failed, don't update UI
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::MediaControlEvent(event) => {
                match event {
                    souvlaki::MediaControlEvent::Play => {
                        if !self.sink.empty() && self.sink.is_paused() {
                            self.sink.play();
                            let _ = self.media_controls.set_playback(MediaPlayback::Playing { progress: Some(souvlaki::MediaPosition(self.track_position)) });
                        }
                    }
                    souvlaki::MediaControlEvent::Pause => {
                        if !self.sink.empty() && !self.sink.is_paused() {
                            self.sink.pause();
                            let _ = self.media_controls.set_playback(MediaPlayback::Paused { progress: Some(souvlaki::MediaPosition(self.track_position)) });
                        }
                    }
                    souvlaki::MediaControlEvent::Toggle => {
                        if !self.sink.empty() {
                            if self.sink.is_paused() {
                                self.sink.play();
                                let _ = self.media_controls.set_playback(MediaPlayback::Playing { progress: Some(souvlaki::MediaPosition(self.track_position)) });
                            } else {
                                self.sink.pause();
                                let _ = self.media_controls.set_playback(MediaPlayback::Paused { progress: Some(souvlaki::MediaPosition(self.track_position)) });
                            }
                        }
                    }
                    souvlaki::MediaControlEvent::Next => {
                        return self.update(Message::NextTrack);
                    }
                    souvlaki::MediaControlEvent::Previous => {
                        return self.update(Message::PreviousTrack);
                    }
                    souvlaki::MediaControlEvent::SeekBy(direction, offset) => {
                        match direction {
                            souvlaki::SeekDirection::Forward => {
                                if !self.sink.empty() {
                                    let cur_pos = self.sink.get_pos();
                                    let new_position = cur_pos + offset;
                                    let _ = self.sink.try_seek(new_position);
                                }
                            },
                            souvlaki::SeekDirection::Backward => {
                                self.seek_backward(offset);
                            }
                        }
                    }
                    souvlaki::MediaControlEvent::SetPosition(position) => {
                        if !self.sink.empty() {
                            let _ = self.sink.try_seek(position.0);
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
            },
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
            },
            Message::TrackEnded => {
                // Automatically play next track when current track ends
                if self.queue_manager.has_next() {
                    Task::done(Message::NextTrack)
                } else {
                    // Queue finished, stop playback
                    self.sink.clear();
                    let _ = self.media_controls.set_playback(MediaPlayback::Stopped);
                    Task::none()
                }
            },
            _ => Task::none()
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
        

    fn view(&self) -> iced::Element<Message> {
        let image = if self.artwork.is_some() {
            image(self.artwork.clone().unwrap()).width(100).height(100)
        } else {
            image("placeholder.png").width(100).height(100)
        };

        let queue = if let Some(current_pos) = self.queue_manager.current_position() {
            text(format!("Queue: {} of {}", current_pos + 1, self.queue_manager.queue_length()))
        } else {
            text("Queue: Empty")
        };

        column![
            container(
                row![
                    image,
                    column![
                        text("Playback").size(24),
                        if self.stream_loading { text("Loading stream...") } else { text(format!("Now Playing: {}", self.title)).shaping(text::Shaping::Advanced) },
                        text(format!("User: {}", self.user)).shaping(text::Shaping::Advanced),
                        text(format!("{} / {}", self.track_position.format_as_mmss(), self.track_duration.format_as_mmss())),
                        
                    ]
                    .padding(5),
                    Space::with_width(Length::Fill),
                    container(
                        column![
                            row![
                                button("Previous")
                                    .on_press(Message::PreviousTrack),
                                button("Play/Pause").on_press(Message::PlayPausePlayback),
                                button("Next")
                                    .on_press(Message::NextTrack),
                            ].spacing(5),
                            queue,
                            row![
                                button(
                                    Svg::new("assets/feed.svg")
                                    .width(22)
                                    .height(22)
                                    .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                                ).on_press(Message::NavigateToFeed),
                                button(
                                    Svg::new("assets/heart.svg")
                                    .width(22)
                                    .height(22)
                                    .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                                ).on_press(Message::NavigateToLikes),
                                button(
                                    Svg::new("assets/search.svg")
                                    .width(22)
                                    .height(22)
                                    .style(|_theme, _status| svg::Style { color: Some(Color::from_rgb(1.0, 1.0, 1.0)), ..Default::default() }),
                                ).on_press(Message::NavigateToSearch),
                            ].spacing(5),
                        ]
                        .spacing(5)
                        .padding(5)
                    ),
                ],
            ).align_y(Vertical::Center),
            row![
                slider(0.0..=100.0, self.progress_bar_value, Message::SeekToPosition)
                    .width(Length::Fill)
                    .step(0.1),
            ]
            .padding(5),
            horizontal_rule(20.0),
            container(
                self.page.view()
            )
            .padding(5)
            .width(Length::Fill)
            .height(Length::FillPortion(1)),
        ]
        .into()
    }
}
