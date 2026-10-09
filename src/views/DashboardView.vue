<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, defineAsyncComponent, markRaw, watch } from "vue";
import { useI18n } from "vue-i18n";
import { formatDuration } from "../utils/format";
import { categoryLabel } from "../utils/categoryName";
import {
  anchorTimeline,
  createTimeline,
  elapsedAt,
  freezeTimeline,
  formatRemaining,
  msUntilNextTick,
  remainingMs,
} from "../utils/pomodoroTimer";
import { useActivity } from "../composables/useActivity";

const { t } = useI18n();
import { listen } from "@tauri-apps/api/event";
import {
  getTodayCategoryStats,
  getPomodoroStatus,
  startPomodoroFocus,
  stopPomodoro,
  pausePomodoro,
  resumePomodoro,
  skipPomodoro,
  getGoalsStatus,
} from "../api";
import type { CategoryStat, PomodoroStatus, GoalStatus } from "../api/types";
import {
  Timer,
  Clock,
  Monitor,
  TrendingUp,
  Briefcase,
  BookOpen,
  Gamepad2,
  MessageCircle,
  MoreHorizontal,
  Folder,
} from "lucide-vue-next";

// Lucide 图标映射（用于从后端返回的图标名动态渲染）
const iconMap: Record<string, any> = {
  Briefcase: markRaw(Briefcase),
  BookOpen: markRaw(BookOpen),
  Gamepad2: markRaw(Gamepad2),
  MessageCircle: markRaw(MessageCircle),
  MoreHorizontal: markRaw(MoreHorizontal),
  Folder: markRaw(Folder),
};

function getCategoryIcon(name?: string | null) {
  return iconMap[name || ""] || Folder;
}

const {
  todayTotal,
  todayStats,
  currentActivity,
  fetchTodayStats,
  fetchCurrentActivity,
} = useActivity();

const categoryStats = ref<CategoryStat[]>([]);
const pomodoroStatus = ref<PomodoroStatus | null>(null);
const goalsStatus = ref<GoalStatus[]>([]);
const refreshTimer = ref<number | null>(null);
const isRefreshing = ref(false);

// 本地秒级计时器，驱动「当前活动时长」实时刷新（不依赖后端轮询）
const tickNow = ref(Date.now());
let tickTimer: number | null = null;

// 番茄钟计时时间轴：锚点(已用毫秒 @ 本地时刻) + 真实流逝时间。
// 只在这里维护，显示层统一向上取整，避免暂停/恢复出现 ±1 秒
const pomodoroTimeline = createTimeline();
// 番茄钟自己的时钟采样点：由对齐到秒边界的定时器驱动
const pomodoroTickNow = ref(Date.now());
let pomodoroTickTimer: number | null = null;

function getPomodoroTargetMs(): number {
  return (pomodoroStatus.value?.targetSeconds || 0) * 1000;
}

/** 当前已用毫秒（暂停时保持冻结值不变） */
function getPomodoroElapsedMs(): number {
  const s = pomodoroStatus.value;
  if (!s?.isRunning) return 0;
  return elapsedAt(pomodoroTimeline, pomodoroTickNow.value, s.isPaused);
}

const pomodoroProgress = computed(() => {
  if (!pomodoroStatus.value || !pomodoroStatus.value.targetSeconds) return 0;
  const target = pomodoroStatus.value.targetSeconds;
  const elapsed = getPomodoroElapsedMs() / 1000;
  return Math.min((elapsed / target) * 100, 100);
});

const pomodoroTimeDisplay = computed(() => {
  if (!pomodoroStatus.value) return "00:00";
  const s = pomodoroStatus.value;
  if (!s.isRunning) {
    // 空闲：显示设置的目标时长
    return formatRemaining((s.targetSeconds || 0) * 1000);
  }
  return formatRemaining(remainingMs(getPomodoroElapsedMs(), getPomodoroTargetMs()));
});

const pomodoroPhaseLabel = computed(() => {
  if (!pomodoroStatus.value?.isRunning) return t('dashboard.notStarted');
  switch (pomodoroStatus.value.sessionType) {
    case "focus":
      return t('dashboard.focusing');
    case "short_break":
      return t('dashboard.shortBreak');
    case "long_break":
      return t('dashboard.longBreak');
    default:
      return "";
  }
});

