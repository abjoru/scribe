mod icons;

use crate::ipc::AppStatus;
use std::sync::{Arc, Mutex};

/// System tray icon manager using `StatusNotifierItem` protocol
pub struct TrayIcon {
    status: Arc<Mutex<AppStatus>>,
}

impl TrayIcon {
    /// Create new tray icon with shared status
    #[must_use]
    pub const fn new(status: Arc<Mutex<AppStatus>>) -> Self {
        Self { status }
    }

    /// Get shared status handle for updating from event loop
    #[must_use]
    pub fn status_handle(&self) -> Arc<Mutex<AppStatus>> {
        Arc::clone(&self.status)
    }
}

impl ksni::Tray for TrayIcon {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Left click - print status to terminal
        if let Ok(status) = self.status.lock() {
            println!("Scribe status: {status:?}");
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let svg_data = {
            let status = self.status.lock().unwrap();
            match *status {
                AppStatus::Idle => icons::ICON_IDLE,
                AppStatus::Recording => icons::ICON_RECORDING,
                AppStatus::Transcribing => icons::ICON_TRANSCRIBING,
                AppStatus::Error(_) => icons::ICON_ERROR,
            }
        };

        // Render SVG to ARGB32, return empty vec on failure
        icons::render_svg_to_argb32(svg_data)
            .map(|icon| vec![icon])
            .unwrap_or_default()
    }

    fn title(&self) -> String {
        let status = self.status.lock().unwrap();
        match &*status {
            AppStatus::Idle => "Scribe: Idle".to_string(),
            AppStatus::Recording => "Scribe: Recording".to_string(),
            AppStatus::Transcribing => "Scribe: Transcribing".to_string(),
            AppStatus::Error(msg) => format!("Scribe: Error - {msg}"),
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![StandardItem {
            label: "Quit".into(),
            activate: Box::new(|_| std::process::exit(0)),
            ..Default::default()
        }
        .into()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ksni::Tray;

    #[test]
    fn test_tray_icon_creation() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));
        assert_eq!(*tray.status.lock().unwrap(), AppStatus::Idle);
    }

    #[test]
    fn test_icon_pixmap() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));

        // Test idle icon
        let pixmap = tray.icon_pixmap();
        assert_eq!(pixmap.len(), 1, "Should return one icon");
        assert_eq!(pixmap[0].width, 96);
        assert_eq!(pixmap[0].height, 96);

        // Test recording icon
        *status.lock().unwrap() = AppStatus::Recording;
        let pixmap = tray.icon_pixmap();
        assert_eq!(pixmap.len(), 1, "Should return one icon");

        // Test transcribing icon
        *status.lock().unwrap() = AppStatus::Transcribing;
        let pixmap = tray.icon_pixmap();
        assert_eq!(pixmap.len(), 1, "Should return one icon");

        // Test error icon
        *status.lock().unwrap() = AppStatus::Error("test error".to_string());
        let pixmap = tray.icon_pixmap();
        assert_eq!(pixmap.len(), 1, "Should return one icon");
    }

    #[test]
    fn test_titles() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));

        assert_eq!(tray.title(), "Scribe: Idle");

        *status.lock().unwrap() = AppStatus::Recording;
        assert_eq!(tray.title(), "Scribe: Recording");

        *status.lock().unwrap() = AppStatus::Transcribing;
        assert_eq!(tray.title(), "Scribe: Transcribing");

        *status.lock().unwrap() = AppStatus::Error("Audio device error".to_string());
        assert_eq!(tray.title(), "Scribe: Error - Audio device error");
    }

    #[test]
    fn test_status_handle() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));

        let handle = tray.status_handle();
        *handle.lock().unwrap() = AppStatus::Recording;

        assert_eq!(*status.lock().unwrap(), AppStatus::Recording);
        assert_eq!(*tray.status.lock().unwrap(), AppStatus::Recording);
    }

    #[test]
    fn test_menu_has_quit() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));
        let menu = tray.menu();

        assert_eq!(menu.len(), 1);
    }
}
