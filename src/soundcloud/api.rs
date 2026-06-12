use std::sync::OnceLock;
use std::time::Duration;

use oauth2::AccessToken;
use tokio::try_join;
use tokio_util::bytes::Bytes;

use crate::models::{
    SearchResults, SoundCloudActivityCollection, SoundCloudPlaylists, SoundCloudStreams,
    SoundCloudTrack, SoundCloudTracks, SoundCloudUser, SoundCloudUserProfile, SoundCloudUsers,
};

/// Shared HTTP client so TLS handshakes and connections are reused across all
/// API calls and HLS segment downloads.
fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    })
}

pub async fn get_liked_tracks_paginated(
    access_token: AccessToken,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_activity_feed_paginated(
    access_token: AccessToken,
    next_href: Option<String>,
) -> Result<SoundCloudActivityCollection, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url =
        next_href.unwrap_or_else(|| "https://api.soundcloud.com/me/activities/tracks".to_string());

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudActivityCollection>().await?;
    Ok(body)
}

pub async fn search_tracks(
    access_token: AccessToken,
    query: &str,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href.unwrap_or_else(|| "https://api.soundcloud.com/tracks".to_string());

    let mut request = c.get(&url).bearer_auth(access_token.secret());

    // Only add query parameters if using the default URL (not a pagination URL)
    if !url.contains("?") {
        request = request.query(&[
            ("q", query),
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ]);
    }

    let response = request.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn search_playlists(
    access_token: AccessToken,
    query: &str,
    next_href: Option<String>,
) -> Result<SoundCloudPlaylists, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href.unwrap_or_else(|| "https://api.soundcloud.com/playlists".to_string());

    let mut request = c.get(&url).bearer_auth(access_token.secret());

    // Only add query parameters if using the default URL (not a pagination URL)
    if !url.contains("?") {
        request = request.query(&[
            ("q", query),
            ("access", "playable,blocked"),
            ("limit", "50"),
            ("linked_partitioning", "true"),
        ]);
    }

    let response = request.send().await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudPlaylists>().await?;
    Ok(body)
}

pub async fn search_user(
    access_token: AccessToken,
    query: &str,
) -> Result<Vec<SoundCloudUser>, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();
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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
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
        search_tracks(access_token.clone(), query, None),
        search_user(access_token.clone(), query),
        search_playlists(access_token.clone(), query, None)
    )?;
    Ok(SearchResults {
        tracks: tracks.collection,
        tracks_next_href: tracks.next_href,
        users,
        playlists: playlists.collection,
        playlists_next_href: playlists.next_href,
    })
}

pub async fn like_track(
    access_token: AccessToken,
    track: SoundCloudTrack,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let u = format!("https://api.soundcloud.com/likes/tracks/{}", track.id);
    let c = http_client();
    c.post(u).bearer_auth(access_token.secret()).send().await?;

    Ok(())
}

pub async fn get_user(
    access_token: AccessToken,
    user_urn: String,
) -> Result<SoundCloudUser, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();
    let response = c
        .get(format!("https://api.soundcloud.com/users/{}", user_urn))
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudUser>().await?;
    Ok(body)
}

pub async fn get_user_tracks(
    access_token: AccessToken,
    user_urn: String,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href
        .unwrap_or_else(|| format!("https://api.soundcloud.com/users/{}/tracks", user_urn));

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_user_playlists(
    access_token: AccessToken,
    user_urn: String,
    next_href: Option<String>,
) -> Result<SoundCloudPlaylists, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href
        .unwrap_or_else(|| format!("https://api.soundcloud.com/users/{}/playlists", user_urn));

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudPlaylists>().await?;
    Ok(body)
}

