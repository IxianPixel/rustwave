use tokio_util::bytes::Bytes;
use iced::widget::image::Handle;
use crate::soundcloud::TokenManager;
use crate::models::SoundCloudTrack;
use crate::soundcloud::api_helpers;

/// Downloads stream data for a track along with artwork and waveform peaks
/// Returns (track_data, artwork_handle, waveform_peaks, token_manager)
pub async fn download_track_stream(
    token_manager: TokenManager,
    track: &SoundCloudTrack,
) -> Result<(Bytes, Option<Handle>, Option<Vec<f32>>, TokenManager), (String, TokenManager)> {
    let stream_url = match &track.stream_url {
        Some(url) => url.clone(),
        None => return Err(("Track has no stream URL".to_string(), token_manager)),
    };

    // Download the stream data
    match api_helpers::get_track_data_with_refresh(token_manager, stream_url).await {
        Ok((track_data, token_manager)) => {
            // Try to get the image handle if we have an artwork URL
            let image_handle = if !track.artwork_url.is_empty() {
                match crate::utilities::download_image(&track.artwork_url).await {
                    Ok(handle) => Some(handle),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Extract peak data for canvas rendering (target 1800 bars to match SoundCloud's resolution)
            let waveform_peaks = if !track.waveform_url.is_empty() {
                match crate::utilities::download_waveform_bytes(&track.waveform_url).await {
                    Ok(bytes) => {
                        crate::utilities::extract_waveform_peaks(&bytes, 1800).ok()
                    },
                    Err(_) => None,
                }
            } else {
                None
            };

            Ok((track_data, image_handle, waveform_peaks, token_manager))
        }
        Err((error, token_manager)) => Err((error.to_string(), token_manager)),
    }
}