const pomodoroPhaseColor = computed(() => {
  if (!pomodoroStatus.value?.isRunning) return "#6B7280";
  switch (pomodoroStatus.value.sessionType) {
    case "focus":
      return "#8B5CF6";
    case "short_break":
      return "#10B981";
    case "long_break":
      return "#3B82F6";
    default:
      return "#6B7280";
  }
});

// 当前活动计时基准
let activityBaselineAt = 0;
let activityBaselineDuration = 0;

function updateActivityBaseline() {
  const act = currentActivity.value;
  if (!act) {
    activityBaselineAt = 0;
    activityBaselineDuration = 0;
    return;
  }
  activityBaselineAt = Date.now();
  activityBaselineDuration = act.duration || 0;
}

// 当前活动实时时长（以基准为锚点，本地累加）
// 空闲段在后端同样是持续增长的记录段，因此空闲时也要继续累加，
// 否则「空闲中」的时长会冻结在进入空闲瞬间的值
const currentDurationLive = computed(() => {
  const act = currentActivity.value;
  if (!act) return 0;
  const elapsedLocal = Math.floor((tickNow.value - activityBaselineAt) / 1000);
  return activityBaselineDuration + elapsedLocal;
});

// 当前空闲会话实时时长（从进入空闲状态开始算）
const idleDurationLive = computed(() => {
  const act = currentActivity.value;
  if (!act || !act.isIdle) return 0;
  const elapsedLocal = Math.floor((tickNow.value - activityBaselineAt) / 1000);
  return activityBaselineDuration + elapsedLocal;
});

// 窗口标题：只有「比应用名多出信息」时才作为副标题展示。
// 例如 chrome.exe → 「某页面 - Google Chrome」要显示；
// timeSense.exe → 「TimeSense」和进程名重复，就不显示（避免重复一行）。
const currentWindowTitle = computed(() => {
  const act = currentActivity.value;
  if (!act) return "";

  const title = (act.windowTitle || "").trim();
  if (!title || title === "—" || title === "-") return "";

  const normalize = (s: string) => s.replace(/\s+/g, "").toLowerCase();
  const compactTitle = normalize(title);
  const names = [act.siteLabel || "", act.processName || ""]
    .map((n) => normalize(n.replace(/\.exe$/i, "")))
    .filter(Boolean);

  // 标题去空格后与应用名（或去掉 .exe 的进程名）完全相同 → 没有额外信息
  if (names.includes(compactTitle)) return "";

  return title;
});

// 番茄钟状态变化时，更新本地计时基准
// - 开始/停止/阶段切换：用后端的已用毫秒重新锚定
// - 暂停：冻结在「此刻屏幕上显示的那个数字」上，之后不再前进
// - 恢复：从冻结值继续，一秒不多一秒不少
// - 普通轮询：不重新锚定（本地时钟同一台机器，不会累积漂移）
let lastPomodoroPhase = ""; // 追踪核心状态：running + sessionType + target
let lastWasPaused = false;

function getPomodoroPhaseKey(s: PomodoroStatus | null): string {
  if (!s) return "null";
  return `${s.isRunning}-${s.sessionType || "idle"}-${s.targetSeconds}`;
}

watch(pomodoroStatus, (newStatus) => {
  const newPhase = getPomodoroPhaseKey(newStatus);
  const newIsPaused = newStatus?.isPaused ?? false;
  const nowMs = Date.now();

  if (newPhase !== lastPomodoroPhase) {
    // 核心状态变了（开始/停止/阶段切换）→ 用后端的已用毫秒重新锚定
    anchorTimeline(pomodoroTimeline, statusElapsedMs(newStatus), nowMs);
  } else if (newIsPaused && !lastWasPaused) {
    // 刚进入暂停 → 冻结当前时刻的已用时长（与屏幕上的数字一致）
    freezeTimeline(pomodoroTimeline, nowMs);
  } else if (!newIsPaused && lastWasPaused && newStatus?.isRunning) {
    // 刚恢复 → 以冻结值为锚点继续
    anchorTimeline(pomodoroTimeline, pomodoroTimeline.frozenMs, nowMs);
  }

  lastPomodoroPhase = newPhase;
  lastWasPaused = newIsPaused;
  // 立即用新锚点刷新一次，并把下一次刷新对齐到秒边界
  pomodoroTickNow.value = nowMs;
  schedulePomodoroTick();
});

