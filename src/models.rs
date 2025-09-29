use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_null_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Deserialize)]
pub struct SoundCloudPrimative {
    pub collection: Vec<SoundCloudItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudTracks {
    pub collection: Vec<SoundCloudTrack>,
}

#[derive(Deserialize)]
pub struct SoundCloudPlaylists {
    pub collection: Vec<SoundCloudPlaylist>,
}

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

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudItem {
    //pub duration: u32,
    pub genre: String,
    //pub release_day: u32,
    pub permalink: String,
    pub title: String,
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
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudUser {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub username: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub full_name: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudActivity {
    #[serde(rename(deserialize = "type"))]
    pub activity_type: String,
    pub origin: SoundCloudTrack,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudActivityCollection {
    pub collection: Vec<SoundCloudActivity>,
}

pub enum SoundCloudRequest {
    GetPlaylists,
    GetLikedTracks,
    GetFollowedTracks,
    GetTrackData(SoundCloudTrack),
    LikeTrack(SoundCloudTrack),
    GetSearchResults(String),
}

pub enum CurrentScreen {
    Playlists,
    LikedTracks,
    Search,
    Feed,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

impl Default for CurrentScreen {
    fn default() -> Self {
        CurrentScreen::Playlists
    }
}
