/**
 * 活动记录
 */
export interface ActivityLog {
  id?: number;
  processName: string;
  windowTitle?: string | null;
  startTime: number;
  endTime: number;
  duration: number;
  isIdle: boolean;
}

/**
 * 应用统计
 */
export interface AppStat {
  processName: string;
  totalSeconds: number;
  percentage: number;
}

/**
 * 今日总览
 */
export interface TodayTotal {
  totalSeconds: number;
  activeSeconds: number;
  idleSeconds: number;
}

/**
 * 当前活动状态
 */
export interface CurrentActivity {
  processName: string;
  windowTitle: string;
  startTime: number;
  duration: number;
  isIdle: boolean;
}
