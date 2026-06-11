use oauth2::AccessToken;
use tokio::try_join;
use tokio_util::bytes::Bytes;

use crate::models::{
    SearchResults, SoundCloudActivityCollection, SoundCloudPlaylist, SoundCloudPlaylists,
    SoundCloudStreams, SoundCloudTrack, SoundCloudTracks, SoundCloudUser, SoundCloudUserProfile,
    SoundCloudUsers,
};

/// Type alias for async HLS download result
type HlsDownloadFuture<'a> = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>>
            + Send
            + 'a,
    >,
>;

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
    let c = reqwest::Client::new();

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
    let c = reqwest::Client::new();

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
    let c = reqwest::Client::new();

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
    let c = reqwest::Client::new();
    c.post(u).bearer_auth(access_token.secret()).send().await?;

    Ok(())
}

pub async fn get_user(
    access_token: AccessToken,
    user_urn: String,
) -> Result<SoundCloudUser, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
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
) -> Result<Vec<SoundCloudTrack>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get(format!(
            "https://api.soundcloud.com/users/{}/tracks",
            user_urn
        ))
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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudTracks>().await?;
    Ok(body.collection)
}

pub async fn get_user_playlists(
    access_token: AccessToken,
    user_urn: String,
) -> Result<Vec<SoundCloudPlaylist>, Box<dyn std::error::Error + Send + Sync>> {
    let c = reqwest::Client::new();
    let response = c
        .get(format!(
            "https://api.soundcloud.com/users/{}/playlists",
            user_urn
        ))
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
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(format!("HTTP {} error: {}", status, error_text).into());
    }

    let body = response.json::<SoundCloudPlaylists>().await?;
    Ok(body.collection)
}

pub async fn get_user_profile(
    access_token: AccessToken,
    user_urn: String,
) -> Result<SoundCloudUserProfile, Box<dyn std::error::Error + Send + Sync>> {
    let (user, tracks, playlists) = try_join!(
        get_user(access_token.clone(), user_urn.clone()),
        get_user_tracks(access_token.clone(), user_urn.clone()),
        get_user_playlists(access_token.clone(), user_urn.clone()),
    )?;
    Ok(SoundCloudUserProfile {
        user,
        tracks,
        playlists,
    })
}

/// Fetches the streaming URLs for a track from the /tracks/{id}/streams endpoint
pub async fn get_track_streams(
    access_token: AccessToken,
    track_id: u64,
) -> Result<SoundCloudStreams, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
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

