use tauri::{
    AppHandle, Emitter, Manager, Wry,
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Result,
};

use crate::db::{ActivityDao, AppStat, PomodoroStatus};
use crate::tray_icon::{self, TrayIconState};
use crate::AppState;

/// 立即更新托盘图标颜色（番茄钟状态变化时调用）
pub fn update_tray_icon(app: &AppHandle<Wry>, status: &PomodoroStatus) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon_state = TrayIconState::from_pomodoro(
            status.is_running,
            status.is_paused,
            status.session_type.as_deref(),
        );
        let _ = tray.set_icon(Some(tray_icon::get_icon(icon_state)));
    }
}

/// 设置系统托盘
pub fn setup_tray(app: &AppHandle) -> Result<TrayIcon> {
    let menu = build_tray_menu(app, &[], None)?;
    let tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("TimeSense")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_main" => {
                show_main_window(&app);
            }
            "toggle_recording" => {
                let _ = app.emit("tray://toggle_recording", ());
            }
            "pomodoro_start" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let status = state.pomodoro.start_focus();
                    let _ = on_pomodoro_phase_change(&app, &status);
                }
            }
            "pomodoro_pause" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let status = state.pomodoro.pause();
                    let _ = on_pomodoro_phase_change(&app, &status);
                }
            }
            "pomodoro_resume" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let in_focus_zone = {
                        let engine = state.activity_engine.lock();
                        crate::engine::is_current_focus_activity(&engine, &state.db)
                    };
                    let status = state.pomodoro.resume(!in_focus_zone);
                    let _ = on_pomodoro_phase_change(&app, &status);
                }
            }
            "pomodoro_stop" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let status = state.pomodoro.stop();
                    let _ = on_pomodoro_phase_change(&app, &status);
                }
            }
            "pomodoro_skip" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let status = state.pomodoro.skip();
                    let _ = on_pomodoro_phase_change(&app, &status);
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_main_window(&app);
            }
        })
        .build(app)?;

    Ok(tray)
}

/// 构建托盘菜单
fn build_tray_menu(app: &AppHandle<Wry>, top_apps: &[AppStat], pom_status: Option<&PomodoroStatus>) -> Result<Menu<Wry>> {
    let open_item = MenuItem::with_id(app, "open_main", "打开主面板", true, None::<&str>)?;
    let stats_submenu = build_stats_submenu(app, top_apps)?;
    let pomodoro_submenu = build_pomodoro_submenu(app, pom_status)?;
    let pause_item = MenuItem::with_id(app, "toggle_recording", "暂停记录", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 TimeSense", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &open_item,
        &stats_submenu,
        &pomodoro_submenu,
        &pause_item,
        &quit_item,
    ])?;

    Ok(menu)
}

/// 构建今日统计子菜单
fn build_stats_submenu(app: &AppHandle<Wry>, top_apps: &[AppStat]) -> Result<Submenu<Wry>> {
    if top_apps.is_empty() {
        let empty_item = MenuItem::with_id(app, "stats_empty", "  暂无数据", true, None::<&str>)?;
        let submenu = Submenu::with_items(app, "今日统计", true, &[&empty_item])?;
        return Ok(submenu);
    }

    let item0 = MenuItem::with_id(
        app,
        "stats_0",
        format!("  1. {}  {}", truncate_str(&top_apps[0].process_name, 14), format_duration(top_apps[0].total_seconds)),
        true,
        None::<&str>,
    )?;

    if top_apps.len() == 1 {
        let submenu = Submenu::with_items(app, "今日统计", true, &[&item0])?;
        return Ok(submenu);
    }

    let item1 = MenuItem::with_id(
        app,
        "stats_1",
        format!("  2. {}  {}", truncate_str(&top_apps[1].process_name, 14), format_duration(top_apps[1].total_seconds)),
        true,
        None::<&str>,
    )?;

    if top_apps.len() == 2 {
        let submenu = Submenu::with_items(app, "今日统计", true, &[&item0, &item1])?;
        return Ok(submenu);
    }

    let item2 = MenuItem::with_id(
        app,
        "stats_2",
        format!("  3. {}  {}", truncate_str(&top_apps[2].process_name, 14), format_duration(top_apps[2].total_seconds)),
        true,
        None::<&str>,
    )?;

    let submenu = Submenu::with_items(app, "今日统计", true, &[&item0, &item1, &item2])?;
    Ok(submenu)
}

