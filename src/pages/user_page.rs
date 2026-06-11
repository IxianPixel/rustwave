use std::collections::HashMap;

use iced::Task;
use tracing::debug;

use crate::managers::TrackListManager;
use crate::models::{
    SoundCloudPlaylist, SoundCloudPlaylists, SoundCloudTrack, SoundCloudTracks, SoundCloudUser,
    SoundCloudUserProfile,
};
use crate::pages::{FeedPage, LikesPage, PlaylistPage, SearchPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use crate::utilities::get_asset_path;
use crate::widgets::get_playlist_widget;
use crate::{Message, Page};
use iced::Color;
use iced::Length;
use iced::widget::image::{self, Handle};
use iced::widget::{Scrollable, column, grid, row, sensor, text};

// Start loading the next page when the bottom sentinel is within 500px of the viewport
const LOAD_MORE_THRESHOLD: f32 = 500.0;

#[derive(Debug, Clone)]
pub enum UserPageMessage {
    LoadUser,
    UserProfileLoaded(SoundCloudUserProfile, TokenManager),
    LoadMoreTracks,
    LoadMorePlaylists,
    MoreTracksLoadedWithToken(SoundCloudTracks, TokenManager),
    MorePlaylistsLoadedWithToken(SoundCloudPlaylists, TokenManager),
    PlaylistImageLoaded(String, Handle),
    PlaylistImageLoadFailed(String),
    ApiErrorWithToken(String, TokenManager),
    RequestTrackImage(u64),
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
    playlists_next_href: Option<String>,
    playlists_loading: bool,
    track_list: TrackListManager,
    tracks_next_href: Option<String>,
    tracks_loading: bool,
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
                playlists_next_href: None,
                playlists_loading: false,
                track_list: TrackListManager::new(),
                tracks_next_href: None,
                tracks_loading: false,
                track_load_failed: false,
            },
            Task::done(Message::UserPage(UserPageMessage::LoadUser)),
        )
    }

    /// Builds the artwork-download tasks for a batch of playlists.
    fn playlist_image_tasks(playlists: &[SoundCloudPlaylist]) -> Vec<Task<Message>> {
        playlists
            .iter()
            .map(|playlist| {
                let playlist_urn = playlist.urn.clone();
                let artwork_url = playlist.artwork_url.clone();
                Task::perform(
                    async move { crate::utilities::download_image(&artwork_url).await },
                    move |result| match result {
                        Ok(handle) => {
                            Message::UserPage(Mu::PlaylistImageLoaded(playlist_urn.clone(), handle))
                        }
                        Err(_) => {
                            Message::UserPage(Mu::PlaylistImageLoadFailed(playlist_urn.clone()))
                        }
                    },
                )
            })
            .collect()
    }
}

