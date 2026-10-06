use rusqlite::{params, Connection};
use anyhow::Result;

/// 当前数据库版本
pub const CURRENT_VERSION: i32 = 5;

/// 所有迁移脚本（按版本号顺序排列）
/// 索引 0 对应 v1（初始建表）
const MIGRATIONS: &[fn(&Connection) -> Result<()>] = &[
    migration_v1,
    migration_v2,
    migration_v3,
    migration_v4,
    migration_v5,
];

/// 执行数据库迁移
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // 确保版本表存在
    conn.execute(
        "CREATE TABLE IF NOT EXISTS db_version (
            version INTEGER PRIMARY KEY
        )",
        [],
    )?;

    // 读取当前版本（如果没有记录则为 0）
    let current_version: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM db_version",
        [],
        |row| row.get(0),
    )?;

    tracing::info!("Database version: {}, target: {}", current_version, CURRENT_VERSION);

    // 按顺序执行未执行的迁移
    for (idx, migration) in MIGRATIONS.iter().enumerate() {
        let version = (idx + 1) as i32;
        if version > current_version {
            tracing::info!("Running migration v{}", version);
            migration(conn)?;
            // 记录版本
            conn.execute(
                "INSERT INTO db_version (version) VALUES (?1)",
                rusqlite::params![version],
            )?;
            tracing::info!("Migration v{} completed", version);
        }
    }

    Ok(())
}

/// v3: 分类图标从 emoji 迁移为 Lucide 图标名
fn migration_v3(conn: &Connection) -> Result<()> {
    let icon_mapping = [
        ("工作", "Briefcase"),
        ("学习", "BookOpen"),
        ("娱乐", "Gamepad2"),
        ("社交", "MessageCircle"),
        ("其他", "MoreHorizontal"),
    ];

    for (name, icon) in icon_mapping {
        conn.execute(
            "UPDATE categories SET icon = ?1 WHERE name = ?2",
            params![icon, name],
        )?;
    }

    Ok(())
}

/// v4: 精简预置规则（25条精选版）
fn migration_v4(conn: &Connection) -> Result<()> {
    // 先清空所有预置规则（保留用户自建的 — 这里简单处理，全部清掉重新插）
    conn.execute("DELETE FROM app_rules", [])?;

    let new_rules = vec![
        // 工作类
        ("工作", "process", "Code.exe", "contains"),
        ("工作", "process", "idea64.exe", "contains"),
        ("工作", "process", "EXCEL.EXE", "contains"),
        ("工作", "process", "ChatGPT.exe", "contains"),
        ("工作", "process", "pycharm64.exe", "contains"),
        // 学习类
        ("学习", "title", "B站", "contains"),
        ("学习", "title", "bilibili", "contains"),
        ("学习", "title", "知乎", "contains"),
        ("学习", "process", "Typora.exe", "contains"),
        ("学习", "process", "obsidian.exe", "contains"),
        ("学习", "process", "marktext.exe", "contains"),
        // 娱乐类
        ("娱乐", "title", "抖音", "contains"),
        ("娱乐", "title", "快手", "contains"),
        ("娱乐", "process", "steam.exe", "contains"),
        ("娱乐", "process", "QQMusic.exe", "contains"),
        ("娱乐", "process", "uu_launcher.exe", "contains"),
        // 社交类
        ("社交", "process", "WeChat.exe", "contains"),
        ("社交", "process", "QQ.exe", "contains"),
        ("社交", "process", "DingtalkLauncher.exe", "contains"),
        ("社交", "process", "Feishu.exe", "contains"),
        // 其他类
        ("其他", "process", "explorer.exe", "contains"),
        ("其他", "process", "notepad.exe", "contains"),
        ("其他", "process", "PixPin.exe", "contains"),
    ];

    for (cat_name, match_type, match_value, match_mode) in new_rules {
        let cat_id: Option<i64> = conn.query_row(
            "SELECT id FROM categories WHERE name = ?1",
            params![cat_name],
            |row| row.get(0),
        ).ok();

        if let Some(cid) = cat_id {
            conn.execute(
                "INSERT INTO app_rules (category_id, match_type, match_value, match_mode, sort_order, enabled) 
                 VALUES (?1, ?2, ?3, ?4, 0, 1)",
                params![cid, match_type, match_value, match_mode],
            )?;
        }
    }

    Ok(())
}

/// v5: 设置表（番茄钟等设置持久化）
fn migration_v5(conn: &Connection) -> Result<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            key       TEXT PRIMARY KEY,
            value     TEXT NOT NULL,
            updated_at INTEGER DEFAULT (strftime('%s','now'))
        )
        "#,
        [],
    )?;
    Ok(())
}