/// 构建番茄钟子菜单
fn build_pomodoro_submenu(app: &AppHandle<Wry>, status: Option<&PomodoroStatus>) -> Result<Submenu<Wry>> {
    match status {
        Some(s) if s.is_running => {
            let phase_label = match s.session_type.as_deref() {
                Some("focus") => "专注中",
                Some("short_break") => "短休息",
                Some("long_break") => "长休息",
                _ => "进行中",
            };
            let remaining = format_remaining(s.remaining_seconds);
            let status_item = MenuItem::with_id(
                app,
                "pom_status",
                format!("  {} · 剩余 {}", phase_label, remaining),
                true,
                None::<&str>,
            )?;
            let count_item = MenuItem::with_id(
                app,
                "pom_count",
                format!("  今日完成 {} 个番茄", s.today_focus_count),
                true,
                None::<&str>,
            )?;
            let skip_item = MenuItem::with_id(app, "pomodoro_skip", "  跳过当前", true, None::<&str>)?;
            let pause_item = if s.is_paused {
                MenuItem::with_id(app, "pomodoro_resume", "  继续", true, None::<&str>)?
            } else {
                MenuItem::with_id(app, "pomodoro_pause", "  暂停", true, None::<&str>)?
            };
            let stop_item = MenuItem::with_id(app, "pomodoro_stop", "  停止番茄钟", true, None::<&str>)?;

            let submenu = Submenu::with_items(
                app,
                "🍅 番茄钟",
                true,
                &[&status_item, &count_item, &pause_item, &skip_item, &stop_item],
            )?;
            Ok(submenu)
        }
        _ => {
            let start_item = MenuItem::with_id(app, "pomodoro_start", "  开始专注", true, None::<&str>)?;
            let submenu = Submenu::with_items(app, "🍅 番茄钟", true, &[&start_item])?;
            Ok(submenu)
        }
    }
}

/// 刷新托盘菜单中的今日统计
pub fn refresh_tray_stats(app: &AppHandle<Wry>) -> Result<()> {
    let state = app.state::<AppState>();
    let conn = state.db.conn().lock();
    let stats = ActivityDao::get_today_stats(&conn).unwrap_or_default();
    drop(conn);

    // 获取番茄钟状态
    let pom_status = state.pomodoro.get_status();

    // 更新 tooltip
    let total_active_secs: i64 = stats.iter().map(|s| s.total_seconds).sum();
    let pom_tip = if pom_status.is_running {
        let phase = match pom_status.session_type.as_deref() {
            Some("focus") => "专注中",
            Some("short_break") => "短休息",
            Some("long_break") => "长休息",
            _ => "",
        };
        format!("\n🍅 {} · 今日 {} 个", phase, pom_status.today_focus_count)
    } else {
        String::new()
    };
    let tooltip = format!("TimeSense - 今日活跃 {}{}", format_duration(total_active_secs), pom_tip);

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
        if let Ok(menu) = build_tray_menu(app, &stats, Some(&pom_status)) {
            let _ = tray.set_menu(Some(menu));
        }
        // 根据番茄钟状态切换图标颜色
        let icon_state = TrayIconState::from_pomodoro(
            pom_status.is_running,
            pom_status.is_paused,
            pom_status.session_type.as_deref(),
        );
        let _ = tray.set_icon(Some(tray_icon::get_icon(icon_state)));
    }

    Ok(())
}

/// 番茄钟状态变化时调用：发通知 + 刷新托盘
/// 只有真正的阶段切换（focus↔break↔idle）才发通知，暂停/继续不发
pub fn on_pomodoro_phase_change(app: &AppHandle<Wry>, status: &PomodoroStatus) -> Result<()> {
    // 立刻换图标
    update_tray_icon(app, status);

    // 只有真正的阶段切换才发系统通知（暂停/继续不算）
    if status.is_phase_transition {
        // 检查是否启用了系统通知
        let notification_enabled = app
            .try_state::<AppState>()
            .map(|s| s.general_settings.read().notification_enabled)
            .unwrap_or(true);

        if notification_enabled {
            if let Some(typ) = status.session_type.as_deref() {
                let (title, body) = match typ {
                    "focus" => ("开始专注", "专注模式已启动，加油！"),
                    "short_break" => ("休息一下", "专注结束，起来活动活动吧 🌿"),
                    "long_break" => ("长休息", "辛苦了，好好休息一下 ☕"),
                    _ => ("", ""),
                };

                if !title.is_empty() {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app
                        .notification()
                        .builder()
                        .title(title)
                        .body(body)
                        .show();
                }
            }
        }
    }

    // 通知前端状态变化
    let _ = app.emit("pomodoro://status-changed", status);

    // 刷新托盘菜单和 tooltip
    let _ = refresh_tray_stats(app);

    Ok(())
}

fn toggle_main_window(app: &AppHandle<Wry>) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        let minimized = win.is_minimized().unwrap_or(false);
        if visible && !minimized {
            // 窗口正常显示中 → 隐藏
            let _ = win.hide();
        } else {
            // 窗口隐藏或最小化 → 显示并激活
            show_main_window(app);
        }
    }
}

pub fn show_main_window(app: &AppHandle<Wry>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        // 如果是最小化状态，先恢复
        if win.is_minimized().unwrap_or(false) {
            let _ = win.unminimize();
        }
        let _ = win.set_focus();
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max.saturating_sub(1)).collect();
        result.push('…');
        result
    }
}

fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m{}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h{}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

fn format_remaining(seconds: i64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}
