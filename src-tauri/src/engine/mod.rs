pub mod activity_state;
pub mod pomodoro;

use std::sync::Arc;
use std::time::Duration;
use parking_lot::{Mutex, RwLock};
use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::monitor;
use crate::tray;
use crate::db::{CategoryDao, Database, GeneralSettings, GoalDao};
use activity_state::ActivityStateMachine;
use pomodoro::PomodoroEngine;

/// 监控循环间隔（秒）
const MONITOR_INTERVAL_SECS: u64 = 2;

/// 番茄钟心跳间隔（毫秒）——到点/阶段切换要精确落在目标时刻，
/// 所以用独立的高频心跳，而不是等 2 秒的监控循环
const POMODORO_TICK_MS: u64 = 250;

/// 防抖阈值（秒）— 小于此时间的窗口切换忽略
const DEBOUNCE_SECS: i64 = 3;

/// 托盘统计刷新间隔（tick 次数）— 每 15 次 tick = 30 秒刷新一次
const TRAY_REFRESH_TICKS: u32 = 15;

/// 判断分类名称是否是专注类（工作/学习）
pub fn is_focus_category(name: &str) -> bool {
    matches!(name, "工作" | "学习")
}

/// 检查当前活动是否属于专注分类（工作/学习）
pub fn is_current_focus_activity(
    engine: &crate::engine::activity_state::ActivityStateMachine,
    db: &crate::db::Database,
) -> bool {
    match engine.get_current_category_id() {
        Some(cid) => {
            let conn = db.conn().lock();
            match crate::db::CategoryDao::get_by_id(&conn, cid) {
                Ok(Some(cat)) => is_focus_category(&cat.name),
                _ => false,
            }
        }
        None => false,
    }
}

/// 启动主监控循环
pub async fn start_monitor_loop(
    app: AppHandle,
    engine: Arc<Mutex<ActivityStateMachine>>,
    pomodoro: Arc<PomodoroEngine>,
    is_recording: Arc<Mutex<bool>>,
    db: Arc<Database>,
    general_settings: Arc<RwLock<GeneralSettings>>,
) {
    let monitor = monitor::create_monitor();
    let mut interval = tokio::time::interval(Duration::from_secs(MONITOR_INTERVAL_SECS));
    let mut tick_count: u32 = 0;
    // 已知的超标分类 ID 集合，避免重复通知
    let mut already_exceeded: std::collections::HashSet<i64> = std::collections::HashSet::new();

    tracing::info!("Monitor loop started (interval: {}s)", MONITOR_INTERVAL_SECS);

    // 启动时先刷新一次托盘统计
    let _ = tray::refresh_tray_stats(&app);

    // 番茄钟独立心跳：保证倒计时归零后立刻切换阶段并发通知
    {
        let pomodoro = pomodoro.clone();
        let is_recording = is_recording.clone();
        let app = app.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(POMODORO_TICK_MS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if !*is_recording.lock() {
                    continue;
                }
                // 只有发生阶段切换时才会返回状态
                if let Some(status) = pomodoro.tick() {
                    let _ = tray::on_pomodoro_phase_change(&app, &status);
                }
            }
        });
    }

    loop {
        interval.tick().await;
        tick_count = tick_count.wrapping_add(1);

        // 检查是否在记录
        if !*is_recording.lock() {
            continue;
        }

        // 获取当前窗口
        let window_info = match monitor.get_active_window() {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!("Failed to get active window: {}", e);
                continue;
            }
        };

        // 获取空闲时间
        let idle_time = match monitor.get_idle_time() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to get idle time: {}", e);
                Duration::from_secs(0)
            }
        };

        // 从设置读取闲置阈值（分钟转秒）
        let idle_threshold_secs = {
            let gs = general_settings.read();
            (gs.idle_threshold_minutes as u64).saturating_mul(60)
        };
        let is_idle = idle_time.as_secs() >= idle_threshold_secs.max(60); // 最少 1 分钟
        // 最后一次用户输入的时间：空闲段以它作为起点（而不是判定为空闲的那一刻）
        let idle_since = if is_idle {
            Some(Utc::now().timestamp() - idle_time.as_secs() as i64)
        } else {
            None
        };

        // 更新活动状态机
        let mut eng = engine.lock();
        let switched = eng.tick(&window_info, is_idle, DEBOUNCE_SECS, idle_since);

        // 当前活动切换（含进入/离开空闲）时立刻通知前端刷新，避免等 10 秒轮询
        if switched {
            let _ = app.emit("activity://changed", ());
        }

        // 自动番茄钟模式：根据当前活动分类自动控制
        let auto_mode_changed = {
            let settings = pomodoro.get_settings();
            if settings.auto_mode && settings.enabled {
                let category_id = eng.get_current_category_id();
                let is_focus = match category_id {
                    Some(cid) => {
                        let conn = db.conn().lock();
                        match CategoryDao::get_by_id(&conn, cid) {
                            Ok(Some(cat)) => {
                                let focus = is_focus_category(&cat.name);
                                tracing::info!("Auto mode: category='{}' (id={}), is_focus={}", cat.name, cid, focus);
                                focus
                            }
                            _ => {
                                tracing::warn!("Auto mode: category id={} not found", cid);
                                false
                            }
                        }
                    }
                    None => {
                        tracing::info!("Auto mode: no category matched for current activity");
                        false
                    }
                };
                pomodoro.auto_mode_update(is_focus, is_idle)
            } else {
                None
            }
        };

        drop(eng);

        // 自动模式触发了状态变化 → 通知前端 + 刷新托盘
        if let Some(status) = auto_mode_changed {
            let _ = tray::on_pomodoro_phase_change(&app, &status);
        }

        // 定期刷新托盘统计
        if tick_count % TRAY_REFRESH_TICKS == 0 {
            let _ = tray::refresh_tray_stats(&app);

            // 检查目标预算超标
            {
                let conn = db.conn().lock();
                if let Ok(newly_exceeded) = GoalDao::check_newly_exceeded(&conn, &already_exceeded) {
                    drop(conn);
                    for cat_name in &newly_exceeded {
                        tracing::warn!("Goal exceeded: category '{}'", cat_name);
                        // 发送系统通知（复用 tauri-plugin-notification）
                        let notification_enabled = general_settings.read().notification_enabled;
                        if notification_enabled {
                            use tauri_plugin_notification::NotificationExt;
                            let _ = app.notification()
                                .builder()
                                .title(format!("「{}」已超出今日预算", cat_name))
                                .body("你已超过该分类的每日时间限额")
                                .show();
                        }
                    }
                    // 把新超标的加入已知集合
                    if let Ok(statuses) = {
                        let conn = db.conn().lock();
                        GoalDao::get_active_goals_status(&conn)
                    } {
                        for s in &statuses {
                            if s.exceeded {
                                already_exceeded.insert(s.goal.category_id);
                            }
                        }
                    }
                }
            }
        }
    }
}