/** 后端状态里的已用毫秒（兼容旧版本后端只返回秒） */
function statusElapsedMs(status: PomodoroStatus | null): number {
  if (!status) return 0;
  if (typeof status.elapsedMs === "number") return status.elapsedMs;
  return (status.elapsedSeconds || 0) * 1000;
}

/**
 * 把刷新定时器对齐到「显示数字下一次变化」的时刻。
 * 暂停/空闲时不需要刷新；归零后等后端的阶段切换事件。
 */
function schedulePomodoroTick() {
  if (pomodoroTickTimer !== null) {
    clearTimeout(pomodoroTickTimer);
    pomodoroTickTimer = null;
  }
  const s = pomodoroStatus.value;
  if (!s?.isRunning || s.isPaused) return;

  const remainMs = remainingMs(
    elapsedAt(pomodoroTimeline, Date.now(), false),
    getPomodoroTargetMs(),
  );
  if (remainMs <= 0) return;

  // +5ms 余量，避免定时器略微提前导致这一拍没变化
  pomodoroTickTimer = window.setTimeout(() => {
    pomodoroTickTimer = null;
    pomodoroTickNow.value = Date.now();
    schedulePomodoroTick();
  }, Math.max(20, msUntilNextTick(remainMs) + 5));
}

/** 窗口回到前台/重新可见时，两个计时器都立刻用真实时间校准一次 */
function handleTimerResync() {
  const nowMs = Date.now();
  tickNow.value = nowMs;
  pomodoroTickNow.value = nowMs;
  schedulePomodoroTick();
}

// 当前活动变化时，更新活动计时基准
// 同样：只在活动切换或空闲状态变化时重置，普通轮询不重置
let lastActivityFingerprint = "";

function getActivityFingerprint(act: any): string {
  if (!act) return "null";
  return `${act.processName}-${act.windowTitle}-${act.isIdle}`;
}

watch(currentActivity, () => {
  const newFp = getActivityFingerprint(currentActivity.value);
  if (newFp !== lastActivityFingerprint) {
    updateActivityBaseline();
    lastActivityFingerprint = newFp;
  }
}, { deep: true });

async function fetchCategoryStats() {
  try {
    categoryStats.value = await getTodayCategoryStats();
  } catch (e) {
    console.error("加载分类统计失败", e);
  }
}

async function fetchPomodoroStatus() {
  try {
    pomodoroStatus.value = await getPomodoroStatus();
  } catch (e) {
    console.error("加载番茄钟状态失败", e);
  }
}

async function fetchGoalsStatus() {
  try {
    goalsStatus.value = await getGoalsStatus();
  } catch (e) {
    console.error("加载目标状态失败", e);
  }
}

async function handleRefresh() {
  if (isRefreshing.value) return;
  isRefreshing.value = true;
  try {
    await Promise.all([
      fetchTodayStats(),
      fetchCurrentActivity(),
      fetchCategoryStats(),
      fetchPomodoroStatus(),
      fetchGoalsStatus(),
    ]);
  } finally {
    setTimeout(() => {
      isRefreshing.value = false;
    }, 300);
  }
}

async function runPomodoroAction(action: () => Promise<PomodoroStatus>) {
  try {
    pomodoroStatus.value = await action();
  } catch (e) {
    console.error("番茄钟操作失败", e);
  }
}

function handleStartPomodoro() {
  return runPomodoroAction(startPomodoroFocus);
}

function handlePausePomodoro() {
  return runPomodoroAction(pausePomodoro);
}

function handleResumePomodoro() {
  return runPomodoroAction(resumePomodoro);
}

function handleStopPomodoro() {
  return runPomodoroAction(stopPomodoro);
}

function handleSkipPomodoro() {
  return runPomodoroAction(skipPomodoro);
}

let unlistenPomodoro: (() => void) | null = null;
let pomodoroListenDisposed = false;
let unlistenActivity: (() => void) | null = null;
let activityListenDisposed = false;

