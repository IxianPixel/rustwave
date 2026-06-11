use crate::managers::TrackListManager;
use crate::models::{
    SearchResults, SoundCloudPlaylist, SoundCloudPlaylists, SoundCloudTrack, SoundCloudTracks,
    SoundCloudUser,
};
use crate::pages::{LikesPage, PlaylistPage, UserPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use crate::widgets::{get_playlist_widget, get_user_widget};
use crate::{Message, Page};
use iced::widget::image::Handle;
use iced::widget::{Scrollable, column, grid, row, sensor, text, text_input};
use iced::{Length, Task};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum SearchPageMessage {
    SearchPressed(String),
    Search(String),
    SearchCompletedWithToken(SearchResults, TokenManager),
    LoadMoreTracks,
    LoadMorePlaylists,
    MoreTracksLoadedWithToken(SoundCloudTracks, TokenManager),
    MorePlaylistsLoadedWithToken(SoundCloudPlaylists, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    UserImageLoaded(String, Handle),
    UserImageLoadFailed(String),
    RequestTrackImage(u64),
    TrackImageLoaded(u64, Handle),
    TrackImageLoadFailed(u64),
    PlayTrack(SoundCloudTrack),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    LoadUser(String),
    LoadPlaylist(SoundCloudPlaylist),
}

type Ms = SearchPageMessage;

// Start loading the next page when the bottom sentinel is within 500px of the viewport
const LOAD_MORE_THRESHOLD: f32 = 500.0;

pub struct SearchPage {
    token_manager: TokenManager,
    search_query: String,
    user_load_failed: bool,
    user_images: HashMap<String, Handle>,
    users: Vec<SoundCloudUser>,
    track_list: TrackListManager,
    tracks_next_href: Option<String>,
    tracks_loading: bool,
    playlists: Vec<SoundCloudPlaylist>,
    playlists_next_href: Option<String>,
    playlists_loading: bool,
}

impl SearchPage {
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            token_manager,
            search_query: String::new(),
            user_load_failed: false,
            user_images: HashMap::new(),
            users: Vec::new(),
            track_list: TrackListManager::new(),
            tracks_next_href: None,
            tracks_loading: false,
            playlists: Vec::new(),
            playlists_next_href: None,
            playlists_loading: false,
        }
    }
}

