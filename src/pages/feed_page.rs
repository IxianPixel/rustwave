use iced::widget::image::Handle;

use crate::api_helpers;
use crate::models::SoundCloudTrack;
use crate::auth::TokenManager;
use crate::pages::SearchPage;
use crate::widgets::get_track_widget;
use std::collections::HashMap;
use crate::Page;
use crate::Message;
use iced::Task;
use iced::widget::{column, row, text, Scrollable};
use iced::Color;
use iced::Length;

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
}

type Mf = FeedPageMessage;

pub struct FeedPage {
    token_manager: TokenManager,
    tracks: Vec<SoundCloudTrack>,
    track_load_failed: bool,
    track_images: HashMap<u64, Handle>,
    current_track_id: u64,
}

impl FeedPage {
    pub fn new(token_manager: TokenManager) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                tracks: Vec::new(),
                track_load_failed: false,
                track_images: HashMap::new(),
                current_track_id: 0,
            },
            Task::done(Message::FeedPage(FeedPageMessage::LoadFeed))
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
                        Task::perform(api_helpers::load_feed_with_refresh(token_manager), |result| {
                            match result {
                                Ok((tracks, token_manager)) => Message::FeedPage(Mf::FeedLoadedWithToken(tracks, token_manager)),
                                Err((error, token_manager)) => Message::FeedPage(Mf::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                }
                FeedPageMessage::FeedLoadedWithToken(tracks, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = false;
                    self.tracks = tracks.clone();

                    // Create tasks to load images for all tracks
                    let image_tasks: Vec<Task<Message>> = tracks
                        .iter()
                        .map(|track| {
                            let track_id = track.id;
                            let artwork_url = track.artwork_url.clone();
                            Task::perform(
                                async move { crate::utilities::download_image(&artwork_url).await },
                                move |result| match result {
                                    Ok(handle) => Message::FeedPage(Mf::ImageLoaded(track_id, handle)),
                                    Err(_) => Message::FeedPage(Mf::ImageLoadFailed(track_id)),
                                }
                            )
                        })
                        .collect();

                    return (None, Task::batch(image_tasks))
                },
                FeedPageMessage::PlayTrack(track) => {
                    self.current_track_id = track.id;
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.tracks.clone(), self.token_manager.clone()))
                    );
                }
                FeedPageMessage::ImageLoaded(track_id, handle) => {
                    self.track_images.insert(track_id, handle);
                    return (None, Task::none())
                }
                FeedPageMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none()) 
                }
                FeedPageMessage::LikeTrack(track) => {
                    let token_manager = self.token_manager.clone();
                    return (
                        None,
                        Task::perform(api_helpers::like_track_with_refresh(token_manager, track.clone()), move |result| {
                            match result {
                                Ok((track_id, token_manager)) => Message::FeedPage(Mf::TrackLikedWithToken(track_id, token_manager)),
                                Err((error, token_manager)) => Message::FeedPage(Mf::ApiErrorWithToken(error.to_string(), token_manager)),
                            }
                        })
                    );
                }
                FeedPageMessage::TrackLikedWithToken(_, token_manager) => todo!(),
                FeedPageMessage::ApiErrorWithToken(_, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    return (None, Task::none())
                },
            }
        }

        if let Message::NavigateToSearch = message {
            return (Some(Box::new(SearchPage::new(self.token_manager.clone()))), Task::none());
        }
        
        (None, Task::none())
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let tracks_column = self
            .tracks
            .iter()
            .fold(column![], |col, track| {
                let image_handle = self.track_images.get(&track.id).cloned();
                col.push(get_track_widget(track, image_handle, |t| Message::FeedPage(FeedPageMessage::PlayTrack(t))))
            });

        column![
        row![ if self.track_load_failed { text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)) } else { text("") } ],
        Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),
    ]
    .into()
    }
}
    