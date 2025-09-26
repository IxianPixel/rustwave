use std::collections::HashMap;

use crate::auth_page::AuthPage;
use iced::widget::{button, column, row, text, text_input, Scrollable};
use iced::widget::image::Handle;
use iced::{Color, Length, Task};
use crate::models::SoundCloudTrack;
use crate::utilities::get_track_widget;
use crate::{Message, Page};
use crate::auth::TokenManager;
use crate::api_helpers;
use tokio_util::bytes::Bytes;

#[derive(Debug, Clone)]
pub enum PageBMessage {
    ButtonPressed,
    LoadFeed,
    LoadFavourites,
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle), // track_id, image_handle
    ImageLoadFailed(u64), // track_id
    LikeTrack(SoundCloudTrack),
    SearchPressed(String),
    Search(String),
    // New messages for handling token manager updates
    FeedLoadedWithToken(Vec<SoundCloudTrack>, TokenManager),
    FavouritesLoadedWithToken(crate::models::SoundCloudTracks, TokenManager),
    SearchCompletedWithToken(Vec<SoundCloudTrack>, TokenManager),
    TrackLikedWithToken(u64, TokenManager),
    StreamDownloadedWithToken(Bytes, Option<Handle>, TokenManager),
    ApiErrorWithToken(String, TokenManager),
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
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(api_helpers::load_feed_with_refresh(token_manager), |result| {
                            match result {
                                Ok((tracks, token_manager)) => Message::PageB(Mb::FeedLoadedWithToken(tracks, token_manager)),
                                Err((error, token_manager)) => Message::PageB(Mb::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                }
                PageBMessage::LoadFavourites => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(api_helpers::load_favourites_with_refresh(token_manager), |result| {
                            match result {
                                Ok((tracks, token_manager)) => Message::PageB(Mb::FavouritesLoadedWithToken(tracks, token_manager)),
                                Err((error, token_manager)) => Message::PageB(Mb::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                } 
                PageBMessage::PlayTrack(track) => {
                    self.current_track_id = track.id;
                    
                    // Send the StartQueue message to main app with the selected track and all tracks
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.tracks.clone(), self.token_manager.clone()))
                    );
                },
                PageBMessage::ImageLoaded(track_id, handle) => {
                    self.track_images.insert(track_id, handle);
                    return (None, Task::none())
                },
                PageBMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none())
                },
                PageBMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(api_helpers::like_track_with_refresh(token_manager, track.clone()), move |result| {
                            match result {
                                Ok((track_id, token_manager)) => Message::PageB(Mb::TrackLikedWithToken(track_id, token_manager)),
                                Err((error, token_manager)) => Message::PageB(Mb::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                },
                PageBMessage::Search(query) => {
                    self.search_query = query.clone();
                    let token_manager = self.token_manager.clone();
                    let search_query = self.search_query.clone(); // Clone the query for the async task
                    return (
                        None,
                        Task::perform(
                            api_helpers::search_with_refresh(token_manager, search_query),
                            |result| {
                                match result {
                                    Ok((tracks, token_manager)) => Message::PageB(Mb::SearchCompletedWithToken(tracks, token_manager)),
                                    Err((error, token_manager)) => Message::PageB(Mb::ApiErrorWithToken(error.to_string(), token_manager)),
                                }
                            }
                        )
                    );
                },
                PageBMessage::SearchPressed(query) => {
                    self.search_query = query.clone();
                    return (None, Task::none())
                },
                // New message handlers for token-aware API calls
                PageBMessage::FeedLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.tracks = tracks.clone();
                    
                    // Create tasks to load images for all tracks
                    let image_tasks: Vec<Task<Message>> = tracks
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
                },
                PageBMessage::FavouritesLoadedWithToken(soundcloud_tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.tracks = soundcloud_tracks.collection.clone();
                    
                    // Create tasks to load images for all tracks
                    let image_tasks: Vec<Task<Message>> = soundcloud_tracks.collection
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
                },
                PageBMessage::SearchCompletedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.tracks = tracks.clone();
                    
                    // Create tasks to load images for all tracks
                    let image_tasks: Vec<Task<Message>> = tracks
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
                },
                PageBMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    println!("Track liked: {}", track_id);
                    return (None, Task::none())
                },
                PageBMessage::StreamDownloadedWithToken(_track_data, _image_handle, token_manager) => {
                    self.token_manager = token_manager;
                    println!("Stream downloaded");
                    // _track_data and _image_handle can be used as needed
                    return (None, Task::none())
                },
                PageBMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    println!("API Error: {}", error_msg);
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
                button("Feed").on_press(Message::PageB(Mb::LoadFeed)),
                button("Favourites").on_press(Message::PageB(Mb::LoadFavourites)),
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