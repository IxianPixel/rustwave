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
mod playlist;
mod track;
mod user;
mod message;
mod search;

// Re-exports to maintain the same public API
pub use activity::{SoundCloudActivityCollection};
pub use playlist::{SoundCloudPlaylist, SoundCloudPlaylists};
pub use track::{SoundCloudTrack, SoundCloudTracks};
pub use user::{SoundCloudUser, SoundCloudUsers, SoundCloudUserProfile};
pub use search::{SearchResults};

// Note: CurrentScreen enum was referenced in the original models.rs but not defined there.
// If this enum exists elsewhere, it should be moved to its appropriate module.