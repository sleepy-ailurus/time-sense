use anyhow::Result;
use chrono::{Datelike, Local, TimeZone};
use rusqlite::{params, Connection};
use serde_json;

use super::models::{PomodoroSession, PomodoroSettings};

pub struct PomodoroDao;

impl PomodoroDao {
    /// 创建一个新的番茄钟会话（未完成状态）
    pub fn create_session(conn: &Connection, session_type: &str, start_time: i64) -> Result<i64> {
        conn.execute(
            "INSERT INTO pomodoro_sessions (session_type, start_time, end_time, duration, completed, interrupted)
             VALUES (?1, ?2, 0, 0, 0, 0)",
            params![session_type, start_time],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 完成会话
    pub fn complete_session(conn: &Connection, id: i64, end_time: i64, duration: i64, interrupted: bool) -> Result<()> {
        conn.execute(
            "UPDATE pomodoro_sessions SET end_time = ?1, duration = ?2, completed = 1, interrupted = ?3 
             WHERE id = ?4",
            params![end_time, duration, interrupted as i32, id],
        )?;
        Ok(())
    }

    /// 获取今日完成的专注番茄数
    pub fn get_today_focus_count(conn: &Connection) -> Result<i32> {
        let (start_of_day, _) = today_timestamp_range();
        let count: i32 = conn.query_row(
            "SELECT COALESCE(COUNT(*), 0) 
             FROM pomodoro_sessions 
             WHERE session_type = 'focus' AND completed = 1 AND interrupted = 0 
               AND start_time >= ?1",
            params![start_of_day],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 获取今日专注总时长
    pub fn get_today_focus_seconds(conn: &Connection) -> Result<i64> {
        let (start_of_day, _) = today_timestamp_range();
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0) 
             FROM pomodoro_sessions 
             WHERE session_type = 'focus' AND completed = 1 
               AND start_time >= ?1",
            params![start_of_day],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    /// 获取最近一个未完成的会话
    pub fn get_open_session(conn: &Connection) -> Result<Option<PomodoroSession>> {
        let mut stmt = conn.prepare(
            "SELECT id, session_type, start_time, end_time, duration, completed, interrupted
             FROM pomodoro_sessions 
             WHERE completed = 0
             ORDER BY id DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let completed_int: i32 = row.get(5)?;
            let interrupted_int: i32 = row.get(6)?;
            Ok(Some(PomodoroSession {
                id: Some(row.get(0)?),
                session_type: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                duration: row.get(4)?,
                completed: completed_int == 1,
                interrupted: interrupted_int == 1,
            }))
        } else {
            Ok(None)
        }
    }

    /// 修复未闭合的番茄钟（程序崩溃重启时）
    pub fn fix_open_sessions(conn: &Connection) -> Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT id, start_time FROM pomodoro_sessions WHERE completed = 0",
        )?;

        let sessions: Vec<(i64, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

        let count = sessions.len();
        for (id, _start_time) in &sessions {
            conn.execute(
                "UPDATE pomodoro_sessions SET end_time = start_time, duration = 0, 
                 completed = 1, interrupted = 1 WHERE id = ?1",
                params![id],
            )?;
        }

        if count > 0 {
            tracing::warn!("Fixed {} unclosed pomodoro sessions", count);
        }
        Ok(count)
    }

    /// 加载番茄钟设置（从 app_settings 表读取 JSON，不存在则返回默认值）
    pub fn load_settings(conn: &Connection) -> PomodoroSettings {
        let result: Result<String, rusqlite::Error> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'pomodoro'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(json_str) => {
                serde_json::from_str(&json_str).unwrap_or_else(|e| {
                    tracing::warn!("Failed to parse pomodoro settings: {}, using defaults", e);
                    PomodoroSettings::default()
                })
            }
            Err(_) => PomodoroSettings::default(),
        }
    }

    /// 保存番茄钟设置（JSON 存入 app_settings 表）
    pub fn save_settings(conn: &Connection, settings: &PomodoroSettings) -> Result<()> {
        let json_str = serde_json::to_string(settings)?;
        let now = chrono::Local::now().timestamp();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('pomodoro', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at = ?2",
            params![json_str, now],
        )?;
        Ok(())
    }
}

fn today_timestamp_range() -> (i64, i64) {
    let now = Local::now();
    let today_start = Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .unwrap()
        .timestamp();
    let tomorrow_start = today_start + 86400;
    (today_start, tomorrow_start)
}
