use crate::models::SoundCloudTrack;

enum NavigationMessage {
    PageB,
    SearchPage,
}

pub enum TrackMessage {
    PlayTrack(SoundCloudTrack),
    LikeTrack(SoundCloudTrack),
    UnknownMessage,
}
