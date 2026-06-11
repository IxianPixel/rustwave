use crate::managers::TrackListManager;
use crate::models::SoundCloudTrack;
use crate::pages::{FeedPage, SearchPage, UserPage};
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use crate::{Message, Page};
use iced::advanced::widget::{Id, operate, operation};
use iced::widget::image::Handle;
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Scrollable, button, column, float, sensor, stack, text};
use iced::{Color, Length, Task, Vector};

#[derive(Debug, Clone)]
pub enum LikesPageMessage {
    LoadFavourites,
    LoadMoreFavourites,
    ScrollToTop,
    RequestImage(u64),
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

// Start loading the next page when the bottom sentinel is within 500px of the viewport
const LOAD_MORE_THRESHOLD: f32 = 500.0;
// Stable id linking the track Scrollable to its scroll-to-top button.
const SCROLL_ID: &str = "likes_scroll";

pub struct LikesPage {
    token_manager: TokenManager,
    track_list: TrackListManager,
    track_load_failed: bool,
    next_href: Option<String>,
    is_loading: bool,
}

impl LikesPage {
    pub fn new(token_manager: TokenManager) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                track_list: TrackListManager::new(),
                track_load_failed: false,
                next_href: None,
                is_loading: false,
            },
            Task::done(Message::LikesPage(LikesPageMessage::LoadFavourites)),
        )
    }
}

impl Page for LikesPage {
    fn is_animating(&self) -> bool {
        self.track_list.is_animating()
    }

    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::LikesPage(msg) = message {
            match msg {
                LikesPageMessage::LoadFavourites => {
                    self.is_loading = true;
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_favourites_paginated_with_refresh(
                                token_manager,
                                None,
                            ),
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
                LikesPageMessage::LoadMoreFavourites => {
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
                            api_helpers::load_favourites_paginated_with_refresh(
                                token_manager,
                                next_href,
                            ),
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
                LikesPageMessage::ScrollToTop => {
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
                LikesPageMessage::RequestImage(track_id) => {
                    return (
                        None,
                        self.track_list.load_image_task(
                            track_id,
                            |id, handle| Message::LikesPage(Ml::ImageLoaded(id, handle)),
                            |id| Message::LikesPage(Ml::ImageLoadFailed(id)),
                        ),
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
                    self.is_loading = false;

                    // Store the next_href for pagination
                    self.next_href = soundcloud_tracks.next_href.clone();

                    // Determine if this is initial load or pagination
                    let is_initial_load = self.track_list.tracks().is_empty();

                    if is_initial_load {
                        // Initial load: replace tracks
                        self.track_list.set_tracks(soundcloud_tracks.collection);
                    } else {
                        // Pagination: append tracks
                        self.track_list.append_tracks(soundcloud_tracks.collection);
                    }

                    // Artwork now loads lazily per row via RequestImage; nothing to do here.
                    return (None, Task::none());
                }
                LikesPageMessage::TrackLikedWithToken(track_id, token_manager) => {
                    self.token_manager = token_manager;
                    println!("Track liked: {}", track_id);
                    return (None, Task::none());
                }
                LikesPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    self.is_loading = false;
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
        let mut tracks_column = self.track_list.render_tracks(
            |t| Message::LikesPage(Ml::PlayTrack(t)),
            |urn| Message::LikesPage(Ml::LoadUser(urn)),
            |t| Message::LikesPage(Ml::LikeTrack(t)),
            |id| Message::LikesPage(Ml::RequestImage(id)),
        );

        if self.next_href.is_some() {
            // Bottom sentinel: fires LoadMoreFavourites when scrolled near the end.
            // Keyed on the track count so it re-triggers after each page is appended.
            tracks_column = tracks_column.push(
                sensor(text("Loading more tracks..."))
                    .on_show(|_| Message::LikesPage(Ml::LoadMoreFavourites))
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
            .on_press(Message::LikesPage(Ml::ScrollToTop)),
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
