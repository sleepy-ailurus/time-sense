use anyhow::Result;
use chrono::{Datelike, Duration, Local, TimeZone};
use rusqlite::{params, Connection};

use super::models::{DailySummary, HeatmapDay, HourlyStat};

pub struct AggregatesDao;

impl AggregatesDao {
    /// 查询指定日期的汇总数据
    pub fn get_daily_summary(conn: &Connection, date: &str) -> Result<DailySummary> {
        let (start_ts, end_ts) = date_timestamp_range(date)?;

        // 按分类聚合活跃时长（非空闲）
        let mut stmt = conn.prepare(
            "SELECT category_id, COALESCE(SUM(duration), 0)
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 0
             GROUP BY category_id",
        )?;

        let mut work_seconds = 0i64;
        let mut study_seconds = 0i64;
        let mut entertainment_seconds = 0i64;
        let mut social_seconds = 0i64;
        let mut other_seconds = 0i64;

        let rows = stmt.query_map(params![start_ts, end_ts], |row| {
            let category_id: Option<i64> = row.get(0)?;
            let duration: i64 = row.get(1)?;
            Ok((category_id, duration))
        })?;

        for row in rows {
            let (category_id, duration) = row?;
            match category_id {
                Some(1) => work_seconds += duration,
                Some(2) => study_seconds += duration,
                Some(3) => entertainment_seconds += duration,
                Some(4) => social_seconds += duration,
                _ => other_seconds += duration, // 分类5（其他）和未分类的归入其他
            }
        }

        let total_seconds = work_seconds + study_seconds + entertainment_seconds + social_seconds + other_seconds;

        // 空闲时长
        let idle_seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0)
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 1",
            params![start_ts, end_ts],
            |row| row.get(0),
        )?;

        // 番茄钟统计（完成的专注番茄钟）
        let pomodoro_count: i32 = conn.query_row(
            "SELECT COALESCE(COUNT(*), 0)
             FROM pomodoro_sessions
             WHERE session_type = 'focus' AND completed = 1
               AND start_time >= ?1 AND start_time < ?2",
            params![start_ts, end_ts],
            |row| row.get(0),
        )?;

        let pomodoro_seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0)
             FROM pomodoro_sessions
             WHERE session_type = 'focus' AND completed = 1
               AND start_time >= ?1 AND start_time < ?2",
            params![start_ts, end_ts],
            |row| row.get(0),
        )?;

        Ok(DailySummary {
            date: date.to_string(),
            total_seconds,
            idle_seconds,
            work_seconds,
            study_seconds,
            entertainment_seconds,
            social_seconds,
            other_seconds,
            pomodoro_count,
            pomodoro_seconds,
        })
    }

    /// 查询过去 7 天（含今天）的每日汇总
    pub fn get_weekly_trend(conn: &Connection) -> Result<Vec<DailySummary>> {
        let now = Local::now();
        let mut result = Vec::with_capacity(7);

        // 生成过去 7 天的日期（含今天），按升序排列
        for i in (0..7).rev() {
            let date = now - Duration::days(i);
            let date_str = format!(
                "{:04}-{:02}-{:02}",
                date.year(),
                date.month(),
                date.day()
            );
            let summary = Self::get_daily_summary(conn, &date_str)?;
            result.push(summary);
        }

        Ok(result)
    }

    /// 查询指定年份的热力图数据（专注总时长=工作+学习）
    pub fn get_heatmap_data(conn: &Connection, year: i32) -> Result<Vec<HeatmapDay>> {
        let year_start = Local
            .with_ymd_and_hms(year, 1, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid year: {}", year))?
            .timestamp();

        let year_end = Local
            .with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid year: {}", year + 1))?
            .timestamp();

        // 按日期聚合工作+学习时长
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', datetime(start_time, 'unixepoch', 'localtime')) as day,
                    COALESCE(SUM(CASE WHEN category_id IN (1, 2) THEN duration ELSE 0 END), 0) as focus_seconds
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 0
             GROUP BY day
             ORDER BY day ASC",
        )?;

        let rows = stmt.query_map(params![year_start, year_end], |row| {
            let day: String = row.get(0)?;
            let focus_seconds: i64 = row.get(1)?;
            Ok((day, focus_seconds))
        })?;

        let mut day_focus: Vec<(String, i64)> = Vec::new();
        for row in rows {
            day_focus.push(row?);
        }

        // 查询番茄钟数据
        let mut pomo_stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', datetime(start_time, 'unixepoch', 'localtime')) as day,
                    COALESCE(COUNT(*), 0) as pomo_count
             FROM pomodoro_sessions
             WHERE session_type = 'focus' AND completed = 1
               AND start_time >= ?1 AND start_time < ?2
             GROUP BY day",
        )?;

        let pomo_rows = pomo_stmt.query_map(params![year_start, year_end], |row| {
            let day: String = row.get(0)?;
            let count: i32 = row.get(1)?;
            Ok((day, count))
        })?;

        let mut day_pomo: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        for row in pomo_rows {
            let (day, count) = row?;
            day_pomo.insert(day, count);
        }

        let mut result = Vec::new();
        for (day, focus_seconds) in day_focus {
            let pomodoro_count = day_pomo.get(&day).copied().unwrap_or(0);
            result.push(HeatmapDay {
                date: day,
                total_seconds: focus_seconds,
                pomodoro_count,
            });
        }

        Ok(result)
    }

    /// 查询时段分布
    /// 如果传了 date 查当天的 24 小时分布
    /// 如果没传 date 查过去 7 天的，用于 24h×7天 热力图
    pub fn get_hourly_distribution(conn: &Connection, date: Option<&str>) -> Result<Vec<HourlyStat>> {
        let (start_ts, end_ts, single_date) = match date {
            Some(d) => {
                let (s, e) = date_timestamp_range(d)?;
                (s, e, true)
            }
            None => {
                // 过去 7 天（含今天）
                let now = Local::now();
                let end_ts = Local
                    .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
                    .unwrap()
                    .timestamp()
                    + 86400; // 明天 0 点
                let start_ts = end_ts - 7 * 86400;
                (start_ts, end_ts, false)
            }
        };

        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', datetime(start_time, 'unixepoch', 'localtime')) as day,
                    CAST(strftime('%H', datetime(start_time, 'unixepoch', 'localtime')) AS INTEGER) as hour,
                    CAST(strftime('%w', datetime(start_time, 'unixepoch', 'localtime')) AS INTEGER) as weekday_sql,
                    COALESCE(SUM(duration), 0) as total_seconds
             FROM activity_logs
             WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 0
             GROUP BY day, hour
             ORDER BY day ASC, hour ASC",
        )?;

        let rows = stmt.query_map(params![start_ts, end_ts], |row| {
            let day: String = row.get(0)?;
            let hour: i32 = row.get(1)?;
            let weekday_sql: i32 = row.get(2)?; // SQLite: 0=周日, 1=周一, ..., 6=周六
            let total_seconds: i64 = row.get(3)?;
            Ok((day, hour, weekday_sql, total_seconds))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (day, hour, weekday_sql, total_seconds) = row?;
            // 转换为 0=周一 ... 6=周日
            let weekday = (weekday_sql + 6) % 7;
            result.push(HourlyStat {
                hour,
                total_seconds,
                weekday,
                date: day,
            });
        }

        // 如果是单日查询，补全 24 小时（没有数据的小时填 0）
        if single_date {
            if let Some(d) = date {
                let weekday = {
                    let (s, _) = date_timestamp_range(d)?;
                    let dt = Local.timestamp_opt(s, 0).single().unwrap();
                    // chrono: Mon=1 ... Sun=7, 转换为 0=周一 ... 6=周日
                    (dt.weekday().number_from_monday() - 1) as i32
                };
                let mut filled = Vec::with_capacity(24);
                for h in 0..24 {
                    if let Some(stat) = result.iter().find(|s| s.hour == h) {
                        filled.push(stat.clone());
                    } else {
                        filled.push(HourlyStat {
                            hour: h,
                            total_seconds: 0,
                            weekday,
                            date: d.to_string(),
                        });
                    }
                }
                return Ok(filled);
            }
        }

        Ok(result)
    }
}

/// 解析 YYYY-MM-DD 日期字符串，返回当天起止时间戳（秒）
fn date_timestamp_range(date_str: &str) -> Result<(i64, i64)> {
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
