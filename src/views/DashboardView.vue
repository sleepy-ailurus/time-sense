<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { formatDuration } from "../utils/format";
import { useActivity } from "../composables/useActivity";

const {
  todayTotal,
  todayStats,
  currentActivity,
  fetchTodayStats,
  fetchCurrentActivity,
} = useActivity();

const refreshTimer = ref<number | null>(null);

onMounted(() => {
  fetchTodayStats();
  fetchCurrentActivity();
  refreshTimer.value = window.setInterval(() => {
    fetchTodayStats();
    fetchCurrentActivity();
  }, 10000);
});

onUnmounted(() => {
  if (refreshTimer.value) {
    clearInterval(refreshTimer.value);
  }
});
</script>

<template>
  <div class="dashboard">
    <!-- 左侧栏 -->
    <div class="left-panel">
      <!-- 今日总时长卡片 -->
      <div class="glass-card total-card">
        <div class="total-glow"></div>
        <div class="total-number">
          <span class="num">{{ Math.floor(todayTotal.activeSeconds / 3600) }}</span>
          <span class="unit">h</span>
          <span class="num">{{ Math.floor((todayTotal.activeSeconds % 3600) / 60) }}</span>
          <span class="unit">m</span>
        </div>
        <div class="total-label">今日活跃时长</div>
        <div class="idle-info">
          空闲 {{ formatDuration(todayTotal.idleSeconds) }}
        </div>
      </div>

      <!-- 当前活动卡片 -->
      <div v-if="currentActivity" class="glass-card current-card">
        <div class="current-label">当前活动</div>
        <div class="current-app">{{ currentActivity.processName }}</div>
        <div class="current-title">{{ currentActivity.windowTitle }}</div>
        <div class="current-duration">
          已持续 <span class="accent-text">{{ formatDuration(currentActivity.duration) }}</span>
        </div>
      </div>

      <div v-else class="glass-card current-card empty-current">
        <div class="current-label">当前活动</div>
        <div class="empty-text">暂无活动数据</div>
      </div>
    </div>

    <!-- 右侧栏：应用排行 -->
    <div class="right-panel glass-card rank-card">
      <div class="rank-header">
        <span class="rank-title">应用时长排行</span>
      </div>

      <div v-if="todayStats.length === 0" class="empty-state">
        <div class="empty-icon">⏱</div>
        <div class="empty-text">今天还没有数据</div>
        <div class="empty-sub">开始使用应用后这里会显示统计</div>
      </div>

      <div v-else class="app-list">
        <div
          v-for="(app, index) in todayStats.slice(0, 8)"
          :key="app.processName"
          class="app-item"
        >
          <div class="app-rank">
            <span
              class="rank-badge"
              :class="{ 'rank-top': index < 3 }"
            >{{ index + 1 }}</span>
          </div>
          <div class="app-info">
            <div class="app-name-row">
              <span class="app-name">{{ app.processName }}</span>
              <span class="app-time">{{ formatDuration(app.totalSeconds) }}</span>
            </div>
            <div class="progress-bar">
              <div
                class="progress-fill"
                :style="{ width: app.percentage + '%' }"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  width: 100%;
  height: 100%;
  display: flex;
  gap: 14px;
}

/* ===== 左侧栏 ===== */
.left-panel {
  width: 42%;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* ===== 总时长卡片 ===== */
.total-card {
  position: relative;
  padding: 24px 20px;
  text-align: center;
  overflow: hidden;
}

.total-glow {
  position: absolute;
  top: -40%;
  left: 50%;
  transform: translateX(-50%);
  width: 200px;
  height: 200px;
  background: radial-gradient(
    circle,
    var(--accent-glow) 0%,
    transparent 70%
  );
  pointer-events: none;
  filter: blur(20px);
}

.total-number {
  position: relative;
  display: flex;
  align-items: baseline;
  justify-content: center;
  gap: 2px;
  margin-bottom: 8px;
}

.total-number .num {
  font-size: 56px;
  font-weight: 700;
  line-height: 1.1;
  background: var(--accent-gradient);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  letter-spacing: -1px;
}

.total-number .unit {
  font-size: 24px;
  font-weight: 500;
  color: var(--accent-primary);
  margin: 0 2px;
}

.total-label {
  position: relative;
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.idle-info {
  position: relative;
  font-size: 12px;
  color: var(--text-tertiary);
}

/* ===== 当前活动卡片 ===== */
.current-card {
  padding: 16px 18px;
  flex: 1;
}

.current-label {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: 10px;
  letter-spacing: 0.5px;
}

.current-app {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.current-title {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.current-duration {
  font-size: 13px;
  color: var(--text-secondary);
}

.accent-text {
  color: var(--accent-primary);
  font-weight: 600;
  font-size: 15px;
}

.empty-current {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.empty-text {
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}

/* ===== 右侧栏 ===== */
.right-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 18px 20px;
  overflow: hidden;
}

.rank-header {
  margin-bottom: 16px;
  flex-shrink: 0;
}

.rank-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

/* 应用列表 */
.app-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-right: 4px;
}

.app-item {
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-rank {
  flex-shrink: 0;
}

.rank-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 500;
  border: 1px solid var(--border-glass);
}

.rank-badge.rank-top {
  background: rgba(233, 69, 96, 0.15);
  color: var(--accent-primary);
  border-color: rgba(233, 69, 96, 0.3);
}

.app-info {
  flex: 1;
  min-width: 0;
}

.app-name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.app-name {
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  margin-right: 8px;
}

.app-time {
  font-size: 12px;
  color: var(--text-secondary);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.progress-bar {
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent-gradient);
  border-radius: 2px;
  transition: width 0.3s ease;
}

/* 空状态 */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 32px;
  opacity: 0.5;
}

.empty-sub {
  font-size: 12px;
  opacity: 0.6;
}
</style>
