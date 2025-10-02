use serde::{Deserialize, Serialize};

use super::{SoundCloudTrack, SoundCloudUser};

#[derive(Deserialize)]
pub struct Playlist {
    //pub duration: u32,
    pub genre: String,
    //pub release_day: u32,
    pub permalink: String,
    pub title: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudPlaylist {
    pub id: u64,
    pub title: String,
    pub playlist_type: Option<String>,
    pub tracks: Vec<SoundCloudTrack>,
    pub user: SoundCloudUser,
}

#[derive(Deserialize)]
pub struct SoundCloudPlaylists {
    pub collection: Vec<SoundCloudPlaylist>,
}