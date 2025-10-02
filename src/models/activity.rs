use serde::{Deserialize, Serialize};

use super::SoundCloudTrack;

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