pub mod commands;
pub mod db;
pub mod monitor;
pub mod engine;
pub mod tray;
pub mod tray_icon;

use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use tauri::Manager;

use db::{Database, GeneralSettings, SettingsDao};
use engine::activity_state::ActivityStateMachine;
use engine::pomodoro::PomodoroEngine;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub activity_engine: Arc<Mutex<ActivityStateMachine>>,
    pub pomodoro: Arc<PomodoroEngine>,
    pub is_recording: Arc<Mutex<bool>>,
    pub general_settings: Arc<RwLock<GeneralSettings>>,
    /// 本次是否由开机自启拉起（启动参数含 --autostart）
    pub started_with_autostart: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timesense_lib=info,warn,error".into()),
        )
        .with_target(false)
        .init();

    tracing::info!("TimeSense starting...");

    tauri::Builder::default()
        // 单实例：再次启动（任务栏 Jump List、双击 exe、快捷方式等）不会开第二个进程，
        // 而是把已经在跑的窗口显示出来。必须放在其它插件之前注册。
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // 开机自启触发的重复拉起：保持静默驻留托盘，不弹窗
            if args.iter().any(|a| a == "--autostart") {
                tracing::info!("Autostart duplicate launch -> keep hidden");
                return;
            }
            tracing::info!("Another instance was launched -> focus existing window");
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 是否由开机自启拉起：开机自启的注册表命令带 --autostart 参数，
            // 此时保持窗口隐藏静默驻留托盘；普通启动由前端恢复位置后显示窗口
            let started_with_autostart = std::env::args().any(|a| a == "--autostart");

            // 初始化数据库
            let db = Arc::new(Database::new(&app.handle())?);
            db.init()?;
            tracing::info!("Database initialized");

            // 初始化活动状态机
            let activity_engine = Arc::new(Mutex::new(ActivityStateMachine::new(db.clone())));
            let is_recording = Arc::new(Mutex::new(true));

            // 初始化番茄钟引擎
            let pomodoro = Arc::new(PomodoroEngine::new(db.clone()));

            // 加载常规设置
            let general_settings = Arc::new(RwLock::new({
                let conn = db.conn().lock();
                SettingsDao::load_general(&conn)
            }));

            // 管理状态
            app.manage(AppState {
                db: db.clone(),
                activity_engine: activity_engine.clone(),
                pomodoro: pomodoro.clone(),
                is_recording: is_recording.clone(),
                general_settings: general_settings.clone(),
                started_with_autostart,
            });

            // 设置托盘
            let _tray = tray::setup_tray(app.handle())?;

            // 启动时同步开机自启注册表：确保 DB 中的设置与系统注册表一致
            // （首次安装默认 auto_start=true，此处把自启写进注册表使其真正生效）
            {
                let gs = general_settings.read();
                if let Err(e) = commands::apply_autostart(app.handle(), gs.auto_start) {
                    tracing::warn!("Failed to sync autostart on startup: {}", e);
                }
            }

            // 监听主窗口事件：点「关闭」时隐藏到托盘，不退出程序
            //
            // 这里只处理 CloseRequested，**不要**把最小化事件转成 hide()：
            // 点击任务栏上的程序图标时，Windows 会先把窗口最小化，如果此时调用 hide()，
            // 窗口和任务栏按钮会一起消失（任务栏右键弹出系统菜单导致失焦时同理）。
            // 让最小化走系统默认行为，任务栏按钮就会一直保留，点一下最小化、再点一下还原。
            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        tracing::info!("Window CloseRequested -> hide to tray");
                        let _ = win_clone.hide();
                    }
                });
            }

            // 启动监控循环
            let app_handle = app.handle().clone();
            let gs_clone = general_settings.clone();
            tauri::async_runtime::spawn(async move {
                engine::start_monitor_loop(app_handle, activity_engine, pomodoro, is_recording, db, gs_clone).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_stats,
            commands::get_today_total,
            commands::get_current_activity,
            commands::is_recording,
            commands::toggle_recording,
            commands::hide_main_window,
            commands::should_show_window_on_start,
            commands::get_app_version,
            commands::open_url,
            commands::check_update,
            commands::get_activity_by_date,
            // 分类
            commands::get_categories,
            commands::get_today_category_stats,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            // 规则
            commands::get_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::toggle_rule,
            commands::refresh_rules,
            // 番茄钟
            commands::get_pomodoro_status,
            commands::get_pomodoro_settings,
            commands::update_pomodoro_settings,
            commands::start_pomodoro_focus,
            commands::start_pomodoro_break,
            commands::stop_pomodoro,
            commands::pause_pomodoro,
            commands::resume_pomodoro,
            commands::skip_pomodoro,
            // 常规设置
            commands::get_general_settings,
            commands::update_general_settings,
            // 数据统计与聚合
            commands::get_daily_summary,
            commands::get_weekly_trend,
            commands::get_heatmap_data,
            commands::get_hourly_distribution,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
