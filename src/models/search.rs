use crate::models::{SoundCloudPlaylist, SoundCloudTrack, SoundCloudUser};

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub tracks: Vec<SoundCloudTrack>,
    pub tracks_next_href: Option<String>,
    pub users: Vec<SoundCloudUser>,
    pub playlists: Vec<SoundCloudPlaylist>,
    pub playlists_next_href: Option<String>,
}
