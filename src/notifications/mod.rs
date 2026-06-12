// Sentinel Notifications: System notifications via notify-send

pub struct NotificationManager;

impl NotificationManager {
    pub fn notify(title: &str, body: &str, urgency: &str) -> anyhow::Result<()> {
        println!("Notification [{}]: {} - {}", urgency, title, body);
        Ok(())
    }

    pub fn memory_saved(key: &str) -> anyhow::Result<()> {
        Self::notify("Memory Saved", &format!("Key: {}", key), "normal")
    }

    pub fn skill_installed(name: &str) -> anyhow::Result<()> {
        Self::notify("Skill Installed", name, "normal")
    }

    pub fn model_loaded(name: &str) -> anyhow::Result<()> {
        Self::notify("Model Loaded", name, "normal")
    }

    pub fn guardian_warning(ram_pct: f32) -> anyhow::Result<()> {
        Self::notify("Memory Alert", &format!("RAM: {:.1}%", ram_pct), "critical")
    }
}
