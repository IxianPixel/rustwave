use std::{io::Cursor, sync::mpsc, time::Duration};

use rodio::{Decoder, OutputStream, Sink};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

/// Find the start of an ADTS frame at or before the given byte offset
fn find_adts_frame_start(data: &[u8], target_offset: usize) -> usize {
    // Start from target and scan backward to find ADTS sync word
    let mut offset = target_offset.min(data.len().saturating_sub(7));

    // Scan backward up to 8KB to find a valid ADTS frame
    let min_offset = offset.saturating_sub(8192);

    while offset > min_offset {
        // Check for ADTS sync word: 0xFF 0xFx
        if data[offset] == 0xFF && offset + 6 < data.len() && (data[offset + 1] & 0xF0) == 0xF0 {
            // Validate this looks like a real ADTS header
            let layer = (data[offset + 1] >> 1) & 0x03;
            let sample_rate_idx = (data[offset + 2] >> 2) & 0x0F;

            if layer == 0 && sample_rate_idx != 15 {
                // Parse frame length to verify
                let frame_length = (((data[offset + 3] & 0x03) as usize) << 11)
                    | ((data[offset + 4] as usize) << 3)
                    | ((data[offset + 5] >> 5) as usize);

                if (7..=8192).contains(&frame_length) {
                    // Check if next frame also has sync word (if within bounds)
                    if offset + frame_length + 2 <= data.len() {
                        if data[offset + frame_length] == 0xFF
                            && (data[offset + frame_length + 1] & 0xF0) == 0xF0
                        {
                            return offset;
                        }
                    } else if offset + frame_length <= data.len() {
                        // Near end of data, accept it
                        return offset;
                    }
                }
            }
        }
        offset = offset.saturating_sub(1);
    }

    // If we couldn't find a valid frame, start from beginning
    0
}

/// Manages audio playback state, seeking, and OS media controls integration
pub struct AudioManager {
    pub stream: OutputStream,
    pub sink: Sink,
    pub track_duration: Duration,
    pub track_position: Duration,
    pub progress_bar_value: f32,
    pub stream_loading: bool,
    pub current_track_data: Option<Vec<u8>>, // Store the current track data for backward seeking
    position_offset: Duration,               // Offset to add to sink.get_pos() after seeking
    media_controls: MediaControls,
    pub media_event_receiver: mpsc::Receiver<souvlaki::MediaControlEvent>,
}

impl AudioManager {
    /// Initialize a new AudioManager with default audio output and media controls
    pub fn new() -> Self {
        let stream = rodio::OutputStreamBuilder::open_default_stream()
            .expect("Failed to open default audio output stream");
        let sink = Sink::connect_new(stream.mixer());

        // Initialize media controls with channel
        let (sender, receiver) = mpsc::channel();
        let hwnd = None; // For Windows, you might need to get the window handle
        let config = PlatformConfig {
            dbus_name: "rustwave",
            display_name: "Rustwave",
            hwnd,
        };

        let mut media_controls =
            MediaControls::new(config).expect("Failed to initialize media controls");

        // Attach the event handler
        media_controls
            .attach(move |event| {
                let _ = sender.send(event);
            })
            .expect("Failed to attach media controls event handler");

        Self {
            stream,
            sink,
            track_duration: Duration::from_secs(0),
            track_position: Duration::from_secs(0),
            progress_bar_value: 0.0,
            stream_loading: false,
            current_track_data: None,
            position_offset: Duration::from_secs(0),
            media_controls,
            media_event_receiver: receiver,
        }
    }

    /// Load and play a track from byte data
    pub fn load_track(&mut self, track_data: tokio_util::bytes::Bytes) -> Result<(), String> {
        // Store the track data for potential backward seeking workaround
        self.current_track_data = Some(track_data.to_vec());
        self.position_offset = Duration::from_secs(0);

        // Recreate a fresh Sink on our existing, long-lived stream's mixer
        self.sink = Sink::connect_new(self.stream.mixer());

        let source = Decoder::new(Cursor::new(track_data))
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        self.sink.clear();
        self.sink.append(source);
        self.sink.play();
        self.stream_loading = false;

        Ok(())
    }

