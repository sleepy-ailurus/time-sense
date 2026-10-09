import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityLog,
  AppRule,
  AppStat,
  Category,
  CategoryStat,
  CurrentActivity,
  GeneralSettings,
  Goal,
  GoalStatus,
  NewAppRule,
  NewGoal,
  PomodoroSettings,
  PomodoroStatus,
  TodayTotal,
  UpdateInfo,
} from "./types";

/**
 * 统一错误处理
 */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    console.error(`[API] ${command} failed:`, e);
    throw e;
  }
}

// ============================================================
// 活动统计相关
// ============================================================

/**
 * 获取今日各应用耗时统计（按耗时倒序）
 */
export function getTodayStats(): Promise<AppStat[]> {
  return call<AppStat[]>("get_today_stats");
}

/**
 * 获取今日总时长概览
 */
export function getTodayTotal(): Promise<TodayTotal> {
  return call<TodayTotal>("get_today_total");
}

/**
 * 获取指定日期的所有活动记录
 * @param date YYYY-MM-DD 格式的日期字符串
 */
export function getActivityByDate(date: string): Promise<ActivityLog[]> {
  return call<ActivityLog[]>("get_activity_by_date", { date });
}

/**
 * 获取当前正在进行的活动
 */
export function getCurrentActivity(): Promise<CurrentActivity | null> {
  return call<CurrentActivity | null>("get_current_activity");
}

// ============================================================
// 记录控制相关
// ============================================================

/**
 * 当前是否在记录中
 */
export function isRecording(): Promise<boolean> {
  return call<boolean>("is_recording");
}

/**
 * 切换记录状态（暂停/恢复）
 * @returns 切换后的状态
 */
export function toggleRecording(): Promise<boolean> {
  return call<boolean>("toggle_recording");
}

// ============================================================
// 窗口控制相关
// ============================================================

/**
 * 隐藏主窗口（收起面板到托盘）
 */
export function hideMainWindow(): Promise<void> {
  return call<void>("hide_main_window");
}

// ============================================================
// 分类相关
// ============================================================

/** 获取所有分类 */
export function getCategories(): Promise<Category[]> {
  return call<Category[]>("get_categories");
}

/** 获取今日分类统计 */
export function getTodayCategoryStats(): Promise<CategoryStat[]> {
  return call<CategoryStat[]>("get_today_category_stats");
}

/** 新增分类 */
export function createCategory(params: {
  name: string;
  color: string;
  icon?: string;
  isFocus?: boolean;
}): Promise<number> {
  return call<number>("create_category", params);
}

/** 更新分类 */
export function updateCategory(params: {
  id: number;
  name: string;
  color: string;
  icon?: string;
  isFocus?: boolean;
}): Promise<void> {
  return call<void>("update_category", params);
}

/** 删除分类 */
export function deleteCategory(id: number): Promise<void> {
  return call<void>("delete_category", { id });
}

// ============================================================
// 规则相关
// ============================================================

/** 获取所有规则 */
export function getRules(): Promise<AppRule[]> {
  return call<AppRule[]>("get_rules");
}

/** 新增规则 */
export function createRule(rule: NewAppRule): Promise<number> {
  return call<number>("create_rule", { rule });
}

/** 更新规则 */
export function updateRule(id: number, rule: NewAppRule): Promise<void> {
  return call<void>("update_rule", { id, rule });
}

/** 删除规则 */
export function deleteRule(id: number): Promise<void> {
  return call<void>("delete_rule", { id });
}

/** 切换规则启用状态 */
export function toggleRule(id: number): Promise<boolean> {
  return call<boolean>("toggle_rule", { id });
}

/** 刷新规则缓存 */
export function refreshRules(): Promise<void> {
  return call<void>("refresh_rules");
}

/** 重排规则顺序 */
export function reorderRules(orderedIds: number[]): Promise<void> {
  return call<void>("reorder_rules", { orderedIds });
}

// ============================================================
// 目标预算相关
// ============================================================

/** 获取所有目标 */
export function getGoals(): Promise<Goal[]> {
  return call<Goal[]>("get_goals");
}

/** 获取目标状态（含今日已用时长） */
export function getGoalsStatus(): Promise<GoalStatus[]> {
  return call<GoalStatus[]>("get_goals_status");
}

/** 新增目标 */
export function createGoal(goal: NewGoal): Promise<number> {
  return call<number>("create_goal", { goal });
}

/** 更新目标 */
export function updateGoal(id: number, goal: NewGoal): Promise<void> {
  return call<void>("update_goal", { id, goal });
}

/** 删除目标 */
export function deleteGoal(id: number): Promise<void> {
  return call<void>("delete_goal", { id });
}

// ============================================================
// 番茄钟相关
// ============================================================

/** 获取番茄钟状态 */
export function getPomodoroStatus(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("get_pomodoro_status");
}

/** 获取番茄钟设置 */
export function getPomodoroSettings(): Promise<PomodoroSettings> {
  return call<PomodoroSettings>("get_pomodoro_settings");
}

/** 更新番茄钟设置 */
export function updatePomodoroSettings(settings: PomodoroSettings): Promise<void> {
  return call<void>("update_pomodoro_settings", { settings });
}

/** 获取常规设置 */
export function getGeneralSettings(): Promise<GeneralSettings> {
  return call<GeneralSettings>("get_general_settings");
}

/** 更新常规设置 */
export function updateGeneralSettings(settings: GeneralSettings): Promise<void> {
  return call<void>("update_general_settings", { settings });
}

/** 获取应用版本号 */
export function getAppVersion(): Promise<string> {
  return call<string>("get_app_version");
}

/** 检查更新：Rust 端依次尝试 Gitee / jsDelivr / GitHub API，返回最新版本信息 */
export function checkUpdateInfo(): Promise<UpdateInfo> {
  return call<UpdateInfo>("check_update");
}

/** 开始专注 */
export function startPomodoroFocus(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("start_pomodoro_focus");
}

/** 开始休息 */
export function startPomodoroBreak(long?: boolean): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("start_pomodoro_break", { long });
}

/** 停止番茄钟 */
export function stopPomodoro(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("stop_pomodoro");
}

/** 暂停番茄钟 */
export function pausePomodoro(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("pause_pomodoro");
}

/** 继续番茄钟 */
export function resumePomodoro(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("resume_pomodoro");
}

/** 跳过当前阶段 */
export function skipPomodoro(): Promise<PomodoroStatus> {
  return call<PomodoroStatus>("skip_pomodoro");
}
