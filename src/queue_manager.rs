use crate::models::SoundCloudTrack;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct QueueManager {
    queue: VecDeque<SoundCloudTrack>,
    current_index: Option<usize>,
    original_tracks: Vec<SoundCloudTrack>, // Keep reference to original track list
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_index: None,
            original_tracks: Vec::new(),
        }
    }

    /// Initialize queue from a specific track in the track list
    pub fn start_queue_from_track(&mut self, track_id: u64, tracks: Vec<SoundCloudTrack>) {
        self.original_tracks = tracks.clone();

        // Use the get_track_queue function from utilities
        let queue_tracks = crate::utilities::get_track_queue(track_id, tracks);

        self.queue = queue_tracks.into_iter().collect();
        self.current_index = if self.queue.is_empty() { None } else { Some(0) };
    }

    /// Get the current track
    pub fn current_track(&self) -> Option<&SoundCloudTrack> {
        if let Some(index) = self.current_index {
            self.queue.get(index)
        } else {
            None
        }
    }

    /// Move to the next track in the queue
    pub fn next_track(&mut self) -> Option<&SoundCloudTrack> {
        if let Some(current) = self.current_index
            && current + 1 < self.queue.len()
        {
            self.current_index = Some(current + 1);
            return self.current_track();
        }
        None
    }

    /// Move to the previous track in the queue
    pub fn previous_track(&mut self) -> Option<&SoundCloudTrack> {
        if let Some(current) = self.current_index
            && current > 0
        {
            self.current_index = Some(current - 1);
            return self.current_track();
        }
        None
    }

    /// Check if there's a next track available
    pub fn has_next(&self) -> bool {
        if let Some(current) = self.current_index {
            current + 1 < self.queue.len()
        } else {
            false
        }
    }

    /// Check if there's a previous track available
    pub fn has_previous(&self) -> bool {
        if let Some(current) = self.current_index {
            current > 0
        } else {
            false
        }
    }

    /// Get the current queue as a vector for display purposes
    pub fn get_queue(&self) -> Vec<&SoundCloudTrack> {
        self.queue.iter().collect()
    }

    /// Get the current position in the queue (0-based)
    pub fn current_position(&self) -> Option<usize> {
        self.current_index
    }

    /// Get the total number of tracks in the queue
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_index = None;
        self.original_tracks.clear();
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}
