use crate::error::Result;

/// Local Whisper transcription using ONNX Runtime
pub struct LocalTranscriber {
    // TODO: Add model fields
}

impl LocalTranscriber {
    pub const fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub const fn transcribe(&self, _audio: &[i16]) -> Result<String> {
        // TODO: Implement local transcription
        Ok(String::new())
    }
}
