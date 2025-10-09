use iced::widget::image::Handle;

use crate::Message;
use crate::Page;
use crate::models::SoundCloudTrack;
use crate::pages::SearchPage;
use crate::pages::UserPage;
use crate::soundcloud::TokenManager;
use crate::soundcloud::api_helpers;
use crate::track_list_manager::TrackListManager;
use iced::Color;
use iced::Length;
use iced::Task;
use iced::widget::{Scrollable, row, text};
use tracing::debug;

#[derive(Debug, Clone)]
pub enum FeedPageMessage {
    LoadFeed,
    FeedLoadedWithToken(Vec<SoundCloudTrack>, TokenManager),
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle),
    ImageLoadFailed(u64),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
}

type Mf = FeedPageMessage;

pub struct FeedPage {
    token_manager: TokenManager,
    track_list: TrackListManager,
    track_load_failed: bool,
}

impl FeedPage {
    pub fn new(token_manager: TokenManager) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                track_list: TrackListManager::new(),
                track_load_failed: false,
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
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(
                            api_helpers::load_feed_with_refresh(token_manager),
                            |result| match result {
                                Ok((tracks, token_manager)) => Message::FeedPage(
                                    Mf::FeedLoadedWithToken(tracks, token_manager),
                                ),
                                Err((error, token_manager)) => Message::FeedPage(
                                    Mf::ApiErrorWithToken(error.to_string(), token_manager),
                                ),
                            },
                        ),
                    );
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
                    return (None, Task::none());
                }
                FeedPageMessage::LoadUser(user_urn) => {
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

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<'_, Message> {
        use iced::widget::column;

        let tracks_column = self.track_list.render_tracks(
            |t| Message::FeedPage(FeedPageMessage::PlayTrack(t)),
            |urn| Message::FeedPage(FeedPageMessage::LoadUser(urn)),
            |t| Message::FeedPage(FeedPageMessage::LikeTrack(t)),
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
