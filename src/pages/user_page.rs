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
use crate::utilities::{NumberFormat, get_asset_path};
use crate::widgets::{empty_state, get_playlist_widget, loading_state, section, spinner};
use crate::{Message, Page};
use iced::widget::image::{self, Handle};
use iced::widget::{Container, Scrollable, column, container, grid, row, sensor, text};
use iced::{Alignment, Font, Length};

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
    AvatarImageLoaded(Handle),
    AvatarImageLoadFailed,
    ApiErrorWithToken(String, TokenManager),
    RequestTrackImage(u64),
    TrackImageLoaded(u64, Handle),
    TrackImageLoadFailed(u64),
    PlayTrack(SoundCloudTrack),
    NavigateToUser(String),
    LoadPlaylist(SoundCloudPlaylist),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    LoadMoreLikedTracks,
    MoreLikedTracksLoadedWithToken(SoundCloudTracks, TokenManager),
    LikedTracksLoadFailedWithToken(String, TokenManager),
    RequestLikedTrackImage(u64),
    LikedTrackImageLoaded(u64, Handle),
    LikedTrackImageLoadFailed(u64),
    PlayLikedTrack(SoundCloudTrack),
    LoadMoreRepostedTracks,
    MoreRepostedTracksLoadedWithToken(SoundCloudTracks, TokenManager),
    RepostedTracksLoadFailedWithToken(String, TokenManager),
    RequestRepostedTrackImage(u64),
    RepostedTrackImageLoaded(u64, Handle),
    RepostedTrackImageLoadFailed(u64),
    PlayRepostedTrack(SoundCloudTrack),
}

type Mu = UserPageMessage;

pub struct UserPage {
    token_manager: TokenManager,
    user_urn: String,
    user: SoundCloudUser,
    avatar_image: Option<Handle>,
    playlists: Vec<SoundCloudPlaylist>,
    playlist_images: HashMap<String, Handle>,
    playlists_next_href: Option<String>,
    playlists_loading: bool,
    track_list: TrackListManager,
    tracks_next_href: Option<String>,
    tracks_loading: bool,
    track_load_failed: bool,
    liked_list: TrackListManager,
    liked_next_href: Option<String>,
    liked_loading: bool,
    liked_load_failed: bool,
    reposted_list: TrackListManager,
    reposted_next_href: Option<String>,
    reposted_loading: bool,
    reposted_load_failed: bool,
}

impl UserPage {
    pub fn new(token_manager: TokenManager, user_urn: String) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                user_urn,
                user: SoundCloudUser::default(),
                avatar_image: None,
                playlists: Vec::new(),
                playlist_images: HashMap::new(),
                playlists_next_href: None,
                playlists_loading: false,
                track_list: TrackListManager::new(),
                tracks_next_href: None,
                tracks_loading: false,
                track_load_failed: false,
                liked_list: TrackListManager::new(),
                liked_next_href: None,
                liked_loading: false,
                liked_load_failed: false,
                reposted_list: TrackListManager::new(),
                reposted_next_href: None,
                reposted_loading: false,
                reposted_load_failed: false,
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

    /// Fetches a page of the user's liked tracks (initial load when
    /// `next_href` is `None`, pagination otherwise).
    fn fetch_liked_tracks_task(&self, next_href: Option<String>) -> Task<Message> {
        Task::perform(
            api_helpers::get_user_liked_tracks_with_refresh(
                self.token_manager.clone(),
                self.user_urn.clone(),
                next_href,
            ),
            |result| match result {
                Ok((tracks, token_manager)) => {
                    Message::UserPage(Mu::MoreLikedTracksLoadedWithToken(tracks, token_manager))
                }
                Err((error, token_manager)) => Message::UserPage(
                    Mu::LikedTracksLoadFailedWithToken(error.to_string(), token_manager),
                ),
            },
        )
    }

