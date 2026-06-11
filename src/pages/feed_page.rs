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
use iced::Vector;
use iced::advanced::widget::{Id, operate, operation};
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Scrollable, button, float, sensor, stack, text};
use tracing::debug;

#[derive(Debug, Clone)]
pub enum FeedPageMessage {
    LoadFeed,
    LoadMoreFeed,
    ScrollToTop,
    RequestImage(u64),
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

// Start loading the next page when the bottom sentinel is within 500px of the viewport
const LOAD_MORE_THRESHOLD: f32 = 500.0;
// Stable id linking the track Scrollable to its scroll-to-top button.
const SCROLL_ID: &str = "feed_scroll";

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
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
    }

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
                FeedPageMessage::ScrollToTop => {
                    return (
                        None,
                        operate(operation::scrollable::scroll_to(
                            Id::new(SCROLL_ID),
                            AbsoluteOffset {
                                x: Some(0.0),
                                y: Some(0.0),
                            },
                        )),
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
                FeedPageMessage::FeedCollectionLoadedWithToken(collection, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.is_loading = false;

                    // Store the next_href for pagination
                    self.next_href = collection.next_href.clone();

                    // Extract tracks from activities
                    let tracks: Vec<SoundCloudTrack> = collection
                        .collection
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

                    // Artwork now loads lazily per row via RequestImage; nothing to do here.
                    return (None, Task::none());
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
                FeedPageMessage::RequestImage(track_id) => {
                    return (
                        None,
                        self.track_list.load_image_task(
                            track_id,
                            |id, handle| Message::FeedPage(Mf::ImageLoaded(id, handle)),
                            |id| Message::FeedPage(Mf::ImageLoadFailed(id)),
                        ),
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
            |id| Message::FeedPage(FeedPageMessage::RequestImage(id)),
        );

        if self.next_href.is_some() {
            // Bottom sentinel: fires LoadMoreFeed when scrolled near the end.
            // Keyed on the track count so it re-triggers after each page is appended.
            tracks_column = tracks_column.push(
                sensor(text("Loading more tracks..."))
                    .on_show(|_| Message::FeedPage(Mf::LoadMoreFeed))
                    .anticipate(LOAD_MORE_THRESHOLD)
                    .key(self.track_list.tracks().len()),
            );
        } else if self.is_loading {
            // Initial load (no next page known yet)
            tracks_column = tracks_column.push(text("Loading tracks..."));
        }

        let mut content = column![];
        if self.track_load_failed {
            content =
                content.push(text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)));
        }
        content = content.push(
            Scrollable::new(tracks_column)
                .id(SCROLL_ID)
                .style(crate::widgets::scrollbar_style)
                .height(Length::FillPortion(1))
                .width(Length::FillPortion(1)),
        );

        if self.track_list.tracks().is_empty() {
            return content.into();
        }

        // Floating "scroll to top" button, anchored to the bottom-right of the list.
        let fab = float(
            button(
                text("↑")
                    .size(22)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            )
            .width(44)
            .height(44)
            .padding(0)
            .on_press(Message::FeedPage(Mf::ScrollToTop)),
        )
        .translate(|bounds, viewport| {
            let margin = 24.0;
            Vector::new(
                (viewport.x + viewport.width - margin - bounds.width) - bounds.x,
                (viewport.y + viewport.height - margin - bounds.height) - bounds.y,
            )
        });

        stack![content, fab].into()
    }
}
