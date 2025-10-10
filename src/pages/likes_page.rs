use crate::managers::TrackListManager;
use crate::models::SoundCloudTrack;
use crate::pages::auth_page::AuthPage;
use crate::pages::{FeedPage, FeedPageMessage, SearchPage, UserPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use crate::{Message, Page};
use iced::widget::image::Handle;
use iced::widget::{Scrollable, button, column, row, text};
use iced::{Color, Length, Task};

#[derive(Debug, Clone)]
pub enum LikesPageMessage {
    LoadFavourites,
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle),
    ImageLoadFailed(u64),
    LikeTrack(SoundCloudTrack),
    FavouritesLoadedWithToken(crate::models::SoundCloudTracks, TokenManager),
    TrackLikedWithToken(u64, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
}
type Ml = LikesPageMessage;

pub struct LikesPage {
    token_manager: TokenManager,
    track_list: TrackListManager,
    track_load_failed: bool,
}

impl LikesPage {
    pub fn new(token_manager: TokenManager) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                track_list: TrackListManager::new(),
                track_load_failed: false,
            },
            Task::done(Message::LikesPage(LikesPageMessage::LoadFavourites)),
        )
    }
}

impl Page for LikesPage {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::LikesPage(msg) = message {
            match msg {
                LikesPageMessage::LoadFavourites => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_favourites_with_refresh(token_manager),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::LikesPage(
                                    Ml::FavouritesLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::LikesPage(
                                    Ml::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                LikesPageMessage::PlayTrack(track) => {
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
                LikesPageMessage::ImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                LikesPageMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                LikesPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => Message::LikesPage(
                                    Ml::TrackLikedWithToken(track_id, token_manager),
                                ),
                                Err((error, token_manager)) => Message::LikesPage(
                                    Ml::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                LikesPageMessage::FavouritesLoadedWithToken(soundcloud_tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.track_list.set_tracks(soundcloud_tracks.collection);

                    // Create tasks to load images for all tracks
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::LikesPage(Ml::ImageLoaded(track_id, handle)),
                        |track_id| Message::LikesPage(Ml::ImageLoadFailed(track_id)),
                    );

                    return (None, Task::batch(image_tasks));
                }
                LikesPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    println!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                LikesPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    println!("API Error: {}", error_msg);
                    return (None, Task::none());
                }
                LikesPageMessage::LoadUser(user_urn) => {
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

    fn view(&self) -> iced::Element<'_, Message> {
        let tracks_column = self.track_list.render_tracks(
            |t| Message::LikesPage(Ml::PlayTrack(t)),
            |urn| Message::LikesPage(Ml::LoadUser(urn)),
            |t| Message::LikesPage(Ml::LikeTrack(t)),
        );

        column![
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