onMounted(() => {
  fetchTodayStats();
  fetchCurrentActivity();
  fetchCategoryStats();
  fetchPomodoroStatus();
  fetchGoalsStatus();

  // 每 10 秒自动刷新后端数据
  refreshTimer.value = window.setInterval(() => {
    fetchTodayStats();
    fetchCurrentActivity();
    fetchCategoryStats();
    fetchPomodoroStatus();
    fetchGoalsStatus();
  }, 10000);

  // 本地秒级计时器，驱动「当前活动时长」实时刷新
  tickTimer = window.setInterval(() => {
    tickNow.value = Date.now();
  }, 1000);

  // 窗口重新可见/获得焦点时立即校准（后台节流会让定时器延迟）
  window.addEventListener("focus", handleTimerResync);
  document.addEventListener("visibilitychange", handleTimerResync);

  // 监听后端番茄钟状态变化（托盘操作 / 阶段切换时立即更新）
  listen<PomodoroStatus>("pomodoro://status-changed", (event) => {
    if (event.payload) {
      pomodoroStatus.value = event.payload;
    }
  }).then((unlisten) => {
    // 组件已卸载时立即注销，防止监听器泄漏
    if (pomodoroListenDisposed) {
      unlisten();
    } else {
      unlistenPomodoro = unlisten;
    }
  });

  // 当前活动切换（进入/离开空闲、换窗口）时立刻刷新，不必等 10 秒轮询
  listen("activity://changed", () => {
    fetchCurrentActivity();
  }).then((unlisten) => {
    if (activityListenDisposed) {
      unlisten();
    } else {
      unlistenActivity = unlisten;
    }
  });
});

onUnmounted(() => {
  pomodoroListenDisposed = true;
  unlistenPomodoro?.();
  activityListenDisposed = true;
  unlistenActivity?.();
  if (refreshTimer.value) {
    clearInterval(refreshTimer.value);
  }
  if (tickTimer) {
    clearInterval(tickTimer);
  }
  if (pomodoroTickTimer !== null) {
    clearTimeout(pomodoroTickTimer);
  }
  window.removeEventListener("focus", handleTimerResync);
  document.removeEventListener("visibilitychange", handleTimerResync);
});
</script>

