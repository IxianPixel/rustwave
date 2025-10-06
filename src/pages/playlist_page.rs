use crate::auth::TokenManager;
use crate::models::SoundCloudPlaylist;
use crate::page_b::PageB;
use crate::pages::{FeedPage, SearchPage};
use crate::pages::UserPage;
use crate::track_list_manager::TrackListManager;
use crate::models::SoundCloudTrack;
use crate::Message;
use crate::Page;
use crate::widgets::get_playlist_widget;
use iced::widget::image::Handle;
use iced::Task;
use iced::widget::{row, text, Scrollable};
use iced::Color;
use iced::Length;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum PlaylistPageMessage {
    LoadPlaylist,
    PlayTrack(SoundCloudTrack),
    ImageLoaded(u64, Handle),
    ImageLoadFailed(u64),
    LikeTrack(SoundCloudTrack),
    TrackLikedWithToken(u64, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    LoadUser(String),
    LoadNothing(SoundCloudPlaylist),
}

type Mp = PlaylistPageMessage;

pub struct PlaylistPage {
    token_manager: TokenManager,
    playlist: SoundCloudPlaylist,
    track_list: TrackListManager,
    track_load_failed: bool,
}

impl PlaylistPage {
    pub fn new(token_manager: TokenManager, playlist: SoundCloudPlaylist) -> (Self, Task<Message>) {
        (
            Self {
                token_manager,
                playlist: playlist.clone(),
                track_list: TrackListManager::new_with_tracks(playlist.tracks.clone()),
                track_load_failed: false,
            },
            Task::done(Message::PlaylistPage(PlaylistPageMessage::LoadPlaylist))
        )
    }
}

impl Page for PlaylistPage {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::PlaylistPage(msg) = message {
            match msg {
                PlaylistPageMessage::LoadPlaylist => {
                    let image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::PlaylistPage(Mp::ImageLoaded(track_id, handle)),
                        |track_id| Message::PlaylistPage(Mp::ImageLoadFailed(track_id))
                    );

                    return (None, Task::batch(image_tasks))
                }
                PlaylistPageMessage::PlayTrack(track) => {
                    self.track_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.track_list.tracks().clone(), self.token_manager.clone()))
                    );
                },
                PlaylistPageMessage::LikeTrack(sound_cloud_track) => todo!(),
                PlaylistPageMessage::TrackLikedWithToken(track_id, token_manager) => todo!(),
                PlaylistPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.track_load_failed = true;
                    debug!("API Error: {}", error_msg);
                    return (None, Task::none())
                },
                PlaylistPageMessage::ImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none())
                },
                PlaylistPageMessage::ImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none())
                },
                PlaylistPageMessage::LoadUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
                },
                PlaylistPageMessage::LoadNothing(playlist_urn) => {
                    debug!("This function does nothing but covers the case");
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

    fn view(&self) -> iced::Element<'_, Message> {
        use iced::widget::column;

        let tracks_column = self.track_list.render_tracks(
            |t| Message::PlaylistPage(PlaylistPageMessage::PlayTrack(t)),
            |urn| Message::PlaylistPage(PlaylistPageMessage::LoadUser(urn))
        );

        column![
            row![ if self.track_load_failed { text("Error Loading Tracks").color(Color::from_rgb(1.0, 0.0, 0.0)) } else { text("") } ],
            Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),
        ]
        .into()
    }
}
