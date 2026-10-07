use tauri::{State, Window, AppHandle, Emitter};

use crate::db::{
    ActivityDao, ActivityLog, AggregatesDao, AppStat, Category, CategoryDao, CategoryStat,
    CurrentActivity, DailySummary, GeneralSettings, HeatmapDay, HourlyStat, NewAppRule,
    PomodoroSettings, PomodoroStatus, RuleDao, SettingsDao, TodayTotal,
};
use crate::AppState;
use crate::tray;

// ==================== 活动相关 ====================

/// 获取各应用耗时统计（按耗时倒序）；date 为 None 时查今天
#[tauri::command]
pub fn get_today_stats(state: State<AppState>, date: Option<String>) -> Result<Vec<AppStat>, String> {
    let conn = state.db.conn().lock();
    ActivityDao::get_stats_by_date(&conn, date.as_deref()).map_err(|e| e.to_string())
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
pub fn toggle_recording(state: State<AppState>, app: AppHandle) -> bool {
    let now_recording = {
        let mut recording = state.is_recording.lock();
        *recording = !*recording;
        *recording
    };
    if !now_recording {
        // 暂停记录：闭合当前活动段，避免恢复后把暂停期间计入旧活动
        let mut engine = state.activity_engine.lock();
        engine.reset_current();
    }
    let _ = app.emit("recording://changed", now_recording);
    now_recording
}

/// 隐藏主窗口（收起面板）
#[tauri::command]
pub fn hide_main_window(window: Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

/// 是否应在启动时显示主窗口（普通启动=true，开机自启静默驻留托盘=false）
#[tauri::command]
pub fn should_show_window_on_start(state: State<AppState>) -> bool {
    !state.started_with_autostart
}

/// 获取应用版本号
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 在默认浏览器中打开 URL
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    webbrowser::open(&url).map_err(|e| e.to_string())
}

// ==================== 检查更新 ====================

/// 最新版本信息
#[derive(serde::Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

/// 更新源（latest.json 静态文件，无 CORS / 限流问题）
const UPDATE_SOURCES: &[&str] = &[
    // Gitee raw：国内访问最稳
    "https://gitee.com/sleepy-ailurus/time-sense/raw/master/latest.json",
    // jsDelivr：GitHub 仓库 CDN 镜像
    "https://cdn.jsdelivr.net/gh/sleepy-ailurus/time-sense@master/latest.json",
];

/// 检查更新：在 Rust 端依次尝试 Gitee / jsDelivr 的 latest.json，
/// 最后回退到 GitHub API（匿名请求可能被限流）。
/// 网络请求放在 Rust 端而非前端 webview，可绕过浏览器 CORS 限制。
#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(fetch_latest_update)
        .await
        .map_err(|e| e.to_string())?
}

