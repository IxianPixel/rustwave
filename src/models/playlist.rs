use serde::{Deserialize, Serialize};

use super::{deserialize_null_default,SoundCloudTrack, SoundCloudUser};

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudPlaylist {
    pub urn: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub title: String,
    pub playlist_type: Option<String>,
    pub tracks: Vec<SoundCloudTrack>,
    pub user: SoundCloudUser,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub artwork_url: String,
    pub track_count: Option<u32>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudPlaylists {
    pub collection: Vec<SoundCloudPlaylist>,
    pub next_href: Option<String>,
}