use rusqlite::Connection;
use anyhow::Result;

/// 当前数据库版本
pub const CURRENT_VERSION: i32 = 1;

/// 所有迁移脚本（按版本号顺序排列）
/// 索引 0 对应 v1（初始建表）
const MIGRATIONS: &[fn(&Connection) -> Result<()>] = &[
    migration_v1,
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