    /// Update track metadata in OS media controls
    pub fn update_metadata(&mut self, title: &str, artist: &str, duration: Duration) {
        let metadata = MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            album: None,
            cover_url: None,
            duration: Some(duration),
        };
        let _ = self.media_controls.set_metadata(metadata);
        let _ = self.media_controls.set_playback(MediaPlayback::Playing {
            progress: Some(souvlaki::MediaPosition(Duration::from_secs(0))),
        });
    }

    /// Toggle play/pause state
    pub fn toggle_play_pause(&mut self) {
        if !self.sink.empty() {
            if self.sink.is_paused() {
                self.sink.play();
                let _ = self.media_controls.set_playback(MediaPlayback::Playing {
                    progress: Some(souvlaki::MediaPosition(self.track_position)),
                });
            } else {
                self.sink.pause();
                let _ = self.media_controls.set_playback(MediaPlayback::Paused {
                    progress: Some(souvlaki::MediaPosition(self.track_position)),
                });
            }
        }
    }

    /// Play the current track
    pub fn play(&mut self) {
        if !self.sink.empty() && self.sink.is_paused() {
            self.sink.play();
            let _ = self.media_controls.set_playback(MediaPlayback::Playing {
                progress: Some(souvlaki::MediaPosition(self.track_position)),
            });
        }
    }

    /// Pause the current track
    pub fn pause(&mut self) {
        if !self.sink.empty() && !self.sink.is_paused() {
            self.sink.pause();
            let _ = self.media_controls.set_playback(MediaPlayback::Paused {
                progress: Some(souvlaki::MediaPosition(self.track_position)),
            });
        }
    }

    /// Seek forward by the specified duration
    pub fn seek_forward(&mut self, seek_amount: Duration) {
        if !self.sink.empty() {
            let cur_pos = self.position_offset + self.sink.get_pos();
            let new_position = cur_pos + seek_amount;
            self.seek_to_absolute(new_position);
        }
    }

    /// Seek to an absolute position by recreating the decoder from the appropriate byte offset
    /// For ADTS streams, we calculate the byte offset and find the nearest frame boundary
    pub fn seek_to_absolute(&mut self, position: Duration) -> bool {
        let Some(ref track_data) = self.current_track_data else {
            return false;
        };

        let was_paused = self.sink.is_paused();

        // Calculate byte offset based on time position ratio
        let target_bytes = if self.track_duration.as_secs_f64() > 0.0 {
            let ratio = position.as_secs_f64() / self.track_duration.as_secs_f64();
            (ratio * track_data.len() as f64) as usize
        } else {
            0
        };

        // Find the nearest ADTS frame boundary at or before target_bytes
        let start_offset = find_adts_frame_start(track_data, target_bytes);

        // Recreate the sink and decoder from the offset
        self.sink = Sink::connect_new(self.stream.mixer());

        match Decoder::new(Cursor::new(track_data[start_offset..].to_vec())) {
            Ok(source) => {
                self.sink.append(source);
                self.position_offset = position;

                if was_paused {
                    self.sink.pause();
                } else {
                    self.sink.play();
                }
                true
            }
            Err(_) => {
                // Fall back to playing from beginning
                if let Ok(source) = Decoder::new(Cursor::new(track_data.clone())) {
                    self.sink.append(source);
                    self.position_offset = Duration::from_secs(0);
                    if was_paused {
                        self.sink.pause();
                    } else {
                        self.sink.play();
                    }
                }
                false
            }
        }
    }

    /// Seek backward by the specified duration
    pub fn seek_backward(&mut self, seek_amount: Duration) -> bool {
        if self.sink.empty() {
            return false;
        }

        let cur_pos = self.position_offset + self.sink.get_pos();
        let new_position = cur_pos.saturating_sub(seek_amount);
        self.seek_to_absolute(new_position)
    }

    /// Seek to a specific position as a percentage (0.0 to 100.0)
    pub fn seek_to_position(&mut self, percent: f32) {
        if !self.sink.empty() {
            let new_position = self.track_duration.mul_f32(percent / 100.0);
            if self.seek_to_absolute(new_position) {
                self.progress_bar_value = percent;
            }
        }
    }

    /// Update playback position and progress bar (call this on a timer)
    pub fn update_position(&mut self) {
        if !self.sink.empty() {
            // Add position_offset to get absolute track position after seeking
            let new_position = self.position_offset + self.sink.get_pos();
            self.track_position = new_position;

            self.progress_bar_value =
                (new_position.as_secs_f32() / self.track_duration.as_secs_f32()) * 100.0;

            // Update media controls with current position
            let playback_state = if self.sink.is_paused() {
                MediaPlayback::Paused {
                    progress: Some(souvlaki::MediaPosition(self.track_position)),
                }
            } else {
                MediaPlayback::Playing {
                    progress: Some(souvlaki::MediaPosition(self.track_position)),
                }
            };
            let _ = self.media_controls.set_playback(playback_state);
        }
    }

    /// Check if the current track has ended
    pub fn has_track_ended(&self) -> bool {
        !self.sink.empty()
            && self.track_position
                >= self
                    .track_duration
                    .saturating_sub(Duration::from_millis(500))
    }

    /// Clear the current track and stop playback
    pub fn clear(&mut self) {
        self.sink.clear();
        let _ = self.media_controls.set_playback(MediaPlayback::Stopped);
    }

    /// Check if sink is empty
    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    /// Check if playback is paused
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
}
