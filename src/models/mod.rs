use serde::{Deserialize, Deserializer};

// Shared utility for deserializing null values as empty strings
pub(crate) fn deserialize_null_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

// Module declarations
mod activity;
mod item;
mod message;
mod playlist;
mod search;
mod track;
mod user;

// Re-exports to maintain the same public API
pub use activity::SoundCloudActivityCollection;
pub use playlist::{SoundCloudPlaylist, SoundCloudPlaylists};
pub use search::SearchResults;
pub use track::{SoundCloudStreams, SoundCloudTrack, SoundCloudTracks};
pub use user::{SoundCloudUser, SoundCloudUserProfile, SoundCloudUsers};

// Note: CurrentScreen enum was referenced in the original models.rs but not defined there.
// If this enum exists elsewhere, it should be moved to its appropriate module.
