use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::AtomicPtr;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// GtkFixed container where tab webviews are placed.
/// Stored as a raw pointer because gtk::Fixed doesn't implement Sync.
/// Only dereferenced on the GTK main thread (enforced by all call sites
/// using on_main_thread).
#[cfg(target_os = "linux")]
pub static FIXED_CONTAINER: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[derive(Debug, Clone, Copy, Default)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Viewport {
    pub fn is_valid(self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }
}

pub static VIEWPORT: std::sync::Mutex<Viewport> = std::sync::Mutex::new(Viewport {
    x: 0.0,
    y: 0.0,
    width: 0.0,
    height: 0.0,
});

/// ponytail: avoids uuid crate. If collisions become a problem, swap to `uuid`.
pub fn generate_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("a{:x}", nanos)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub platform: String,
    pub label: String,
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub accounts: Vec<Account>,
}

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub config_path: PathBuf,
    pub sessions_dir: PathBuf,
}

impl AppState {
    pub fn load_config(path: &PathBuf) -> AppConfig {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
        let content = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    pub fn app_data_dir() -> PathBuf {
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                PathBuf::from(home).join(".local").join("share")
            });
        base.join("social-manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_id_should_return_non_empty_string() {
        let id = generate_id();
        assert!(!id.is_empty(), "id should not be empty");
        assert!(id.starts_with('a'), "id should start with 'a'");
    }

    #[test]
    fn generate_id_should_be_unique_on_consecutive_calls() {
        let a = generate_id();
        let b = generate_id();
        assert_ne!(a, b, "consecutive IDs should differ");
    }

    #[test]
    fn app_data_dir_should_contain_social_manager() {
        let dir = AppState::app_data_dir();
        assert!(dir.to_string_lossy().contains("social-manager"));
    }

    #[test]
    fn load_config_should_return_default_when_file_missing() {
        let path = PathBuf::from("/tmp/nonexistent_social_manager_test_config.toml");
        let config = AppState::load_config(&path);
        assert!(config.accounts.is_empty());
    }

    #[test]
    fn account_should_roundtrip_through_toml() {
        let account = Account {
            id: "test-1".into(),
            platform: "zalo".into(),
            label: "Test".into(),
            url: "https://chat.zalo.me".into(),
            enabled: true,
        };
        let config = AppConfig {
            accounts: vec![account.clone()],
        };
        let serialized = toml::to_string_pretty(&config).expect("should serialize");
        let deserialized: AppConfig = toml::from_str(&serialized).expect("should deserialize");
        assert_eq!(deserialized.accounts.len(), 1);
        assert_eq!(deserialized.accounts[0].id, "test-1");
        assert_eq!(deserialized.accounts[0].platform, "zalo");
    }
}
