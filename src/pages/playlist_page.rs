use crate::Message;
use crate::Page;
use crate::managers::TrackListManager;
use crate::models::SoundCloudPlaylist;
use crate::models::SoundCloudTrack;
use crate::models::SoundCloudTracks;
use crate::pages::LikesPage;
use crate::pages::UserPage;
use crate::pages::{FeedPage, SearchPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use iced::Color;
use iced::Length;
use iced::Task;
use iced::widget::image::Handle;
use iced::widget::{Scrollable, sensor, text};
use tracing::debug;

// Start loading the next page when the bottom sentinel is within 500px of the viewport
const LOAD_MORE_THRESHOLD: f32 = 500.0;

#[derive(Debug, Clone)]
pub enum PlaylistPageMessage {
    LoadPlaylist,
    LoadMoreTracks,
    TracksLoadedWithToken(SoundCloudTracks, TokenManager),
    RequestImage(u64),
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle),
    ImageLoadFailed(u64),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
}

type Mp = PlaylistPageMessage;

pub struct PlaylistPage {
    token_manager: TokenManager,
    playlist_urn: String,
    track_list: TrackListManager,
    tracks_next_href: Option<String>,
    tracks_loading: bool,
    track_load_failed: bool,
}

impl PlaylistPage {
    pub fn new(token_manager: TokenManager, playlist: SoundCloudPlaylist) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                playlist_urn: playlist.urn,
                track_list: TrackListManager::new(),
                tracks_next_href: None,
                tracks_loading: false,
                track_load_failed: false,
            },
            Task::done(Message::PlaylistPage(PlaylistPageMessage::LoadPlaylist)),
        )
    }
}

impl Page for PlaylistPage {
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
    }

    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::PlaylistPage(msg) = message {
            match msg {
                PlaylistPageMessage::LoadPlaylist => {
                    // Fetch the playlist's tracks (first page) from the API.
                    self.tracks_loading = true;
                    let token_manager = self.token_manager.clone();
                    let playlist_urn = self.playlist_urn.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::get_playlist_tracks_with_refresh(
                                token_manager,
                                playlist_urn,
                                None,
                            ),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::PlaylistPage(
                                    Mp::TracksLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::PlaylistPage(
                                    Mp::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PlaylistPageMessage::LoadMoreTracks => {
                    if self.tracks_loading || self.tracks_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.tracks_loading = true;
                    let token_manager = self.token_manager.clone();
                    let playlist_urn = self.playlist_urn.clone();
                    let next_href = self.tracks_next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::get_playlist_tracks_with_refresh(
                                token_manager,
                                playlist_urn,
                                next_href,
                            ),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::PlaylistPage(
                                    Mp::TracksLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::PlaylistPage(
                                    Mp::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PlaylistPageMessage::TracksLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.tracks_loading = false;
                    self.tracks_next_href = tracks.next_href.clone();
                    if self.track_list.tracks().is_empty() {
                        self.track_list.set_tracks(tracks.collection);
                    } else {
                        self.track_list.append_tracks(tracks.collection);
                    }
                    return (None, Task::none());
                }
                PlaylistPageMessage::RequestImage(track_id) => {
                    return (
                        None,
                        self.track_list.load_image_task(
                            track_id,
                            |id, handle| Message::PlaylistPage(Mp::ImageLoaded(id, handle)),
                            |id| Message::PlaylistPage(Mp::ImageLoadFailed(id)),
                        ),
                    );
                }
                PlaylistPageMessage::PlayTrack(track) => {
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
                PlaylistPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => Message::PlaylistPage(
                                    Mp::TrackLikedWithToken(track_id, token_manager),
                                ),
                                Err((error, token_manager)) => Message::PlaylistPage(
                                    Mp::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                PlaylistPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    debug!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                PlaylistPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    self.tracks_loading = false;
                    debug!("API Error: {}", error_msg);
                    return (None, Task::none());
                }
                PlaylistPageMessage::ImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                PlaylistPageMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                PlaylistPageMessage::LoadUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
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
        use iced::widget::column;

        let mut tracks_column = self.track_list.render_tracks(
            |t| Message::PlaylistPage(PlaylistPageMessage::PlayTrack(t)),
            |urn| Message::PlaylistPage(PlaylistPageMessage::LoadUser(urn)),
            |t| Message::PlaylistPage(PlaylistPageMessage::LikeTrack(t)),
            |id| Message::PlaylistPage(PlaylistPageMessage::RequestImage(id)),
        );
        if self.tracks_next_href.is_some() {
            // Bottom sentinel: loads the next page of tracks when scrolled near the end.
            tracks_column = tracks_column.push(
                sensor(text("Loading more tracks..."))
                    .on_show(|_| Message::PlaylistPage(Mp::LoadMoreTracks))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.track_list.tracks().len()),
            );
        }

        let mut content = column![];
        if self.track_load_failed {
            content =
                content.push(text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)));
        }
        content
            .push(
                Scrollable::new(tracks_column)
                    .style(crate::widgets::scrollbar_style)
                    .height(Length::FillPortion(1))
                    .width(Length::FillPortion(1)),
            )
            .into()
    }
}
