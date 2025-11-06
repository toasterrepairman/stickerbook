use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentItem {
    pub path: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentStore {
    items: Vec<RecentItem>,
    max_items: usize,
}

impl RecentStore {
    pub fn new(max_items: usize) -> Self {
        Self {
            items: Vec::new(),
            max_items,
        }
    }

    pub fn load() -> Self {
        let config_path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(store) = serde_json::from_str(&content) {
                return store;
            }
        }
        Self::new(50)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn add(&mut self, path: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Remove if already exists
        self.items.retain(|item| item.path != path);

        // Add to front
        self.items.insert(0, RecentItem { path, timestamp });

        // Trim to max_items
        if self.items.len() > self.max_items {
            self.items.truncate(self.max_items);
        }
    }

    pub fn remove(&mut self, path: &str) {
        self.items.retain(|item| item.path != path);
    }

    pub fn items(&self) -> &[RecentItem] {
        &self.items
    }

    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("stickerbook");
        path.push("recent.json");
        path
    }
}
