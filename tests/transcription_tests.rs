use scribe::config::schema::TranscriptionConfig;
use scribe::transcription::Backend;

#[test]
fn test_backend_selection_local() {
    let config = TranscriptionConfig {
        backend: "local".to_string(),
        model: "tiny".to_string(), // Use tiny model for faster tests
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: None,
        api_model: None,
        api_timeout_secs: None,
    };

    // This will now download the model from HuggingFace Hub
    // It may succeed (if network available) or fail (if no network)
    let result = Backend::from_config(&config);

    // Either the backend was created successfully, or we got an error
    // related to downloading/network issues
    if let Err(e) = result {
        let err_str = e.to_string();
        // Accept network/download errors as valid test outcomes
        assert!(
            err_str.contains("Failed to download")
                || err_str.contains("Failed to initialize HuggingFace API")
                || err_str.contains("Failed to load")
                || err_str.contains("network"),
            "Unexpected error: {err_str}"
        );
    } else {
        // Success means the model was downloaded and loaded
        assert_eq!(result.unwrap().backend_name(), "local");
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
    // Test with local backend (doesn't require API key)
    let config = TranscriptionConfig {
        backend: "local".to_string(),
        model: "base".to_string(),
        device: "cpu".to_string(),
        language: "en".to_string(),
        initial_prompt: None,
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        api_model: Some("whisper-1".to_string()),
        api_timeout_secs: Some(30),
    };

    let backend = Backend::from_config(&config).unwrap();
    assert_eq!(backend.backend_name(), "local");
}
