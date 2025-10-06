use std::collections::HashMap;
use iced::widget::{column, Column};
use iced::widget::image::Handle;
use iced::Task;
use crate::models::SoundCloudTrack;
use crate::widgets::get_track_widget;
use crate::Message;

/// Manages common track list functionality shared across multiple pages
pub struct TrackListManager {
    tracks: Vec<SoundCloudTrack>,
    track_images: HashMap<u64, Handle>,
    current_track_id: u64,
}

impl TrackListManager {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            track_images: HashMap::new(),
            current_track_id: 0,
        }
    }

    pub fn new_with_tracks(tracks: Vec<SoundCloudTrack>) -> Self {
        Self {
            tracks,
            track_images: HashMap::new(),
            current_track_id: 0,
        }
    }

    pub fn tracks(&self) -> &Vec<SoundCloudTrack> {
        &self.tracks
    }

    pub fn set_tracks(&mut self, tracks: Vec<SoundCloudTrack>) {
        self.tracks = tracks;
    }

    pub fn current_track_id(&self) -> u64 {
        self.current_track_id
    }

    pub fn set_current_track_id(&mut self, track_id: u64) {
        self.current_track_id = track_id;
    }

    /// Handle a track image being loaded
    pub fn handle_image_loaded(&mut self, track_id: u64, handle: Handle) {
        self.track_images.insert(track_id, handle);
    }

    /// Create tasks to load images for all tracks
    /// Takes a closure that maps (track_id, Result<Handle, Error>) to a Message
    pub fn create_image_load_tasks<F>(&self, on_loaded: F, on_failed: fn(u64) -> Message) -> Vec<Task<Message>>
    where
        F: Fn(u64, Handle) -> Message + Clone + Send + 'static,
    {
        self.tracks
            .iter()
            .map(|track| {
                let track_id = track.id;
                let artwork_url = track.artwork_url.clone();
                let on_loaded = on_loaded.clone();
                Task::perform(
                    async move { crate::utilities::download_image(&artwork_url).await },
                    move |result| match result {
                        Ok(handle) => on_loaded(track_id, handle),
                        Err(_) => on_failed(track_id),
                    }
                )
            })
            .collect()
    }

    /// Render the tracks as a column of track widgets
    /// Takes closures to map track interactions to page-specific messages
    pub fn render_tracks<F1, F2>(
        &self,
        on_play: F1,
        on_user_click: F2,
    ) -> Column<'_, Message>
    where
        F1: Fn(SoundCloudTrack) -> Message + Clone + 'static,
        F2: Fn(String) -> Message + Clone + 'static,
    {
        self.tracks
            .iter()
            .fold(column![], |col, track| {
                let image_handle = self.track_images.get(&track.id).cloned();
                let on_play_clone = on_play.clone();
                let on_user_clone = on_user_click.clone();
                col.push(get_track_widget(
                    track,
                    image_handle,
                    on_play_clone,
                    on_user_clone,
                ))
            })
    }
}

impl Default for TrackListManager {
    fn default() -> Self {
        Self::new()
    }
}
