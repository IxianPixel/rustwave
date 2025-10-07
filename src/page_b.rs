use crate::api_helpers;
use crate::auth::TokenManager;
use crate::auth_page::AuthPage;
use crate::models::SoundCloudTrack;
use crate::pages::{FeedPage, SearchPage, UserPage};
use crate::track_list_manager::TrackListManager;
use crate::{Message, Page};
use iced::widget::image::Handle;
use iced::widget::{Scrollable, button, column, row, text, text_input};
use iced::{Color, Length, Task};
use tokio_util::bytes::Bytes;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub enum PageBMessage {
    ButtonPressed,
    LoadFeed,
    LoadFavourites,
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle), // track_id, image_handle
    ImageLoadFailed(u64),     // track_id
    LikeTrack(SoundCloudTrack),
    // New messages for handling token manager updates
    FeedLoadedWithToken(Vec<SoundCloudTrack>, TokenManager),
    FavouritesLoadedWithToken(crate::models::SoundCloudTracks, TokenManager),
    TrackLikedWithToken(u64, TokenManager),
    StreamDownloadedWithToken(Bytes, Option<Handle>, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
}
type Mb = PageBMessage;

pub struct PageB {
    token_manager: TokenManager,
    track_list: TrackListManager,
    track_load_failed: bool,
    search_query: String,
}

impl PageB {
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            token_manager,
            track_list: TrackListManager::new(),
            track_load_failed: false,
            search_query: String::new(),
        }
    }
}

impl Page for PageB {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::PageB(msg) = message {
            match msg {
                PageBMessage::ButtonPressed => {
                    return (Some(Box::new(AuthPage::new())), Task::none());
                }
                PageBMessage::LoadFeed => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_feed_with_refresh(token_manager),
                            |result| match result {
                                Ok((tracks, token_manager)) => {
                                    Message::PageB(Mb::FeedLoadedWithToken(tracks, token_manager))
                                }
                                Err((error, token_manager)) => Message::PageB(
                                    Mb::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PageBMessage::LoadFavourites => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_favourites_with_refresh(token_manager),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::PageB(
                                    Mb::FavouritesLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::PageB(
                                    Mb::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PageBMessage::PlayTrack(track) => {
                    self.track_list.set_current_track_id(track.id);

                    // Send the StartQueue message to main app with the selected track and all tracks
                    return (
                        None,
                        Task::done(Message::StartQueue(
                            track.clone(),
                            self.track_list.tracks().clone(),
                            self.token_manager.clone(),
                        )),
                    );
                }
                PageBMessage::ImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                PageBMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                PageBMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => {
                                    Message::PageB(Mb::TrackLikedWithToken(track_id, token_manager))
                                }
                                Err((error, token_manager)) => Message::PageB(
                                    Mb::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PageBMessage::FeedLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.track_list.set_tracks(tracks);

                    // Create tasks to load images for all tracks
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::PageB(Mb::ImageLoaded(track_id, handle)),
                        |track_id| Message::PageB(Mb::ImageLoadFailed(track_id)),
                    );

                    return (None, Task::batch(image_tasks));
                }
                PageBMessage::FavouritesLoadedWithToken(soundcloud_tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.track_list.set_tracks(soundcloud_tracks.collection);

                    // Create tasks to load images for all tracks
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::PageB(Mb::ImageLoaded(track_id, handle)),
                        |track_id| Message::PageB(Mb::ImageLoadFailed(track_id)),
                    );

                    return (None, Task::batch(image_tasks));
                }
                PageBMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    println!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                PageBMessage::StreamDownloadedWithToken(
                    _track_data,
                    _image_handle,
                    token_manager,
                ) => {
                    self.token_manager = token_manager;
                    println!("Stream downloaded");
                    // _track_data and _image_handle can be used as needed
                    return (None, Task::none());
                }
                PageBMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    println!("API Error: {}", error_msg);
                    return (None, Task::none());
                }
                PageBMessage::LoadUser(user_urn) => {
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
                }
            }
        }

        if let Message::NavigateToSearch = message {
            return (
                Some(Box::new(SearchPage::new(self.token_manager.clone()))),
                Task::none(),
            );
        }

        if let Message::NavigateToFeed = message {
            let (page, task) = FeedPage::new(self.token_manager.clone());
            return (Some(Box::new(page)), task);
        }

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<Message> {
        let tracks_column = self.track_list.render_tracks(
            |t| Message::PageB(Mb::PlayTrack(t)),
            |urn| Message::PageB(Mb::LoadUser(urn)),
            |t| Message::PageB(Mb::LikeTrack(t)),
        );

        column![
            row![
                button("Feed").on_press(Message::PageB(Mb::LoadFeed)),
                button("Favourites").on_press(Message::PageB(Mb::LoadFavourites)),
                button("Log out").on_press(Message::PageB(Mb::ButtonPressed)),
            ]
            .spacing(10),
            row![if self.track_load_failed {
                text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0))
            } else {
                text("")
            }],
            Scrollable::new(tracks_column)
                .height(Length::FillPortion(1))
                .width(Length::FillPortion(1)),
        ]
        .into()
    }
}