/// v2: 分类、规则、番茄钟
fn migration_v2(conn: &Connection) -> Result<()> {
    // 分类表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS categories (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            color       TEXT    NOT NULL,
            icon        TEXT,
            sort_order  INTEGER DEFAULT 0,
            is_default  INTEGER DEFAULT 0,
            created_at  INTEGER DEFAULT (strftime('%s','now'))
        )
        "#,
        [],
    )?;

    // 规则表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS app_rules (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            category_id INTEGER NOT NULL,
            match_type  TEXT    NOT NULL, -- process / title
            match_value TEXT    NOT NULL,
            match_mode  TEXT    NOT NULL DEFAULT 'contains', -- exact / contains / regex
            sort_order  INTEGER DEFAULT 0,
            enabled     INTEGER DEFAULT 1,
            created_at  INTEGER DEFAULT (strftime('%s','now')),
            FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
        )
        "#,
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_app_rules_category 
         ON app_rules(category_id)",
        [],
    )?;

    // 番茄钟记录表
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS pomodoro_sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            session_type TEXT   NOT NULL, -- focus / short_break / long_break
            start_time  INTEGER NOT NULL,
            end_time    INTEGER NOT NULL DEFAULT 0,
            duration    INTEGER NOT NULL DEFAULT 0,
            completed   INTEGER DEFAULT 0,
            interrupted INTEGER DEFAULT 0,
            created_at  INTEGER DEFAULT (strftime('%s','now'))
        )
        "#,
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_pomodoro_start_time 
         ON pomodoro_sessions(start_time)",
        [],
    )?;

    // 活动记录表加分类字段
    conn.execute(
        "ALTER TABLE activity_logs ADD COLUMN category_id INTEGER DEFAULT NULL 
         REFERENCES categories(id) ON DELETE SET NULL",
        [],
    ).ok(); // 忽略已存在的错误

    // 插入预置分类
    let preset_categories = vec![
        ("工作", "#3B82F6", "Briefcase", 1, 1),
        ("学习", "#10B981", "BookOpen", 2, 1),
        ("娱乐", "#EF4444", "Gamepad2", 3, 1),
        ("社交", "#8B5CF6", "MessageCircle", 4, 1),
        ("其他", "#6B7280", "MoreHorizontal", 5, 1),
    ];

    for (name, color, icon, sort_order, is_default) in preset_categories {
        conn.execute(
            "INSERT OR IGNORE INTO categories (name, color, icon, sort_order, is_default) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, color, icon, sort_order, is_default],
        )?;
    }

    // 插入预置规则
    let preset_rules = vec![
        // 工作类
        ("工作", "process", "Code.exe", "contains"),
        ("工作", "process", "idea64.exe", "contains"),
        ("工作", "process", "EXCEL.EXE", "contains"),
        ("工作", "process", "ChatGPT.exe", "contains"),
        ("工作", "process", "pycharm64.exe", "contains"),
        // 学习类
        ("学习", "title", "B站", "contains"),
        ("学习", "title", "bilibili", "contains"),
        ("学习", "title", "知乎", "contains"),
        ("学习", "process", "Typora.exe", "contains"),
        ("学习", "process", "obsidian.exe", "contains"),
        ("学习", "process", "marktext.exe", "contains"),
        // 娱乐类
        ("娱乐", "title", "抖音", "contains"),
        ("娱乐", "title", "快手", "contains"),
        ("娱乐", "process", "steam.exe", "contains"),
        ("娱乐", "process", "QQMusic.exe", "contains"),
        ("娱乐", "process", "uu_launcher.exe", "contains"),
        // 社交类
        ("社交", "process", "WeChat.exe", "contains"),
        ("社交", "process", "QQ.exe", "contains"),
        ("社交", "process", "DingtalkLauncher.exe", "contains"),
        ("社交", "process", "Feishu.exe", "contains"),
        // 其他类
        ("其他", "process", "explorer.exe", "contains"),
        ("其他", "process", "notepad.exe", "contains"),
        ("其他", "process", "PixPin.exe", "contains"),
    ];

    for (cat_name, match_type, match_value, match_mode) in preset_rules {
        // 先查分类 id
        let cat_id: Option<i64> = conn.query_row(
            "SELECT id FROM categories WHERE name = ?1",
            params![cat_name],
            |row| row.get(0),
        ).ok();

        if let Some(cid) = cat_id {
            conn.execute(
                "INSERT OR IGNORE INTO app_rules (category_id, match_type, match_value, match_mode, sort_order) 
                 VALUES (?1, ?2, ?3, ?4, 0)",
                params![cid, match_type, match_value, match_mode],
            )?;
        }
    }

    Ok(())
}

/// v1: 初始建表 — activity_logs
fn migration_v1(conn: &Connection) -> Result<()> {
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