/// Extracts AAC audio from fMP4 and converts to ADTS format for better seeking support
fn extract_aac_from_fmp4(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Parse AudioSpecificConfig from moov/trak/mdia/minf/stbl/stsd/mp4a/esds
    // We need: object_type, sample_rate_index, channel_config

    let containers: &[&[u8; 4]] = &[
        b"moov", b"trak", b"mdia", b"minf", b"stbl", b"stsd", b"mp4a",
    ];

    // Find esds box
    let esds_data = if let Some((offset, size)) = find_box_recursive(data, b"esds", containers) {
        if offset + size <= data.len() {
            Some(&data[offset..offset + size])
        } else {
            None
        }
    } else {
        None
    };

    // Default AAC-LC parameters if we can't parse esds
    let mut object_type: u8 = 2; // AAC-LC
    let mut sample_rate_index: u8 = 4; // 44100 Hz
    let mut channel_config: u8 = 2; // Stereo

    if let Some(esds) = esds_data {
        // esds box: 4 bytes size, 4 bytes type, 4 bytes version/flags, then ES_Descriptor
        // We need to find the DecoderSpecificInfo which contains AudioSpecificConfig
        // Search for the pattern: tag 0x05 followed by length and ASC
        for i in 12..esds.len().saturating_sub(4) {
            if esds[i] == 0x05 {
                // DecoderSpecificInfo tag
                let len_byte = esds.get(i + 1).copied().unwrap_or(0);
                let asc_start = i + 2;
                if len_byte >= 2 && asc_start + 2 <= esds.len() {
                    // AudioSpecificConfig: 5 bits object type, 4 bits sample rate index, 4 bits channel config
                    let byte0 = esds[asc_start];
                    let byte1 = esds[asc_start + 1];
                    object_type = (byte0 >> 3) & 0x1F;
                    sample_rate_index = ((byte0 & 0x07) << 1) | ((byte1 >> 7) & 0x01);
                    channel_config = (byte1 >> 3) & 0x0F;
                    break;
                }
            }
        }
    }

    // Collect all mdat box contents
    let mut raw_aac_data = Vec::new();
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

        if btype == b"mdat" {
            // mdat payload starts after 8-byte header
            let payload_start = offset + 8;
            let payload_end = offset + size;
            if payload_start < payload_end {
                raw_aac_data.extend_from_slice(&data[payload_start..payload_end]);
            }
        }

        offset += size;
    }

    if raw_aac_data.is_empty() {
        return Err("No mdat boxes found in fMP4".into());
    }

    // Check if mdat already contains ADTS frames (some encoders include headers)
    if raw_aac_data.len() >= 2 && raw_aac_data[0] == 0xFF && (raw_aac_data[1] & 0xF0) == 0xF0 {
        return Ok(raw_aac_data);
    }

    // Parse sample sizes from moof/traf/trun boxes
    // Each moof+mdat pair is a fragment
    let mut adts_data = Vec::new();

    // Process each fragment (moof + mdat pair)
    let mut box_offset = 0;
    while box_offset + 8 <= data.len() {
        let box_size = u32::from_be_bytes([
            data[box_offset],
            data[box_offset + 1],
            data[box_offset + 2],
            data[box_offset + 3],
        ]) as usize;
        let box_type = &data[box_offset + 4..box_offset + 8];

        if box_size < 8 || box_offset + box_size > data.len() {
            break;
        }

        if box_type == b"moof" {
            // Find trun inside moof/traf to get sample sizes
            let moof_data = &data[box_offset..box_offset + box_size];
            if let Some(sample_sizes) = parse_trun_sample_sizes(moof_data) {
                // Find the following mdat
                let next_box_offset = box_offset + box_size;
                if next_box_offset + 8 <= data.len() {
                    let next_size = u32::from_be_bytes([
                        data[next_box_offset],
                        data[next_box_offset + 1],
                        data[next_box_offset + 2],
                        data[next_box_offset + 3],
                    ]) as usize;
                    let next_type = &data[next_box_offset + 4..next_box_offset + 8];

                    if next_type == b"mdat" && next_box_offset + next_size <= data.len() {
                        let mdat_payload = &data[next_box_offset + 8..next_box_offset + next_size];
                        let mut sample_offset = 0;

                        for sample_size in &sample_sizes {
                            let sample_size = *sample_size as usize;
                            if sample_offset + sample_size <= mdat_payload.len() {
                                let sample =
                                    &mdat_payload[sample_offset..sample_offset + sample_size];

                                // Create ADTS header
                                let frame_length = 7 + sample_size;
                                let mut header = [0u8; 7];
                                header[0] = 0xFF;
                                header[1] = 0xF1;
                                let profile = object_type.saturating_sub(1) & 0x03;
                                header[2] = (profile << 6)
                                    | (sample_rate_index << 2)
                                    | ((channel_config >> 2) & 0x01);
                                header[3] = ((channel_config & 0x03) << 6)
                                    | ((frame_length >> 11) & 0x03) as u8;
                                header[4] = ((frame_length >> 3) & 0xFF) as u8;
                                header[5] = (((frame_length & 0x07) << 5) | 0x1F) as u8;
                                header[6] = 0xFC;

                                adts_data.extend_from_slice(&header);
                                adts_data.extend_from_slice(sample);
                                sample_offset += sample_size;
                            }
                        }
                    }
                }
            }
        }

        box_offset += box_size;
    }

    if adts_data.is_empty() {
        return Err("Failed to extract AAC frames from fMP4".into());
    }

    Ok(adts_data)
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

