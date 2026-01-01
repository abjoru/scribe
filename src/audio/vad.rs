use crate::error::{Result, ScribeError};
use webrtc_vad::{SampleRate, Vad, VadMode};

/// Voice Activity Detector using WebRTC VAD
pub struct VoiceActivityDetector {
    vad: Vad,
    sample_rate: u32,
    frame_duration_ms: u32,
    frame_size: usize,
    silence_threshold_frames: u32,
    skip_initial_frames: u32,
    min_duration_ms: u32,
}

/// Configuration for VAD
#[derive(Debug, Clone)]
pub struct VadConfig {
    pub sample_rate: u32,
    pub aggressiveness: u8,
    pub silence_ms: u32,
    pub min_duration_ms: u32,
    pub skip_initial_ms: u32,
}

impl VadConfig {
    /// Create default VAD config (matches `WhisperWriter` parameters)
    #[must_use]
    pub const fn default_16khz() -> Self {
        Self {
            sample_rate: 16000,
            aggressiveness: 2,
            silence_ms: 900,
            min_duration_ms: 500,
            skip_initial_ms: 150,
        }
    }
}

impl VoiceActivityDetector {
    /// Create new VAD with specified configuration
    pub fn new(config: &VadConfig) -> Result<Self> {
        let sample_rate = match config.sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => {
                return Err(ScribeError::Vad(format!(
                    "Unsupported sample rate: {} (must be 8000, 16000, 32000, or 48000)",
                    config.sample_rate
                )))
            }
        };

        let mode = match config.aggressiveness {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            3 => VadMode::VeryAggressive,
            _ => {
                return Err(ScribeError::Vad(format!(
                    "Invalid aggressiveness: {} (must be 0-3)",
                    config.aggressiveness
                )))
            }
        };

        let vad = Vad::new_with_rate_and_mode(sample_rate, mode);

        // Frame duration is fixed at 30ms for optimal VAD performance
        let frame_duration_ms = 30;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame_size =
            (f64::from(config.sample_rate) * f64::from(frame_duration_ms) / 1000.0) as usize;

        let silence_threshold_frames = config.silence_ms / frame_duration_ms;
        let skip_initial_frames = config.skip_initial_ms / frame_duration_ms;

        Ok(Self {
            vad,
            sample_rate: config.sample_rate,
            frame_duration_ms,
            frame_size,
            silence_threshold_frames,
            skip_initial_frames,
            min_duration_ms: config.min_duration_ms,
        })
    }

    /// Process a single frame and return whether speech is detected
    ///
    /// Frame must be exactly `frame_size` samples (480 for 16kHz)
    pub fn is_voice_frame(&mut self, frame: &[i16]) -> Result<bool> {
        if frame.len() != self.frame_size {
            return Err(ScribeError::Vad(format!(
                "Invalid frame size: {} (expected {})",
                frame.len(),
                self.frame_size
            )));
        }

        self.vad
            .is_voice_segment(frame)
            .map_err(|()| ScribeError::Vad("VAD processing failed".to_string()))
    }

    /// Extract speech segment from continuous audio stream
    ///
    /// Returns `Ok(Some(audio))` when speech segment detected and silence threshold reached
    /// Returns `Ok(None)` if no speech detected or recording too short
    /// Returns `Err` on VAD processing errors
    pub fn extract_speech_from_frames<I>(&mut self, frames: I) -> Result<Option<Vec<i16>>>
    where
        I: IntoIterator<Item = Vec<i16>>,
    {
        let mut recording = Vec::new();
        let mut speech_detected = false;
        let mut silence_count = 0u32;
        let mut skip_count = self.skip_initial_frames;

        for frame in frames {
            // Skip initial frames to avoid keyboard noise
            if skip_count > 0 {
                skip_count -= 1;
                continue;
            }

            let is_speech = self.is_voice_frame(&frame)?;

            if is_speech {
                silence_count = 0;
                speech_detected = true;
                recording.extend_from_slice(&frame);
            } else if speech_detected {
                silence_count += 1;
                recording.extend_from_slice(&frame);

                // Check if silence threshold reached
                if silence_count >= self.silence_threshold_frames {
                    break;
                }
            }
        }

        // Check minimum duration
        if !speech_detected {
            return Ok(None);
        }

        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let duration_ms = ((recording.len() as f64 / f64::from(self.sample_rate)) * 1000.0) as u32;

        if duration_ms < self.min_duration_ms {
            return Ok(None);
        }

        Ok(Some(recording))
    }

    /// Get the expected frame size for this VAD
    #[must_use]
    pub const fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Get sample rate
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get frame duration in milliseconds
    #[must_use]
    pub const fn frame_duration_ms(&self) -> u32 {
        self.frame_duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_config_default() {
        let config = VadConfig::default_16khz();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.aggressiveness, 2);
        assert_eq!(config.silence_ms, 900);
        assert_eq!(config.min_duration_ms, 500);
        assert_eq!(config.skip_initial_ms, 150);
    }

    #[test]
    fn test_vad_creation() {
        let config = VadConfig::default_16khz();
        let vad = VoiceActivityDetector::new(&config).unwrap();
        assert_eq!(vad.sample_rate(), 16000);
        assert_eq!(vad.frame_size(), 480);
        assert_eq!(vad.frame_duration_ms(), 30);
    }

    #[test]
    fn test_vad_invalid_sample_rate() {
        let config = VadConfig {
            sample_rate: 44100,
            ..VadConfig::default_16khz()
        };
        assert!(VoiceActivityDetector::new(&config).is_err());
    }

    #[test]
    fn test_vad_invalid_aggressiveness() {
        let config = VadConfig {
            aggressiveness: 4,
            ..VadConfig::default_16khz()
        };
        assert!(VoiceActivityDetector::new(&config).is_err());
    }

    #[test]
    fn test_vad_invalid_frame_size() {
        let config = VadConfig::default_16khz();
        let mut vad = VoiceActivityDetector::new(&config).unwrap();

        // Wrong size frame
        let frame = vec![0i16; 100];
        assert!(vad.is_voice_frame(&frame).is_err());
    }

    #[test]
    fn test_vad_silence_frame() {
        let config = VadConfig::default_16khz();
        let mut vad = VoiceActivityDetector::new(&config).unwrap();

        // Silence frame (all zeros)
        let frame = vec![0i16; 480];
        let result = vad.is_voice_frame(&frame).unwrap();
        // Silence should not be detected as speech
        assert!(!result);
    }

    #[test]
    fn test_extract_speech_no_speech() {
        let config = VadConfig::default_16khz();
        let mut vad = VoiceActivityDetector::new(&config).unwrap();

        // Create silent frames
        let frames: Vec<Vec<i16>> = (0..100).map(|_| vec![0i16; 480]).collect();

        let result = vad.extract_speech_from_frames(frames).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_speech_with_noise() {
        let config = VadConfig::default_16khz();
        let mut vad = VoiceActivityDetector::new(&config).unwrap();

        // Test that random noise doesn't crash the VAD
        // Result is non-deterministic (depends on random values)
        let mut frames: Vec<Vec<i16>> = Vec::new();

        // 5 frames of skip
        for _ in 0..5 {
            frames.push(vec![0i16; 480]);
        }

        // Some frames with random noise
        for _ in 0..20 {
            let frame: Vec<i16> = (0..480).map(|_| rand::random::<i16>() / 4).collect();
            frames.push(frame);
        }

        // Many frames of silence
        for _ in 0..40 {
            frames.push(vec![0i16; 480]);
        }

        // Just verify it doesn't panic - result may be Some or None
        let _result = vad.extract_speech_from_frames(frames).unwrap();
    }
}
