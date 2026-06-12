use std::sync::Arc;

use crate::managers::audio_buffer::SharedAudioBuffer;
use crate::models::SoundCloudTrack;
use crate::soundcloud::TokenManager;
use crate::soundcloud::{api, api_helpers};
use futures::StreamExt;
use iced::widget::image::Handle;
use tokio::sync::oneshot;

/// How many segment downloads to keep in flight at once
const SEGMENT_CONCURRENCY: usize = 8;

/// Rough ADTS bytes per second at 160 kbps, used to pre-size the audio buffer
const BUFFER_BYTES_PER_SEC: usize = 20_000;

/// Resolves a track's HLS stream and starts buffering it in the background,
/// fetching artwork and waveform peaks concurrently. Returns as soon as the
/// first audio segment is buffered, so playback can begin while the rest of
/// the track downloads.
/// Returns (audio_buffer, artwork_handle, waveform_peaks, token_manager)
pub async fn download_track_stream(
    token_manager: TokenManager,
    track: &SoundCloudTrack,
) -> Result<
    (
        Arc<SharedAudioBuffer>,
        Option<Handle>,
        Option<Vec<f32>>,
        TokenManager,
    ),
    (String, TokenManager),
> {
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

    // Get a fresh token for the HLS download
    let access_token = match token_manager.get_fresh_token().await {
        Ok(token) => token,
        Err(error) => return Err((error.to_string(), token_manager)),
    };
    let token_secret = access_token.secret().to_string();

    // Resolve the playlist down to a concrete segment list
    let playlist = match api::resolve_hls_playlist(&token_secret, &hls_url).await {
        Ok(playlist) => playlist,
        Err(e) => {
            return Err((
                format!("Failed to resolve HLS playlist: {}", e),
                token_manager,
            ));
        }
    };
    if playlist.segment_urls.is_empty() {
        return Err((
            "HLS playlist contains no segments".to_string(),
            token_manager,
        ));
    }

    // Pre-size the buffer from the track duration to avoid reallocations
    let capacity = ((track.duration as usize / 1000) + 10) * BUFFER_BYTES_PER_SEC;
    let buffer = SharedAudioBuffer::new(
        playlist.segment_urls.len() as u32,
        capacity.min(64 * 1024 * 1024),
    );

    // Download and demux in the background; ready_rx fires once the first
    // segment's audio is in the buffer
    let (ready_tx, ready_rx) = oneshot::channel();
    tokio::spawn(run_hls_download(
        token_secret,
        playlist,
        Arc::clone(&buffer),
        ready_tx,
    ));

    // Artwork and waveform download concurrently with the audio buffering
    let artwork_fut = async {
        if track.artwork_url.is_empty() {
            return None;
        }
        crate::utilities::download_image(&track.artwork_url)
            .await
            .ok()
    };
    let waveform_fut = async {
        if track.waveform_url.is_empty() {
            return None;
        }
        match crate::utilities::download_waveform_bytes(&track.waveform_url).await {
            Ok(bytes) => crate::utilities::extract_waveform_peaks(&bytes, 1800).ok(),
            Err(_) => None,
        }
    };

    let (ready, image_handle, waveform_peaks) = tokio::join!(ready_rx, artwork_fut, waveform_fut);

    match ready {
        Ok(Ok(())) => Ok((buffer, image_handle, waveform_peaks, token_manager)),
        Ok(Err(e)) => Err((
            format!("Failed to download HLS stream: {}", e),
            token_manager,
        )),
        Err(_) => Err((
            "HLS download task stopped unexpectedly".to_string(),
            token_manager,
        )),
    }
}

/// Marks the buffer finished when the download task exits by any path, so a
/// reader blocked on the audio thread can never wait forever
struct FinishGuard(Arc<SharedAudioBuffer>);

impl Drop for FinishGuard {
    fn drop(&mut self) {
        self.0.finish();
    }
}

async fn run_hls_download(
    token_secret: String,
    playlist: api::HlsPlaylist,
    buffer: Arc<SharedAudioBuffer>,
    ready_tx: oneshot::Sender<Result<(), String>>,
) {
    let _guard = FinishGuard(Arc::clone(&buffer));
    let mut ready_tx = Some(ready_tx);

    let result = download_loop(&token_secret, &playlist, &buffer, &mut ready_tx).await;

    if let Err(e) = &result {
        eprintln!("HLS download failed: {}", e);
    }
    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(result);
    }
}

async fn download_loop(
    token_secret: &str,
    playlist: &api::HlsPlaylist,
    buffer: &SharedAudioBuffer,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
) -> Result<(), String> {
    let mut demuxer = api::HlsDemuxer::new();

    if let Some(init_url) = &playlist.init_url {
        let init = api::fetch_segment(token_secret, init_url)
            .await
            .map_err(|e| e.to_string())?;
        demuxer.push_init(&init);
    }

    // Pipelined, ordered segment downloads: up to SEGMENT_CONCURRENCY in
    // flight, demuxed and appended as each one completes
    let urls = playlist.segment_urls.clone();
    let mut segments = futures::stream::iter(urls.into_iter().map(|url: String| {
        let token = token_secret.to_string();
        async move { api::fetch_segment(&token, &url).await }
    }))
    .buffered(SEGMENT_CONCURRENCY);

    while let Some(result) = segments.next().await {
        if buffer.is_cancelled() {
            return Ok(());
        }
        let segment = result.map_err(|e| e.to_string())?;
        let adts = demuxer.push_segment(&segment).map_err(|e| e.to_string())?;
        buffer.append_segment(&adts);

        if buffer.available() > 0
            && let Some(tx) = ready_tx.take()
        {
            let _ = tx.send(Ok(()));
        }
    }

    buffer.append(&demuxer.finish());

    if buffer.available() == 0 {
        return Err("No AAC audio data found in stream".to_string());
    }
    Ok(())
}
