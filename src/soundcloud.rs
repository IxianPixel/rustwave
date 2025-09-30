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
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let start_time = std::time::Instant::now();

    println!("[DEBUG-API-{}] Starting get_liked_tracks request", request_id);

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
        .await;

    match r {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            println!("[DEBUG-API-{}] HTTP response received - Status: {}, Duration: {:?}ms",
                request_id, status, start_time.elapsed().as_millis());

            // Log rate limit headers
            if let Some(rate_remaining) = headers.get("x-ratelimit-remaining") {
                println!("[DEBUG-API-{}] Rate remaining: {:?}", request_id, rate_remaining);
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
                println!("[ERROR-API-{}] HTTP error response: {} - {}", request_id, status, error_text);
                return Err(format!("HTTP {} error: {}", status, error_text).into());
            }

            match response.json::<SoundCloudTracks>().await {
                Ok(body) => {
                    println!("[DEBUG-API-{}] Successfully parsed {} liked tracks from response",
                        request_id, body.collection.len());
                    Ok(body)
                }
                Err(e) => {
                    println!("[ERROR-API-{}] Failed to parse JSON response: {}", request_id, e);
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            println!("[ERROR-API-{}] HTTP request failed after {:?}ms: {}",
                request_id, start_time.elapsed().as_millis(), e);
            Err(e.into())
        }
    }
}

pub async fn get_activity_feed(
    access_token: AccessToken,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let start_time = std::time::Instant::now();

    println!("[DEBUG-API-{}] Starting get_activity_feed request", request_id);

    let c = reqwest::Client::new();
    let r = c
        .get("https://api.soundcloud.com/me/activities/tracks")
        .query(&[("access", "playable,blocked"), ("limit", "50")])
        .bearer_auth(access_token.secret())
        .send()
        .await;

    match r {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            println!("[DEBUG-API-{}] HTTP response received - Status: {}, Duration: {:?}ms",
                request_id, status, start_time.elapsed().as_millis());

            // Log rate limit headers
            if let Some(rate_remaining) = headers.get("x-ratelimit-remaining") {
                println!("[DEBUG-API-{}] Rate remaining: {:?}", request_id, rate_remaining);
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
                println!("[ERROR-API-{}] HTTP error response: {} - {}", request_id, status, error_text);
                return Err(format!("HTTP {} error: {}", status, error_text).into());
            }

            match response.json::<SoundCloudActivityCollection>().await {
                Ok(body) => {
                    let mut tracks: Vec<SoundCloudTrack> = Vec::new();
                    for activity in body.collection {
                        tracks.push(activity.origin);
                    }

                    println!("[DEBUG-API-{}] Successfully parsed {} activity tracks from response",
                        request_id, tracks.len());
                    Ok(tracks)
                }
                Err(e) => {
                    println!("[ERROR-API-{}] Failed to parse JSON response: {}", request_id, e);
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            println!("[ERROR-API-{}] HTTP request failed after {:?}ms: {}",
                request_id, start_time.elapsed().as_millis(), e);
            Err(e.into())
        }
    }
}

pub async fn search(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let start_time = std::time::Instant::now();

    println!("[DEBUG-API-{}] Starting search request for query: '{}'", request_id, query);

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
        .await;

    match r {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            println!("[DEBUG-API-{}] HTTP response received - Status: {}, Duration: {:?}ms",
                request_id, status, start_time.elapsed().as_millis());

            // Log important headers for debugging rate limits
            if let Some(rate_limit) = headers.get("x-ratelimit-limit") {
                println!("[DEBUG-API-{}] Rate limit: {:?}", request_id, rate_limit);
            }
            if let Some(rate_remaining) = headers.get("x-ratelimit-remaining") {
                println!("[DEBUG-API-{}] Rate remaining: {:?}", request_id, rate_remaining);
            }
            if let Some(rate_reset) = headers.get("x-ratelimit-reset") {
                println!("[DEBUG-API-{}] Rate reset: {:?}", request_id, rate_reset);
            }

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
                println!("[ERROR-API-{}] HTTP error response: {} - {}", request_id, status, error_text);
                return Err(format!("HTTP {} error: {}", status, error_text).into());
            }

            match response.json::<SoundCloudTracks>().await {
                Ok(body) => {
                    println!("[DEBUG-API-{}] Successfully parsed {} tracks from response",
                        request_id, body.collection.len());
                    Ok(body.collection)
                }
                Err(e) => {
                    println!("[ERROR-API-{}] Failed to parse JSON response: {}", request_id, e);
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            println!("[ERROR-API-{}] HTTP request failed after {:?}ms: {}",
                request_id, start_time.elapsed().as_millis(), e);
            Err(e.into())
        }
    }
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