fn fetch_latest_update() -> Result<UpdateInfo, String> {
    const GITHUB_RELEASES_URL: &str = "https://github.com/sleepy-ailurus/time-sense/releases";

    // 1) 静态 latest.json 源
    for source in UPDATE_SOURCES {
        if let Ok(resp) = ureq::get(source).timeout(std::time::Duration::from_secs(8)).call() {
            if resp.status() == 200 {
                if let Ok(json) = resp.into_json::<serde_json::Value>() {
                    if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                        if !version.is_empty() {
                            let url = json
                                .get("url")
                                .and_then(|v| v.as_str())
                                .unwrap_or(GITHUB_RELEASES_URL);
                            return Ok(UpdateInfo {
                                version: version.trim_start_matches('v').to_string(),
                                url: url.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 2) GitHub API 兜底
    if let Ok(resp) = ureq::get("https://api.github.com/repos/sleepy-ailurus/time-sense/releases/latest")
        .set("User-Agent", "TimeSense-Updater")
        .timeout(std::time::Duration::from_secs(8))
        .call()
    {
        if resp.status() == 200 {
            if let Ok(json) = resp.into_json::<serde_json::Value>() {
                if let Some(tag) = json.get("tag_name").and_then(|v| v.as_str()) {
                    if !tag.is_empty() {
                        let url = json
                            .get("html_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or(GITHUB_RELEASES_URL);
                        return Ok(UpdateInfo {
                            version: tag.trim_start_matches('v').to_string(),
                            url: url.to_string(),
                        });
                    }
                }
            }
        }
    }

    Err("所有更新源均不可用，请检查网络后重试".into())
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

/// 获取分类统计（按耗时倒序）；date 为 None 时查今天
#[tauri::command]
pub fn get_today_category_stats(state: State<AppState>, date: Option<String>) -> Result<Vec<CategoryStat>, String> {
    let conn = state.db.conn().lock();
    CategoryDao::get_category_stats_by_date(&conn, date.as_deref()).map_err(|e| e.to_string())
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

/// 获取常规设置
#[tauri::command]
pub fn get_general_settings(state: State<AppState>) -> GeneralSettings {
    state.general_settings.read().clone()
}

/// 更新常规设置
#[tauri::command]
pub fn update_general_settings(
    state: State<AppState>,
    app: AppHandle,
    settings: GeneralSettings,
) -> Result<(), String> {
    tracing::info!("update_general_settings called: auto_start={}, notification_enabled={}, idle_threshold_minutes={}",
        settings.auto_start, settings.notification_enabled, settings.idle_threshold_minutes);

    // 更新内存 + 持久化
    {
        let mut gs = state.general_settings.write();
        *gs = settings.clone();
        let conn = state.db.conn().lock();
        SettingsDao::save_general(&conn, &settings).map_err(|e| {
            tracing::error!("Failed to save general settings: {}", e);
            e.to_string()
        })?;
    }

    // 应用开机自启设置
    if let Err(e) = apply_autostart(&app, settings.auto_start) {
        tracing::error!("Failed to apply autostart: {}", e);
        return Err(e);
    }

    Ok(())
}

/// 应用开机自启设置（Windows 注册表方式）
#[cfg(target_os = "windows")]
pub fn apply_autostart(_app: &AppHandle, enable: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    tracing::info!("apply_autostart: enable={}", enable);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_ALL_ACCESS)
        .map_err(|e| format!("打开注册表失败: {}", e))?;

    let app_name = "TimeSense";

    if enable {
        // 获取当前 exe 路径
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取 exe 路径失败: {}", e))?;
        // 带 --autostart 参数：开机自启时静默驻留托盘，不弹窗口
        let cmd = format!("\"{}\" --autostart", exe_path.to_string_lossy());
        tracing::info!("Writing autostart registry value: {} -> {}", app_name, cmd);
        run_key
            .set_value(app_name, &cmd)
            .map_err(|e| format!("写入注册表失败: {}", e))?;
        tracing::info!("Autostart registry value written successfully");
    } else {
        tracing::info!("Removing autostart registry value: {}", app_name);
        match run_key.delete_value(app_name) {
            Ok(_) => tracing::info!("Autostart registry value removed successfully"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("Autostart registry value not found (already removed)");
            }
            Err(e) => return Err(format!("删除注册表失败: {}", e)),
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_autostart(_app: &AppHandle, _enable: bool) -> Result<(), String> {
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

// ==================== 数据统计与聚合 ====================

/// 获取指定日期的汇总数据
#[tauri::command]
pub fn get_daily_summary(state: State<AppState>, date: String) -> Result<DailySummary, String> {
    let conn = state.db.conn().lock();
    AggregatesDao::get_daily_summary(&conn, &date).map_err(|e| e.to_string())
}

/// 获取过去7天趋势
#[tauri::command]
pub fn get_weekly_trend(state: State<AppState>) -> Result<Vec<DailySummary>, String> {
    let conn = state.db.conn().lock();
    AggregatesDao::get_weekly_trend(&conn).map_err(|e| e.to_string())
}

/// 获取年度热力图数据
#[tauri::command]
pub fn get_heatmap_data(state: State<AppState>, year: i32) -> Result<Vec<HeatmapDay>, String> {
    let conn = state.db.conn().lock();
    AggregatesDao::get_heatmap_data(&conn, year).map_err(|e| e.to_string())
}

/// 获取时段分布数据
#[tauri::command]
pub fn get_hourly_distribution(state: State<AppState>, date: Option<String>) -> Result<Vec<HourlyStat>, String> {
    let conn = state.db.conn().lock();
    AggregatesDao::get_hourly_distribution(&conn, date.as_deref()).map_err(|e| e.to_string())
}