pub async fn get_user_profile(
    access_token: AccessToken,
    user_urn: String,
) -> Result<SoundCloudUserProfile, Box<dyn std::error::Error + Send + Sync>> {
    let (user, tracks, playlists) = try_join!(
        get_user(access_token.clone(), user_urn.clone()),
        get_user_tracks(access_token.clone(), user_urn.clone(), None),
        get_user_playlists(access_token.clone(), user_urn.clone(), None),
    )?;
    Ok(SoundCloudUserProfile {
        user,
        tracks: tracks.collection,
        tracks_next_href: tracks.next_href,
        playlists: playlists.collection,
        playlists_next_href: playlists.next_href,
    })
}

pub async fn get_playlist_tracks(
    access_token: AccessToken,
    playlist_urn: String,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href.unwrap_or_else(|| {
        format!(
            "https://api.soundcloud.com/playlists/{}/tracks",
            playlist_urn
        )
    });

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_user_liked_tracks(
    access_token: AccessToken,
    user_urn: String,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href
        .unwrap_or_else(|| format!("https://api.soundcloud.com/users/{}/likes/tracks", user_urn));

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

pub async fn get_user_reposted_tracks(
    access_token: AccessToken,
    user_urn: String,
    next_href: Option<String>,
) -> Result<SoundCloudTracks, Box<dyn std::error::Error + Send + Sync>> {
    let c = http_client();

    let url = next_href.unwrap_or_else(|| {
        format!(
            "https://api.soundcloud.com/users/{}/reposts/tracks",
            user_urn
        )
    });

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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body)
}

/// Fetches the streaming URLs for a track from the /tracks/{id}/streams endpoint
pub async fn get_track_streams(
    access_token: AccessToken,
    track_id: u64,
) -> Result<SoundCloudStreams, Box<dyn std::error::Error + Send + Sync>> {
    let client = http_client();
    let url = format!("https://api.soundcloud.com/tracks/{}/streams", track_id);

    let response = client
        .get(&url)
        .bearer_auth(access_token.secret())
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error fetching streams: {}", status, error_text).into());
    }

    let streams = response.json::<SoundCloudStreams>().await?;
    Ok(streams)
}

/// Finds an MP4 box by type, returns (offset, size) if found
fn find_box(data: &[u8], box_type: &[u8; 4]) -> Option<(usize, usize)> {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        let btype = &data[offset + 4..offset + 8];

        if size < 8 || offset + size > data.len() {
            break;
        }

        if btype == box_type {
            return Some((offset, size));
        }

        offset += size;
    }
    None
}

/// Recursively searches for a box type within container boxes
fn find_box_recursive(
    data: &[u8],
    box_type: &[u8; 4],
    containers: &[&[u8; 4]],
) -> Option<(usize, usize)> {
    // First check at current level
    if let Some(result) = find_box(data, box_type) {
        return Some(result);
    }

    // Search in container boxes
    for container in containers {
        if let Some((offset, size)) = find_box(data, container) {
            let inner_start = offset + 8;
            let inner_end = offset + size;
            if inner_start < inner_end
                && inner_end <= data.len()
                && let Some((inner_offset, inner_size)) =
                    find_box_recursive(&data[inner_start..inner_end], box_type, containers)
            {
                return Some((inner_start + inner_offset, inner_size));
            }
        }
    }

    None
}

/// Reads a 32-bit big-endian MP4 box size at the given offset
fn read_box_size(data: &[u8], offset: usize) -> usize {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]) as usize
}

/// Checks whether a payload already starts with an ADTS sync word
/// (some encoders include ADTS headers inside mdat)
fn payload_is_adts(payload: &[u8]) -> bool {
    payload.len() >= 2 && payload[0] == 0xFF && (payload[1] & 0xF0) == 0xF0
}

#[derive(Clone, Copy, PartialEq)]
enum ContainerKind {
    Fmp4,
    MpegTs,
}

