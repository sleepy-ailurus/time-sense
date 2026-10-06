import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityLog,
  AppStat,
  CurrentActivity,
  TodayTotal,
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
