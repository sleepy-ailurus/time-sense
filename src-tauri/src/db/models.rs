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
}

/// 应用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStat {
    pub process_name: String,
    pub total_seconds: i64,
    pub percentage: f64,
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
}
