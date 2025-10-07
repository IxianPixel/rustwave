use serde::{Deserialize, Serialize};

use super::{SoundCloudUser, deserialize_null_default};

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudTracks {
    pub collection: Vec<SoundCloudTrack>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudTrack {
    pub id: u64,
    pub stream_url: Option<String>,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub title: String,
    pub user: SoundCloudUser,
    pub duration: u64,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub access: String,
    pub playback_count: Option<u64>,
    pub favoritings_count: Option<u32>,
    pub reposts_count: Option<u32>,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub artwork_url: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub waveform_url: String,
}
