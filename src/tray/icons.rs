//! Embedded tray icon assets
//!
//! Custom SVG icons matching the Scribe brand identity.
//! All icons use a hexagon shape derived from the main logo.
//!
//! Icons are rendered from embedded SVG data to ARGB32 pixmaps for `StatusNotifierItem`.

/// Idle state icon - Grey hexagon with microphone
///
/// Color: Grey (#6b7280) with light grey mic (#d1d5db)
/// Indicates: App ready, waiting for command
pub const ICON_IDLE: &[u8] = include_bytes!("../../icons/tray/scribe-tray-idle.svg");

/// Recording state icon - Orange-red gradient hexagon with active microphone
///
/// Color: Orange to red gradient (#f97316 → #dc2626) - matches logo
/// Indicates: Currently capturing audio
pub const ICON_RECORDING: &[u8] = include_bytes!("../../icons/tray/scribe-tray-recording.svg");

/// Transcribing state icon - Blue/yellow hexagon with spinner
///
/// Color: Yellow/blue gradient with animated spinner
/// Indicates: Processing audio with Whisper model
pub const ICON_TRANSCRIBING: &[u8] =
    include_bytes!("../../icons/tray/scribe-tray-transcribing.svg");

/// Error state icon - Red hexagon with exclamation mark
///
/// Color: Red (#dc2626) with white exclamation mark
/// Indicates: Error occurred (audio device, transcription failed, etc.)
pub const ICON_ERROR: &[u8] = include_bytes!("../../icons/tray/scribe-tray-error.svg");

/// Standard tray icon size (96x96 pixels for better visibility)
const ICON_SIZE: u32 = 96;

/// Render SVG bytes to ARGB32 pixmap for `StatusNotifierItem`
///
/// Returns `ksni::Icon` with ARGB32 pixel data, or `None` on render failure.
///
/// # Arguments
/// * `svg_data` - Raw SVG bytes (from embedded assets)
///
/// # Format
/// ARGB32: 4 bytes per pixel in order: Alpha, Red, Green, Blue
#[allow(clippy::cast_precision_loss)] // 48 fits in f32 mantissa
pub fn render_svg_to_argb32(svg_data: &[u8]) -> Option<ksni::Icon> {
    // Parse SVG tree
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_data, &opts).ok()?;

    // Create pixmap for rendering
    let mut pixmap = resvg::tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE)?;

    // Render SVG to pixmap
    let render_ts = resvg::tiny_skia::Transform::from_scale(
        ICON_SIZE as f32 / tree.size().width(),
        ICON_SIZE as f32 / tree.size().height(),
    );
    resvg::render(&tree, render_ts, &mut pixmap.as_mut());

    // Convert RGBA to ARGB32 (StatusNotifierItem format)
    // tiny_skia uses premultiplied RGBA, we need ARGB
    let rgba_data = pixmap.data();
    let mut argb_data = Vec::with_capacity(rgba_data.len());

    for chunk in rgba_data.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];

        // Convert from premultiplied RGBA to straight ARGB
        argb_data.push(a);
        argb_data.push(r);
        argb_data.push(g);
        argb_data.push(b);
    }

    #[allow(clippy::cast_possible_wrap)] // ICON_SIZE=48 is safe for i32
    let icon_size_i32 = ICON_SIZE as i32;

    Some(ksni::Icon {
        width: icon_size_i32,
        height: icon_size_i32,
        data: argb_data,
    })
}

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

    #[test]
    fn test_render_svg_to_argb32() {
        // Test rendering all icons
        let idle_icon = render_svg_to_argb32(ICON_IDLE);
        assert!(idle_icon.is_some(), "Failed to render idle icon");

        let recording_icon = render_svg_to_argb32(ICON_RECORDING);
        assert!(recording_icon.is_some(), "Failed to render recording icon");

        let transcribing_icon = render_svg_to_argb32(ICON_TRANSCRIBING);
        assert!(
            transcribing_icon.is_some(),
            "Failed to render transcribing icon"
        );

        let error_icon = render_svg_to_argb32(ICON_ERROR);
        assert!(error_icon.is_some(), "Failed to render error icon");
    }

    #[test]
    fn test_rendered_icon_dimensions() {
        let icon = render_svg_to_argb32(ICON_IDLE).expect("Failed to render icon");

        // Should be 48x48
        #[allow(clippy::cast_possible_wrap)] // ICON_SIZE=48 is safe for i32
        let expected_size = ICON_SIZE as i32;
        assert_eq!(icon.width, expected_size);
        assert_eq!(icon.height, expected_size);

        // ARGB32: 4 bytes per pixel
        let expected_bytes = (ICON_SIZE * ICON_SIZE * 4) as usize;
        assert_eq!(icon.data.len(), expected_bytes);
    }

    #[test]
    fn test_invalid_svg() {
        let invalid_svg = b"<not valid svg>";
        let result = render_svg_to_argb32(invalid_svg);
        assert!(result.is_none(), "Should return None for invalid SVG");
    }
}
