use std::sync::Arc;
use chrono::Utc;

use crate::db::{ActivityDao, ActivityLog, Database};
use crate::monitor::WindowInfo;

/// 当前活动状态
#[derive(Debug, Clone)]
struct CurrentState {
    process_name: String,
    window_title: String,
    start_time: i64,
    is_idle: bool,
}

/// 活动状态机 — 合并窗口变化和空闲检测，负责写入数据库
pub struct ActivityStateMachine {
    db: Arc<Database>,
    current: Option<CurrentState>,
    /// 短暂切换记录（用于防抖）：(目标进程, 目标标题, 开始时间, is_idle)
    pending_switch: Option<(String, String, i64, bool)>,
}

impl ActivityStateMachine {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            current: None,
            pending_switch: None,
        }
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
    }

    /// 切换到新状态，并将旧状态写入数据库
    fn switch_state(&mut self, process: String, title: String, now: i64, is_idle: bool) {
        // 如果有旧状态，写入数据库
        if let Some(old) = &self.current {
            let duration = now - old.start_time;
            if duration > 0 {
                let log = ActivityLog {
                    id: None,
                    process_name: old.process_name.clone(),
                    window_title: if old.window_title.is_empty() { None } else { Some(old.window_title.clone()) },
                    start_time: old.start_time,
                    end_time: now,
                    duration,
                    is_idle: old.is_idle,
                };

                let conn = self.db.conn().lock();
                if let Err(e) = ActivityDao::insert(&conn, &log) {
                    tracing::error!("Failed to insert activity log: {}", e);
                }
            }
        }

        // 更新当前状态
        self.current = Some(CurrentState {
            process_name: process,
            window_title: title,
            start_time: now,
            is_idle,
        });
    }

    /// 获取当前正在进行的活动
    pub fn get_current(&self) -> Option<(String, String, i64, bool)> {
        let now = Utc::now().timestamp();
        self.current.as_ref().map(|cur| {
            (
                cur.process_name.clone(),
                cur.window_title.clone(),
                cur.start_time,
                cur.is_idle,
            )
        })
    }

    pub fn get_current_duration(&self) -> i64 {
        match &self.current {
            Some(cur) => Utc::now().timestamp() - cur.start_time,
            None => 0,
        }
    }
}

// 程序退出时，把当前状态写入数据库（这里暂不实现 Drop，由 Tauri 退出时处理）
