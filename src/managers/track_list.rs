use crate::Message;
use crate::models::SoundCloudTrack;
use crate::widgets::get_track_widget;
use iced::Element;
use iced::Task;
use iced::widget::image::Handle;
use iced::widget::{Column, column, sensor};
use std::collections::{HashMap, HashSet};

// Start fetching a track's artwork when its row is within this many pixels of the viewport.
const IMAGE_PREFETCH_DISTANCE: f32 = 300.0;

/// Manages common track list functionality shared across multiple pages
pub struct TrackListManager {
    tracks: Vec<SoundCloudTrack>,
    track_images: HashMap<u64, Handle>,
    requested: HashSet<u64>,
    current_track_id: u64,
}

impl TrackListManager {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            track_images: HashMap::new(),
            requested: HashSet::new(),
            current_track_id: 0,
        }
    }

    pub fn new_with_tracks(tracks: Vec<SoundCloudTrack>) -> Self {
        Self {
            tracks,
            track_images: HashMap::new(),
            requested: HashSet::new(),
            current_track_id: 0,
        }
    }

    pub fn tracks(&self) -> &Vec<SoundCloudTrack> {
        &self.tracks
    }

    pub fn set_tracks(&mut self, tracks: Vec<SoundCloudTrack>) {
        self.tracks = tracks;
        self.track_images.clear();
        self.requested.clear();
    }

    pub fn append_tracks(&mut self, mut tracks: Vec<SoundCloudTrack>) {
        self.tracks.append(&mut tracks);
    }

    #[allow(dead_code)]
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

    /// Lazily download a single track's artwork on demand (driven by the row's
    /// visibility sensor). Returns `Task::none()` if the image is already loaded
    /// or a request is already in flight, so it is safe to call repeatedly.
    pub fn load_image_task<F>(
        &mut self,
        track_id: u64,
        on_loaded: F,
        on_failed: fn(u64) -> Message,
    ) -> Task<Message>
    where
        F: Fn(u64, Handle) -> Message + Send + 'static,
    {
        if self.track_images.contains_key(&track_id) || self.requested.contains(&track_id) {
            return Task::none();
        }

        let Some(track) = self.tracks.iter().find(|t| t.id == track_id) else {
            return Task::none();
        };

        let artwork_url = track.artwork_url.clone();
        if artwork_url.is_empty() {
            return Task::none();
        }

        self.requested.insert(track_id);
        Task::perform(
            async move { crate::utilities::download_image(&artwork_url).await },
            move |result| match result {
                Ok(handle) => on_loaded(track_id, handle),
                Err(_) => on_failed(track_id),
            },
        )
    }

    /// Render the tracks as a column of track widgets.
    /// Takes closures to map track interactions to page-specific messages.
    /// `on_request_image` is fired (via a visibility sensor) when a row scrolls
    /// into view, so artwork is only downloaded as the user reaches it.
    pub fn render_tracks<F1, F2, F3, F4>(
        &self,
        on_play: F1,
        on_user_click: F2,
        on_like: F3,
        on_request_image: F4,
    ) -> Column<'_, Message>
    where
        F1: Fn(SoundCloudTrack) -> Message + Clone + 'static,
        F2: Fn(String) -> Message + Clone + 'static,
        F3: Fn(SoundCloudTrack) -> Message + Clone + 'static,
        F4: Fn(u64) -> Message + Clone + 'static,
    {
        self.tracks.iter().fold(column![], |col, track| {
            let track_id = track.id;
            let image_handle = self.track_images.get(&track_id).cloned();
            let widget = get_track_widget(
                track,
                image_handle,
                on_play.clone(),
                on_user_click.clone(),
                on_like.clone(),
            );

            // Wrap each row in a sensor so its artwork loads only when it nears
            // the viewport. load_image_task() guards against duplicate requests,
            // so firing on_show again after the image is loaded is harmless.
            let on_request = on_request_image.clone();
            let row: Element<'_, Message> = sensor(widget)
                .on_show(move |_| on_request(track_id))
                .anticipate(IMAGE_PREFETCH_DISTANCE)
                .into();
            col.push(row)
        })
    }
}

impl Default for TrackListManager {
    fn default() -> Self {
        Self::new()
    }
}
