//! Embedded tray icon assets
//!
//! Custom SVG icons matching the Scribe brand identity.
//! All icons use a hexagon shape derived from the main logo.
//!
//! Note: Currently these SVG icons are referenced via `icon_theme_path()` and `icon_name()`.
//! Future improvement: Convert to ARGB32 format for use with `icon_pixmap()` method.

/// Idle state icon - Grey hexagon with microphone
///
/// Color: Grey (#6b7280) with light grey mic (#d1d5db)
/// Indicates: App ready, waiting for command
#[allow(dead_code)] // Reserved for future icon_pixmap() implementation
pub const ICON_IDLE: &[u8] = include_bytes!("../../icons/tray/scribe-tray-idle.svg");

/// Recording state icon - Orange-red gradient hexagon with active microphone
///
/// Color: Orange to red gradient (#f97316 → #dc2626) - matches logo
/// Indicates: Currently capturing audio
#[allow(dead_code)] // Reserved for future icon_pixmap() implementation
pub const ICON_RECORDING: &[u8] = include_bytes!("../../icons/tray/scribe-tray-recording.svg");

/// Transcribing state icon - Blue/yellow hexagon with spinner
///
/// Color: Yellow/blue gradient with animated spinner
/// Indicates: Processing audio with Whisper model
#[allow(dead_code)] // Reserved for future icon_pixmap() implementation
pub const ICON_TRANSCRIBING: &[u8] =
    include_bytes!("../../icons/tray/scribe-tray-transcribing.svg");

/// Error state icon - Red hexagon with exclamation mark
///
/// Color: Red (#dc2626) with white exclamation mark
/// Indicates: Error occurred (audio device, transcription failed, etc.)
#[allow(dead_code)] // Reserved for future icon_pixmap() implementation
pub const ICON_ERROR: &[u8] = include_bytes!("../../icons/tray/scribe-tray-error.svg");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_embedded() {
        // Verify all icons are embedded and non-empty
        assert!(!ICON_IDLE.is_empty(), "Idle icon should be embedded");
        assert!(
            !ICON_RECORDING.is_empty(),
            "Recording icon should be embedded"
        );
        assert!(
            !ICON_TRANSCRIBING.is_empty(),
            "Transcribing icon should be embedded"
        );
        assert!(!ICON_ERROR.is_empty(), "Error icon should be embedded");
    }

    #[test]
    fn test_icons_are_svg() {
        // Verify icons are valid SVG by checking they start with SVG header
        assert!(
            ICON_IDLE.starts_with(b"<svg"),
            "Idle icon should be SVG format"
        );
        assert!(
            ICON_RECORDING.starts_with(b"<svg"),
            "Recording icon should be SVG format"
        );
        assert!(
            ICON_TRANSCRIBING.starts_with(b"<svg"),
            "Transcribing icon should be SVG format"
        );
        assert!(
            ICON_ERROR.starts_with(b"<svg"),
            "Error icon should be SVG format"
        );
    }

    #[test]
    fn test_icon_sizes() {
        // Icons should be small (under 2KB each as per design docs)
        assert!(
            ICON_IDLE.len() < 2048,
            "Idle icon should be under 2KB, got {} bytes",
            ICON_IDLE.len()
        );
        assert!(
            ICON_RECORDING.len() < 2048,
            "Recording icon should be under 2KB, got {} bytes",
            ICON_RECORDING.len()
        );
        assert!(
            ICON_TRANSCRIBING.len() < 2048,
            "Transcribing icon should be under 2KB, got {} bytes",
            ICON_TRANSCRIBING.len()
        );
        assert!(
            ICON_ERROR.len() < 2048,
            "Error icon should be under 2KB, got {} bytes",
            ICON_ERROR.len()
        );
    }
}