    /// Fetches a page of the user's reposted tracks (initial load when
    /// `next_href` is `None`, pagination otherwise).
    fn fetch_reposted_tracks_task(&self, next_href: Option<String>) -> Task<Message> {
        Task::perform(
            api_helpers::get_user_reposted_tracks_with_refresh(
                self.token_manager.clone(),
                self.user_urn.clone(),
                next_href,
            ),
            |result| match result {
                Ok((tracks, token_manager)) => {
                    Message::UserPage(Mu::MoreRepostedTracksLoadedWithToken(tracks, token_manager))
                }
                Err((error, token_manager)) => Message::UserPage(
                    Mu::RepostedTracksLoadFailedWithToken(error.to_string(), token_manager),
                ),
            },
        )
    }

    /// Builds a quadrant panel around a track list, handling the error,
    /// loading, and empty states plus the pagination sentinel. The Tracks,
    /// Likes, and Reposts quadrants only differ in state and messages.
    #[allow(clippy::too_many_arguments)]
    fn track_list_panel<'a>(
        &'a self,
        title: &'a str,
        list: &'a TrackListManager,
        has_more: bool,
        loading: bool,
        load_failed: bool,
        empty_title: &str,
        empty_subtitle: &str,
        on_play: fn(SoundCloudTrack) -> UserPageMessage,
        on_request_image: fn(u64) -> UserPageMessage,
        load_more: UserPageMessage,
    ) -> Container<'a, Message> {
        let body: iced::Element<'a, Message> = if load_failed {
            empty_state(
                None,
                format!("Couldn't load {}", title.to_lowercase()),
                "Something went wrong talking to SoundCloud".to_string(),
            )
        } else if list.tracks().is_empty() {
            if loading {
                loading_state()
            } else {
                empty_state(None, empty_title.to_string(), empty_subtitle.to_string())
            }
        } else {
            let mut tracks_column = list.render_tracks(
                move |t| Message::UserPage(on_play(t)),
                |urn| Message::UserPage(UserPageMessage::NavigateToUser(urn)),
                |t| Message::UserPage(UserPageMessage::LikeTrack(t)),
                move |id| Message::UserPage(on_request_image(id)),
            );
            if has_more {
                // Bottom sentinel: loads the next page when scrolled near the end.
                tracks_column = tracks_column.push(
                    sensor(container(spinner(24.0)).center_x(Length::Fill).padding(8))
                        .on_show(move |_| Message::UserPage(load_more.clone()))
                        .anticipate(LOAD_MORE_THRESHOLD)
                        .key(list.tracks().len()),
                );
            }
            Scrollable::new(tracks_column)
                .style(crate::widgets::scrollbar_style)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };
        section(title, badge_label(list.tracks().len(), has_more), body)
    }
}

impl Page for UserPage {
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
            || self.liked_list.is_animating()
            || self.reposted_list.is_animating()
            // Keep frames flowing while any loading spinner is on screen.
            || self.user.urn.is_empty()
            || self.tracks_loading
            || self.playlists_loading
            || self.liked_loading
            || self.reposted_loading
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
                    // playlist thumbnails and the header avatar are fetched eagerly here.
                    let mut tasks = Self::playlist_image_tasks(&self.playlists);
                    let avatar_url = self.user.avatar_url.clone();
                    tasks.push(Task::perform(
                        async move { crate::utilities::download_image(&avatar_url).await },
                        |result| match result {
                            Ok(handle) => Message::UserPage(Mu::AvatarImageLoaded(handle)),
                            Err(_) => Message::UserPage(Mu::AvatarImageLoadFailed),
                        },
                    ));

