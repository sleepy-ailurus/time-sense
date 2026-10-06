pub mod commands;
pub mod db;
pub mod monitor;
pub mod engine;
pub mod tray;

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::{Manager, Emitter};

use db::Database;
use engine::activity_state::ActivityStateMachine;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub activity_engine: Arc<Mutex<ActivityStateMachine>>,
    pub is_recording: Arc<Mutex<bool>>,
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
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // 初始化数据库
            let db = Arc::new(Database::new(&app.handle())?);
            db.init()?;
            tracing::info!("Database initialized");

            // 初始化活动状态机
            let activity_engine = Arc::new(Mutex::new(ActivityStateMachine::new(db.clone())));
            let is_recording = Arc::new(Mutex::new(true));

            // 管理状态
            app.manage(AppState {
                db: db.clone(),
                activity_engine: activity_engine.clone(),
                is_recording: is_recording.clone(),
            });

            // 设置托盘
            let _tray = tray::setup_tray(app.handle())?;

            // 启动监控循环
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                engine::start_monitor_loop(app_handle, activity_engine, is_recording).await;
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
            commands::get_activity_by_date,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
