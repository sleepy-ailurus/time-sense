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
  categoryId?: number | null;
  /** 站点标签（title 规则命中时记录，如「抖音」） */
  siteLabel?: string | null;
}

/**
 * 应用统计
 */
export interface AppStat {
  processName: string;
  totalSeconds: number;
  percentage: number;
  categoryId?: number | null;
  categoryName?: string | null;
  categoryColor?: string | null;
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
 * 更新信息（来自 latest.json / GitHub API）
 */
export interface UpdateInfo {
  version: string;
  url: string;
  notes?: string;
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
  categoryId?: number | null;
  categoryName?: string | null;
  /** 站点标签（当前活动命中 title 规则时的展示名，如「B站」） */
  siteLabel?: string | null;
}

// ==================== 分类 ====================

export interface Category {
  id: number;
  name: string;
  color: string;
  icon?: string | null;
  sortOrder: number;
  isDefault: boolean;
  /** 是否计入「专注时长」（热力图 / 连续天数） */
  isFocus?: boolean;
}

export interface CategoryStat {
  categoryId: number;
  categoryName: string;
  categoryColor: string;
  categoryIcon?: string | null;
  totalSeconds: number;
  percentage: number;
}

// ==================== 规则 ====================

export type MatchType = "process" | "title";
export type MatchMode = "exact" | "contains" | "regex";

export interface AppRule {
  id: number;
  categoryId: number;
  categoryName?: string | null;
  matchType: MatchType;
  matchValue: string;
  matchMode: MatchMode;
  /** 显示名：title 规则命中时统计展示用（如 bilibili → B站） */
  label?: string | null;
  sortOrder: number;
  enabled: boolean;
}

export interface NewAppRule {
  categoryId: number;
  matchType: MatchType;
  matchValue: string;
  matchMode: MatchMode;
  /** 显示名（可选）：title 规则命中时统计展示用；传 null/省略表示不聚合 */
  label?: string | null;
}

// ==================== 统计 ====================

/**
 * 每日汇总
 */
export interface DailySummary {
  date: string
  totalSeconds: number
  idleSeconds: number
  workSeconds: number
  studySeconds: number
  entertainmentSeconds: number
  socialSeconds: number
  otherSeconds: number
  /** 各分类明细（含用户自建分类；未分类并入「其他」） */
  categorySeconds?: CategorySlice[]
  pomodoroCount: number
  pomodoroSeconds: number
}

/** 单日某个分类的活跃时长 */
export interface CategorySlice {
  categoryId: number
  categoryName: string
  categoryColor: string
  seconds: number
}

/**
 * 热力图单日数据
 */
export interface HeatmapDay {
  date: string
  totalSeconds: number
  pomodoroCount: number
}

/**
 * 时段分布统计
 */
export interface HourlyStat {
  hour: number
  totalSeconds: number
  weekday: number  // 0=周一, 6=周日
  date: string
}

// ==================== 目标预算 ====================

export interface Goal {
  id: number;
  categoryId: number;
  categoryName?: string | null;
  categoryColor?: string | null;
  dailyLimitMinutes: number;
  enabled: boolean;
}

export interface NewGoal {
  categoryId: number;
  dailyLimitMinutes: number;
  enabled?: boolean;
}

export interface GoalStatus {
  goal: Goal;
  usedSeconds: number;
  limitSeconds: number;
  exceeded: boolean;
}

// ==================== 番茄钟 ====================

export type PomodoroSessionType = "focus" | "short_break" | "long_break";

export interface PomodoroStatus {
  isRunning: boolean;
  isPaused: boolean;
  sessionType?: PomodoroSessionType | null;
  startTime?: number | null;
  /** 已用毫秒（毫秒精度，暂停时冻结） */
  elapsedMs?: number;
  /** 剩余毫秒（毫秒精度） */
  remainingMs?: number;
  /** 后端生成该状态时的时间戳（毫秒） */
  serverTimeMs?: number;
  elapsedSeconds: number;
  targetSeconds: number;
  remainingSeconds: number;
  todayFocusCount: number;
  todayFocusSeconds: number;
}

export interface PomodoroSettings {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  longBreakInterval: number;
  autoStartBreak: boolean;
  autoStartFocus: boolean;
  autoMode: boolean;
  enabled: boolean;
}

export interface GeneralSettings {
  autoStart: boolean;
  notificationEnabled: boolean;
  idleThresholdMinutes: number;
  /** 全局快捷键：显示/隐藏主面板（空字符串表示不注册） */
  shortcutToggleWindow?: string;
  /** 全局快捷键：开始/暂停番茄钟（空字符串表示不注册） */
  shortcutTogglePomodoro?: string;
}
