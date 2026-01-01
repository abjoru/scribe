use scribe::config::schema::TranscriptionConfig;
use scribe::transcription::Backend;

#[test]
fn test_backend_selection_local() {
    let config = TranscriptionConfig {
        backend: "local".to_string(),
        model: "base".to_string(),
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: None,
        api_model: None,
        api_timeout_secs: None,
    };

    // This will fail if model doesn't exist, which is expected in test environment
    let result = Backend::from_config(&config);

    // We expect an error because the model file doesn't exist
    // but we can verify the backend type selection worked
    if let Err(e) = result {
        // Expected - model file doesn't exist in test environment
        assert!(e.to_string().contains("Model not found"));
    }
}

#[test]
fn test_backend_selection_openai_missing_key() {
    // Remove API key to test error handling
    let original = std::env::var("OPENAI_API_KEY_TEST").ok();
    std::env::remove_var("OPENAI_API_KEY_TEST");

    let config = TranscriptionConfig {
        backend: "openai".to_string(),
        model: "base".to_string(),
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: Some("OPENAI_API_KEY_TEST".to_string()),
        api_model: Some("whisper-1".to_string()),
        api_timeout_secs: Some(30),
    };

    let result = Backend::from_config(&config);
    assert!(result.is_err());

    // Verify we get the right error type
    assert!(result.unwrap_err().to_string().contains("Invalid API key"));

    // Restore original
    if let Some(val) = original {
        std::env::set_var("OPENAI_API_KEY_TEST", val);
    }
}

#[test]
fn test_backend_selection_invalid() {
    let config = TranscriptionConfig {
        backend: "invalid".to_string(),
        model: "base".to_string(),
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: None,
        api_model: None,
        api_timeout_secs: None,
    };

    let result = Backend::from_config(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown backend"));
}

#[test]
fn test_backend_name() {
    // Test with a valid API key for OpenAI backend
    std::env::set_var("OPENAI_API_KEY_TEST", "test-key");

    let config = TranscriptionConfig {
        backend: "openai".to_string(),
        model: "base".to_string(),
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: Some("OPENAI_API_KEY_TEST".to_string()),
        api_model: Some("whisper-1".to_string()),
        api_timeout_secs: Some(30),
    };

    let backend = Backend::from_config(&config).unwrap();
    assert_eq!(backend.backend_name(), "openai");

    std::env::remove_var("OPENAI_API_KEY_TEST");
}