                    // The liked/reposted panels load after the profile so they
                    // can reuse the freshly refreshed token.
                    self.liked_loading = true;
                    self.reposted_loading = true;
                    tasks.push(self.fetch_liked_tracks_task(None));
                    tasks.push(self.fetch_reposted_tracks_task(None));
                    return (None, Task::batch(tasks));
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
                UserPageMessage::AvatarImageLoaded(handle) => {
                    self.avatar_image = Some(handle);
                    return (None, Task::none());
                }
                UserPageMessage::AvatarImageLoadFailed => {
                    // Header simply renders without an avatar.
                    debug!("Failed to load avatar for user {}", self.user.urn);
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
                UserPageMessage::LoadMoreLikedTracks => {
                    if self.liked_loading || self.liked_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.liked_loading = true;
                    let next_href = self.liked_next_href.clone();
                    return (None, self.fetch_liked_tracks_task(next_href));
                }
                UserPageMessage::MoreLikedTracksLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.liked_loading = false;
                    self.liked_next_href = tracks.next_href.clone();
                    self.liked_list.append_tracks(tracks.collection);
                    return (None, Task::none());
                }
                UserPageMessage::LikedTracksLoadFailedWithToken(error_msg, token_manager) => {
                    debug!("Failed to load liked tracks: {}", error_msg);
                    self.token_manager = token_manager;
                    self.liked_loading = false;
                    self.liked_load_failed = true;
                    return (None, Task::none());
                }
                UserPageMessage::RequestLikedTrackImage(track_id) => {
                    return (
                        None,
                        self.liked_list.load_image_task(
                            track_id,
                            |id, handle| Message::UserPage(Mu::LikedTrackImageLoaded(id, handle)),
                            |id| Message::UserPage(Mu::LikedTrackImageLoadFailed(id)),
                        ),
                    );
                }
                UserPageMessage::LikedTrackImageLoaded(track_id, handle) => {
                    self.liked_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                UserPageMessage::LikedTrackImageLoadFailed(track_id) => {
                    debug!("Failed to load image for liked track {}", track_id);
                    return (None, Task::none());
                }
                UserPageMessage::PlayLikedTrack(track) => {
                    self.liked_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(
                            track.clone(),
                            self.liked_list.tracks().clone(),
                            self.token_manager.clone(),
                        )),
                    );
                }
                UserPageMessage::LoadMoreRepostedTracks => {
                    if self.reposted_loading || self.reposted_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.reposted_loading = true;
                    let next_href = self.reposted_next_href.clone();
                    return (None, self.fetch_reposted_tracks_task(next_href));
                }
                UserPageMessage::MoreRepostedTracksLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.reposted_loading = false;
                    self.reposted_next_href = tracks.next_href.clone();
                    self.reposted_list.append_tracks(tracks.collection);
                    return (None, Task::none());
                }
                UserPageMessage::RepostedTracksLoadFailedWithToken(error_msg, token_manager) => {
                    debug!("Failed to load reposted tracks: {}", error_msg);
                    self.token_manager = token_manager;
                    self.reposted_loading = false;
                    self.reposted_load_failed = true;
                    return (None, Task::none());
                }
                UserPageMessage::RequestRepostedTrackImage(track_id) => {
                    return (
                        None,
                        self.reposted_list.load_image_task(
                            track_id,
                            |id, handle| {
                                Message::UserPage(Mu::RepostedTrackImageLoaded(id, handle))
                            },
                            |id| Message::UserPage(Mu::RepostedTrackImageLoadFailed(id)),
                        ),
                    );
                }
                UserPageMessage::RepostedTrackImageLoaded(track_id, handle) => {
                    self.reposted_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                UserPageMessage::RepostedTrackImageLoadFailed(track_id) => {
                    debug!("Failed to load image for reposted track {}", track_id);
                    return (None, Task::none());
                }
                UserPageMessage::PlayRepostedTrack(track) => {
                    self.reposted_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(
                            track.clone(),
                            self.reposted_list.tracks().clone(),
                            self.token_manager.clone(),
                        )),
                    );
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
        // An empty urn means the profile request hasn't completed yet.
        let profile_loading = self.user.urn.is_empty();

        // Header strip: avatar, username, follower count.
        let bold = Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        };
        let mut header = row![].spacing(12).align_y(Alignment::Center);
        if let Some(handle) = &self.avatar_image {
            header = header.push(iced::widget::image(handle.clone()).width(48).height(48));
        }
        header = header.push(column![
            text(self.user.username.clone())
                .size(24)
                .font(bold)
                .shaping(text::Shaping::Auto),
            text(format!(
                "{} followers",
                self.user
                    .followers_count
                    .unwrap_or(0)
                    .format_compact_number()
            ))
            .size(14)
            .style(text::secondary),
        ]);

        // Top-left: the user's own tracks.
        let tracks_panel = self.track_list_panel(
            "Tracks",
            &self.track_list,
            self.tracks_next_href.is_some(),
            profile_loading,
            self.track_load_failed,
            "No tracks",
            "This user hasn't posted any tracks",
            UserPageMessage::PlayTrack,
            UserPageMessage::RequestTrackImage,
            UserPageMessage::LoadMoreTracks,
        );

        // Top-right: the user's playlists.
        let playlists_body: iced::Element<'_, Message> = if self.playlists.is_empty() {
            if profile_loading {
                loading_state()
            } else {
                empty_state(
                    None,
                    "No playlists".to_string(),
                    "This user hasn't published any playlists".to_string(),
                )
            }
        } else {
            // Responsive grid of playlist cards: column count adapts to available width.
            let playlist_cells = self.playlists.iter().map(|playlist| {
                let image_handle = self.playlist_images.get(&playlist.user.urn).cloned();
                iced::Element::from(get_playlist_widget(playlist, image_handle, |urn| {
                    Message::UserPage(UserPageMessage::LoadPlaylist(urn))
                }))
            });
            let playlists_grid = grid(playlist_cells)
                .fluid(240)
                .spacing(10)
                .height(Length::Shrink);
            let mut playlists_content = column![playlists_grid];
            if self.playlists_next_href.is_some() {
                // Bottom sentinel: loads the next page of playlists when scrolled near the end.
                playlists_content = playlists_content.push(
                    sensor(container(spinner(24.0)).center_x(Length::Fill).padding(8))
                        .on_show(|_| Message::UserPage(Mu::LoadMorePlaylists))
                        .anticipate(LOAD_MORE_THRESHOLD)
                        .key(self.playlists.len()),
                );
            }
            Scrollable::new(playlists_content)
                .style(crate::widgets::scrollbar_style)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };
        let playlists_panel = section(
            "Playlists",
            badge_label(self.playlists.len(), self.playlists_next_href.is_some()),
            playlists_body,
        );

        // Bottom-left: tracks the user has liked. The fetch starts once the
        // profile loads, so the panel also reads as loading until then.
        let likes_panel = self.track_list_panel(
            "Likes",
            &self.liked_list,
            self.liked_next_href.is_some(),
            profile_loading || self.liked_loading,
            self.liked_load_failed,
            "No likes",
            "This user hasn't liked any tracks",
            UserPageMessage::PlayLikedTrack,
            UserPageMessage::RequestLikedTrackImage,
            UserPageMessage::LoadMoreLikedTracks,
        );

        // Bottom-right: tracks the user has reposted.
        let reposts_panel = self.track_list_panel(
            "Reposts",
            &self.reposted_list,
            self.reposted_next_href.is_some(),
            profile_loading || self.reposted_loading,
            self.reposted_load_failed,
            "No reposts",
            "This user hasn't reposted any tracks",
            UserPageMessage::PlayRepostedTrack,
            UserPageMessage::RequestRepostedTrackImage,
            UserPageMessage::LoadMoreRepostedTracks,
        );

        let top = row![tracks_panel, playlists_panel]
            .spacing(12)
            .height(Length::FillPortion(1));
        let bottom = row![likes_panel, reposts_panel]
            .spacing(12)
            .height(Length::FillPortion(1));

        column![header, top, bottom].spacing(12).into()
    }
}

/// Pill label for a section heading: the loaded count, with a `+` suffix
/// while more pages remain so the number never claims to be a total.
fn badge_label(loaded: usize, has_more: bool) -> Option<String> {
    if loaded == 0 {
        return None;
    }
    Some(if has_more {
        format!("{}+", loaded)
    } else {
        loaded.to_string()
    })
}
