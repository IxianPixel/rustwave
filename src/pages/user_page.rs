use iced::Task;
use tracing::debug;

use crate::auth::TokenManager;
use crate::page_b::PageB;
use crate::pages::{FeedPage, SearchPage};
use crate::track_list_manager::TrackListManager;
use crate::{api_helpers, Message, Page};
use iced::widget::image::Handle;
use iced::widget::{column, row, text, Scrollable};
use iced::Color;
use iced::Length;
use crate::models::{SoundCloudTrack, SoundCloudUser, SoundCloudUserProfile};

#[derive(Debug, Clone)]
pub enum UserPageMessage {
    LoadUser,
    UserProfileLoaded(SoundCloudUserProfile, TokenManager),
    UserImageLoaded(String, Handle),
    UserImageLoadFailed(String),
    ApiErrorWithToken(String, TokenManager),
    TrackImageLoaded(u64, Handle),
    TrackImageLoadFailed(u64),
    PlayTrack(SoundCloudTrack),
    NavigateToUser(String),
}

type Mu = UserPageMessage;

pub struct UserPage {
    token_manager: TokenManager,
    user_urn: String,
    user: SoundCloudUser,
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
                track_list: TrackListManager::new(),
                track_load_failed: false,
            },
            Task::done(Message::UserPage(UserPageMessage::LoadUser))
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
                        Task::perform(api_helpers::load_user_profile_with_refresh(token_manager, user_urn), |result| {
                            match result {
                                Ok((user, token_manager)) => Message::UserPage(UserPageMessage::UserProfileLoaded(user, token_manager)),
                                Err((error, token_manager)) => Message::UserPage(UserPageMessage::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                }
                UserPageMessage::UserProfileLoaded(profile, token_manager) => {
                    self.token_manager = token_manager;
                    self.user = profile.user.clone();
                    self.track_list.set_tracks(profile.tracks);

                    // Create tasks to load images for all tracks
                    let track_image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::UserPage(UserPageMessage::TrackImageLoaded(track_id, handle)),
                        |track_id| Message::UserPage(UserPageMessage::TrackImageLoadFailed(track_id))
                    );

                    return (None, Task::batch(track_image_tasks))
                }
                UserPageMessage::TrackImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none())
                }
                UserPageMessage::TrackImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none())
                }
                UserPageMessage::UserImageLoaded(_, handle) => todo!(),
                UserPageMessage::UserImageLoadFailed(_) => todo!(),
                UserPageMessage::ApiErrorWithToken(_, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    return (None, Task::none())
                },
                UserPageMessage::PlayTrack(track) => {
                    self.track_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.track_list.tracks().clone(), self.token_manager.clone()))
                    );
                }
                UserPageMessage::NavigateToUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    return (None, Task::none());
                },
            }
        }

        if let Message::NavigateToFeed = message {
            let (page, task) = FeedPage::new(self.token_manager.clone());
            return (Some(Box::new(page)), task);
        }

        if let Message::NavigateToLikes = message {
            return (Some(Box::new(PageB::new(self.token_manager.clone()))), Task::none());
        }

        if let Message::NavigateToSearch = message {
            return (Some(Box::new(SearchPage::new(self.token_manager.clone()))), Task::none());
        }

        (None, Task::none())
    }

    fn view(&self) -> iced::Element<Message> {
        let tracks_column = self.track_list.render_tracks(
            |t| Message::UserPage(UserPageMessage::PlayTrack(t)),
            |urn| Message::UserPage(UserPageMessage::NavigateToUser(urn))
        );

        column![
            row![ if self.track_load_failed { text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)) } else { text("") } ],
            Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),
        ]
        .into()
    }
}