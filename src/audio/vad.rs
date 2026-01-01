use crate::error::Result;

/// Voice Activity Detector using WebRTC VAD
#[allow(dead_code)] // TODO: Remove when implemented
pub struct VoiceActivityDetector {
    sample_rate: u32,
    aggressiveness: u8,
}

impl VoiceActivityDetector {
    pub const fn new(sample_rate: u32, aggressiveness: u8) -> Result<Self> {
        Ok(Self {
            sample_rate,
            aggressiveness,
        })
    }

    // TODO: Implement VAD methods
}
