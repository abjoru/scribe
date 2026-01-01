use crate::error::Result;

/// `OpenAI` API transcription
#[allow(dead_code)] // TODO: Remove when implemented
pub struct ApiTranscriber {
    api_key: String,
}

impl ApiTranscriber {
    pub fn new(api_key: String) -> Result<Self> {
        Ok(Self { api_key })
    }

    #[allow(clippy::unused_async)] // TODO: Will be async when implemented
    pub async fn transcribe(&self, _audio: &[i16]) -> Result<String> {
        // TODO: Implement API transcription
        Ok(String::new())
    }
}
