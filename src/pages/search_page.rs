use crate::page_b::PageB;
use crate::pages::UserPage;
use crate::widgets::get_user_widget;
use crate::track_list_manager::TrackListManager;
use crate::{api_helpers, Message, Page};
use iced::widget::{column, row, text_input, Scrollable};
use iced::{Length, Task};
use tracing::debug;
use crate::auth::TokenManager;
use crate::models::{SearchResults, SoundCloudTrack, SoundCloudUser};
use iced::widget::image::Handle;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SearchPageMessage {
    SearchPressed(String),
    Search(String),
    SearchCompletedWithToken(SearchResults, TokenManager),
    ApiErrorWithToken(String, TokenManager),
    UserImageLoaded(String, Handle),
    UserImageLoadFailed(String),
    TrackImageLoaded(u64, Handle),
    TrackImageLoadFailed(u64),
    PlayTrack(SoundCloudTrack),
    LikeTrack(SoundCloudTrack),
    LoadUser(String),
}

type Ms = SearchPageMessage;

pub struct SearchPage {
    token_manager: TokenManager,
    search_query: String,
    user_load_failed: bool,
    user_images: HashMap<String, Handle>,
    users: Vec<SoundCloudUser>,
    track_list: TrackListManager,
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
        }
    }
}   

impl Page for SearchPage {
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
                            |result| {
                                match result {
                                    Ok((results, token_manager)) => Message::SearchPage(Ms::SearchCompletedWithToken(results, token_manager)),
                                    Err((error, token_manager)) => Message::SearchPage(Ms::ApiErrorWithToken(error.to_string(), token_manager)),
                                }
                            }
                        )
                    );
                }
                SearchPageMessage::SearchCompletedWithToken(results, token_manager) => {
                    self.token_manager = token_manager;
                    self.user_load_failed = false;
                    self.users = results.users.clone();
                    self.track_list.set_tracks(results.tracks);

                    // Create tasks to load images for all users
                    let image_tasks: Vec<Task<Message>> = self.users
                        .iter()
                        .map(|user| {
                            let user_urn = user.urn.clone();
                            let artwork_url = user.avatar_url.clone();
                            Task::perform(
                                async move { crate::utilities::download_image(&artwork_url).await },
                                move |result| match result {
                                    Ok(handle) => Message::SearchPage(Ms::UserImageLoaded(user_urn.clone(), handle)),
                                    Err(_) => Message::SearchPage(Ms::UserImageLoadFailed(user_urn.clone())),
                                }
                            )
                        })
                        .collect();

                    // Create tasks to load images for all tracks
                    let track_image_tasks = self.track_list.create_image_load_tasks(
                        |track_id, handle| Message::SearchPage(Ms::TrackImageLoaded(track_id, handle)),
                        |track_id| Message::SearchPage(Ms::TrackImageLoadFailed(track_id))
                    );

                    return (None, Task::batch(image_tasks.into_iter().chain(track_image_tasks)))
                },
                SearchPageMessage::ApiErrorWithToken(error_msg, token_manager) => {
                    self.token_manager = token_manager;
                    self.user_load_failed = true;
                    debug!("API Error: {}", error_msg);
                    return (None, Task::none())
                },
                SearchPageMessage::UserImageLoaded(user_urn, handle) => {
                    self.user_images.insert(user_urn, handle);
                    return (None, Task::none())
                },
                SearchPageMessage::UserImageLoadFailed(user_urn) => {
                    debug!("Failed to load image for user {}", user_urn);
                    return (None, Task::none())
                },
                SearchPageMessage::TrackImageLoaded(track_id, handle) => {
                    self.track_list.handle_image_loaded(track_id, handle);
                    return (None, Task::none())
                }
                SearchPageMessage::TrackImageLoadFailed(track_id) => {
                    println!("Failed to load image for track {}", track_id);
                    return (None, Task::none())
                }
                SearchPageMessage::PlayTrack(track) => {
                    self.track_list.set_current_track_id(track.id);
                    return (
                        None,
                        Task::done(Message::StartQueue(track.clone(), self.track_list.tracks().clone(), self.token_manager.clone()))
                    );
                },
                SearchPageMessage::LikeTrack(sound_cloud_track) => todo!(),
                SearchPageMessage::LoadUser(user_urn) => {
                    debug!("Loading user {}", user_urn);
                    let (user_page, task) = UserPage::new(self.token_manager.clone(), user_urn);
                    return (Some(Box::new(user_page)), task);
                },
            }
        }

        if let Message::NavigateToLikes = message {
            return (Some(Box::new(PageB::new(self.token_manager.clone()))), Task::none());
        }
        
        (None, Task::none())
    }

    fn view(&self) -> iced::Element<Message> {
        let mut indices: Vec<usize> = (0..self.users.len()).collect();
        indices.sort_by(|&a, &b| {
            let count_a = self.users[a].followers_count.unwrap_or(0);
            let count_b = self.users[b].followers_count.unwrap_or(0);
            count_b.cmp(&count_a)
        });

        let users_column = indices
            .iter()
            .fold(row![], |col, &idx| {
                let user = &self.users[idx];
                let image_handle = self.user_images.get(&user.urn).cloned();
                col.push(get_user_widget(user, image_handle, |urn| Message::SearchPage(SearchPageMessage::LoadUser(urn))))
            });

        let tracks_column = self.track_list.render_tracks(
            |t| Message::SearchPage(SearchPageMessage::PlayTrack(t)),
            |urn| Message::SearchPage(SearchPageMessage::LoadUser(urn))
        );

        column![
            row![
                text_input("Search", self.search_query.as_str())
                    .on_submit(Message::SearchPage(Ms::Search(self.search_query.clone())))
                    .on_input(|s| Message::SearchPage(Ms::SearchPressed(s))),
            ].spacing(10),
            row![
                users_column
            ].spacing(10),
            Scrollable::new(tracks_column).height(Length::FillPortion(1)).width(Length::FillPortion(1)),

        ]
        .into()
    }
}
