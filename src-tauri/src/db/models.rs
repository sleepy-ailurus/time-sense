use serde::{Deserialize, Serialize};

/// 活动日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLog {
    pub id: Option<i64>,
    pub process_name: String,
    pub window_title: Option<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
    pub is_idle: bool,
    pub category_id: Option<i64>,
}

/// 应用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStat {
    pub process_name: String,
    pub total_seconds: i64,
    pub percentage: f64,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_color: Option<String>,
}

/// 今日总览
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayTotal {
    pub total_seconds: i64,
    pub active_seconds: i64,
    pub idle_seconds: i64,
}

/// 当前活动状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentActivity {
    pub process_name: String,
    pub window_title: String,
    pub start_time: i64,
    pub duration: i64,
    pub is_idle: bool,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
}

// ---------- 分类 ----------

/// 分类
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub is_default: bool,
}

/// 分类统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    pub category_id: i64,
    pub category_name: String,
    pub category_color: String,
    pub category_icon: Option<String>,
    pub total_seconds: i64,
    pub percentage: f64,
}

// ---------- 规则 ----------

/// 匹配规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRule {
    pub id: i64,
    pub category_id: i64,
    pub category_name: Option<String>,
    pub match_type: String, // process / title
    pub match_value: String,
    pub match_mode: String, // exact / contains / regex
    pub sort_order: i32,
    pub enabled: bool,
}

/// 新建规则入参
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAppRule {
    pub category_id: i64,
    pub match_type: String,
    pub match_value: String,
    pub match_mode: String,
}

// ---------- 番茄钟 ----------

/// 番茄钟会话
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSession {
    pub id: Option<i64>,
    pub session_type: String, // focus / short_break / long_break
    pub start_time: i64,
    pub end_time: i64,
    pub duration: i64,
    pub completed: bool,
    pub interrupted: bool,
}

/// 番茄钟状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroStatus {
    pub is_running: bool,
    pub is_paused: bool,
    pub session_type: Option<String>,
    pub start_time: Option<i64>,
    /// 已用毫秒（毫秒精度，避免暂停/恢复出现 ±1 秒误差）
    pub elapsed_ms: i64,
    /// 剩余毫秒（毫秒精度）
    pub remaining_ms: i64,
    /// 生成该状态时的后端时间戳（毫秒，供前端对齐计时基准）
    pub server_time_ms: i64,
    pub elapsed_seconds: i64,
    pub target_seconds: i64,
    pub remaining_seconds: i64,
    pub today_focus_count: i32,
    pub today_focus_seconds: i64,
    /// 是否是真正的阶段切换（focus↔break↔idle），暂停/继续/普通查询为 false
    pub is_phase_transition: bool,
    /// 当前专注会话是否是用户手动启动的
    /// 手动启动的会话，自动模式不会自动暂停（尊重用户手动操作的意图）
    pub is_manually_started: bool,
}

/// 番茄钟设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSettings {
    pub focus_minutes: i32,
    pub short_break_minutes: i32,
    pub long_break_minutes: i32,
    pub long_break_interval: i32, // 每几个番茄后长休息
    pub auto_start_break: bool,
    pub auto_start_focus: bool,
    pub auto_mode: bool, // 自动番茄钟：根据活动分类自动计时
    pub enabled: bool,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        Self {
            focus_minutes: 45,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_interval: 4,
            auto_start_break: true,
            auto_start_focus: false,
            auto_mode: false,
            enabled: true,
        }
    }
}

/// 常规设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub auto_start: bool,         // 开机自启
    pub notification_enabled: bool, // 系统通知
    pub idle_threshold_minutes: i32, // 闲置检测阈值（分钟）
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            auto_start: true,
            notification_enabled: true,
            idle_threshold_minutes: 3,
        }
    }
}

// ---------- 数据统计与聚合 ----------

/// 每日汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    pub date: String,              // YYYY-MM-DD
    pub total_seconds: i64,        // 活跃总时长（不含空闲）
    pub idle_seconds: i64,         // 空闲总时长
    pub work_seconds: i64,         // 工作类时长
    pub study_seconds: i64,        // 学习类时长
    pub entertainment_seconds: i64,// 娱乐类时长
    pub social_seconds: i64,       // 社交类时长
    pub other_seconds: i64,        // 其他类时长
    pub pomodoro_count: i32,       // 完成番茄钟数
    pub pomodoro_seconds: i64,     // 番茄钟专注总时长
}

/// 热力图单天数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    pub date: String,         // YYYY-MM-DD
    pub total_seconds: i64,   // 专注总时长（工作+学习）
    pub pomodoro_count: i32,  // 完成番茄钟数
}

/// 时段统计（小时维度）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyStat {
    pub hour: i32,            // 0-23
    pub total_seconds: i64,   // 该小时总活跃时长
    pub weekday: i32,         // 0-6 (0=周一, 6=周日)，用于7天热力分布
    pub date: String,         // YYYY-MM-DD
}
