use iced::widget::image::Handle;

use crate::Message;
use crate::Page;
use crate::managers::TrackListManager;
use crate::models::{SoundCloudActivityCollection, SoundCloudTrack};
use crate::pages::UserPage;
use crate::pages::{LikesPage, SearchPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use iced::Color;
use iced::Length;
use iced::Task;
use iced::widget::{Scrollable, row, text};
use iced::widget::scrollable::Viewport;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum FeedPageMessage {
    LoadFeed,
    LoadMoreFeed,
    Scrolled(Viewport),
    FeedLoadedWithToken(Vec<SoundCloudTrack>, TokenManager),
    FeedCollectionLoadedWithToken(SoundCloudActivityCollection, TokenManager),
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle),
    ImageLoadFailed(u64),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
}

type Mf = FeedPageMessage;

// Trigger loading more content when within 500 pixels of the bottom
const LOAD_MORE_THRESHOLD: f32 = 500.0;

pub struct FeedPage {
    token_manager: TokenManager,
    track_list: TrackListManager,
    track_load_failed: bool,
    next_href: Option<String>,
    is_loading: bool,
}

impl FeedPage {
    pub fn new(token_manager: TokenManager) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                track_list: TrackListManager::new(),
                track_load_failed: false,
                next_href: None,
                is_loading: false,
            },
            Task::done(Message::FeedPage(FeedPageMessage::LoadFeed)),
        )
    }
}

impl Page for FeedPage {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::FeedPage(msg) = message {
            match msg {
                FeedPageMessage::LoadFeed => {
                    self.is_loading = true;
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_feed_paginated_with_refresh(token_manager, None),
                            |result| match result {
                                Ok((collection, token_manager)) => Message::FeedPage(
                                    Mf::FeedCollectionLoadedWithToken(collection, token_manager),
                                ),
                                Err((error, token_manager)) => Message::FeedPage(
                                    Mf::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                FeedPageMessage::LoadMoreFeed => {
                    // Don't load if already loading or no next page
                    if self.is_loading || self.next_href.is_none() {
                        return (None, Task::none());
                    }

                    self.is_loading = true;
                    let token_manager = self.token_manager.clone();
                    let next_href = self.next_href.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_feed_paginated_with_refresh(token_manager, next_href),
                            |result| match result {
                                Ok((collection, token_manager)) => Message::FeedPage(
                                    Mf::FeedCollectionLoadedWithToken(collection, token_manager),
                                ),
                                Err((error, token_manager)) => Message::FeedPage(
                                    Mf::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                FeedPageMessage::Scrolled(viewport) => {
                    // Check if we're near the bottom of the scrollable area
                    let offset = viewport.absolute_offset();
                    let viewport_height = viewport.bounds().height;
                    let content_height = viewport.content_bounds().height;

                    // Trigger load if within threshold of the bottom
                    if offset.y + viewport_height + LOAD_MORE_THRESHOLD >= content_height {
                        // Don't load if already loading or no next page
                        if !self.is_loading && self.next_href.is_some() {
                            return (
                                None,
                                Task::done(Message::FeedPage(Mf::LoadMoreFeed)),
                            );
                        }
                    }

                    return (None, Task::none());
                }
                FeedPageMessage::FeedLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.track_list.set_tracks(tracks);

                    // Create tasks to load images for all tracks
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::FeedPage(Mf::ImageLoaded(track_id, handle)),
                        |track_id| Message::FeedPage(Mf::ImageLoadFailed(track_id)),
                    );

                    return (None, Task::batch(image_tasks));
                }
                FeedPageMessage::FeedCollectionLoadedWithToken(collection, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.is_loading = false;

                    // Store the next_href for pagination
                    self.next_href = collection.next_href.clone();

                    // Extract tracks from activities
                    let tracks: Vec<SoundCloudTrack> = collection.collection
                        .into_iter()
                        .map(|activity| activity.origin)
                        .collect();

                    // Determine if this is initial load or pagination
                    let is_initial_load = self.track_list.tracks().is_empty();

                    if is_initial_load {
                        // Initial load: replace tracks
                        self.track_list.set_tracks(tracks);
                    } else {
                        // Pagination: append tracks
                        self.track_list.append_tracks(tracks);
                    }

                    // Create tasks to load images for all tracks
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::FeedPage(Mf::ImageLoaded(track_id, handle)),
                        |track_id| Message::FeedPage(Mf::ImageLoadFailed(track_id)),
                    );

                    return (None, Task::batch(image_tasks));
                }
                FeedPageMessage::PlayTrack(track) => {
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
                FeedPageMessage::ImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none());
                }
                FeedPageMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none());
                }
                FeedPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::like_track_with_refresh(token_manager, track.clone()),
                            move |result| match result {
                                Ok((track_id, token_manager)) => Message::FeedPage(
                                    Mf::TrackLikedWithToken(track_id, token_manager),
                                ),
                                Err((error, token_manager)) => Message::FeedPage(
                                    Mf::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
                }
                FeedPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    debug!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                FeedPageMessage::ApiErrorWithToken(_error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    self.is_loading = false;
                    return (None, Task::none());
                }
                FeedPageMessage::LoadUser(user_urn) => {
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
                }
            }
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
            |t| Message::FeedPage(FeedPageMessage::PlayTrack(t)),
            |urn| Message::FeedPage(FeedPageMessage::LoadUser(urn)),
            |t| Message::FeedPage(FeedPageMessage::LikeTrack(t)),
        );

        // Add loading indicator at the bottom
        if self.is_loading {
            tracks_column = tracks_column.push(text("Loading more tracks..."));
        }

        column![
            row![if self.track_load_failed {
                text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0))
            } else {
                text("")
            }],
            Scrollable::new(tracks_column)
                .height(Length::FillPortion(1))
                .width(Length::FillPortion(1))
                .on_scroll(|viewport| Message::FeedPage(Mf::Scrolled(viewport))),
        ]
        .into()
    }
}
