use serde::{Deserialize, Serialize};

use super::{SoundCloudUser, deserialize_null_default};

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudStreams {
    pub hls_aac_160_url: Option<String>,
    pub hls_aac_96_url: Option<String>,
    #[allow(dead_code)]
    http_mp3_128_url: Option<String>,  // deprecated as of Dec 31, 2025
    #[allow(dead_code)]
    preview_mp3_128_url: Option<String>,
}

impl SoundCloudStreams {
    pub fn get_hls_url(&self) -> Option<&String> {
        self.hls_aac_160_url.as_ref()
            .or(self.hls_aac_96_url.as_ref())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudTracks {
    pub collection: Vec<SoundCloudTrack>,
    pub next_href: Option<String>,
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
    #[serde(deserialize_with = "deserialize_null_default")]
    pub genre: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub created_at: String,
}
