use std::collections::HashMap;

use crate::auth_page::AuthPage;
use iced::widget::{button, column, row, text, text_input, Scrollable};
use iced::widget::image::Handle;
use iced::{Color, Length, Task};
use crate::models::SoundCloudTrack;
use crate::utilities::get_track_widget;
use crate::{Message, Page};
use crate::auth::TokenManager;
use crate::soundcloud;
use tokio_util::bytes::Bytes;

#[derive(Debug, Clone)]
pub enum PageBMessage {
    ButtonPressed,
    LoadFeed,
    LoadFavourites,
    TracksLoaded(Vec<SoundCloudTrack>),
    TrackLoadFailed,
    PlayTrack(SoundCloudTrack),
    StreamDownloaded(Bytes, Option<Handle>),
    StreamLoadFailed,
    ImageLoaded(u64, Handle), // track_id, image_handle
    ImageLoadFailed(u64), // track_id
    LikeTrack(SoundCloudTrack),
    TrackLiked(u64),
    TrackLikeFailed,
    SearchPressed(String),
    Search(String),
}
type Mb = PageBMessage;

pub struct PageB {
    token_manager: TokenManager,
    tracks: Vec<SoundCloudTrack>,
    track_load_failed: bool,
    track_images: HashMap<u64, Handle>, // track_id -> image_handle
    current_track_id: u64,
    search_query: String,
}

impl PageB {
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            token_manager,
            tracks: Vec::new(),
            track_load_failed: false,
            track_images: HashMap::new(),
            current_track_id: 0,
            search_query: String::new(),
        }
    }
}

impl Page for PageB {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::PageB(msg) = message {
            match msg {
                PageBMessage::ButtonPressed => return (Some(Box::new(AuthPage::new())), Task::none()),
                PageBMessage::LoadFeed => {
                                let token = self.token_manager.get_access_token();
                                return (
                                    None,
                                    Task::perform(soundcloud::get_activity_feed(token), |result| {
                                        match result {
                                            Ok(tracks) => Message::PageB(Mb::TracksLoaded(tracks)),
                                            Err(_ee) => Message::PageB(Mb::TrackLoadFailed),
                                        }
                                    })
                                );
                            }
                PageBMessage::LoadFavourites => {
                                let token = self.token_manager.get_access_token();
                                return (
                                    None,
                                    Task::perform(soundcloud::get_liked_tracks(token), |result| {
                                        match result {
                                            Ok(tracks) => Message::PageB(Mb::TracksLoaded(tracks.collection)),
                                            Err(_ee) => Message::PageB(Mb::TrackLoadFailed),
                                        }
                                    })
                                );
                            }
                PageBMessage::TracksLoaded(sound_cloud_tracks) => {
                                self.track_load_failed = false;
                                self.tracks = sound_cloud_tracks.clone();
                    
                                // Create tasks to load images for all tracks
                                let image_tasks: Vec<Task<Message>> = sound_cloud_tracks
                                    .iter()
                                    .map(|track| {
                                        let track_id = track.id;
                                        let artwork_url = track.artwork_url.clone();
                                        Task::perform(
                                            async move { crate::utilities::download_image(&artwork_url).await },
                                            move |result| match result {
                                                Ok(handle) => Message::PageB(Mb::ImageLoaded(track_id, handle)),
                                                Err(_) => Message::PageB(Mb::ImageLoadFailed(track_id)),
                                            }
                                        )
                                    })
                                    .collect();
                    
                                return (None, Task::batch(image_tasks))
                            }
                PageBMessage::TrackLoadFailed => { self.track_load_failed = true; return (None, Task::none()) }
                PageBMessage::PlayTrack(track) => {
                                let token = self.token_manager.get_access_token();
                                self.current_track_id = track.id;

                                let stream_url = match &track.stream_url {
                                    Some(url) => url.clone(),
                                    None => String::new(),
                                };
                                let track_id = track.id;
                                let image_handle = self.track_images.get(&track_id).cloned();
                                return (
                                    None,
                                    Task::perform(soundcloud::get_track_data(token, stream_url), move |result| {
                                        match result {
                                            Ok(track_data) => Message::PageB(Mb::StreamDownloaded(track_data, image_handle.clone())),
                                            Err(_ee) => Message::PageB(Mb::StreamLoadFailed),
                                        }
                                    })
                                );
                            },
                PageBMessage::StreamDownloaded(_artwork, _image_handle) => {
                                println!("Stream downloaded");
                                // _image_handle is now Option<Handle>, you can use it as needed
                    
                                return (None, Task::none())
                            },
                PageBMessage::StreamLoadFailed => todo!(),
                PageBMessage::ImageLoaded(track_id, handle) => {
                                self.track_images.insert(track_id, handle);
                                return (None, Task::none())
                            },
                PageBMessage::ImageLoadFailed(track_id) => {
                                println!("Failed to load image for track {}", track_id);
                                return (None, Task::none())
                            },
                PageBMessage::LikeTrack(track) => {
                                let token = self.token_manager.get_access_token();
                                return (
                                    None,
                                    Task::perform(soundcloud::like_track(token, track.clone()), move |result| {
                                        match result {
                                            Ok(_) => Message::PageB(Mb::TrackLiked(track.id)),
                                            Err(_ee) => Message::PageB(Mb::TrackLikeFailed),
                                        }
                                    })
                                );
                            },
                PageBMessage::TrackLiked(track_id) => {
                                println!("Track liked: {}", track_id);
                                return (None, Task::none())
                            },
                PageBMessage::TrackLikeFailed => {
                                println!("Failed to like track");
                                return (None, Task::none())
                            },
                PageBMessage::Search(query) => {
                                self.search_query = query.clone();
                                let token = self.token_manager.get_access_token();
                                let search_query = self.search_query.clone(); // Clone the query for the async task
                                return (
                                    None,
                                    Task::perform(
                                        async move { soundcloud::search(token, &search_query).await },
                                        |result| {
                                            match result {
                                                Ok(tracks) => Message::PageB(Mb::TracksLoaded(tracks)),
                                                Err(_ee) => Message::PageB(Mb::TrackLoadFailed),
                                            }
                                        }
                                    )
                                );
                            },
                PageBMessage::SearchPressed(query) => {
                    self.search_query = query.clone();
                    return (None, Task::none())
                },
            }
        }
        (None, Task::none())
    }

    fn view(&self) -> iced::Element<Message> {
        // Build a column of track titles from the iterator. A for-loop inside
        // the `column![]` macro yields `()` and causes `Element: From<()>` errors.
        let tracks_column = self
            .tracks
            .iter()
            .fold(column![], |col, track| {
                let image_handle = self.track_images.get(&track.id).cloned();
                col.push(get_track_widget(track, image_handle))
            });

        column![
            row![
                button("Load Feed").on_press(Message::PageB(Mb::LoadFeed)),
                button("Load Favourites").on_press(Message::PageB(Mb::LoadFavourites)),
                text_input("Search", self.search_query.as_str())
                    .on_submit(Message::PageB(Mb::Search(self.search_query.clone())))
                    .on_input(|s| Message::PageB(Mb::SearchPressed(s))),
                button("Log out").on_press(Message::PageB(Mb::ButtonPressed)),
            ].spacing(10),
            row![ if self.track_load_failed { text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)) } else { text("") } ],
            Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),
        ]
        .into()
    }
}