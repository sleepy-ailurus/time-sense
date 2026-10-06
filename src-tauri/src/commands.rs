use tauri::{State, Window};

use crate::db::{ActivityDao, ActivityLog, AppStat, CurrentActivity, TodayTotal};
use crate::AppState;

/// 获取今日各应用耗时统计
#[tauri::command]
pub fn get_today_stats(state: State<AppState>) -> Result<Vec<AppStat>, String> {
    let conn = state.db.conn().lock();
    ActivityDao::get_today_stats(&conn).map_err(|e| e.to_string())
}

/// 获取今日总时长
#[tauri::command]
pub fn get_today_total(state: State<AppState>) -> Result<TodayTotal, String> {
    let conn = state.db.conn().lock();
    ActivityDao::get_today_total(&conn).map_err(|e| e.to_string())
}

/// 获取当前正在进行的活动
#[tauri::command]
pub fn get_current_activity(state: State<AppState>) -> Result<Option<CurrentActivity>, String> {
    let engine = state.activity_engine.lock();
    let current = engine.get_current();

    match current {
        Some((process_name, window_title, start_time, is_idle)) => {
            let duration = engine.get_current_duration();
            Ok(Some(CurrentActivity {
                process_name,
                window_title,
                start_time,
                duration,
                is_idle,
            }))
        }
        None => Ok(None),
    }
}

/// 当前是否在记录
#[tauri::command]
pub fn is_recording(state: State<AppState>) -> bool {
    *state.is_recording.lock()
}

/// 切换记录状态
#[tauri::command]
pub fn toggle_recording(state: State<AppState>) -> bool {
    let mut recording = state.is_recording.lock();
    *recording = !*recording;
    *recording
}

/// 隐藏主窗口（收起面板）
#[tauri::command]
pub fn hide_main_window(window: Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

/// 获取指定日期的所有活动记录
#[tauri::command]
pub fn get_activity_by_date(state: State<AppState>, date: String) -> Result<Vec<ActivityLog>, String> {
    let conn = state.db.conn().lock();
    ActivityDao::get_activity_by_date(&conn, &date).map_err(|e| e.to_string())
}
