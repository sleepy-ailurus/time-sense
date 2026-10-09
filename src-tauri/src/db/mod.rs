pub mod models;
pub mod activity_dao;
pub mod category_dao;
pub mod rule_dao;
pub mod pomodoro_dao;
pub mod settings_dao;
pub mod aggregates_dao;
pub mod goal_dao;
pub mod migration;

use std::path::PathBuf;
use rusqlite::Connection;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use anyhow::Result;

pub use models::*;
pub use activity_dao::ActivityDao;
pub use category_dao::CategoryDao;
pub use rule_dao::{RuleDao, RuleMatcher};
pub use pomodoro_dao::PomodoroDao;
pub use settings_dao::SettingsDao;
pub use aggregates_dao::AggregatesDao;
pub use goal_dao::GoalDao;

/// 数据库封装
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let app_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_dir)?;

        let db_path: PathBuf = app_dir.join("data.db");
        tracing::info!("Database path: {}", db_path.display());

        let conn = Connection::open(db_path)?;
        // 开启 WAL 模式，提升并发性能
        let _: String = conn.query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 初始化数据库（执行迁移）
    pub fn init(&self) -> Result<()> {
        let conn = self.conn.lock();
        migration::run_migrations(&conn)?;
        Ok(())
    }

    pub fn conn(&self) -> &Mutex<Connection> {
        &self.conn
    }
}
