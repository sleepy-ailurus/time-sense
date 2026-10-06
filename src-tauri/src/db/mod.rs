pub mod models;
pub mod activity_dao;

use std::path::PathBuf;
use rusqlite::Connection;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use anyhow::Result;

pub use models::*;
pub use activity_dao::ActivityDao;

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

    /// 初始化数据库表
    pub fn init(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS activity_logs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                process_name TEXT    NOT NULL,
                window_title TEXT,
                start_time  INTEGER NOT NULL,
                end_time    INTEGER NOT NULL,
                duration    INTEGER NOT NULL,
                is_idle     INTEGER DEFAULT 0,
                created_at  INTEGER DEFAULT (strftime('%s','now'))
            )
            "#,
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_activity_logs_start_time 
             ON activity_logs(start_time)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_activity_logs_process 
             ON activity_logs(process_name)",
            [],
        )?;

        Ok(())
    }

    pub fn conn(&self) -> &Mutex<Connection> {
        &self.conn
    }
}