/// Extracts AAC ADTS frames from MPEG-TS container data.
///
/// Parses TS packets to extract payloads, then scans for valid ADTS frames
/// with sync word validation between consecutive frames.
fn extract_aac_from_mpegts(
    data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    const TS_PACKET_SIZE: usize = 188;

    let mut offset = 0;

    // Find TS sync byte
    while offset < data.len() && data[offset] != 0x47 {
        offset += 1;
    }

    // Collect all TS packet payloads
    let mut pes_buffer: Vec<u8> = Vec::new();

    while offset + TS_PACKET_SIZE <= data.len() {
        if data[offset] != 0x47 {
            offset += 1;
            continue;
        }

        let packet = &data[offset..offset + TS_PACKET_SIZE];
        offset += TS_PACKET_SIZE;

        let adaptation_field_control = (packet[3] >> 4) & 0x03;
        if adaptation_field_control == 2 {
            continue; // No payload
        }

        let mut payload_offset = 4;
        if adaptation_field_control == 3 && packet.len() > 4 {
            payload_offset = 5 + packet[4] as usize;
        }

        if payload_offset < TS_PACKET_SIZE {
            pes_buffer.extend_from_slice(&packet[payload_offset..]);
        }
    }

    // Scan for ADTS frames with validation
    let mut adts_data = Vec::new();
    let mut i = 0;
    let mut found_first_frame = false;
    let mut expected_profile: Option<u8> = None;
    let mut expected_sample_rate: Option<u8> = None;

    while i + 7 <= pes_buffer.len() {
        // Check for ADTS sync word: 0xFFF
        if pes_buffer[i] == 0xFF && (pes_buffer[i + 1] & 0xF0) == 0xF0 {
            let layer = (pes_buffer[i + 1] >> 1) & 0x03;
            let profile = (pes_buffer[i + 2] >> 6) & 0x03;
            let sample_rate_idx = (pes_buffer[i + 2] >> 2) & 0x0F;

            // Validate: layer must be 0, sample rate index must not be 15
            if layer == 0 && sample_rate_idx != 15 {
                let frame_length = (((pes_buffer[i + 3] & 0x03) as usize) << 11)
                    | ((pes_buffer[i + 4] as usize) << 3)
                    | ((pes_buffer[i + 5] >> 5) as usize);

                if (7..=8192).contains(&frame_length) && i + frame_length <= pes_buffer.len() {
                    // For first frame, require next frame validation
                    // For subsequent frames, check profile/sample rate consistency
                    let valid = if !found_first_frame {
                        // For first frame, look ahead for another sync word
                        let mut has_next = false;
                        let mut j = i + frame_length;
                        while j + 7 <= pes_buffer.len() && j < i + frame_length + 20 {
                            if pes_buffer[j] == 0xFF && (pes_buffer[j + 1] & 0xF0) == 0xF0 {
                                has_next = true;
                                break;
                            }
                            j += 1;
                        }
                        has_next || i + frame_length >= pes_buffer.len() - 20
                    } else {
                        // For subsequent frames, check consistency with first frame
                        expected_profile.is_none_or(|p| p == profile)
                            && expected_sample_rate.is_none_or(|s| s == sample_rate_idx)
                    };

                    if valid {
                        if !found_first_frame {
                            found_first_frame = true;
                            expected_profile = Some(profile);
                            expected_sample_rate = Some(sample_rate_idx);
                        }
                        adts_data.extend_from_slice(&pes_buffer[i..i + frame_length]);
                        i += frame_length;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    if adts_data.is_empty() {
        return Err("No AAC audio data found in stream".into());
    }

    Ok(adts_data)
}

/// Downloads HLS stream by parsing the m3u8 playlist and fetching all segments
pub fn download_hls_stream(token_secret: String, hls_url: &str) -> HlsDownloadFuture<'_> {
    Box::pin(async move {
        let client = reqwest::Client::new();

        // Fetch the master playlist
        let playlist_response = client
            .get(hls_url)
            .bearer_auth(&token_secret)
            .send()
            .await?;
        let playlist_text = playlist_response.text().await?;

        // Parse the m3u8 playlist
        let parsed = m3u8_rs::parse_playlist_res(playlist_text.as_bytes())
            .map_err(|e| format!("Failed to parse m3u8 playlist: {:?}", e))?;

        // Determine the base URL for resolving relative segment URLs
        let base_url = {
            let mut url = url::Url::parse(hls_url)?;
            url.path_segments_mut()
                .map_err(|_| "Cannot be base URL")?
                .pop();
            url.to_string()
        };

        match parsed {
            m3u8_rs::Playlist::MasterPlaylist(master) => {
                // If it's a master playlist, get the first variant stream
                if let Some(variant) = master.variants.first() {
                    let variant_url = if variant.uri.starts_with("http") {
                        variant.uri.clone()
                    } else {
                        format!("{}/{}", base_url, variant.uri)
                    };
                    // Recursively fetch the media playlist
                    return download_hls_stream(token_secret.clone(), &variant_url).await;
                }
                Err("No variants found in master playlist".into())
            }
            m3u8_rs::Playlist::MediaPlaylist(media) => {
                let mut all_bytes = Vec::new();

                // Check for initialization segment (EXT-X-MAP) - required for fMP4
                if let Some(first_seg) = media.segments.first()
                    && let Some(map) = &first_seg.map
                {
                    let init_url = if map.uri.starts_with("http") {
                        map.uri.clone()
                    } else {
                        format!("{}/{}", base_url, map.uri)
                    };
                    let init_response = client
                        .get(&init_url)
                        .bearer_auth(&token_secret)
                        .send()
                        .await?;
                    let init_bytes = init_response.bytes().await?;
                    all_bytes.extend_from_slice(&init_bytes);
                }

                // Collect all segment URLs
                let segment_urls: Vec<String> = media
                    .segments
                    .iter()
                    .map(|seg| {
                        if seg.uri.starts_with("http") {
                            seg.uri.clone()
                        } else {
                            format!("{}/{}", base_url, seg.uri)
                        }
                    })
                    .collect();

                // Download all segments concurrently (in batches to avoid overwhelming)
                for chunk in segment_urls.chunks(10) {
                    let futures: Vec<_> = chunk
                        .iter()
                        .map(|url| {
                            let client = client.clone();
                            let url = url.clone();
                            let token = token_secret.clone();
                            async move {
                                let response = client.get(&url).bearer_auth(&token).send().await?;
                                response.bytes().await
                            }
                        })
                        .collect();

                    let results = futures::future::join_all(futures).await;
                    for result in results {
                        match result {
                            Ok(bytes) => all_bytes.extend_from_slice(&bytes),
                            Err(e) => {
                                return Err(format!("Failed to download segment: {}", e).into());
                            }
                        }
                    }
                }

                // Detect format: fMP4 starts with box structure (ftyp/styp), MPEG-TS starts with 0x47
                let is_fmp4 = all_bytes.len() >= 8
                    && (&all_bytes[4..8] == b"ftyp"
                        || &all_bytes[4..8] == b"styp"
                        || &all_bytes[4..8] == b"moov");

                if is_fmp4 {
                    // Extract AAC from fMP4 and convert to ADTS for seeking support
                    let aac_data = extract_aac_from_fmp4(&all_bytes)?;
                    Ok(Bytes::from(aac_data))
                } else {
                    // MPEG-TS: demux to extract AAC ADTS frames
                    let aac_data = extract_aac_from_mpegts(&all_bytes)?;
                    Ok(Bytes::from(aac_data))
                }
            }
        }
    })
}
