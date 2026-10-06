use std::sync::Arc;
use chrono::Utc;

use crate::db::{ActivityDao, ActivityLog, Database, RuleDao, RuleMatcher, AppRule};
use crate::monitor::WindowInfo;

/// 当前活动状态
#[derive(Debug, Clone)]
struct CurrentState {
    process_name: String,
    window_title: String,
    start_time: i64,
    is_idle: bool,
    category_id: Option<i64>,
    /// 当前活动在数据库中的记录 ID（活动开始时立即插入）
    log_id: Option<i64>,
}

/// 活动状态机 — 合并窗口变化和空闲检测，负责写入数据库
/// 
/// 写入策略：
/// - 活动开始时立即插入一条记录（end_time = start_time, duration = 0），保存 log_id
/// - 每次 tick 时更新当前记录的 end_time 和 duration（实际每几个 tick 更新一次以降低 IO）
/// - 活动结束时（切换状态）更新最终的 end_time 和 duration
/// - 程序启动时检查并修复未闭合的记录
pub struct ActivityStateMachine {
    db: Arc<Database>,
    current: Option<CurrentState>,
    /// 短暂切换记录（用于防抖）：(目标进程, 目标标题, 开始时间, is_idle)
    pending_switch: Option<(String, String, i64, bool)>,
    /// 距离上次刷新数据库的 tick 计数
    flush_tick_count: u32,
    /// 缓存的匹配规则
    rules: Vec<AppRule>,
}

/// 每 N 次 tick 更新一次当前活动的 end_time（降低数据库写入频率）
const FLUSH_INTERVAL_TICKS: u32 = 6; // 6 * 5s = 30 秒

impl ActivityStateMachine {
    pub fn new(db: Arc<Database>) -> Self {
        // 启动时修复未闭合的记录
        {
            let conn = db.conn().lock();
            match ActivityDao::fix_open_records(&conn) {
                Ok(count) if count > 0 => {
                    tracing::info!("Recovered from unclean shutdown: fixed {} records", count);
                }
                Err(e) => {
                    tracing::error!("Failed to fix open records: {}", e);
                }
                _ => {}
            }
        }

        // 加载规则
        let rules = {
            let conn = db.conn().lock();
            RuleDao::list_enabled(&conn).unwrap_or_default()
        };
        tracing::info!("Loaded {} classification rules", rules.len());

        Self {
            db,
            current: None,
            pending_switch: None,
            flush_tick_count: 0,
            rules,
        }
    }

    /// 刷新规则缓存（修改规则后调用）
    pub fn refresh_rules(&mut self) {
        let rules = {
            let conn = self.db.conn().lock();
            RuleDao::list_enabled(&conn).unwrap_or_default()
        };
        tracing::info!("Refreshed classification rules: {} active", rules.len());
        self.rules = rules;
    }

    /// 每次轮询调用，传入当前窗口信息和是否空闲
    pub fn tick(&mut self, window: &WindowInfo, is_idle: bool, debounce_secs: i64) {
        let now = Utc::now().timestamp();
        let target_process = window.process_name.clone();
        let target_title = window.window_title.clone();

        // 计算当前状态的哈希（进程+标题+是否空闲）
        let same_as_current = match &self.current {
            Some(cur) => {
                cur.process_name == target_process
                    && cur.window_title == target_title
                    && cur.is_idle == is_idle
            }
            None => false,
        };

        if same_as_current {
            // 状态没变，清除待切换
            self.pending_switch = None;
            // 定期刷新当前活动的 end_time
            self.flush_tick_count += 1;
            if self.flush_tick_count >= FLUSH_INTERVAL_TICKS {
                self.flush_current(now);
                self.flush_tick_count = 0;
            }
            return;
        }

        // 状态变化了，检查防抖
        let should_switch = {
            match &self.pending_switch {
                Some((proc, title, start, idle)) => {
                    if proc == &target_process && title == &target_title && *idle == is_idle {
                        // 同一个待切换状态，检查持续时间
                        (now - start) >= debounce_secs
                    } else {
                        // 目标变了，重置待切换
                        false
                    }
                }
                None => {
                    // 第一次遇到新状态，记录待切换
                    false
                }
            }
        };

        if !should_switch {
            if self.pending_switch.as_ref().map(|(p, t, _, i)| {
                p != &target_process || t != &target_title || *i != is_idle
            }).unwrap_or(true) {
                // 新的待切换
                self.pending_switch = Some((target_process, target_title, now, is_idle));
            }
            return;
        }

        // 防抖通过，正式切换状态
        self.switch_state(target_process, target_title, now, is_idle);
        self.pending_switch = None;
        self.flush_tick_count = 0;
    }

