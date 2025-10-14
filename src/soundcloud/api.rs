use oauth2::AccessToken;
use tokio_util::bytes::Bytes;
use tokio::try_join;

use crate::
    models::{
        SearchResults, SoundCloudActivityCollection, SoundCloudPlaylist, SoundCloudPlaylists, SoundCloudTrack, SoundCloudTracks, SoundCloudUser, SoundCloudUserProfile, SoundCloudUsers
    }
;

pub async fn get_liked_tracks(
    access_token: AccessToken,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get("https://api.soundcloud.com/me/likes/tracks")
        .query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_liked_tracks_paginated(
    access_token: AccessToken,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();

    let url = next_href.unwrap_or_else(|| "https://api.soundcloud.com/me/likes/tracks".to_string());

    let mut request = c.get(&url).bearer_auth(access_token.secret());

    // Only add query parameters if using the default URL (not a pagination URL)
    if !url.contains("?") {
        request = request.query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ]);
    }

    let response = request.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_activity_feed(
    access_token: AccessToken,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get("https://api.soundcloud.com/me/activities/tracks")
        .query(&[("access", "playable,blocked"), ("limit", "50")])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudActivityCollection>().await?;
    let tracks = body.collection.into_iter().map(|activity| activity.origin).collect();
    Ok(tracks)
}

pub async fn get_activity_feed_paginated(
    access_token: AccessToken,
    next_href: Option<String>,
) -> Result<SoundCloudActivityCollection, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();

    let url = next_href.unwrap_or_else(|| "https://api.soundcloud.com/me/activities/tracks".to_string());

    let mut request = c.get(&url).bearer_auth(access_token.secret());

    // Only add query parameters if using the default URL (not a pagination URL)
    if !url.contains("?") {
        request = request.query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ]);
    }

    let response = request.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudActivityCollection>().await?;
    Ok(body)
}

pub async fn search_tracks(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
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

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body.collection)
}

pub async fn search_playlists(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudPlaylist>, Box<dyn std::error::Error + Send + Sync>> {
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

pub async fn search_user(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudUser>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get("https://api.soundcloud.com/users")
        .query(&[
            ("q", query),
            ("access", "playable,blocked"),
            ("limit", "5"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudUsers>().await?;
    Ok(body.collection)
}

pub async fn search(
    access_token: AccessToken,
    query: &str,
) -> Result<SearchResults, Box<dyn std::error::Error + Send + Sync>> {
    let (tracks, users, playlists) = try_join!(
        search_tracks(access_token.clone(), query),
        search_user(access_token.clone(), query),
        search_playlists(access_token.clone(), query)
    )?;
    Ok(SearchResults { tracks, users, playlists })
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
    c.post(u).bearer_auth(access_token.secret()).send().await?;

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

pub async fn get_user(access_token: AccessToken, user_urn: String) -> Result<SoundCloudUser, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get(format!("https://api.soundcloud.com/users/{}", user_urn))
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudUser>().await?;
    Ok(body)
}

pub async fn get_user_tracks(
    access_token: AccessToken, user_urn: String
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get(format!("https://api.soundcloud.com/users/{}/tracks", user_urn))
        .query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body.collection)
}

pub async fn get_user_playlists(
    access_token: AccessToken, user_urn: String
) -> Result<Vec<SoundCloudPlaylist>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get(format!("https://api.soundcloud.com/users/{}/playlists", user_urn))
        .query(&[
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ])
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudPlaylists>().await?;
    Ok(body.collection)
}

pub async fn get_user_profile(
    access_token: AccessToken, user_urn: String
) -> Result<SoundCloudUserProfile, Box<dyn std::error::Error + Send + Sync>> {
    let (user, tracks, playlists) = try_join!(
        get_user(access_token.clone(), user_urn.clone()),
        get_user_tracks(access_token.clone(), user_urn.clone()),
        get_user_playlists(access_token.clone(), user_urn.clone()),
    )?;
    Ok(SoundCloudUserProfile { user, tracks, playlists })
}