import { invoke } from "@tauri-apps/api/core";

/**
 * 封装 Tauri invoke 调用，统一错误处理
 */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    console.error(`[IPC Error] ${command}:`, err);
    throw err;
  }
}

// ---- 类型定义 ----

export interface AppStat {
  processName: string;
  totalSeconds: number;
  percentage: number;
}

export interface TodayTotal {
  totalSeconds: number;
  activeSeconds: number;
  idleSeconds: number;
}

export interface CurrentActivity {
  processName: string;
  windowTitle: string;
  startTime: number;
  duration: number;
  isIdle: boolean;
}

// ---- API 封装 ----

export function getTodayStats(): Promise<AppStat[]> {
  return call<AppStat[]>("get_today_stats");
}

export function getTodayTotal(): Promise<TodayTotal> {
  return call<TodayTotal>("get_today_total");
}

export function getCurrentActivity(): Promise<CurrentActivity | null> {
  return call<CurrentActivity | null>("get_current_activity");
}

export function isRecording(): Promise<boolean> {
  return call<boolean>("is_recording");
}

export function toggleRecording(): Promise<boolean> {
  return call<boolean>("toggle_recording");
}
