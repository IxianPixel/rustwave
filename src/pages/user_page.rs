use std::collections::HashMap;

use iced::Task;
use tracing::debug;

use crate::auth::TokenManager;
use crate::models::{SoundCloudPlaylist, SoundCloudTrack, SoundCloudUser, SoundCloudUserProfile};
use crate::page_b::PageB;
use crate::pages::{FeedPage, PlaylistPage, SearchPage, SearchPageMessage};
use crate::track_list_manager::TrackListManager;
use crate::utilities::get_asset_path;
use crate::widgets::get_playlist_widget;
use crate::{Message, Page, api_helpers};
use iced::Color;
use iced::Length;
use iced::widget::image::{self, Handle};
use iced::widget::{Scrollable, column, row, text};

#[derive(Debug, Clone)]
pub enum UserPageMessage {
    LoadUser,
    UserProfileLoaded(SoundCloudUserProfile, TokenManager),
    UserImageLoaded(String, Handle),
    UserImageLoadFailed(String),
    PlaylistImageLoaded(String, Handle),
    PlaylistImageLoadFailed(String),
    ApiErrorWithToken(String, TokenManager),
    TrackImageLoaded(u64, Handle),
    TrackImageLoadFailed(u64),
    PlayTrack(SoundCloudTrack),
    NavigateToUser(String),
    LoadPlaylist(SoundCloudPlaylist),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
}

type Mu = UserPageMessage;

pub struct UserPage {
    token_manager: TokenManager,
    user_urn: String,
    user: SoundCloudUser,
    playlists: Vec<SoundCloudPlaylist>,
    playlist_images: HashMap<String, Handle>,
    track_list: TrackListManager,
    track_load_failed: bool,
}

impl UserPage {
    pub fn new(token_manager: TokenManager, user_urn: String) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                user_urn,
                user: SoundCloudUser::default(),
                playlists: Vec::new(),
                playlist_images: HashMap::new(),
                track_list: TrackListManager::new(),
                track_load_failed: false,
            },
            Task::done(Message::UserPage(UserPageMessage::LoadUser)),
        )
    }
}

impl Page for UserPage {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::UserPage(msg) = message {
            match msg {
                UserPageMessage::LoadUser => {
                    let token_manager = self.token_manager.clone();
                    let user_urn = self.user_urn.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_user_profile_with_refresh(token_manager, user_urn),
                            |result| match result {
                                Ok((user, token_manager)) => Message::UserPage(
                                    UserPageMessage::UserProfileLoaded(user, token_manager),
                                ),
                                Err((error, token_manager)) => {
                                    Message::UserPage(UserPageMessage::ApiErrorWithToken(
                                        error.to_string(),
                                        token_manager,
                                    ))
                                }
                            },
                        ),
                    );
                }
                UserPageMessage::UserProfileLoaded(profile, token_manager) => {
                    self.token_manager = token_manager;
                    self.user = profile.user.clone();
                    self.playlists = profile.playlists.clone();
                    self.track_list.set_tracks(profile.tracks);

                    // Create tasks to load images for all tracks
                    let track_image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| {
                            Message::UserPage(UserPageMessage::TrackImageLoaded(track_id, handle))
                        },
                        |track_id| {
                            Message::UserPage(UserPageMessage::TrackImageLoadFailed(track_id))
                        },
                    );

                    // Create tasks to load images for all playlists
                    let playlist_image_tasks: Vec<Task<Message>> = self
                        .playlists
                        .iter()
                        .map(|playlist| {
                            let playlist_urn = playlist.urn.clone();
                            let artwork_url = playlist.artwork_url.clone();
                            debug!("Downloading for {}", artwork_url);
                            Task::perform(
                                async move { crate::utilities::download_image(&artwork_url).await },
                                move |result| match result {
                                    Ok(handle) => Message::UserPage(Mu::PlaylistImageLoaded(
                                        playlist_urn.clone(),
                                        handle,
                                    )),
                                    Err(_) => Message::UserPage(Mu::PlaylistImageLoadFailed(
                                        playlist_urn.clone(),
                                    )),
                                },
                            )
                        })
                        .collect();

                    return (
                        None,
                        Task::batch(track_image_tasks.into_iter().chain(playlist_image_tasks)),
                    );
                }
                UserPageMessage::TrackImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                UserPageMessage::TrackImageLoadFailed(track_id) => {
                    debug!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                UserPageMessage::PlaylistImageLoaded(urn, handle) => {
                    self.playlist_images.insert(urn, handle);
                    return (None, Task::none());
                }
                UserPageMessage::PlaylistImageLoadFailed(urn) => {
                    debug!("Failed to load image for playlist {}", urn);
                    let handle = image::Handle::from_path(get_asset_path("assets/icon/png"));
                    self.playlist_images.insert(urn, handle);
                    return (None, Task::none());
                }
                UserPageMessage::UserImageLoaded(_, handle) => todo!(),
                UserPageMessage::UserImageLoadFailed(_) => todo!(),
                UserPageMessage::ApiErrorWithToken(_, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    return (None, Task::none());
                }
                UserPageMessage::PlayTrack(track) => {
                    self.track_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(
                            track.clone(),
                            self.track_list.tracks().clone(),
                            self.token_manager.clone(),
                        )),
                    );
                }
                UserPageMessage::NavigateToUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    return (None, Task::none());
                }
                UserPageMessage::LoadPlaylist(playlist) => {
                    let (playlist_page, task) =
                        PlaylistPage::new(self.token_manager.clone(), playlist);
                    return (Some(Box::new(playlist_page)), task);
                }
                UserPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => Message::UserPage(
                                    Mu::TrackLikedWithToken(track_id, token_manager),
                                ),
                                Err((error, token_manager)) => Message::UserPage(
                                    Mu::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                UserPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    debug!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
            }
        }

        if let Message::NavigateToFeed = message {
            let (page, task) = FeedPage::new(self.token_manager.clone());
            return (Some(Box::new(page)), task);
        }

        if let Message::NavigateToLikes = message {
            return (
                Some(Box::new(PageB::new(self.token_manager.clone()))),
                Task::none(),
            );
        }

        if let Message::NavigateToSearch = message {
            return (
                Some(Box::new(SearchPage::new(self.token_manager.clone()))),
                Task::none(),
            );
        }

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<Message> {
        let tracks_column = self.track_list.render_tracks(
            |t| Message::UserPage(UserPageMessage::PlayTrack(t)),
            |urn| Message::UserPage(UserPageMessage::NavigateToUser(urn)),
            |t| Message::SearchPage(SearchPageMessage::LikeTrack(t)),
        );

        let playlists_column = self.playlists.iter().fold(column![], |col, playlist| {
            let image_handle = self.playlist_images.get(&playlist.user.urn).cloned();
            col.push(get_playlist_widget(playlist, image_handle, |urn| {
                Message::UserPage(UserPageMessage::LoadPlaylist(urn))
            }))
        });

        column![
            row![if self.track_load_failed {
                text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0))
            } else {
                text("")
            }],
            row![
                Scrollable::new(tracks_column)
                    .height(Length::FillPortion(1))
                    .width(Length::FillPortion(1)),
                Scrollable::new(playlists_column)
                    .height(Length::FillPortion(1))
                    .width(Length::FillPortion(1)),
            ]
            .spacing(10)
        ]
        .into()
    }
}