impl Page for SearchPage {
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
    }

    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::SearchPage(msg) = message {
            match msg {
                SearchPageMessage::SearchPressed(query) => {
                    self.search_query = query.clone();
                    return (None, Task::none());
                }
                SearchPageMessage::Search(query) => {
                    self.search_query = query.clone();
                    let token_manager = self.token_manager.clone();
                    let search_query = self.search_query.clone();

                    return (
                        None,
                        Task::perform(
                            api_helpers::search_with_refresh(token_manager, search_query),
                            |result| match result {
                                Ok((results, token_manager)) => Message::SearchPage(
                                    Ms::SearchCompletedWithToken(results, token_manager),
                                ),
                                Err((error, token_manager)) => Message::SearchPage(
                                    Ms::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                SearchPageMessage::SearchCompletedWithToken(results, token_manager) => {
                    self.token_manager = token_manager;
                    self.user_load_failed = false;
                    self.users = results.users.clone();
                    self.playlists = results.playlists.clone();
                    self.playlists_next_href = results.playlists_next_href.clone();
                    self.tracks_next_href = results.tracks_next_href.clone();
                    self.tracks_loading = false;
                    self.playlists_loading = false;
                    self.track_list.set_tracks(results.tracks);

                    // Create tasks to load images for all users
                    let image_tasks: Vec<Task<Message>> = self
                        .users
                        .iter()
                        .map(|user| {
                            let user_urn = user.urn.clone();
                            let artwork_url = user.avatar_url.clone();
                            Task::perform(
                                async move { crate::utilities::download_image(&artwork_url).await },
                                move |result| match result {
                                    Ok(handle) => Message::SearchPage(Ms::UserImageLoaded(
                                        user_urn.clone(),
                                        handle,
                                    )),
                                    Err(_) => Message::SearchPage(Ms::UserImageLoadFailed(
                                        user_urn.clone(),
                                    )),
                                },
                            )
                        })
                        .collect();

                    // Track artwork now loads lazily per row via RequestTrackImage.
                    return (None, Task::batch(image_tasks));
                }
                SearchPageMessage::LoadMoreTracks => {
                    if self.tracks_loading || self.tracks_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.tracks_loading = true;
                    let token_manager = self.token_manager.clone();
                    let query = self.search_query.clone();
                    let next_href = self.tracks_next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::search_tracks_with_refresh(
                                token_manager,
                                query,
                                next_href,
                            ),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::SearchPage(
                                    Ms::MoreTracksLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::SearchPage(
                                    Ms::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                SearchPageMessage::MoreTracksLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.tracks_loading = false;
                    self.tracks_next_href = tracks.next_href.clone();
                    self.track_list.append_tracks(tracks.collection);
                    return (None, Task::none());
                }
                SearchPageMessage::LoadMorePlaylists => {
                    if self.playlists_loading || self.playlists_next_href.is_none() {
                        return (None, Task::none());
                    }
                    self.playlists_loading = true;
                    let token_manager = self.token_manager.clone();
                    let query = self.search_query.clone();
                    let next_href = self.playlists_next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::search_playlists_with_refresh(
                                token_manager,
                                query,
                                next_href,
                            ),
                            |result| match result {
                                Ok((playlists, token_manager)) => Message::SearchPage(
                                    Ms::MorePlaylistsLoadedWithToken(playlists, token_manager),
                                ),
                                Err((error, token_manager)) => Message::SearchPage(
                                    Ms::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                SearchPageMessage::MorePlaylistsLoadedWithToken(playlists, token_manager) => {
                    self.token_manager = token_manager;
                    self.playlists_loading = false;
                    self.playlists_next_href = playlists.next_href.clone();
                    self.playlists.extend(playlists.collection);
                    return (None, Task::none());
                }
                SearchPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.user_load_failed = true;
                    self.tracks_loading = false;
                    self.playlists_loading = false;
                    debug!("API Error: {}", error_msg);
                    return (None, Task::none());
                }
                SearchPageMessage::UserImageLoaded(user_urn, handle) => {
                    self.user_images.insert(user_urn, handle);
                    return (None, Task::none());
                }
                SearchPageMessage::UserImageLoadFailed(user_urn) => {
                    debug!("Failed to load image for user {}", user_urn);
                    return (None, Task::none());
                }
                SearchPageMessage::RequestTrackImage(track_id) => {
                    return (
                        None,
                        self.track_list.load_image_task(
                            track_id,
                            |id, handle| Message::SearchPage(Ms::TrackImageLoaded(id, handle)),
                            |id| Message::SearchPage(Ms::TrackImageLoadFailed(id)),
                        ),
                    );
                }
                SearchPageMessage::TrackImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                SearchPageMessage::TrackImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                SearchPageMessage::PlayTrack(track) => {
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
                SearchPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => Message::SearchPage(
                                    Ms::TrackLikedWithToken(track_id, token_manager),
                                ),
                                Err((error, token_manager)) => Message::SearchPage(
                                    Ms::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                SearchPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    debug!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                SearchPageMessage::LoadUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
                }
                SearchPageMessage::LoadPlaylist(playlist) => {
                    let (playlist_page, task) =
                        PlaylistPage::new(self.token_manager.clone(), playlist);
                    return (Some(Box::new(playlist_page)), task);
                }
            }
        }

        if let Message::NavigateToLikes = message {
            let (page, task) = LikesPage::new(self.token_manager.clone());
            return (Some(Box::new(page)), task);
        }

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let mut indices: Vec<usize> = (0..self.users.len()).collect();
        indices.sort_by(|&a, &b| {
            let count_a = self.users[a].followers_count.unwrap_or(0);
            let count_b = self.users[b].followers_count.unwrap_or(0);
            count_b.cmp(&count_a)
        });

        // Responsive grid of user cards: column count adapts to available width.
        let user_cells = indices.iter().map(|&idx| {
            let user = &self.users[idx];
            let image_handle = self.user_images.get(&user.urn).cloned();
            iced::Element::from(get_user_widget(user, image_handle, |urn| {
                Message::SearchPage(SearchPageMessage::LoadUser(urn))
            }))
        });
        let users_grid = grid(user_cells)
            .fluid(300)
            .spacing(10)
            .height(Length::Shrink);

        let mut tracks_column = self.track_list.render_tracks(
            |t| Message::SearchPage(SearchPageMessage::PlayTrack(t)),
            |urn| Message::SearchPage(SearchPageMessage::LoadUser(urn)),
            |t| Message::SearchPage(SearchPageMessage::LikeTrack(t)),
            |id| Message::SearchPage(SearchPageMessage::RequestTrackImage(id)),
        );
        if self.tracks_next_href.is_some() {
            // Bottom sentinel: loads the next page of tracks when scrolled near the end.
            tracks_column = tracks_column.push(
                sensor(text("Loading more tracks..."))
                    .on_show(|_| Message::SearchPage(Ms::LoadMoreTracks))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.track_list.tracks().len()),
            );
        }

        let playlist_cells = self.playlists.iter().map(|playlist| {
            let image_handle = self.user_images.get(&playlist.user.urn).cloned();
            iced::Element::from(get_playlist_widget(playlist, image_handle, |urn| {
                Message::SearchPage(SearchPageMessage::LoadPlaylist(urn))
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
                    .on_show(|_| Message::SearchPage(Ms::LoadMorePlaylists))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.playlists.len()),
            );
        }

        column![
            row![
                text_input("Search", self.search_query.as_str())
                    .on_submit(Message::SearchPage(Ms::Search(self.search_query.clone())))
                    .on_input(|s| Message::SearchPage(Ms::SearchPressed(s))),
            ]
            .spacing(10),
            row![users_grid].spacing(10),
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
        ]
        .into()
    }
}
