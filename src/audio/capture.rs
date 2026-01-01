use crate::error::{Result, ScribeError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Audio capture configuration and control
pub struct AudioCapture {
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_rate: u32,
}

/// Handle to a running audio stream
pub struct AudioStream {
    stream: cpal::Stream,
    receiver: mpsc::Receiver<Vec<i16>>,
}

impl AudioCapture {
    /// Create new `AudioCapture` with specified sample rate
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz (typically 16000 for Whisper)
    /// * `device_name` - Optional device name (None = default input device)
    pub fn new(sample_rate: u32, device_name: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            host.input_devices()
                .map_err(|e| ScribeError::Audio(format!("Failed to enumerate devices: {e}")))?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| ScribeError::Audio(format!("Device '{name}' not found")))?
        } else {
            host.default_input_device()
                .ok_or_else(|| ScribeError::Audio("No default input device found".to_string()))?
        };

        // Find supported config closest to our requirements
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| ScribeError::Audio(format!("Failed to get supported configs: {e}")))?;

        let mut best_config = None;
        let mut best_diff = u32::MAX;

        for supported in supported_configs {
            if supported.channels() == 1 && supported.sample_format() == cpal::SampleFormat::I16 {
                for rate in [sample_rate, 16000, 48000, 44100] {
                    if supported.min_sample_rate().0 <= rate
                        && supported.max_sample_rate().0 >= rate
                    {
                        let diff = rate.abs_diff(sample_rate);
                        if diff < best_diff {
                            best_diff = diff;
                            best_config = Some((supported, rate));
                        }
                        break;
                    }
                }
            }
        }

        let (_supported, actual_rate) = best_config.ok_or_else(|| {
            ScribeError::Audio("No supported config found (need mono i16 at 16kHz)".to_string())
        })?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(actual_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            device,
            config,
            sample_rate: actual_rate,
        })
    }

    /// Start recording audio
    ///
    /// Returns `AudioStream` handle with receiver for audio frames
    pub fn start_recording(self) -> Result<AudioStream> {
        let (tx, rx) = mpsc::channel(100);
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame_size = (f64::from(self.sample_rate) * 0.03) as usize; // 30ms frames

        let stream = self
            .device
            .build_input_stream(
                &self.config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.extend_from_slice(data);

                    // Send complete frames
                    while buf.len() >= frame_size {
                        let frame: Vec<i16> = buf.drain(..frame_size).collect();
                        drop(buf);
                        if tx.blocking_send(frame).is_err() {
                            // Receiver dropped, stop buffering
                            return;
                        }
                        buf = buffer_clone.lock().unwrap();
                    }
                    drop(buf);
                },
                move |err| {
                    eprintln!("Audio stream error: {err}");
                },
                None,
            )
            .map_err(|e| ScribeError::Audio(format!("Failed to build input stream: {e}")))?;

        stream
            .play()
            .map_err(|e| ScribeError::Audio(format!("Failed to start stream: {e}")))?;

        Ok(AudioStream {
            stream,
            receiver: rx,
        })
    }

    /// List all available input devices
    #[must_use]
    pub fn list_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .ok()
            .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }

    /// Get sample rate
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl AudioStream {
    /// Receive next audio frame (blocking)
    ///
    /// Returns `None` when stream ends
    #[allow(clippy::future_not_send)]
    pub async fn recv(&mut self) -> Option<Vec<i16>> {
        self.receiver.recv().await
    }

    /// Stop the audio stream
    pub fn stop(self) {
        drop(self.stream);
        drop(self.receiver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires audio devices - may segfault in CI"]
    fn test_list_devices() {
        let devices = AudioCapture::list_devices();
        // Just verify it doesn't panic - may be empty in CI
        println!("Available input devices: {devices:?}");
    }

    #[test]
    #[ignore = "requires audio devices - may segfault in CI"]
    fn test_create_audio_capture() {
        // This may fail in CI without audio devices
        match AudioCapture::new(16000, None) {
            Ok(capture) => {
                assert_eq!(capture.sample_rate(), 16000);
            }
            Err(e) => {
                println!("Expected failure in CI: {e}");
            }
        }
    }
}
