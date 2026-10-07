use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json;

use super::models::GeneralSettings;

pub struct SettingsDao;

impl SettingsDao {
    /// 加载常规设置
    pub fn load_general(conn: &Connection) -> GeneralSettings {
        let result: Result<String, rusqlite::Error> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'general'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(json_str) => {
                serde_json::from_str(&json_str).unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse general settings: {}, using defaults", e);
                    GeneralSettings::default()
                })
            }
            Err(_) => GeneralSettings::default(),
        }
    }

    /// 保存常规设置
    pub fn save_general(conn: &Connection, settings: &GeneralSettings) -> Result<()> {
        let json_str = serde_json::to_string(settings)?;
        let now = chrono::Local::now().timestamp();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('general', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = ?2",
            params![json_str, now],
        )?;
        Ok(())
    }
}
