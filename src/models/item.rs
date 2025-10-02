use serde::Deserialize;

#[derive(Deserialize)]
pub struct SoundCloudItem {
    // Fields for SoundCloudItem would go here
    // Note: The original models.rs didn't show the struct definition
}

#[derive(Deserialize)]
pub struct SoundCloudPrimative {
    pub collection: Vec<SoundCloudItem>,
}