/// Incremental demuxer that converts HLS segments (fMP4 or MPEG-TS) into a
/// continuous AAC ADTS stream one segment at a time, so playback can start
/// before the full track has downloaded.
pub struct HlsDemuxer {
    kind: Option<ContainerKind>,
    // AudioSpecificConfig used to build ADTS headers (fMP4 path).
    // Defaults: AAC-LC, 44100 Hz, stereo.
    object_type: u8,
    sample_rate_index: u8,
    channel_config: u8,
    // Whether mdat payloads already carry ADTS headers (checked on first mdat)
    mdat_is_adts: Option<bool>,
    // Bytes of an incomplete box carried across segment boundaries
    fmp4_remainder: Vec<u8>,
    // MPEG-TS state: partial TS packet and PES bytes not yet framed
    ts_remainder: Vec<u8>,
    pes_buffer: Vec<u8>,
    found_first_frame: bool,
    expected_profile: Option<u8>,
    expected_sample_rate: Option<u8>,
}

impl HlsDemuxer {
    pub fn new() -> Self {
        Self {
            kind: None,
            object_type: 2,
            sample_rate_index: 4,
            channel_config: 2,
            mdat_is_adts: None,
            fmp4_remainder: Vec::new(),
            ts_remainder: Vec::new(),
            pes_buffer: Vec::new(),
            found_first_frame: false,
            expected_profile: None,
            expected_sample_rate: None,
        }
    }

    /// Feed the EXT-X-MAP initialization segment (ftyp + moov) to parse the
    /// AudioSpecificConfig
    pub fn push_init(&mut self, data: &[u8]) {
        self.kind = Some(ContainerKind::Fmp4);
        self.parse_asc(data);
    }

    /// Demux one media segment, returning the ADTS bytes it produced. Bytes
    /// belonging to frames that span into the next segment are carried over.
    pub fn push_segment(
        &mut self,
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let kind = match self.kind {
            Some(kind) => kind,
            None => {
                let detected = Self::detect_container(data)?;
                self.kind = Some(detected);
                detected
            }
        };

        match kind {
            ContainerKind::Fmp4 => Ok(self.demux_fmp4(data)),
            ContainerKind::MpegTs => {
                self.ingest_ts(data);
                Ok(self.scan_adts_frames(false))
            }
        }
    }

    /// Flush any frames held back waiting for more data
    pub fn finish(&mut self) -> Vec<u8> {
        match self.kind {
            Some(ContainerKind::MpegTs) => self.scan_adts_frames(true),
            // A trailing fMP4 remainder is an incomplete box; nothing playable
            _ => Vec::new(),
        }
    }

    fn detect_container(
        data: &[u8],
    ) -> Result<ContainerKind, Box<dyn std::error::Error + Send + Sync>> {
        if data.len() >= 8
            && (&data[4..8] == b"ftyp"
                || &data[4..8] == b"styp"
                || &data[4..8] == b"moov"
                || &data[4..8] == b"moof")
        {
            Ok(ContainerKind::Fmp4)
        } else if data.first() == Some(&0x47) {
            Ok(ContainerKind::MpegTs)
        } else {
            Err("Unrecognized HLS segment container format".into())
        }
    }

    /// Parse AudioSpecificConfig from moov/trak/mdia/minf/stbl/stsd/mp4a/esds
    fn parse_asc(&mut self, data: &[u8]) {
        let containers: &[&[u8; 4]] = &[
            b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stsd", b"mp4a",
        ];

        let esds_data = find_box_recursive(data, b"esds", containers)
            .and_then(|(offset, size)| data.get(offset..offset + size));

        if let Some(esds) = esds_data {
            // esds box: 4 bytes size, 4 bytes type, 4 bytes version/flags, then
            // ES_Descriptor. Search for the DecoderSpecificInfo (tag 0x05)
            // which contains the AudioSpecificConfig.
            for i in 12..esds.len().saturating_sub(4) {
                if esds[i] == 0x05 {
                    let len_byte = esds.get(i + 1).copied().unwrap_or(0);
                    let asc_start = i + 2;
                    if len_byte >= 2 && asc_start + 2 <= esds.len() {
                        // ASC: 5 bits object type, 4 bits sample rate index, 4 bits channel config
                        let byte0 = esds[asc_start];
                        let byte1 = esds[asc_start + 1];
                        self.object_type = (byte0 >> 3) & 0x1F;
                        self.sample_rate_index = ((byte0 & 0x07) << 1) | ((byte1 >> 7) & 0x01);
                        self.channel_config = (byte1 >> 3) & 0x0F;
                        break;
                    }
                }
            }
        }
    }

