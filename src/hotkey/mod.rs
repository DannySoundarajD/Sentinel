// Sentinel Hotkey: Global hotkey registration for Super+Space

#[derive(Debug, Clone)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

pub struct HotkeyManager;

impl HotkeyManager {
    pub fn detect_display_server() -> DisplayServer {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            DisplayServer::Wayland
        } else if std::env::var("DISPLAY").is_ok() {
            DisplayServer::X11
        } else {
            DisplayServer::Unknown
        }
    }

    pub fn register_hotkey() -> anyhow::Result<()> {
        let ds = Self::detect_display_server();
        match ds {
            DisplayServer::X11 => Ok(()),
            DisplayServer::Wayland => Ok(()),
            DisplayServer::Unknown => Err(anyhow::anyhow!("No display server detected")),
        }
    }

    pub fn toggle_window() -> anyhow::Result<()> {
        Ok(())
    }
}
