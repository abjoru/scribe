use crate::error::Result;

/// Audio capture configuration
#[allow(dead_code)] // TODO: Remove when implemented
pub struct AudioCapture {
    sample_rate: u32,
}

impl AudioCapture {
    pub fn new(sample_rate: u32) -> Result<Self> {
        Ok(Self { sample_rate })
    }

    // TODO: Implement audio capture methods
}
