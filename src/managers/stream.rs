use crate::models::SoundCloudTrack;
use crate::soundcloud::TokenManager;
use crate::soundcloud::{api, api_helpers};
use iced::widget::image::Handle;
use tokio_util::bytes::Bytes;

/// Downloads stream data for a track along with artwork and waveform peaks
/// Returns (track_data, artwork_handle, waveform_peaks, token_manager)
pub async fn download_track_stream(
    token_manager: TokenManager,
    track: &SoundCloudTrack,
) -> Result<(Bytes, Option<Handle>, Option<Vec<f32>>, TokenManager), (String, TokenManager)> {
    // First, get the streaming URLs from the /tracks/{id}/streams endpoint
    let (streams, mut token_manager) =
        match api_helpers::get_track_streams_with_refresh(token_manager, track.id).await {
            Ok((streams, tm)) => (streams, tm),
            Err((error, tm)) => return Err((error.to_string(), tm)),
        };

    // Get the HLS URL (prefer 160kbps, fall back to 96kbps)
    let hls_url = match streams.get_hls_url() {
        Some(url) => url.clone(),
        None => {
            return Err((
                "No HLS stream URL available for track".to_string(),
                token_manager,
            ));
        }
    };

    println!("HLS Stream URL: {}", hls_url);

    // Get a fresh token for the HLS download
    let access_token = match token_manager.get_fresh_token().await {
        Ok(token) => token,
        Err(error) => return Err((error.to_string(), token_manager)),
    };

    // Download the HLS stream (all segments combined into one buffer)
    let track_data =
        match api::download_hls_stream(access_token.secret().to_string(), &hls_url).await {
            Ok(data) => data,
            Err(e) => {
                return Err((
                    format!("Failed to download HLS stream: {}", e),
                    token_manager,
                ));
            }
        };

    // Try to get the image handle if we have an artwork URL
    let image_handle = if !track.artwork_url.is_empty() {
        crate::utilities::download_image(&track.artwork_url)
            .await
            .ok()
    } else {
        None
    };

    // Extract peak data for canvas rendering (target 1800 bars to match SoundCloud's resolution)
    let waveform_peaks = if !track.waveform_url.is_empty() {
        match crate::utilities::download_waveform_bytes(&track.waveform_url).await {
            Ok(bytes) => crate::utilities::extract_waveform_peaks(&bytes, 1800).ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    Ok((track_data, image_handle, waveform_peaks, token_manager))
}
