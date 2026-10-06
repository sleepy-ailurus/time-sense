use std::time::Duration;
use std::path::Path;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::StationsAndDesktops::*;

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
        // 如果锁屏了，直接返回很大的空闲时间（立即触发空闲状态）
        if is_session_locked() {
            return Ok(Duration::from_secs(3600)); // 1 小时，足够触发任何空闲阈值
        }

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

/// 检测当前会话是否被锁定
/// 
/// 原理：尝试打开输入桌面（默认桌面），如果失败说明当前不在默认桌面（可能是锁屏状态）
fn is_session_locked() -> bool {
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) {
            Ok(hdesk) => {
                // 成功打开，说明没锁屏
                let _ = CloseDesktop(hdesk);
                false
            }
            Err(_) => {
                // 打开失败，可能是锁屏了
                true
            }
        }
    }
}