<template>
  <div class="dashboard">
    <!-- 左栏 -->
    <div class="left-col">
      <!-- 总时长卡片 -->
      <div class="card total-card">
        <div class="card-label">{{ t('dashboard.todayActive') }}</div>
        <div class="total-time">
          <span class="hours">{{ Math.floor(todayTotal.activeSeconds / 3600) }}</span>
          <span class="unit">h</span>
          <span class="minutes">{{ Math.floor((todayTotal.activeSeconds % 3600) / 60) }}</span>
          <span class="unit">m</span>
        </div>
        <div class="total-sub">
          <span class="idle-time">{{ t('dashboard.idle') }} {{ formatDuration(todayTotal.idleSeconds) }}</span>
        </div>
        <div class="glow-orb"></div>
      </div>

      <!-- 番茄钟卡片 -->
      <div class="card pomodoro-card" :style="{ '--pom-color': pomodoroPhaseColor }">
        <div class="pomodoro-header">
          <Timer class="pomodoro-icon" :size="18" :stroke-width="1.8" />
          <span class="pomodoro-title">{{ t('dashboard.pomodoro') }}</span>
          <span class="pomodoro-phase" v-if="pomodoroStatus?.isRunning">
            {{ pomodoroPhaseLabel }}
          </span>
        </div>

        <div class="pomodoro-timer">
          <svg class="timer-ring" viewBox="0 0 120 120">
            <circle class="ring-bg" cx="60" cy="60" r="52" />
            <circle
              class="ring-progress"
              cx="60"
              cy="60"
              r="52"
              :stroke-dasharray="326.7"
              :stroke-dashoffset="326.7 - (326.7 * pomodoroProgress) / 100"
              :style="{ stroke: pomodoroPhaseColor }"
            />
          </svg>
          <div class="timer-text">
            <div class="timer-time">{{ pomodoroTimeDisplay }}</div>
            <div class="timer-count">
              {{ t('dashboard.focusCount', { count: pomodoroStatus?.todayFocusCount || 0 }) }}
            </div>
          </div>
        </div>

        <div class="pomodoro-actions">
          <button
            v-if="!pomodoroStatus?.isRunning"
            class="pom-btn start"
            @click="handleStartPomodoro"
          >
            {{ t('dashboard.startFocus') }}
          </button>
          <template v-else-if="pomodoroStatus?.isPaused">
            <button class="pom-btn resume" @click="handleResumePomodoro">{{ t('dashboard.resume') }}</button>
            <button class="pom-btn stop" @click="handleStopPomodoro">{{ t('dashboard.stop') }}</button>
          </template>
          <template v-else>
            <button class="pom-btn pause" @click="handlePausePomodoro">{{ t('dashboard.pause') }}</button>
            <button class="pom-btn skip" @click="handleSkipPomodoro">{{ t('dashboard.skip') }}</button>
            <button class="pom-btn stop" @click="handleStopPomodoro">{{ t('dashboard.stop') }}</button>
          </template>
        </div>
      </div>

      <!-- 当前活动 -->
      <div class="card current-card" v-if="currentActivity">
        <div class="current-label">
          <span class="dot" :class="{ idle: currentActivity.isIdle }"></span>
          {{ currentActivity.isIdle ? t('dashboard.idleStatus') : t('dashboard.currentActivity') }}
        </div>
        <!-- 应用名与它已持续的时间同一行（时长靠右）；标题只在有额外信息时作为副标题显示 -->
        <div class="current-head">
          <span class="current-name" :title="currentActivity.windowTitle || ''">
            {{ currentActivity.siteLabel || currentActivity.processName }}
          </span>
          <span class="current-duration">
            {{ formatDuration(currentDurationLive) }}
            <span v-if="currentActivity.categoryName" class="current-cat">
              · {{ categoryLabel(currentActivity.categoryName) }}
            </span>
          </span>
        </div>
        <div v-if="currentWindowTitle" class="current-title" :title="currentWindowTitle">
          {{ currentWindowTitle }}
        </div>
        <div v-if="currentActivity.isIdle" class="current-idle">
          {{ t('dashboard.idleFor', { duration: formatDuration(idleDurationLive) }) }}
        </div>
      </div>

      <!-- 目标预算 -->
      <div class="card goals-card" v-if="goalsStatus.length > 0">
        <div class="card-header">
          <span class="card-title">{{ t('dashboard.todayBudget') }}</span>
        </div>
        <div class="goals-list">
          <div
            v-for="gs in goalsStatus"
            :key="gs.goal.id"
            class="goal-item"
          >
            <div class="goal-info">
              <span class="goal-cat" :style="{ color: gs.goal.categoryColor || '#6B7280' }">
                {{ gs.goal.categoryName ? categoryLabel(gs.goal.categoryName) : t('common.unknown') }}
              </span>
              <span class="goal-usage" :class="{ exceeded: gs.exceeded }">
                {{ formatDuration(gs.usedSeconds) }} / {{ formatDuration(gs.limitSeconds) }}
              </span>
            </div>
            <div class="goal-bar">
              <div
                class="goal-fill"
                :style="{
                  width: Math.min((gs.usedSeconds / gs.limitSeconds) * 100, 100) + '%',
                  background: gs.exceeded
                    ? 'linear-gradient(90deg, #ef4444, #dc2626)'
                    : `linear-gradient(90deg, ${gs.goal.categoryColor || '#8B5CF6'}88, ${gs.goal.categoryColor || '#8B5CF6'})`,
                }"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 右栏 -->
    <div class="right-col">
      <!-- 应用排行 -->
      <div class="card apps-card">
        <div class="card-header">
          <span class="card-title">{{ t('dashboard.appRanking') }}</span>
          <button class="refresh-btn" @click="handleRefresh" :class="{ spinning: isRefreshing }">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path
                d="M11.6667 7.00033C11.6667 9.57775 9.57738 11.667 7.00004 11.667C4.42266 11.667 2.33337 9.57775 2.33337 7.00033C2.33337 4.42291 4.42266 2.33362 7.00004 2.33362C8.47626 2.33362 9.80455 3.0274 10.6807 4.12695M10.6667 1.16699V3.50033M10.6667 3.50033H8.33337"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>

        <div v-if="todayStats.length === 0" class="empty-state">
          <div class="empty-icon">⏱️</div>
          <div class="empty-text">{{ t('dashboard.noData') }}</div>
        </div>

        <div v-else class="app-list">
          <div
            v-for="(app, index) in todayStats"
            :key="app.processName"
            class="app-item"
          >
            <div class="app-rank">{{ index + 1 }}</div>
            <div class="app-info">
              <div class="app-name-row">
                <span class="app-name">{{ app.processName }}</span>
                <span class="app-duration">{{ formatDuration(app.totalSeconds) }}</span>
              </div>
              <div class="progress-bar">
                <div
                  class="progress-fill"
                  :style="{
                    width: app.percentage + '%',
                    background: app.categoryColor || 'linear-gradient(90deg, #8B5CF6, #EC4899)',
                  }"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 分类排行 -->
      <div class="card categories-card">
        <div class="card-header">
          <span class="card-title">{{ t('dashboard.categoryStats') }}</span>
        </div>

        <div v-if="categoryStats.length === 0" class="empty-state small">
          <div class="empty-text">{{ t('dashboard.noCategoryData') }}</div>
        </div>

        <div v-else class="category-mini-list">
          <div
            v-for="cat in categoryStats.slice(0, 5)"
            :key="cat.categoryId"
            class="cat-mini-item"
          >
            <component :is="getCategoryIcon(cat.categoryIcon)" class="cat-mini-icon" :size="14" :stroke-width="1.8" :style="{ color: cat.categoryColor }" />
            <span class="cat-mini-name">{{ categoryLabel(cat.categoryName) }}</span>
            <div class="cat-mini-bar">
              <div
                class="cat-mini-fill"
                :style="{
                  width: cat.percentage + '%',
                  background: cat.categoryColor,
                }"
              ></div>
            </div>
            <span class="cat-mini-time">{{ formatDuration(cat.totalSeconds) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  gap: 12px;
  height: 100%;
}

.left-col {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.right-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.card {
  background: rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(20px);
  border: 1px solid var(--border-glass);
  border-radius: 14px;
  padding: 14px;
  position: relative;
  overflow: hidden;
}

/* 应用排行卡片：最多显示 8 个，超出滚动 */
.apps-card {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

/* 分类统计卡片：固定在下面 */
.categories-card {
  flex-shrink: 0;
}

/* 总时长卡片 */
.total-card {
  text-align: center;
}

.card-label {
  font-size: 11px;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 8px;
}

.total-time {
  display: flex;
  align-items: baseline;
  justify-content: center;
  gap: 2px;
}

.hours {
  font-size: 42px;
  font-weight: 700;
  background: linear-gradient(135deg, #a78bfa, #f472b6);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  line-height: 1;
  text-shadow: 0 0 30px rgba(139, 92, 246, 0.5);
}

.minutes {
  font-size: 28px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1;
}

.unit {
  font-size: 14px;
  color: var(--text-tertiary);
  margin-right: 4px;
}

.total-sub {
  margin-top: 8px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.idle-time {
  /* 空闲时长：淡蓝色，与「今日活跃时长」的粉色主色区分开 */
  color: var(--idle-accent);
  opacity: 0.95;
}

.glow-orb {
  position: absolute;
  width: 120px;
  height: 120px;
  background: radial-gradient(circle, rgba(139, 92, 246, 0.25), transparent 70%);
  top: -30px;
  right: -30px;
  pointer-events: none;
}

/* 番茄钟卡片 */
.pomodoro-card {
  text-align: center;
}

.pomodoro-header {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin-bottom: 10px;
}

.pomodoro-icon {
  font-size: 16px;
}

.pomodoro-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.pomodoro-phase {
  font-size: 10px;
  padding: 2px 6px;
  background: var(--pom-color);
  color: white;
  border-radius: 4px;
  font-weight: 500;
}

.pomodoro-timer {
  position: relative;
  width: 120px;
  height: 120px;
  margin: 0 auto 12px;
}

.timer-ring {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

.ring-bg {
  fill: none;
  stroke: rgba(255, 255, 255, 0.08);
  stroke-width: 6;
}

.ring-progress {
  fill: none;
  stroke-width: 6;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.5s ease;
  filter: drop-shadow(0 0 6px currentColor);
}

.timer-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
}

.timer-time {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: "SF Mono", Menlo, monospace;
}

.timer-count {
  font-size: 10px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.pomodoro-actions {
  display: flex;
  gap: 6px;
}

.pom-btn {
  flex: 1;
  padding: 7px 0;
  border: 1px solid var(--border-glass);
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-secondary);
  border-radius: 8px;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.pom-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}

.pom-btn.start {
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
  color: white;
  border: none;
}

.pom-btn.start:hover {
  box-shadow: 0 2px 8px rgba(139, 92, 246, 0.4);
}

.pom-btn.pause {
  background: rgba(245, 158, 11, 0.15);
  color: #fbbf24;
  border: 1px solid rgba(245, 158, 11, 0.3);
}

.pom-btn.pause:hover {
  background: rgba(245, 158, 11, 0.25);
}

.pom-btn.resume {
  background: linear-gradient(135deg, #10b981, #059669);
  color: white;
  border: none;
}

.pom-btn.resume:hover {
  box-shadow: 0 2px 8px rgba(16, 185, 129, 0.4);
}

/* 当前活动卡片 */
.current-card {
  font-size: 12px;
}

.current-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-tertiary);
  margin-bottom: 6px;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #10b981;
  animation: pulse 2s infinite;
}

.dot.idle {
  background: #f59e0b;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.current-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.current-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.current-duration {
  flex-shrink: 0;
  white-space: nowrap;
  font-size: 11px;
  color: var(--text-secondary);
}

.current-title {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.current-cat {
  color: var(--text-tertiary);
}

.current-idle {
  font-size: 11px;
  color: #f59e0b;
  margin-top: 4px;
}

/* 目标预算卡片 */
.goals-card {
  font-size: 12px;
}

.goals-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.goal-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.goal-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.goal-cat {
  font-size: 12px;
  font-weight: 500;
}

.goal-usage {
  font-size: 10px;
  font-family: monospace;
  color: var(--text-tertiary);
}

.goal-usage.exceeded {
  color: #ef4444;
  font-weight: 600;
}

.goal-bar {
  height: 4px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
}

.goal-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s ease;
}

/* 应用排行 */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.card-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.refresh-btn {
  width: 26px;
  height: 26px;
  border: 1px solid var(--border-glass);
  background: transparent;
  color: var(--text-secondary);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.refresh-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.refresh-btn.spinning svg {
  animation: spin 0.6s linear;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.empty-state {
  text-align: center;
  padding: 24px 0;
  color: var(--text-tertiary);
}

.empty-state.small {
  padding: 14px 0;
}

.empty-icon {
  font-size: 28px;
  margin-bottom: 6px;
  opacity: 0.5;
}

.empty-text {
  font-size: 12px;
  color: var(--text-tertiary);
}

.app-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 280px;
  overflow-y: auto;
  padding-right: 4px;
  margin-right: -4px;
}

.app-list::-webkit-scrollbar {
  width: 4px;
}

.app-list::-webkit-scrollbar-track {
  background: transparent;
}

.app-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}

.app-list::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.25);
}

.app-item {
  display: flex;
  align-items: center;
  gap: 10px;
}

.app-rank {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 600;
  color: var(--text-tertiary);
  background: rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  flex-shrink: 0;
}

.app-item:nth-child(1) .app-rank {
  background: linear-gradient(135deg, #f59e0b, #d97706);
  color: white;
}
.app-item:nth-child(2) .app-rank {
  background: linear-gradient(135deg, #9ca3af, #6b7280);
  color: white;
}
.app-item:nth-child(3) .app-rank {
  background: linear-gradient(135deg, #b45309, #92400e);
  color: white;
}

.app-info {
  flex: 1;
  min-width: 0;
}

.app-name-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.app-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 60%;
}

.app-duration {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
  flex-shrink: 0;
}

.progress-bar {
  height: 4px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s ease;
}

/* 分类排行 */
.category-mini-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cat-mini-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}

.cat-mini-icon {
  font-size: 14px;
  flex-shrink: 0;
}

.cat-mini-name {
  color: var(--text-secondary);
  width: 50px;
  flex-shrink: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cat-mini-bar {
  flex: 1;
  height: 4px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
}

.cat-mini-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.5s ease;
}

.cat-mini-time {
  color: var(--text-tertiary);
  font-family: monospace;
  font-size: 10px;
  flex-shrink: 0;
  min-width: 35px;
  text-align: right;
}
</style>
