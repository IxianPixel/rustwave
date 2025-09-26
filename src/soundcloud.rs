use oauth2::AccessToken;
use tokio_util::bytes::Bytes;

use crate::
    models::{
        SoundCloudActivityCollection, SoundCloudPlaylist, SoundCloudPlaylists, SoundCloudPrimative,
        SoundCloudTrack, SoundCloudTracks,
    }
;

pub async fn get_playlists(
    access_token: &AccessToken,
) -> Result<SoundCloudPrimative, Box<dyn std::error::Error>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/me/playlists?limit=50&linked_partitioning=true")
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<SoundCloudPrimative>().await?;

    Ok(body)
}

pub async fn get_liked_tracks(
    access_token: AccessToken,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/me/likes/tracks")
        .query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<SoundCloudTracks>().await?;

    Ok(body)
}

pub async fn get_activity_feed(
    access_token: AccessToken,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/me/activities/tracks")
        .query(&[("access", "playable,blocked"), ("limit", "50")])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<SoundCloudActivityCollection>().await?;

    let mut tracks: Vec<SoundCloudTrack> = Vec::new();
    for activity in body.collection {
        tracks.push(activity.origin);
    }

    Ok(tracks)
}

pub async fn search(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/tracks")
        .query(&[
            ("q", query),
            ("access", "playable,blocked"),
            ("limit", "20"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<SoundCloudTracks>().await?;

    Ok(body.collection)
}

pub async fn search_playlists(
    access_token: &AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudPlaylist>, Box<dyn std::error::Error>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/playlists")
        .query(&[
            ("q", query),
            ("access", "playable,blocked"),
            ("limit", "20"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<SoundCloudPlaylists>().await?;

    Ok(body.collection)
}

pub async fn get_followed_tracks(
    access_token: AccessToken,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/me/followings/tracks?access=playable,blocked&limit=100")
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let body = r.json::<Vec<SoundCloudTrack>>().await?;

    Ok(body)
}

pub async fn like_track(
    access_token: AccessToken,
    track: SoundCloudTrack,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let u = format!("https://api.soundcloud.com/likes/tracks/{}", track.id);
    let c = reqwest::Client::new();
    let r = c.post(u).bearer_auth(access_token.secret()).send().await?;

    Ok(())
}

pub async fn get_track_data(
    access_token: AccessToken,
    stream_url: String,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    // Fetch the audio stream from the URL
    let response = client
        .get(stream_url)
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let b = response.bytes().await?;

    Ok(b)
}
