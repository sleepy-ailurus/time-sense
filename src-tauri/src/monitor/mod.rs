#[cfg(target_os = "windows")]
mod windows;

use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("Failed to get active window: {0}")]
    WindowError(String),
    #[error("Failed to get idle time: {0}")]
    IdleError(String),
}

/// 窗口信息
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub process_name: String,
    pub window_title: String,
    pub pid: u32,
}

/// 活动监控 trait
pub trait ActivityMonitor: Send + Sync {
    fn get_active_window(&self) -> Result<WindowInfo, MonitorError>;
    fn get_idle_time(&self) -> Result<Duration, MonitorError>;
}

/// 创建当前平台的监控器
pub fn create_monitor() -> Box<dyn ActivityMonitor> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsMonitor)
    }
    #[cfg(not(target_os = "windows"))]
    {
        compile_error!("Unsupported platform");
    }
}
