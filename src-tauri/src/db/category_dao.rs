use anyhow::Result;
use chrono::{Datelike, Local, TimeZone};
use rusqlite::{params, Connection};

use super::models::{Category, CategoryStat};

pub struct CategoryDao;

impl CategoryDao {
    /// 获取所有分类（按排序号）
    pub fn list_all(conn: &Connection) -> Result<Vec<Category>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, sort_order, is_default 
             FROM categories 
             ORDER BY sort_order ASC, id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let is_default_int: i32 = row.get(5)?;
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                sort_order: row.get(4)?,
                is_default: is_default_int == 1,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 按 ID 获取分类
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Category>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, sort_order, is_default 
             FROM categories WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let is_default_int: i32 = row.get(5)?;
            Ok(Some(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                sort_order: row.get(4)?,
                is_default: is_default_int == 1,
            }))
        } else {
            Ok(None)
        }
    }

    /// 按名称查找分类
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Category>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, sort_order, is_default 
             FROM categories WHERE name = ?1",
        )?;

        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            let is_default_int: i32 = row.get(5)?;
            Ok(Some(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                sort_order: row.get(4)?,
                is_default: is_default_int == 1,
            }))
        } else {
            Ok(None)
        }
    }

    /// 新增分类
    pub fn create(conn: &Connection, name: &str, color: &str, icon: Option<&str>) -> Result<i64> {
        conn.execute(
            "INSERT INTO categories (name, color, icon) VALUES (?1, ?2, ?3)",
            params![name, color, icon],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新分类
    pub fn update(conn: &Connection, id: i64, name: &str, color: &str, icon: Option<&str>) -> Result<()> {
        conn.execute(
            "UPDATE categories SET name = ?1, color = ?2, icon = ?3 WHERE id = ?4",
            params![name, color, icon, id],
        )?;
        Ok(())
    }

    /// 删除分类（非默认分类才能删）
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        conn.execute("DELETE FROM categories WHERE id = ?1 AND is_default = 0", params![id])?;
        Ok(())
    }

    /// 获取今日分类统计（按耗时倒序）
    pub fn get_today_category_stats(conn: &Connection) -> Result<Vec<CategoryStat>> {
        let now = chrono::Local::now();
        let today_start = chrono::Local
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap()
            .timestamp();

        // 总活跃时长
        let total_active: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0) 
             FROM activity_logs 
             WHERE start_time >= ?1 AND is_idle = 0",
            params![today_start],
            |row| row.get(0),
        )?;

        // 按分类聚合
        let mut stmt = conn.prepare(
            "SELECT 
                COALESCE(c.id, 0) as cat_id,
                COALESCE(c.name, '未分类') as cat_name,
                COALESCE(c.color, '#6B7280') as cat_color,
                c.icon as cat_icon,
                COALESCE(SUM(al.duration), 0) as total
             FROM activity_logs al
             LEFT JOIN categories c ON al.category_id = c.id
             WHERE al.start_time >= ?1 AND al.is_idle = 0
             GROUP BY cat_id
             ORDER BY total DESC",
        )?;

        let rows = stmt.query_map(params![today_start], |row| {
            let total_seconds: i64 = row.get(4)?;
            let percentage = if total_active > 0 {
                (total_seconds as f64 / total_active as f64) * 100.0
            } else {
                0.0
            };
            Ok(CategoryStat {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                category_color: row.get(2)?,
                category_icon: row.get(3)?,
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
}
