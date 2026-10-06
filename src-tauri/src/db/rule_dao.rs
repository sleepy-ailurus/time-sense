use anyhow::Result;
use rusqlite::{params, Connection};

use super::models::{AppRule, NewAppRule};

pub struct RuleDao;

impl RuleDao {
    /// 获取所有规则（带分类名称，按排序号）
    pub fn list_all(conn: &Connection) -> Result<Vec<AppRule>> {
        let mut stmt = conn.prepare(
            "SELECT r.id, r.category_id, c.name, r.match_type, r.match_value, 
                    r.match_mode, r.sort_order, r.enabled
             FROM app_rules r
             LEFT JOIN categories c ON r.category_id = c.id
             ORDER BY r.sort_order ASC, r.id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let enabled_int: i32 = row.get(7)?;
            Ok(AppRule {
                id: row.get(0)?,
                category_id: row.get(1)?,
                category_name: row.get(2)?,
                match_type: row.get(3)?,
                match_value: row.get(4)?,
                match_mode: row.get(5)?,
                sort_order: row.get(6)?,
                enabled: enabled_int == 1,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 获取所有启用的规则（用于匹配）
    pub fn list_enabled(conn: &Connection) -> Result<Vec<AppRule>> {
        let mut stmt = conn.prepare(
            "SELECT r.id, r.category_id, c.name, r.match_type, r.match_value, 
                    r.match_mode, r.sort_order, r.enabled
             FROM app_rules r
             LEFT JOIN categories c ON r.category_id = c.id
             WHERE r.enabled = 1
             ORDER BY r.sort_order ASC, r.id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(AppRule {
                id: row.get(0)?,
                category_id: row.get(1)?,
                category_name: row.get(2)?,
                match_type: row.get(3)?,
                match_value: row.get(4)?,
                match_mode: row.get(5)?,
                sort_order: row.get(6)?,
                enabled: true,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// 新增规则
    pub fn create(conn: &Connection, rule: &NewAppRule) -> Result<i64> {
        conn.execute(
            "INSERT INTO app_rules (category_id, match_type, match_value, match_mode) 
             VALUES (?1, ?2, ?3, ?4)",
            params![
                rule.category_id,
                rule.match_type,
                rule.match_value,
                rule.match_mode,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 更新规则
    pub fn update(conn: &Connection, id: i64, rule: &NewAppRule) -> Result<()> {
        conn.execute(
            "UPDATE app_rules SET category_id = ?1, match_type = ?2, 
             match_value = ?3, match_mode = ?4 WHERE id = ?5",
            params![
                rule.category_id,
                rule.match_type,
                rule.match_value,
                rule.match_mode,
                id,
            ],
        )?;
        Ok(())
    }

    /// 删除规则
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        conn.execute("DELETE FROM app_rules WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 切换规则启用状态
    pub fn toggle_enabled(conn: &Connection, id: i64) -> Result<bool> {
        let current: i32 = conn.query_row(
            "SELECT enabled FROM app_rules WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        let new_val = if current == 1 { 0 } else { 1 };
        conn.execute(
            "UPDATE app_rules SET enabled = ?1 WHERE id = ?2",
            params![new_val, id],
        )?;
        Ok(new_val == 1)
    }
}

/// 规则匹配引擎
pub struct RuleMatcher;

impl RuleMatcher {
    /// 根据进程名和窗口标题匹配分类 ID
    /// 优先级：title 规则 > process 规则；先匹配到的优先
    pub fn match_category(
        rules: &[AppRule],
        process_name: &str,
        window_title: Option<&str>,
    ) -> Option<i64> {
        // 第一遍：匹配窗口标题（优先级更高）
        if let Some(title) = window_title {
            for rule in rules.iter().filter(|r| r.match_type == "title") {
                if Self::does_match(&rule.match_value, &rule.match_mode, title) {
                    return Some(rule.category_id);
                }
            }
        }

        // 第二遍：匹配进程名
        for rule in rules.iter().filter(|r| r.match_type == "process") {
            if Self::does_match(&rule.match_value, &rule.match_mode, process_name) {
                return Some(rule.category_id);
            }
        }

        None
    }

    fn does_match(pattern: &str, mode: &str, text: &str) -> bool {
        match mode {
            "exact" => pattern.eq_ignore_ascii_case(text),
            "contains" => text.to_lowercase().contains(&pattern.to_lowercase()),
            "regex" => {
                // 简单正则匹配，失败就当不匹配
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(text))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
}
