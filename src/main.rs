use std::{io::Cursor, time::Duration, sync::mpsc};

use iced::{
    alignment::Vertical, event::{self, Status}, keyboard::{key::Named, Event::KeyPressed, Key}, time, widget::{button, column, container, horizontal_rule, progress_bar, row, slider, text, Space}, window, Event, Length, Subscription, Task
};
use iced::widget::{image, image::Handle};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig, SeekDirection};

use crate::utilities::DurationFormat;

fn main() -> iced::Result {
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

#[derive(Debug, Clone)]
enum Message {
    PageB(page_b::PageBMessage),
    AuthPage(auth_page::AuthPageMessage),
    PlayPausePlayback,
    SeekForwards,
    SeekBackwards,
    UiTick,
    ProgressBarClicked,
    ProgressBarReleased,
    SeekToPosition(f32),
    MediaControlEvent(souvlaki::MediaControlEvent),
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
}

impl MyApp {
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
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let (maybe_page, task) = self.page.update(message.clone());
        if let Some(page) = maybe_page {
            self.page = page;
        }

        match message {
            Message::PageB(page_b::PageBMessage::PlayTrack(track)) => {
                self.title = track.title.clone();
                self.user = track.user.username.clone();
                self.track_duration = Duration::from_millis(track.duration);
                self.stream_loading = true;
                self.sink.clear();
            },
            Message::PageB(page_b::PageBMessage::StreamDownloadedWithToken(track_data, image_handle, _)) => {
                // Store the track data for potential backward seeking workaround
                self.current_track_data = Some(track_data.to_vec());
                
                // Recreate a fresh Sink on our existing, long-lived stream's mixer
                self.sink = Sink::connect_new(self.stream.mixer());

                let source = match Decoder::new(Cursor::new(track_data)) {
                    Ok(source) => source,
                    Err(e) => {
                        eprintln!("Failed to create decoder: {e}");
                        return Task::none()
                    }
                };
                self.sink.clear();
                self.sink.append(source);
                self.sink.play();
                self.stream_loading = false;
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
            },
            Message::SeekForwards => {
                if !self.sink.empty() {
                    let seek_limit = Duration::from_secs(10);
                    let cur_pos = self.sink.get_pos();
                    let new_position = cur_pos + seek_limit;

                    let _ = self.sink.try_seek(new_position);
                }
            },
            Message::SeekBackwards => {
                let seek_limit = Duration::from_secs(10);
                self.seek_backward(seek_limit);
            },
            Message::UiTick => {
                // Check for media control events
                while let Ok(event) = self.media_event_receiver.try_recv() {
                    // Process the media control event by calling update recursively
                    return self.update(Message::MediaControlEvent(event));
                }
                
                if !self.sink.empty() {
                    let new_position = self.sink.get_pos();
                    self.track_position = new_position;

                    self.progress_bar_value = (new_position.as_secs_f32() / self.track_duration.as_secs_f32()) * 100.0;
                    
                    // Update media controls with current position
                    let playback_state = if self.sink.is_paused() {
                        MediaPlayback::Paused { progress: Some(souvlaki::MediaPosition(self.track_position)) }
                    } else {
                        MediaPlayback::Playing { progress: Some(souvlaki::MediaPosition(self.track_position)) }
                    };
                    let _ = self.media_controls.set_playback(playback_state);
                }
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
                        // You can implement next track functionality here
                        // For now, we'll just seek forward
                        if !self.sink.empty() {
                            let seek_limit = Duration::from_secs(10);
                            let cur_pos = self.sink.get_pos();
                            let new_position = cur_pos + seek_limit;
                            let _ = self.sink.try_seek(new_position);
                        }
                    }
                    souvlaki::MediaControlEvent::Previous => {
                        // You can implement previous track functionality here
                        // For now, we'll just seek backward
                        let seek_limit = Duration::from_secs(10);
                        self.seek_backward(seek_limit);
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
            }
            _ => {}
        }

        task
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
                        button("Play/Pause").on_press(Message::PlayPausePlayback),
                    )
                    .align_y(Vertical::Center)
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
