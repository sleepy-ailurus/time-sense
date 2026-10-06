//! 托盘图标生成：根据状态生成不同颜色的图标
//!
//! 直接用代码生成 32x32 的 RGBA 圆形图标
//! 托盘图标尺寸小，纯色圆形辨识度最高

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::image::Image;

/// 托盘图标状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayIconState {
    Idle,       // 空闲/默认：灰色
    Focus,      // 专注中：紫色
    ShortBreak, // 短休息：绿色
    LongBreak,  // 长休息：蓝色
    Paused,     // 暂停：黄色
}

impl TrayIconState {
    pub fn from_pomodoro(is_running: bool, is_paused: bool, session_type: Option<&str>) -> Self {
        if !is_running {
            return TrayIconState::Idle;
        }
        if is_paused {
            return TrayIconState::Paused;
        }
        match session_type {
            Some("focus") => TrayIconState::Focus,
            Some("short_break") => TrayIconState::ShortBreak,
            Some("long_break") => TrayIconState::LongBreak,
            _ => TrayIconState::Idle,
        }
    }

    fn color(&self) -> (u8, u8, u8) {
        match self {
            TrayIconState::Idle => (107, 114, 128),   // 灰色 #6B7280
            TrayIconState::Focus => (139, 92, 246),   // 紫色 #8B5CF6
            TrayIconState::ShortBreak => (16, 185, 129), // 绿色 #10B981
            TrayIconState::LongBreak => (59, 130, 246), // 蓝色 #3B82F6
            TrayIconState::Paused => (245, 158, 11),  // 琥珀色 #F59E0B
        }
    }
}

// 缓存：每种状态只生成一次图标
static ICON_CACHE: Mutex<Option<HashMap<TrayIconState, Image<'static>>>> = Mutex::new(None);

/// 获取指定状态的托盘图标（带缓存）
pub fn get_icon(state: TrayIconState) -> Image<'static> {
    let mut cache = ICON_CACHE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(HashMap::new());
    }
    let map = cache.as_mut().unwrap();

    if let Some(icon) = map.get(&state) {
        return icon.clone();
    }

    let icon = generate_icon(state);
    map.insert(state, icon.clone());
    icon
}

/// 生成一个 32x32 的圆形图标（带抗锯齿边缘）
fn generate_icon(state: TrayIconState) -> Image<'static> {
    const SIZE: usize = 32;
    const CENTER: f32 = 15.5;
    const RADIUS: f32 = 12.0;

    let (r, g, b) = state.color();
    let mut rgba = Vec::with_capacity(SIZE * SIZE * 4);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - CENTER;
            let dy = y as f32 - CENTER;
            let dist = (dx * dx + dy * dy).sqrt();

            // 圆形 + 边缘抗锯齿
            let alpha = if dist <= RADIUS - 1.0 {
                255
            } else if dist <= RADIUS + 1.0 {
                let t = (RADIUS + 1.0 - dist) / 2.0;
                (t.max(0.0) * 255.0) as u8
            } else {
                0
            };

            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(alpha);
        }
    }

    // leak 成 'static 生命周期，Image 需要引用数据
    // 因为有缓存，总共只生成 5 个图标，leak 可以忽略
    let rgba_slice: &'static [u8] = rgba.leak();
    Image::new(rgba_slice, SIZE as u32, SIZE as u32)
}
