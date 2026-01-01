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

    fn icon_name(&self) -> String {
        let icon_name = {
            let status = self.status.lock().unwrap();
            match *status {
                AppStatus::Idle => "scribe-tray-idle",
                AppStatus::Recording => "scribe-tray-recording",
                AppStatus::Transcribing => "scribe-tray-transcribing",
            }
        };
        icon_name.to_string()
    }

    fn icon_theme_path(&self) -> String {
        // Point to our custom icon directory
        // In development: use project icons, in production: use installed icons
        env!("CARGO_MANIFEST_DIR").to_string() + "/icons/tray"
    }

    fn title(&self) -> String {
        let status = self.status.lock().unwrap();
        match *status {
            AppStatus::Idle => "Scribe: Idle",
            AppStatus::Recording => "Scribe: Recording",
            AppStatus::Transcribing => "Scribe: Transcribing",
        }
        .to_string()
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
    fn test_icon_names() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));

        assert_eq!(tray.icon_name(), "scribe-tray-idle");

        *status.lock().unwrap() = AppStatus::Recording;
        assert_eq!(tray.icon_name(), "scribe-tray-recording");

        *status.lock().unwrap() = AppStatus::Transcribing;
        assert_eq!(tray.icon_name(), "scribe-tray-transcribing");
    }

    #[test]
    fn test_icon_theme_path() {
        let status = Arc::new(Mutex::new(AppStatus::Idle));
        let tray = TrayIcon::new(Arc::clone(&status));
        let path = tray.icon_theme_path();

        // Should point to our custom icon directory
        assert!(path.ends_with("/icons/tray"));
        assert!(path.contains("scribe"));
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
