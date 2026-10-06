use std::time::Duration;
use std::path::Path;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::System::SystemInformation::GetTickCount;

use super::{ActivityMonitor, MonitorError, WindowInfo};

pub struct WindowsMonitor;

impl ActivityMonitor for WindowsMonitor {
    fn get_active_window(&self) -> Result<WindowInfo, MonitorError> {
        // 使用 active-win-pos-rs 库获取活跃窗口
        let win = active_win_pos_rs::get_active_window()
            .map_err(|_| MonitorError::WindowError("Failed to get active window".into()))?;

        // 从 process_path 中提取进程名
        let process_name = Path::new(&win.process_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| win.process_path.to_string_lossy().into_owned());

        Ok(WindowInfo {
            process_name,
            window_title: win.title,
            pid: win.process_id as u32,
        })
    }

    fn get_idle_time(&self) -> Result<Duration, MonitorError> {
        // 使用 Windows API GetLastInputInfo
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        let ok = unsafe { GetLastInputInfo(&mut lii) };
        if !ok.as_bool() {
            return Err(MonitorError::IdleError("GetLastInputInfo failed".into()));
        }

        let tick_count = unsafe { GetTickCount() };
        let idle_ms = tick_count.saturating_sub(lii.dwTime);

        Ok(Duration::from_millis(idle_ms as u64))
    }
}
