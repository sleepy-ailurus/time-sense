use tauri::{
    AppHandle, Emitter, Manager, Wry,
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Result,
};

use crate::db::{ActivityDao, AppStat};
use crate::AppState;

/// 设置系统托盘
pub fn setup_tray(app: &AppHandle) -> Result<TrayIcon> {
    let menu = build_tray_menu(app, &[])?;
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

/// 构建托盘菜单（包含今日统计 Top 3）
fn build_tray_menu(app: &AppHandle<Wry>, top_apps: &[AppStat]) -> Result<Menu<Wry>> {
    let open_item = MenuItem::with_id(app, "open_main", "打开主面板", true, None::<&str>)?;
    let stats_submenu = build_stats_submenu(app, top_apps)?;
    let pause_item = MenuItem::with_id(app, "toggle_recording", "暂停记录", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 TimeSense", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &open_item,
        &stats_submenu,
        &pause_item,
        &quit_item,
    ])?;

    Ok(menu)
}

/// 构建今日统计子菜单
fn build_stats_submenu(app: &AppHandle<Wry>, top_apps: &[AppStat]) -> Result<Submenu<Wry>> {
    if top_apps.is_empty() {
        let empty_item = MenuItem::with_id(app, "stats_empty", "  暂无数据", false, None::<&str>)?;
        let submenu = Submenu::with_items(app, "今日统计", true, &[&empty_item])?;
        return Ok(submenu);
    }

    let item0 = MenuItem::with_id(
        app,
        "stats_0",
        format!("  1. {}  {}", truncate_str(&top_apps[0].process_name, 14), format_duration(top_apps[0].total_seconds)),
        false,
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
        false,
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
        false,
        None::<&str>,
    )?;

    let submenu = Submenu::with_items(app, "今日统计", true, &[&item0, &item1, &item2])?;
    Ok(submenu)
}

/// 刷新托盘菜单中的今日统计
pub fn refresh_tray_stats(app: &AppHandle<Wry>) -> Result<()> {
    let state = app.state::<AppState>();
    let conn = state.db.conn().lock();
    let stats = ActivityDao::get_today_stats(&conn).unwrap_or_default();
    drop(conn);

    // 更新 tooltip
    let total_active_secs: i64 = stats.iter().map(|s| s.total_seconds).sum();
    let tooltip = format!("TimeSense - 今日活跃 {}", format_duration(total_active_secs));

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
        if let Ok(menu) = build_tray_menu(app, &stats) {
            let _ = tray.set_menu(Some(menu));
        }
    }

    Ok(())
}

fn toggle_main_window(app: &AppHandle<Wry>) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            show_main_window(app);
        }
    }
}

fn show_main_window(app: &AppHandle<Wry>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
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
