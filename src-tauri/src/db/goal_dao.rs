use anyhow::Result;
use rusqlite::{params, Connection};

use super::models::{Goal, GoalStatus, NewGoal};
use super::activity_dao::today_timestamp_range;

pub struct GoalDao;

impl GoalDao {
    /// 获取所有目标
    pub fn list_all(conn: &Connection) -> Result<Vec<Goal>> {
        let mut stmt = conn.prepare(
            "SELECT g.id, g.category_id, c.name, c.color, g.daily_limit_minutes, g.enabled
             FROM goals g
             LEFT JOIN categories c ON g.category_id = c.id
             ORDER BY g.id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let enabled_int: i32 = row.get(5)?;
            Ok(Goal {
                id: row.get(0)?,
                category_id: row.get(1)?,
                category_name: row.get(2)?,
                category_color: row.get(3)?,
                daily_limit_minutes: row.get(4)?,
                enabled: enabled_int == 1,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 新增目标
    pub fn create(conn: &Connection, goal: &NewGoal) -> Result<i64> {
        conn.execute(
            "INSERT INTO goals (category_id, daily_limit_minutes, enabled)
             VALUES (?1, ?2, ?3)",
            params![goal.category_id, goal.daily_limit_minutes, goal.enabled as i32],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新目标
    pub fn update(conn: &Connection, id: i64, goal: &NewGoal) -> Result<()> {
        conn.execute(
            "UPDATE goals SET category_id = ?1, daily_limit_minutes = ?2, enabled = ?3 WHERE id = ?4",
            params![goal.category_id, goal.daily_limit_minutes, goal.enabled as i32, id],
        )?;
        Ok(())
    }

    /// 删除目标
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        conn.execute("DELETE FROM goals WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 获取所有启用的目标 + 今日已用时长
    pub fn get_active_goals_status(conn: &Connection) -> Result<Vec<GoalStatus>> {
        let goals = Self::list_all(conn)?;
        let (start_ts, end_ts) = today_timestamp_range();

        let mut result = Vec::new();
        for goal in goals {
            if !goal.enabled {
                continue;
            }
            // 查询该分类今日总活跃时长
            let used_seconds: i64 = conn.query_row(
                "SELECT COALESCE(SUM(duration), 0)
                 FROM activity_logs
                 WHERE start_time >= ?1 AND start_time < ?2 AND is_idle = 0 AND category_id = ?3",
                params![start_ts, end_ts, goal.category_id],
                |row| row.get(0),
            )?;

            let limit_seconds = (goal.daily_limit_minutes as i64) * 60;
            result.push(GoalStatus {
                goal,
                used_seconds,
                limit_seconds,
                exceeded: used_seconds > limit_seconds,
            });
        }
        Ok(result)
    }

    /// 检查是否有新超标的目标（返回本次刚超标的分类名列表）
    /// already_exceeded：之前已知的超标分类 ID 集合，只返回新超标的
    pub fn check_newly_exceeded(conn: &Connection, already_exceeded: &std::collections::HashSet<i64>) -> Result<Vec<String>> {
        let statuses = Self::get_active_goals_status(conn)?;
        let mut newly = Vec::new();
        for s in &statuses {
            if s.exceeded && !already_exceeded.contains(&s.goal.category_id) {
                if let Some(ref name) = s.goal.category_name {
                    newly.push(name.clone());
                }
            }
        }
        Ok(newly)
    }
}
