use crate::error::Result;

/// Text injector using dotool
#[allow(dead_code)] // TODO: Remove when implemented
pub struct TextInjector {
    delay_ms: u64,
}

impl TextInjector {
    pub fn new(delay_ms: u64) -> Result<Self> {
        Ok(Self { delay_ms })
    }

    pub fn inject(&self, _text: &str) -> Result<()> {
        // TODO: Implement text injection via dotool
        Ok(())
    }
}