    /// 匹配分类
    fn match_category(&self, process: &str, title: &str, is_idle: bool) -> Option<i64> {
        if is_idle {
            return None;
        }
        RuleMatcher::match_category(&self.rules, process, Some(title))
    }

    /// 切换到新状态：关闭旧状态（更新数据库），开启新状态（立即插入数据库）
    fn switch_state(&mut self, process: String, title: String, now: i64, is_idle: bool) {
        // 如果有旧状态，更新其 end_time
        if let Some(old) = &self.current {
            let duration = now - old.start_time;
            if duration > 0 {
                if let Some(log_id) = old.log_id {
                    let conn = self.db.conn().lock();
                    if let Err(e) = ActivityDao::update_end_time(&conn, log_id, now, duration, old.category_id) {
                        tracing::error!("Failed to update activity end time: {}", e);
                    }
                }
            } else if let Some(log_id) = old.log_id {
                // duration 为 0，直接删掉这条空记录
                let conn = self.db.conn().lock();
                let _ = conn.execute("DELETE FROM activity_logs WHERE id = ?1", rusqlite::params![log_id]);
            }
        }

        // 匹配分类
        let category_id = self.match_category(&process, &title, is_idle);

        // 插入新活动记录（开始时即写入，duration=0 表示进行中）
        let log = ActivityLog {
            id: None,
            process_name: process.clone(),
            window_title: if title.is_empty() { None } else { Some(title.clone()) },
            start_time: now,
            end_time: now,
            duration: 0,
            is_idle,
            category_id,
        };

        let log_id = {
            let conn = self.db.conn().lock();
            match ActivityDao::insert(&conn, &log) {
                Ok(id) => Some(id),
                Err(e) => {
                    tracing::error!("Failed to insert activity log: {}", e);
                    None
                }
            }
        };

        // 更新当前状态
        self.current = Some(CurrentState {
            process_name: process,
            window_title: title,
            start_time: now,
            is_idle,
            category_id,
            log_id,
        });
    }

    /// 刷新当前活动的 end_time（定期调用，防止崩溃丢失太多数据）
    fn flush_current(&mut self, now: i64) {
        if let Some(cur) = &self.current {
            if let Some(log_id) = cur.log_id {
                let duration = now - cur.start_time;
                if duration > 0 {
                    let conn = self.db.conn().lock();
                    if let Err(e) = ActivityDao::update_end_time(&conn, log_id, now, duration, cur.category_id) {
                        tracing::error!("Failed to flush current activity: {}", e);
                    }
                }
            }
        }
    }

    /// 手动 flush（程序退出时调用）
    pub fn flush(&mut self) {
        let now = Utc::now().timestamp();
        self.flush_current(now);
    }

    /// 获取当前正在进行的活动
    pub fn get_current(&self) -> Option<(String, String, i64, bool, Option<i64>)> {
        self.current.as_ref().map(|cur| {
            (
                cur.process_name.clone(),
                cur.window_title.clone(),
                cur.start_time,
                cur.is_idle,
                cur.category_id,
            )
        })
    }

    pub fn get_current_duration(&self) -> i64 {
        match &self.current {
            Some(cur) => Utc::now().timestamp() - cur.start_time,
            None => 0,
        }
    }

    /// 获取当前活动的分类 ID
    pub fn get_current_category_id(&self) -> Option<i64> {
        self.current.as_ref().and_then(|cur| cur.category_id)
    }
}