impl Page for UserPage {
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
    }

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
                    self.playlists_next_href = profile.playlists_next_href.clone();
                    self.tracks_next_href = profile.tracks_next_href.clone();
                    self.tracks_loading = false;
                    self.playlists_loading = false;
                    self.track_list.set_tracks(profile.tracks);

                    // Track artwork loads lazily per row via RequestTrackImage; the
                    // playlist thumbnails are fetched eagerly here.
                    let playlist_image_tasks = Self::playlist_image_tasks(&self.playlists);
                    return (None, Task::batch(playlist_image_tasks));
                }
                UserPageMessage::LoadMoreTracks => {
                    if self.tracks_loading || self.tracks_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.tracks_loading = true;
                    let token_manager = self.token_manager.clone();
                    let user_urn = self.user_urn.clone();
                    let next_href = self.tracks_next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::get_user_tracks_with_refresh(
                                token_manager,
                                user_urn,
                                next_href,
                            ),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::UserPage(
                                    Mu::MoreTracksLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::UserPage(
                                    Mu::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                UserPageMessage::MoreTracksLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.tracks_loading = false;
                    self.tracks_next_href = tracks.next_href.clone();
                    self.track_list.append_tracks(tracks.collection);
                    return (None, Task::none());
                }
                UserPageMessage::LoadMorePlaylists => {
                    if self.playlists_loading || self.playlists_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.playlists_loading = true;
                    let token_manager = self.token_manager.clone();
                    let user_urn = self.user_urn.clone();
                    let next_href = self.playlists_next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::get_user_playlists_with_refresh(
                                token_manager,
                                user_urn,
                                next_href,
                            ),
                            |result| match result {
                                Ok((playlists, token_manager)) => Message::UserPage(
                                    Mu::MorePlaylistsLoadedWithToken(playlists, token_manager),
                                ),
                                Err((error, token_manager)) => Message::UserPage(
                                    Mu::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                UserPageMessage::MorePlaylistsLoadedWithToken(playlists, token_manager) => {
                    self.token_manager = token_manager;
                    self.playlists_loading = false;
                    self.playlists_next_href = playlists.next_href.clone();
                    let image_tasks = Self::playlist_image_tasks(&playlists.collection);
                    self.playlists.extend(playlists.collection);
                    return (None, Task::batch(image_tasks));
                }
                UserPageMessage::RequestTrackImage(track_id) => {
                    return (
                        None,
                        self.track_list.load_image_task(
                            track_id,
                            |id, handle| Message::UserPage(Mu::TrackImageLoaded(id, handle)),
                            |id| Message::UserPage(Mu::TrackImageLoadFailed(id)),
                        ),
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
                UserPageMessage::ApiErrorWithToken(_error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    self.tracks_loading = false;
                    self.playlists_loading = false;
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
            let (page, task) = LikesPage::new(self.token_manager.clone());
            return (Some(Box::new(page)), task);
        }

        if let Message::NavigateToSearch = message {
            return (
                Some(Box::new(SearchPage::new(self.token_manager.clone()))),
                Task::none(),
            );
        }

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let mut tracks_column = self.track_list.render_tracks(
            |t| Message::UserPage(UserPageMessage::PlayTrack(t)),
            |urn| Message::UserPage(UserPageMessage::NavigateToUser(urn)),
            |t| Message::UserPage(UserPageMessage::LikeTrack(t)),
            |id| Message::UserPage(UserPageMessage::RequestTrackImage(id)),
        );
        if self.tracks_next_href.is_some() {
            // Bottom sentinel: loads the next page of tracks when scrolled near the end.
            tracks_column = tracks_column.push(
                sensor(text("Loading more tracks..."))
                    .on_show(|_| Message::UserPage(Mu::LoadMoreTracks))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.track_list.tracks().len()),
            );
        }

        // Responsive grid of playlist cards: column count adapts to available width.
        let playlist_cells = self.playlists.iter().map(|playlist| {
            let image_handle = self.playlist_images.get(&playlist.user.urn).cloned();
            iced::Element::from(get_playlist_widget(playlist, image_handle, |urn| {
                Message::UserPage(UserPageMessage::LoadPlaylist(urn))
            }))
        });
        let playlists_grid = grid(playlist_cells)
            .fluid(300)
            .spacing(10)
            .height(Length::Shrink);
        let mut playlists_content = column![playlists_grid];
        if self.playlists_next_href.is_some() {
            // Bottom sentinel: loads the next page of playlists when scrolled near the end.
            playlists_content = playlists_content.push(
                sensor(text("Loading more playlists..."))
                    .on_show(|_| Message::UserPage(Mu::LoadMorePlaylists))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.playlists.len()),
            );
        }

        let mut content = column![];
        if self.track_load_failed {
            content =
                content.push(text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)));
        }
        content
            .push(
                row![
                    Scrollable::new(tracks_column)
                        .style(crate::widgets::scrollbar_style)
                        .height(Length::FillPortion(1))
                        .width(Length::FillPortion(1)),
                    Scrollable::new(playlists_content)
                        .style(crate::widgets::scrollbar_style)
                        .height(Length::FillPortion(1))
                        .width(Length::FillPortion(1)),
                ]
                .spacing(10),
            )
            .into()
    }
}
