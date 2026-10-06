use anyhow::Result;
use chrono::{Datelike, Local, TimeZone};
use rusqlite::{params, Connection};

use super::models::{ActivityLog, AppStat, TodayTotal};

pub struct ActivityDao;

impl ActivityDao {
    /// 插入一条活动记录
    pub fn insert(conn: &Connection, log: &ActivityLog) -> Result<i64> {
        conn.execute(
            "INSERT INTO activity_logs 
             (process_name, window_title, start_time, end_time, duration, is_idle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                log.process_name,
                log.window_title,
                log.start_time,
                log.end_time,
                log.duration,
                log.is_idle as i32,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取今日的应用统计（按耗时倒序）
    pub fn get_today_stats(conn: &Connection) -> Result<Vec<AppStat>> {
        let (start_of_day, _) = today_timestamp_range();

        // 先计算总活跃时长
        let total_active: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0) 
             FROM activity_logs 
             WHERE start_time >= ?1 AND is_idle = 0",
            params![start_of_day],
            |row| row.get(0),
        )?;

        // 按进程分组统计
        let mut stmt = conn.prepare(
            "SELECT process_name, SUM(duration) as total
             FROM activity_logs
             WHERE start_time >= ?1 AND is_idle = 0
             GROUP BY process_name
             ORDER BY total DESC",
        )?;

        let rows = stmt.query_map(params![start_of_day], |row| {
            let process_name: String = row.get(0)?;
            let total_seconds: i64 = row.get(1)?;
            let percentage = if total_active > 0 {
                (total_seconds as f64 / total_active as f64) * 100.0
            } else {
                0.0
            };
            Ok(AppStat {
                process_name,
                total_seconds,
                percentage,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 获取今日总时长
    pub fn get_today_total(conn: &Connection) -> Result<TodayTotal> {
        let (start_of_day, _) = today_timestamp_range();

        let mut stmt = conn.prepare(
            "SELECT is_idle, COALESCE(SUM(duration), 0)
             FROM activity_logs
             WHERE start_time >= ?1
             GROUP BY is_idle",
        )?;

        let mut active_seconds = 0i64;
        let mut idle_seconds = 0i64;

        let rows = stmt.query_map(params![start_of_day], |row| {
            let is_idle: i32 = row.get(0)?;
            let duration: i64 = row.get(1)?;
            Ok((is_idle, duration))
        })?;

        for row in rows {
            let (is_idle, duration) = row?;
            if is_idle == 1 {
                idle_seconds = duration;
            } else {
                active_seconds = duration;
            }
        }

        Ok(TodayTotal {
            total_seconds: active_seconds + idle_seconds,
            active_seconds,
            idle_seconds,
        })
    }
}

/// 获取今日 00:00 和明日 00:00 的 Unix 时间戳（秒）
fn today_timestamp_range() -> (i64, i64) {
    let now = Local::now();
    let today_start = Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .unwrap()
        .timestamp();
    let tomorrow_start = today_start + 86400;
    (today_start, tomorrow_start)
}
