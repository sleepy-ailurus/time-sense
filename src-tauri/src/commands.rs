use tauri::{State, Window, Emitter, AppHandle};

use crate::db::{
    ActivityDao, ActivityLog, AppStat, Category, CategoryDao, CategoryStat,
    CurrentActivity, NewAppRule, PomodoroSettings, PomodoroStatus, RuleDao, TodayTotal,
};
use crate::AppState;
use crate::tray;

// ==================== 活动相关 ====================

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
        Some((process_name, window_title, start_time, is_idle, category_id)) => {
            let duration = engine.get_current_duration();
            // 查分类名称
            let category_name = if let Some(cid) = category_id {
                let conn = state.db.conn().lock();
                CategoryDao::get_by_id(&conn, cid)
                    .ok()
                    .flatten()
                    .map(|c| c.name)
            } else {
                None
            };
            Ok(Some(CurrentActivity {
                process_name,
                window_title,
                start_time,
                duration,
                is_idle,
                category_id,
                category_name,
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

// ==================== 分类相关 ====================

/// 获取所有分类
#[tauri::command]
pub fn get_categories(state: State<AppState>) -> Result<Vec<Category>, String> {
    let conn = state.db.conn().lock();
    CategoryDao::list_all(&conn).map_err(|e| e.to_string())
}

/// 获取今日分类统计
#[tauri::command]
pub fn get_today_category_stats(state: State<AppState>) -> Result<Vec<CategoryStat>, String> {
    let conn = state.db.conn().lock();
    CategoryDao::get_today_category_stats(&conn).map_err(|e| e.to_string())
}

/// 新增分类
#[tauri::command]
pub fn create_category(state: State<AppState>, name: String, color: String, icon: Option<String>) -> Result<i64, String> {
    let conn = state.db.conn().lock();
    CategoryDao::create(&conn, &name, &color, icon.as_deref()).map_err(|e| e.to_string())
}

/// 更新分类
#[tauri::command]
pub fn update_category(state: State<AppState>, id: i64, name: String, color: String, icon: Option<String>) -> Result<(), String> {
    let conn = state.db.conn().lock();
    CategoryDao::update(&conn, id, &name, &color, icon.as_deref()).map_err(|e| e.to_string())
}

/// 删除分类
#[tauri::command]
pub fn delete_category(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.conn().lock();
    CategoryDao::delete(&conn, id).map_err(|e| e.to_string())
}

// ==================== 规则相关 ====================

/// 获取所有规则
#[tauri::command]
pub fn get_rules(state: State<AppState>) -> Result<Vec<crate::db::AppRule>, String> {
    let conn = state.db.conn().lock();
    RuleDao::list_all(&conn).map_err(|e| e.to_string())
}

/// 新增规则
#[tauri::command]
pub fn create_rule(state: State<AppState>, rule: NewAppRule) -> Result<i64, String> {
    let id = {
        let conn = state.db.conn().lock();
        RuleDao::create(&conn, &rule).map_err(|e| e.to_string())?
    };
    // 刷新规则缓存（注意：必须先释放 DB 锁，否则会死锁）
    let mut engine = state.activity_engine.lock();
    engine.refresh_rules();
    Ok(id)
}

/// 更新规则
#[tauri::command]
pub fn update_rule(state: State<AppState>, id: i64, rule: NewAppRule) -> Result<(), String> {
    {
        let conn = state.db.conn().lock();
        RuleDao::update(&conn, id, &rule).map_err(|e| e.to_string())?;
    }
    // 刷新规则缓存
    let mut engine = state.activity_engine.lock();
    engine.refresh_rules();
    Ok(())
}

/// 删除规则
#[tauri::command]
pub fn delete_rule(state: State<AppState>, id: i64) -> Result<(), String> {
    {
        let conn = state.db.conn().lock();
        RuleDao::delete(&conn, id).map_err(|e| e.to_string())?;
    }
    // 刷新规则缓存
    let mut engine = state.activity_engine.lock();
    engine.refresh_rules();
    Ok(())
}

/// 切换规则启用状态
#[tauri::command]
pub fn toggle_rule(state: State<AppState>, id: i64) -> Result<bool, String> {
    let enabled = {
        let conn = state.db.conn().lock();
        RuleDao::toggle_enabled(&conn, id).map_err(|e| e.to_string())?
    };
    // 刷新规则缓存
    let mut engine = state.activity_engine.lock();
    engine.refresh_rules();
    Ok(enabled)
}

/// 手动刷新规则缓存
#[tauri::command]
pub fn refresh_rules(state: State<AppState>) -> Result<(), String> {
    let mut engine = state.activity_engine.lock();
    engine.refresh_rules();
    Ok(())
}

// ==================== 番茄钟相关 ====================

/// 获取番茄钟状态
#[tauri::command]
pub fn get_pomodoro_status(state: State<AppState>) -> PomodoroStatus {
    state.pomodoro.get_status()
}

/// 获取番茄钟设置
#[tauri::command]
pub fn get_pomodoro_settings(state: State<AppState>) -> PomodoroSettings {
    state.pomodoro.get_settings()
}

/// 更新番茄钟设置
#[tauri::command]
pub fn update_pomodoro_settings(state: State<AppState>, settings: PomodoroSettings) -> Result<(), String> {
    state.pomodoro.update_settings(settings);
    Ok(())
}

/// 开始专注
#[tauri::command]
pub fn start_pomodoro_focus(state: State<AppState>, app: AppHandle) -> PomodoroStatus {
    let status = state.pomodoro.start_focus();
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}

/// 开始休息（短休息）
#[tauri::command]
pub fn start_pomodoro_break(state: State<AppState>, app: AppHandle, long: Option<bool>) -> PomodoroStatus {
    let status = if long.unwrap_or(false) {
        state.pomodoro.start_long_break()
    } else {
        state.pomodoro.start_short_break()
    };
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}

/// 停止番茄钟
#[tauri::command]
pub fn stop_pomodoro(state: State<AppState>, app: AppHandle) -> PomodoroStatus {
    let status = state.pomodoro.stop();
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}

/// 暂停番茄钟
#[tauri::command]
pub fn pause_pomodoro(state: State<AppState>, app: AppHandle) -> PomodoroStatus {
    let status = state.pomodoro.pause();
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}

/// 继续/恢复番茄钟
#[tauri::command]
pub fn resume_pomodoro(state: State<AppState>, app: AppHandle) -> PomodoroStatus {
    // 判断当前是否在工作/学习区：在工作区手动继续 → 保持自动；不在 → 升级为手动
    let in_focus_zone = {
        let engine = state.activity_engine.lock();
        crate::engine::is_current_focus_activity(&engine, &state.db)
    };
    let mark_as_manual = !in_focus_zone;
    let status = state.pomodoro.resume(mark_as_manual);
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}

/// 跳过当前阶段
#[tauri::command]
pub fn skip_pomodoro(state: State<AppState>, app: AppHandle) -> PomodoroStatus {
    let status = state.pomodoro.skip();
    let _ = tray::on_pomodoro_phase_change(&app, &status);
    status
}
