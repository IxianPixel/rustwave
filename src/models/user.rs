use serde::{Deserialize, Serialize};

use crate::models::{SoundCloudTrack, SoundCloudTracks};

use super::deserialize_null_default;

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudUsers {
    pub collection: Vec<SoundCloudUser>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
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

impl Default for SoundCloudUser {
    fn default() -> Self {
        Self {
            urn: String::new(),
            username: String::new(),
            full_name: String::new(),
            avatar_url: String::new(),
            followers_count: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudUserProfile {
    pub user: SoundCloudUser,
    pub tracks: Vec<SoundCloudTrack>,
}

