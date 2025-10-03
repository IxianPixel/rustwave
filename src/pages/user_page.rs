use std::collections::HashMap;

use iced::Task;
use tracing::debug;

use crate::auth::TokenManager;
use crate::page_b::PageB;
use crate::pages::{FeedPage, SearchPage};
use crate::widgets::get_track_widget;
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
    tracks: Vec<SoundCloudTrack>,
    track_images: HashMap<u64, Handle>,
    current_track_id: u64,
    track_load_failed: bool,
}

impl UserPage {
    pub fn new(token_manager: TokenManager, user_urn: String) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                user_urn,
                user: SoundCloudUser::default(),
                tracks: Vec::new(),
                track_images: HashMap::new(),
                current_track_id: 0,
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
                    self.tracks = profile.tracks.clone();

                    // Create tasks to load images for all tracks
                    let track_image_tasks: Vec<Task<Message>> = self.tracks
                        .iter()
                        .map(|track| {
                            let track_id = track.id;
                            let artwork_url = track.artwork_url.clone();
                            Task::perform(
                                async move { crate::utilities::download_image(&artwork_url).await },
                                move |result| match result {
                                    Ok(handle) => Message::UserPage(UserPageMessage::TrackImageLoaded(track_id, handle)),
                                    Err(_) => Message::UserPage(UserPageMessage::TrackImageLoadFailed(track_id)),
                                }
                            )
                        })
                        .collect();

                    return (None, Task::batch(track_image_tasks))
                }
                UserPageMessage::TrackImageLoaded(track_id, handle) => {
                    self.track_images.insert(track_id, handle);
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
                    self.current_track_id = track.id;
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.tracks.clone(), self.token_manager.clone()))
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
        let tracks_column = self
            .tracks
            .iter()
            .fold(column![], |col, track| {
                let image_handle = self.track_images.get(&track.id).cloned();
                col.push(get_track_widget(
                    track,
                    image_handle,
                    |t| Message::UserPage(UserPageMessage::PlayTrack(t)),
                    |urn| Message::UserPage(UserPageMessage::NavigateToUser(urn))
                ))
            });

        column![
            row![ if self.track_load_failed { text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)) } else { text("") } ],
            Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),
        ]
        .into()
    }
}