    fn demux_fmp4(&mut self, data: &[u8]) -> Vec<u8> {
        // Prepend bytes carried over from the previous segment boundary
        let owned;
        let data: &[u8] = if self.fmp4_remainder.is_empty() {
            data
        } else {
            self.fmp4_remainder.extend_from_slice(data);
            owned = std::mem::take(&mut self.fmp4_remainder);
            &owned
        };

        let mut out = Vec::new();
        let mut offset = 0;

        while offset + 8 <= data.len() {
            let size = read_box_size(data, offset);
            let btype = &data[offset + 4..offset + 8];

            if size < 8 {
                break;
            }
            if offset + size > data.len() {
                // Incomplete box: keep it for the next segment
                self.fmp4_remainder = data[offset..].to_vec();
                return out;
            }

            match btype {
                // moov can appear inline when there is no EXT-X-MAP init segment
                b"moov" => self.parse_asc(&data[offset..offset + size]),
                b"moof" => {
                    // A moof's samples live in the mdat that follows it; if
                    // that mdat isn't fully here yet, carry both over.
                    let moof_end = offset + size;
                    if moof_end + 8 > data.len() {
                        self.fmp4_remainder = data[offset..].to_vec();
                        return out;
                    }
                    let mdat_size = read_box_size(data, moof_end);
                    let mdat_type = &data[moof_end + 4..moof_end + 8];
                    if mdat_type != b"mdat" || mdat_size < 8 {
                        offset = moof_end;
                        continue;
                    }
                    if moof_end + mdat_size > data.len() {
                        self.fmp4_remainder = data[offset..].to_vec();
                        return out;
                    }

                    let payload = &data[moof_end + 8..moof_end + mdat_size];
                    if *self
                        .mdat_is_adts
                        .get_or_insert_with(|| payload_is_adts(payload))
                    {
                        out.extend_from_slice(payload);
                    } else if let Some(sample_sizes) =
                        parse_trun_sample_sizes(&data[offset..moof_end])
                    {
                        self.write_adts_frames(payload, &sample_sizes, &mut out);
                    }

                    offset = moof_end + mdat_size;
                    continue;
                }
                b"mdat" => {
                    // mdat without a preceding moof is only usable if the
                    // payload already carries ADTS headers
                    let payload = &data[offset + 8..offset + size];
                    if *self
                        .mdat_is_adts
                        .get_or_insert_with(|| payload_is_adts(payload))
                    {
                        out.extend_from_slice(payload);
                    }
                }
                _ => {}
            }

            offset += size;
        }

        out
    }

    /// Wrap raw AAC samples in ADTS headers using the parsed ASC parameters
    fn write_adts_frames(&self, payload: &[u8], sample_sizes: &[u32], out: &mut Vec<u8>) {
        let mut sample_offset = 0;
        for &sample_size in sample_sizes {
            let sample_size = sample_size as usize;
            if sample_offset + sample_size > payload.len() {
                break;
            }

            let frame_length = 7 + sample_size;
            let mut header = [0u8; 7];
            header[0] = 0xFF;
            header[1] = 0xF1;
            let profile = self.object_type.saturating_sub(1) & 0x03;
            header[2] = (profile << 6)
                | (self.sample_rate_index << 2)
                | ((self.channel_config >> 2) & 0x01);
            header[3] = ((self.channel_config & 0x03) << 6) | ((frame_length >> 11) & 0x03) as u8;
            header[4] = ((frame_length >> 3) & 0xFF) as u8;
            header[5] = (((frame_length & 0x07) << 5) | 0x1F) as u8;
            header[6] = 0xFC;

            out.extend_from_slice(&header);
            out.extend_from_slice(&payload[sample_offset..sample_offset + sample_size]);
            sample_offset += sample_size;
        }
    }
}

