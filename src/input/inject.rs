use crate::error::{Result, ScribeError};
use std::io::Write;
use std::process::{Child, Command, Stdio};

/// Text injector using dotool
///
/// dotool is a command-line tool that sends keyboard/mouse events to the system.
/// It reads commands from stdin, one per line:
/// - `typedelay X` - Set delay between keystrokes (in ms)
/// - `type TEXT` - Type the specified text
///
/// This struct maintains a long-lived dotool process for efficient text injection.
#[derive(Debug)]
pub struct TextInjector {
    dotool_process: Option<Child>,
    delay_ms: u64,
}

impl TextInjector {
    /// Create a new text injector with specified typing delay
    ///
    /// This will spawn a dotool process and keep it alive for reuse.
    /// The process is kept alive until `cleanup()` is called or the struct is dropped.
    ///
    /// # Errors
    /// - Returns error if dotool binary not found in PATH
    /// - Returns error if dotool process fails to start
    pub fn new(delay_ms: u64) -> Result<Self> {
        // Verify dotool is available
        if which::which("dotool").is_err() {
            return Err(ScribeError::Injection(
                "dotool binary not found in PATH. Install with: cargo install dotool".to_string(),
            ));
        }

        Ok(Self {
            dotool_process: None,
            delay_ms,
        })
    }

    /// Inject text into the active window
    ///
    /// This sends the text to dotool for typing. The process is spawned on first use
    /// and reused for subsequent calls for efficiency.
    ///
    /// # Errors
    /// - Returns error if dotool process fails to spawn
    /// - Returns error if writing to dotool stdin fails
    /// - Returns error if process unexpectedly terminates
    pub fn inject(&mut self, text: &str) -> Result<()> {
        // Ensure process is running
        self.ensure_process_running()?;

        let process = self
            .dotool_process
            .as_mut()
            .ok_or_else(|| ScribeError::Injection("dotool process not available".to_string()))?;

        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| ScribeError::Injection("dotool stdin not available".to_string()))?;

        // Set typing delay
        writeln!(stdin, "typedelay {}", self.delay_ms).map_err(|e| {
            ScribeError::Injection(format!("Failed to write typedelay command: {e}"))
        })?;

        // Send text - dotool expects "type TEXT" where TEXT is the literal string
        writeln!(stdin, "type {text}")
            .map_err(|e| ScribeError::Injection(format!("Failed to write type command: {e}")))?;

        // Flush to ensure commands are sent immediately
        stdin
            .flush()
            .map_err(|e| ScribeError::Injection(format!("Failed to flush stdin: {e}")))?;

        Ok(())
    }

    /// Clean up the dotool process
    ///
    /// This kills the dotool process and cleans up resources.
    /// Called automatically on drop, but can be called manually for explicit cleanup.
    pub fn cleanup(&mut self) {
        if let Some(mut process) = self.dotool_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
    }

    /// Ensure the dotool process is running, spawning it if necessary
    fn ensure_process_running(&mut self) -> Result<()> {
        // Check if process is still alive
        if let Some(process) = &mut self.dotool_process {
            if let Ok(Some(_)) = process.try_wait() {
                // Process exited, need to restart
                self.dotool_process = None;
            }
        }

        // Spawn new process if needed
        if self.dotool_process.is_none() {
            let process = Command::new("dotool")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| ScribeError::Injection(format!("Failed to spawn dotool: {e}")))?;

            self.dotool_process = Some(process);
        }

        Ok(())
    }
}

impl Drop for TextInjector {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_without_dotool() {
        // This test verifies error handling when dotool is not available
        // In CI, dotool won't be installed, so we expect an error
        let result = TextInjector::new(2);

        // If dotool is installed, test passes
        // If not installed, verify we get the right error
        if let Err(err) = result {
            assert!(matches!(err, ScribeError::Injection(_)));
            assert!(err.to_string().contains("dotool binary not found"));
        }
    }

    #[test]
    fn test_new_with_delay() {
        // Test that we can create an injector with various delays
        let delays = [0, 2, 5, 10, 50];

        for delay in delays {
            let injector = TextInjector::new(delay);
            // If dotool available, should succeed
            // If not, should fail with expected error
            if let Ok(inj) = injector {
                assert_eq!(inj.delay_ms, delay);
            }
        }
    }

    #[test]
    fn test_cleanup() {
        // Test that cleanup doesn't panic even when no process exists
        let mut injector = TextInjector {
            dotool_process: None,
            delay_ms: 2,
        };

        injector.cleanup();
        // Should not panic
    }

    // Integration test - only runs if dotool is available
    #[test]
    #[ignore = "requires dotool binary to be installed"]
    fn test_inject_text() {
        let mut injector = TextInjector::new(2).expect("dotool must be installed for this test");

        // This would actually type text, so we skip in normal tests
        // In a real integration test, you'd verify the text appears
        let result = injector.inject("Hello, World!");

        assert!(
            result.is_ok(),
            "Failed to inject text: {}",
            result.unwrap_err()
        );
    }

    #[test]
    #[ignore = "requires dotool binary to be installed"]
    fn test_inject_multiline() {
        let mut injector = TextInjector::new(2).expect("dotool must be installed for this test");

        let text = "Line 1\nLine 2\nLine 3";
        let result = injector.inject(text);

        assert!(
            result.is_ok(),
            "Failed to inject multiline text: {}",
            result.unwrap_err()
        );
    }

    #[test]
    #[ignore = "requires dotool binary to be installed"]
    fn test_inject_special_chars() {
        let mut injector = TextInjector::new(2).expect("dotool must be installed for this test");

        // Test with special characters
        let texts = vec![
            "Hello, World!",
            "Test with \"quotes\"",
            "Symbols: @#$%^&*()",
            "Unicode: café, naïve, 日本語",
        ];

        for text in texts {
            let result = injector.inject(text);
            assert!(
                result.is_ok(),
                "Failed to inject '{}': {}",
                text,
                result.unwrap_err()
            );
        }
    }

    #[test]
    #[ignore = "requires dotool binary to be installed"]
    fn test_multiple_injections() {
        let mut injector = TextInjector::new(2).expect("dotool must be installed for this test");

        // Test that we can reuse the same process for multiple injections
        for i in 1..=5 {
            let result = injector.inject(&format!("Injection {i} "));
            assert!(
                result.is_ok(),
                "Failed injection {}: {}",
                i,
                result.unwrap_err()
            );
        }
    }
}
