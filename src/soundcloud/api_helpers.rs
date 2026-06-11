use crate::models::{
    SearchResults, SoundCloudActivityCollection, SoundCloudPlaylists, SoundCloudStreams,
    SoundCloudTrack, SoundCloudTracks, SoundCloudUserProfile,
};
use crate::soundcloud::api;
use crate::soundcloud::auth::{AuthError, TokenManager};

/// Helper functions that combine token refresh with API calls for use with Iced Tasks
pub async fn load_feed_paginated_with_refresh(
    mut token_manager: TokenManager,
    next_href: Option<String>,
) -> Result<(SoundCloudActivityCollection, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_activity_feed_paginated(token, next_href).await {
            Ok(collection) => Ok((collection, token_manager)),
            Err(e) => {
                let error_msg = format!("{}", e);
                if error_msg.contains("401")
                    || error_msg.contains("403")
                    || error_msg.contains("Unauthorized")
                {
                    Err((
                        AuthError::OAuth(
                            "Authentication failed while loading activity feed".to_string(),
                        ),
                        token_manager,
                    ))
                } else {
                    Err((
                        AuthError::Other(format!("Failed to load activity feed: {}", e)),
                        token_manager,
                    ))
                }
            }
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn load_favourites_paginated_with_refresh(
    mut token_manager: TokenManager,
    next_href: Option<String>,
) -> Result<(SoundCloudTracks, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_liked_tracks_paginated(token, next_href).await {
            Ok(tracks) => Ok((tracks, token_manager)),
            Err(_) => Err((
                AuthError::Other("Failed to load liked tracks".to_string()),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn search_with_refresh(
    mut token_manager: TokenManager,
    query: String,
) -> Result<(SearchResults, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::search(token, &query).await {
            Ok(results) => Ok((results, token_manager)),
            Err(e) => {
                let error_msg = format!("{}", e);
                if error_msg.contains("401")
                    || error_msg.contains("403")
                    || error_msg.contains("Unauthorized")
                {
                    Err((
                        AuthError::OAuth("Authentication failed while searching".to_string()),
                        token_manager,
                    ))
                } else if error_msg.contains("429") || error_msg.contains("Rate") {
                    Err((
                        AuthError::Other(format!("Rate limited while searching: {}", e)),
                        token_manager,
                    ))
                } else {
                    Err((
                        AuthError::Other(format!("Failed to search: {}", e)),
                        token_manager,
                    ))
                }
            }
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn search_tracks_with_refresh(
    mut token_manager: TokenManager,
    query: String,
    next_href: Option<String>,
) -> Result<(SoundCloudTracks, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::search_tracks(token, &query, next_href).await {
            Ok(tracks) => Ok((tracks, token_manager)),
            Err(e) => Err((
                AuthError::Other(format!("Failed to load more tracks: {}", e)),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn search_playlists_with_refresh(
    mut token_manager: TokenManager,
    query: String,
    next_href: Option<String>,
) -> Result<(SoundCloudPlaylists, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::search_playlists(token, &query, next_href).await {
            Ok(playlists) => Ok((playlists, token_manager)),
            Err(e) => Err((
                AuthError::Other(format!("Failed to load more playlists: {}", e)),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn load_user_profile_with_refresh(
    mut token_manager: TokenManager,
    user_urn: String,
) -> Result<(SoundCloudUserProfile, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_user_profile(token, user_urn).await {
            Ok(results) => Ok((results, token_manager)),
            Err(e) => {
                let error_msg = format!("{}", e);
                if error_msg.contains("401")
                    || error_msg.contains("403")
                    || error_msg.contains("Unauthorized")
                {
                    Err((
                        AuthError::OAuth(
                            "Authentication failed while loading user profile".to_string(),
                        ),
                        token_manager,
                    ))
                } else if error_msg.contains("429") || error_msg.contains("Rate") {
                    Err((
                        AuthError::Other(format!("Rate limited while loading user profile: {}", e)),
                        token_manager,
                    ))
                } else {
                    Err((
                        AuthError::Other(format!("Failed to load user profile: {}", e)),
                        token_manager,
                    ))
                }
            }
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn get_playlist_tracks_with_refresh(
    mut token_manager: TokenManager,
    playlist_urn: String,
    next_href: Option<String>,
) -> Result<(SoundCloudTracks, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_playlist_tracks(token, playlist_urn, next_href).await {
            Ok(tracks) => Ok((tracks, token_manager)),
            Err(e) => Err((
                AuthError::Other(format!("Failed to load playlist tracks: {}", e)),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn get_user_tracks_with_refresh(
    mut token_manager: TokenManager,
    user_urn: String,
    next_href: Option<String>,
) -> Result<(SoundCloudTracks, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_user_tracks(token, user_urn, next_href).await {
            Ok(tracks) => Ok((tracks, token_manager)),
            Err(e) => Err((
                AuthError::Other(format!("Failed to load more user tracks: {}", e)),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn get_user_playlists_with_refresh(
    mut token_manager: TokenManager,
    user_urn: String,
    next_href: Option<String>,
) -> Result<(SoundCloudPlaylists, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_user_playlists(token, user_urn, next_href).await {
            Ok(playlists) => Ok((playlists, token_manager)),
            Err(e) => Err((
                AuthError::Other(format!("Failed to load more user playlists: {}", e)),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn like_track_with_refresh(
    mut token_manager: TokenManager,
    track: SoundCloudTrack,
) -> Result<(u64, TokenManager), (AuthError, TokenManager)> {
    let track_id = track.id;
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::like_track(token, track).await {
            Ok(_) => Ok((track_id, token_manager)),
            Err(_) => Err((
                AuthError::Other("Failed to like track".to_string()),
                token_manager,
            )),
        },
        Err(e) => Err((e, token_manager)),
    }
}

pub async fn get_track_streams_with_refresh(
    mut token_manager: TokenManager,
    track_id: u64,
) -> Result<(SoundCloudStreams, TokenManager), (AuthError, TokenManager)> {
    match token_manager.get_fresh_token().await {
        Ok(token) => match api::get_track_streams(token, track_id).await {
            Ok(streams) => Ok((streams, token_manager)),
            Err(e) => {
                let error_msg = format!("{}", e);
                if error_msg.contains("401")
                    || error_msg.contains("403")
                    || error_msg.contains("Unauthorized")
                {
                    Err((
                        AuthError::OAuth(
                            "Authentication failed while fetching streams".to_string(),
                        ),
                        token_manager,
                    ))
                } else {
                    Err((
                        AuthError::Other(format!("Failed to get track streams: {}", e)),
                        token_manager,
                    ))
                }
            }
        },
        Err(e) => Err((e, token_manager)),
    }
}
