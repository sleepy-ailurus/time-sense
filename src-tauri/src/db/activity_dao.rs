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
             (process_name, window_title, start_time, end_time, duration, is_idle, category_id, site_label)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                log.process_name,
                log.window_title,
                log.start_time,
                log.end_time,
                log.duration,
                log.is_idle as i32,
                log.category_id,
                log.site_label,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新活动记录的结束时间、持续时长和分类
    pub fn update_end_time(conn: &Connection, id: i64, end_time: i64, duration: i64, category_id: Option<i64>) -> Result<()> {
        conn.execute(
            "UPDATE activity_logs SET end_time = ?1, duration = ?2, category_id = ?3 WHERE id = ?4",
            params![end_time, duration, category_id, id],
        )?;
        Ok(())
    }

    /// 获取应用统计（按耗时倒序，带分类信息）
    /// date 为 None 时查今天，Some 时查指定日期（YYYY-MM-DD）
    pub fn get_stats_by_date(conn: &Connection, date: Option<&str>) -> Result<Vec<AppStat>> {
        let (start_ts, end_ts) = match date {
            Some(d) => date_timestamp_range(d)?,
            None => today_timestamp_range(),
        };

        // 先计算总活跃时长
        let total_active: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0)
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 0",
            params![start_ts, end_ts],
            |row| row.get(0),
        )?;

        // 按展示名分组统计，关联分类：
        // 命中 title 规则且带站点标签的活动按站点名聚合（如「抖音」），其余按进程名
        let mut stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(al.site_label, ''), al.process_name) as display_name,
                    SUM(al.duration) as total,
                    c.id, c.name, c.color
             FROM activity_logs al
             LEFT JOIN categories c ON al.category_id = c.id
             WHERE al.start_time >= ?1 AND al.start_time < ?2 AND al.is_idle = 0
             GROUP BY display_name
             ORDER BY total DESC",
        )?;

        let rows = stmt.query_map(params![start_ts, end_ts], |row| {
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
                category_id: row.get(2)?,
                category_name: row.get(3)?,
                category_color: row.get(4)?,
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

    /// 获取指定日期的所有活动记录（按开始时间正序）
    pub fn get_activity_by_date(conn: &Connection, date: &str) -> Result<Vec<ActivityLog>> {
        let (start_ts, end_ts) = date_timestamp_range(date)?;

        let mut stmt = conn.prepare(
            "SELECT id, process_name, window_title, start_time, end_time, duration, is_idle, category_id, site_label
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2
             ORDER BY start_time ASC",
        )?;

        let rows = stmt.query_map(params![start_ts, end_ts], |row| {
            let is_idle_int: i32 = row.get(6)?;
            Ok(ActivityLog {
                id: Some(row.get(0)?),
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration: row.get(5)?,
                is_idle: is_idle_int == 1,
                category_id: row.get(7)?,
                site_label: row.get(8)?,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 修复未闭合的记录（end_time = 0 或 end_time = start_time）
    /// 程序异常退出后重启时调用，将未闭合记录的 end_time 设为 start_time，duration 设为 0
    pub fn fix_open_records(conn: &Connection) -> Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT id, start_time FROM activity_logs WHERE end_time = 0 OR duration = 0",
        )?;

        let ids: Vec<i64> = stmt.query_map([], |row| row.get::<_, i64>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let count = ids.len();

        for id in &ids {
            conn.execute(
                "UPDATE activity_logs SET end_time = start_time, duration = 0 WHERE id = ?1",
                params![id],
            )?;
        }

        if count > 0 {
            tracing::warn!("Fixed {} unclosed activity records", count);
        }

        Ok(count)
    }
}

/// 获取今日 00:00 和明日 00:00 的 Unix 时间戳（秒）
pub(super) fn today_timestamp_range() -> (i64, i64) {
    let now = Local::now();
    let today_start = Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .unwrap()
        .timestamp();
    let tomorrow_start = today_start + 86400;
    (today_start, tomorrow_start)
}

/// 解析 YYYY-MM-DD 日期字符串，返回当天起止时间戳（秒）
pub(super) fn date_timestamp_range(date_str: &str) -> Result<(i64, i64)> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid date format, expected YYYY-MM-DD: {}", date_str);
    }

    let year: i32 = parts[0].parse().map_err(|_| anyhow::anyhow!("Invalid year"))?;
    let month: u32 = parts[1].parse().map_err(|_| anyhow::anyhow!("Invalid month"))?;
    let day: u32 = parts[2].parse().map_err(|_| anyhow::anyhow!("Invalid day"))?;

    let start = Local
        .with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", date_str))?
        .timestamp();

    let end = start + 86400;
    Ok((start, end))
}