/// Parse sample sizes from trun box inside moof
fn parse_trun_sample_sizes(moof_data: &[u8]) -> Option<Vec<u32>> {
    // Find traf inside moof
    let mut offset = 8; // skip moof header
    while offset + 8 <= moof_data.len() {
        let size = u32::from_be_bytes([
            moof_data[offset],
            moof_data[offset + 1],
            moof_data[offset + 2],
            moof_data[offset + 3],
        ]) as usize;
        let btype = &moof_data[offset + 4..offset + 8];

        if size < 8 || offset + size > moof_data.len() {
            break;
        }

        if btype == b"traf" {
            // Find trun inside traf
            let traf_data = &moof_data[offset..offset + size];
            let mut traf_offset = 8;
            while traf_offset + 8 <= traf_data.len() {
                let trun_size = u32::from_be_bytes([
                    traf_data[traf_offset],
                    traf_data[traf_offset + 1],
                    traf_data[traf_offset + 2],
                    traf_data[traf_offset + 3],
                ]) as usize;
                let trun_type = &traf_data[traf_offset + 4..traf_offset + 8];

                if trun_size < 8 || traf_offset + trun_size > traf_data.len() {
                    break;
                }

                if trun_type == b"trun" {
                    return parse_trun_box(&traf_data[traf_offset..traf_offset + trun_size]);
                }

                traf_offset += trun_size;
            }
        }

        offset += size;
    }
    None
}

/// Parse trun box to extract sample sizes
fn parse_trun_box(trun_data: &[u8]) -> Option<Vec<u32>> {
    if trun_data.len() < 12 {
        return None;
    }

    // trun box: 4 size, 4 type, 1 version, 3 flags, 4 sample_count, [optional fields], [samples]
    let flags = u32::from_be_bytes([0, trun_data[9], trun_data[10], trun_data[11]]);
    let sample_count =
        u32::from_be_bytes([trun_data[12], trun_data[13], trun_data[14], trun_data[15]]) as usize;

    let mut offset = 16;

    // Skip optional fields based on flags
    if flags & 0x001 != 0 {
        offset += 4;
    } // data_offset
    if flags & 0x004 != 0 {
        offset += 4;
    } // first_sample_flags

    let has_duration = flags & 0x100 != 0;
    let has_size = flags & 0x200 != 0;
    let has_flags = flags & 0x400 != 0;
    let has_cts = flags & 0x800 != 0;

    if !has_size {
        // No per-sample sizes, would need default from tfhd
        return None;
    }

    let mut sizes = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        if has_duration {
            offset += 4;
        }
        if has_size {
            if offset + 4 > trun_data.len() {
                break;
            }
            let size = u32::from_be_bytes([
                trun_data[offset],
                trun_data[offset + 1],
                trun_data[offset + 2],
                trun_data[offset + 3],
            ]);
            sizes.push(size);
            offset += 4;
        }
        if has_flags {
            offset += 4;
        }
        if has_cts {
            offset += 4;
        }
    }

    if sizes.is_empty() { None } else { Some(sizes) }
}

impl HlsDemuxer {
    /// Extract TS packet payloads into the PES buffer. A partial trailing
    /// packet is carried over to the next segment.
    fn ingest_ts(&mut self, data: &[u8]) {
        const TS_PACKET_SIZE: usize = 188;

        let owned;
        let data: &[u8] = if self.ts_remainder.is_empty() {
            data
        } else {
            self.ts_remainder.extend_from_slice(data);
            owned = std::mem::take(&mut self.ts_remainder);
            &owned
        };

        let mut offset = 0;
        while offset < data.len() {
            // Resync on the TS sync byte
            if data[offset] != 0x47 {
                offset += 1;
                continue;
            }
            if offset + TS_PACKET_SIZE > data.len() {
                self.ts_remainder = data[offset..].to_vec();
                return;
            }

            let packet = &data[offset..offset + TS_PACKET_SIZE];
            offset += TS_PACKET_SIZE;

            let adaptation_field_control = (packet[3] >> 4) & 0x03;
            if adaptation_field_control == 2 {
                continue; // No payload
            }

            let mut payload_offset = 4;
            if adaptation_field_control == 3 {
                payload_offset = 5 + packet[4] as usize;
            }

            if payload_offset < TS_PACKET_SIZE {
                self.pes_buffer.extend_from_slice(&packet[payload_offset..]);
            }
        }
    }

