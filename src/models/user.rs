use serde::{Deserialize, Serialize};

use crate::models::{SoundCloudPlaylist, SoundCloudTrack};

use super::deserialize_null_default;

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudUsers {
    pub collection: Vec<SoundCloudUser>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct SoundCloudUser {
    pub urn: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub username: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub full_name: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub avatar_url: String,
    pub followers_count: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudUserProfile {
    pub user: SoundCloudUser,
    pub tracks: Vec<SoundCloudTrack>,
    pub playlists: Vec<SoundCloudPlaylist>,
}
