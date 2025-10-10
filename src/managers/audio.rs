use std::{io::Cursor, sync::mpsc, time::Duration};

use rodio::{Decoder, OutputStream, Sink};
use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

/// Manages audio playback state, seeking, and OS media controls integration
pub struct AudioManager {
    pub stream: OutputStream,
    pub sink: Sink,
    pub track_duration: Duration,
    pub track_position: Duration,
    pub progress_bar_value: f32,
    pub stream_loading: bool,
    pub current_track_data: Option<Vec<u8>>, // Store the current track data for backward seeking
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
            media_controls,
            media_event_receiver: receiver,
        }
    }

    /// Load and play a track from byte data
    pub fn load_track(&mut self, track_data: tokio_util::bytes::Bytes) -> Result<(), String> {
        // Store the track data for potential backward seeking workaround
        self.current_track_data = Some(track_data.to_vec());

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
            let cur_pos = self.sink.get_pos();
            let new_position = cur_pos + seek_amount;
            let _ = self.sink.try_seek(new_position);
        }
    }

    /// Seek backward by the specified duration
    /// Uses workaround to recreate audio source when direct backward seeking fails
    pub fn seek_backward(&mut self, seek_amount: Duration) -> bool {
        if self.sink.empty() {
            return false;
        }

        let cur_pos = self.sink.get_pos();
        let new_position = cur_pos.saturating_sub(seek_amount);

        // Try direct backward seek first
        match self.sink.try_seek(new_position) {
            Ok(_) => {
                self.track_position = new_position;
                true
            }
            Err(_) => {
                // Advanced workaround: recreate the audio source and seek forward
                if let Some(ref track_data) = self.current_track_data {
                    // Remember if we were paused
                    let was_paused = self.sink.is_paused();

                    // Recreate the sink and source
                    self.sink = Sink::connect_new(self.stream.mixer());

                    match Decoder::new(Cursor::new(track_data.clone())) {
                        Ok(source) => {
                            self.sink.clear();
                            self.sink.append(source);

                            // If we want to seek to a position > 0, do forward seek
                            if new_position > Duration::from_secs(0) {
                                match self.sink.try_seek(new_position) {
                                    Ok(_) => {
                                        self.track_position = new_position;

                                        // Restore play/pause state
                                        if was_paused {
                                            self.sink.pause();
                                        } else {
                                            self.sink.play();
                                        }
                                        true
                                    }
                                    Err(_) => false,
                                }
                            } else {
                                self.track_position = Duration::from_secs(0);

                                // Restore play/pause state
                                if was_paused {
                                    self.sink.pause();
                                } else {
                                    self.sink.play();
                                }
                                true
                            }
                        }
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Seek to a specific position as a percentage (0.0 to 100.0)
    pub fn seek_to_position(&mut self, percent: f32) {
        if !self.sink.empty() {
            let new_position = self.track_duration.mul_f32(percent / 100.0);
            let current_position = self.sink.get_pos();

            // Determine if this is forward or backward seeking
            if new_position < current_position {
                // Backward seeking - use our unified function
                let seek_amount = current_position - new_position;
                if self.seek_backward(seek_amount) {
                    self.progress_bar_value = percent;
                }
            } else {
                // Forward seeking - use direct seek
                match self.sink.try_seek(new_position) {
                    Ok(_) => {
                        self.track_position = new_position;
                        self.progress_bar_value = percent;
                    }
                    Err(_) => {
                        // Forward seek failed, don't update UI
                    }
                }
            }
        }
    }

    /// Update playback position and progress bar (call this on a timer)
    pub fn update_position(&mut self) {
        if !self.sink.empty() {
            let new_position = self.sink.get_pos();
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