    /// Consume complete, validated ADTS frames from the PES buffer. Frames
    /// that need more data (incomplete, or a first frame lacking lookahead to
    /// validate) stay buffered unless `eos` is set.
    fn scan_adts_frames(&mut self, eos: bool) -> Vec<u8> {
        let buf = &self.pes_buffer;
        let mut out = Vec::new();
        let mut i = 0;

        while i + 7 <= buf.len() {
            // Check for ADTS sync word: 0xFFF
            if buf[i] == 0xFF && (buf[i + 1] & 0xF0) == 0xF0 {
                let layer = (buf[i + 1] >> 1) & 0x03;
                let profile = (buf[i + 2] >> 6) & 0x03;
                let sample_rate_idx = (buf[i + 2] >> 2) & 0x0F;

                // Validate: layer must be 0, sample rate index must not be 15
                if layer == 0 && sample_rate_idx != 15 {
                    let frame_length = (((buf[i + 3] & 0x03) as usize) << 11)
                        | ((buf[i + 4] as usize) << 3)
                        | ((buf[i + 5] >> 5) as usize);

                    if (7..=8192).contains(&frame_length) {
                        if i + frame_length > buf.len() {
                            if eos {
                                // Truncated final frame: drop it
                                i += 1;
                                continue;
                            }
                            break; // wait for the rest of the frame
                        }

                        let valid = if !self.found_first_frame {
                            // Validate the first frame by finding another sync
                            // word right after it
                            let lookahead_end = i + frame_length + 20;
                            let mut has_next = false;
                            let mut j = i + frame_length;
                            while j + 7 <= buf.len() && j < lookahead_end {
                                if buf[j] == 0xFF && (buf[j + 1] & 0xF0) == 0xF0 {
                                    has_next = true;
                                    break;
                                }
                                j += 1;
                            }
                            if has_next {
                                true
                            } else if !eos && lookahead_end + 7 > buf.len() {
                                break; // not enough lookahead yet
                            } else {
                                // At end of stream, accept a final frame close
                                // to the end of the data
                                eos && i + frame_length >= buf.len().saturating_sub(20)
                            }
                        } else {
                            // Subsequent frames must match the first frame
                            self.expected_profile.is_none_or(|p| p == profile)
                                && self
                                    .expected_sample_rate
                                    .is_none_or(|s| s == sample_rate_idx)
                        };

                        if valid {
                            if !self.found_first_frame {
                                self.found_first_frame = true;
                                self.expected_profile = Some(profile);
                                self.expected_sample_rate = Some(sample_rate_idx);
                            }
                            out.extend_from_slice(&buf[i..i + frame_length]);
                            i += frame_length;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }

        self.pes_buffer.drain(..i);
        out
    }
}

/// A resolved HLS media playlist: the optional fMP4 init segment plus the
/// ordered media segment URLs
pub struct HlsPlaylist {
    pub init_url: Option<String>,
    pub segment_urls: Vec<String>,
}

/// Resolves a relative segment/variant URI against the playlist's parent URL
fn resolve_uri(base_url: &str, uri: &str) -> String {
    if uri.starts_with("http") {
        uri.to_string()
    } else {
        format!("{}/{}", base_url, uri)
    }
}

/// URL of the directory containing the given playlist URL
fn parent_url(url_str: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = url::Url::parse(url_str)?;
    url.path_segments_mut()
        .map_err(|_| "Cannot be base URL")?
        .pop();
    Ok(url.to_string())
}

/// Fetches the m3u8 playlist, following master playlists down to a media
/// playlist, and returns the resolved segment URLs
pub async fn resolve_hls_playlist(
    token_secret: &str,
    hls_url: &str,
) -> Result<HlsPlaylist, Box<dyn std::error::Error + Send + Sync>> {
    let client = http_client();
    let mut url = hls_url.to_string();

    // Bounded loop in case of nested master playlists
    for _ in 0..4 {
        let response = client.get(&url).bearer_auth(token_secret).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {} error fetching playlist", status).into());
        }
        let playlist_text = response.text().await?;

        let parsed = m3u8_rs::parse_playlist_res(playlist_text.as_bytes())
            .map_err(|e| format!("Failed to parse m3u8 playlist: {:?}", e))?;
        let base_url = parent_url(&url)?;

        match parsed {
            m3u8_rs::Playlist::MasterPlaylist(master) => {
                let variant = master
                    .variants
                    .first()
                    .ok_or("No variants found in master playlist")?;
                url = resolve_uri(&base_url, &variant.uri);
            }
            m3u8_rs::Playlist::MediaPlaylist(media) => {
                // Initialization segment (EXT-X-MAP) - required for fMP4
                let init_url = media
                    .segments
                    .first()
                    .and_then(|seg| seg.map.as_ref())
                    .map(|map| resolve_uri(&base_url, &map.uri));

                let segment_urls = media
                    .segments
                    .iter()
                    .map(|seg| resolve_uri(&base_url, &seg.uri))
                    .collect();

                return Ok(HlsPlaylist {
                    init_url,
                    segment_urls,
                });
            }
        }
    }

    Err("Too many nested master playlists".into())
}

/// Downloads a single HLS segment with a couple of retries
pub async fn fetch_segment(
    token_secret: &str,
    url: &str,
) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let client = http_client();
    let mut last_error = String::new();

    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        match client.get(url).bearer_auth(token_secret).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_error = format!("HTTP {}", status);
                    continue;
                }
                match response.bytes().await {
                    Ok(bytes) => return Ok(bytes),
                    Err(e) => last_error = e.to_string(),
                }
            }
            Err(e) => last_error = e.to_string(),
        }
    }

    Err(format!("Failed to download segment {}: {}", url, last_error).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid ADTS frame (7-byte header + body) matching the header
    /// layout produced by write_adts_frames
    fn adts_frame(body_len: usize) -> Vec<u8> {
        let frame_length = 7 + body_len;
        let mut frame = vec![
            0xFF,
            0xF1,
            (1 << 6) | (4 << 2), // AAC-LC, 44100 Hz, stereo
            ((2 & 0x03) << 6) | ((frame_length >> 11) & 0x03) as u8,
            ((frame_length >> 3) & 0xFF) as u8,
            (((frame_length & 0x07) << 5) | 0x1F) as u8,
            0xFC,
        ];
        frame.extend(std::iter::repeat_n(0xAB, body_len));
        frame
    }

    /// Wrap a byte stream into 188-byte TS packets (payload-only)
    fn ts_wrap(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in data.chunks(184) {
            out.push(0x47);
            out.push(0x00);
            out.push(0x00);
            out.push(0x10); // adaptation_field_control = 01 (payload only)
            out.extend_from_slice(chunk);
            out.resize(out.len() + (184 - chunk.len()), 0x00);
        }
        out
    }

    #[test]
    fn ts_demux_reassembles_frames_across_segment_split() {
        let mut adts_stream = Vec::new();
        for len in [100, 250, 37, 512, 180] {
            adts_stream.extend(adts_frame(len));
        }
        let ts = ts_wrap(&adts_stream);

        // Split mid-packet to exercise the TS remainder carry-over
        let split = 188 * 2 + 100;
        let mut demuxer = HlsDemuxer::new();
        let mut out = demuxer.push_segment(&ts[..split]).unwrap();
        out.extend(demuxer.push_segment(&ts[split..]).unwrap());
        out.extend(demuxer.finish());

        assert_eq!(out, adts_stream);
    }

    /// Build an MP4 box with the given type and content
    fn mp4_box(box_type: &[u8; 4], content: &[u8]) -> Vec<u8> {
        let mut out = ((content.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(box_type);
        out.extend_from_slice(content);
        out
    }

    fn fmp4_init() -> Vec<u8> {
        // esds with DecoderSpecificInfo (tag 0x05): AAC-LC, 44100 Hz, stereo,
        // padded so the tag search window (starting at offset 12) covers it
        let mut esds_content = vec![0, 0, 0, 0]; // version/flags
        esds_content.extend_from_slice(&[0x05, 0x02, 0x12, 0x10, 0, 0, 0, 0]);
        let esds = mp4_box(b"esds", &esds_content);

        let nested = [
            b"mp4a", b"stsd", b"stbl", b"minf", b"mdia", b"trak", b"moov",
        ]
        .iter()
        .fold(esds, |inner, box_type| mp4_box(box_type, &inner));

        let mut init = mp4_box(b"ftyp", b"isom");
        init.extend(nested);
        init
    }

    fn fmp4_segment(sample_sizes: &[u32]) -> (Vec<u8>, Vec<u8>) {
        // trun: version + flags (sample sizes present) + count + sizes
        let mut trun_content = vec![0x00, 0x00, 0x02, 0x00];
        trun_content.extend((sample_sizes.len() as u32).to_be_bytes());
        for size in sample_sizes {
            trun_content.extend(size.to_be_bytes());
        }
        let moof = mp4_box(b"moof", &mp4_box(b"traf", &mp4_box(b"trun", &trun_content)));

        let payload: Vec<u8> = sample_sizes
            .iter()
            .flat_map(|&size| std::iter::repeat_n(0xCD, size as usize))
            .collect();

        let mut segment = moof;
        segment.extend(mp4_box(b"mdat", &payload));
        (segment, payload)
    }

    #[test]
    fn fmp4_demux_writes_adts_headers_and_carries_partial_boxes() {
        let sample_sizes = [120u32, 300, 64];
        let (segment, _) = fmp4_segment(&sample_sizes);

        let mut expected = Vec::new();
        for &size in &sample_sizes {
            let mut frame = adts_frame(0);
            frame[3..7].copy_from_slice(&{
                let frame_length = 7 + size as usize;
                [
                    ((2 & 0x03) << 6) | ((frame_length >> 11) & 0x03) as u8,
                    ((frame_length >> 3) & 0xFF) as u8,
                    (((frame_length & 0x07) << 5) | 0x1F) as u8,
                    0xFC,
                ]
            });
            frame.extend(std::iter::repeat_n(0xCD, size as usize));
            expected.extend(frame);
        }

        // Split mid-mdat to exercise the fMP4 remainder carry-over
        let mut demuxer = HlsDemuxer::new();
        demuxer.push_init(&fmp4_init());
        let split = segment.len() - 50;
        let first = demuxer.push_segment(&segment[..split]).unwrap();
        assert!(first.is_empty(), "incomplete moof+mdat must be held back");
        let mut out = first;
        out.extend(demuxer.push_segment(&segment[split..]).unwrap());
        out.extend(demuxer.finish());

        assert_eq!(out, expected);
    }

    #[test]
    fn fmp4_passthrough_when_mdat_already_adts() {
        // mdat payload that already carries ADTS headers must not be re-wrapped
        let adts_stream = [adts_frame(80), adts_frame(120)].concat();
        let (mut segment, _) = fmp4_segment(&[adts_stream.len() as u32]);
        // Rebuild the mdat with the ADTS payload
        let moof_len = segment.len() - (8 + adts_stream.len());
        segment.truncate(moof_len);
        segment.extend(mp4_box(b"mdat", &adts_stream));

        let mut demuxer = HlsDemuxer::new();
        demuxer.push_init(&fmp4_init());
        let out = demuxer.push_segment(&segment).unwrap();

        assert_eq!(out, adts_stream);
    }
}
