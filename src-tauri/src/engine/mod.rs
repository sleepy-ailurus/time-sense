pub mod activity_state;

use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use tauri::AppHandle;

use crate::monitor;
use activity_state::ActivityStateMachine;

/// 监控循环间隔（秒）
const MONITOR_INTERVAL_SECS: u64 = 5;

/// 空闲阈值（秒）— 超过此值判定为空闲
const IDLE_THRESHOLD_SECS: u64 = 300; // 5 分钟

/// 防抖阈值（秒）— 小于此时间的窗口切换忽略
const DEBOUNCE_SECS: i64 = 3;

/// 启动主监控循环
pub async fn start_monitor_loop(
    _app: AppHandle,
    engine: Arc<Mutex<ActivityStateMachine>>,
    is_recording: Arc<Mutex<bool>>,
) {
    let monitor = monitor::create_monitor();
    let mut interval = tokio::time::interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

    tracing::info!("Monitor loop started (interval: {}s)", MONITOR_INTERVAL_SECS);

    loop {
        interval.tick().await;

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

        let is_idle = idle_time.as_secs() >= IDLE_THRESHOLD_SECS;

        // 更新状态机
        let mut eng = engine.lock();
        eng.tick(&window_info, is_idle, DEBOUNCE_SECS);
    }
}
