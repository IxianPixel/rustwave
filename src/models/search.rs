use crate::models::{SoundCloudPlaylist, SoundCloudTrack, SoundCloudUser};

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub tracks: Vec<SoundCloudTrack>,
    pub users: Vec<SoundCloudUser>,
    pub playlists: Vec<SoundCloudPlaylist>,
}
    
