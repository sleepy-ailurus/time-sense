<script setup lang="ts">
import { ref, onMounted, markRaw } from "vue";
import { getTodayCategoryStats } from "../api";
import type { CategoryStat } from "../api/types";
import { formatDuration } from "../utils/format";
import {
  BarChart3,
  Briefcase,
  BookOpen,
  Gamepad2,
  MessageCircle,
  MoreHorizontal,
  Folder,
} from "lucide-vue-next";

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

const categoryStats = ref<CategoryStat[]>([]);
const loading = ref(false);

async function loadData() {
  loading.value = true;
  try {
    categoryStats.value = await getTodayCategoryStats();
  } catch (e) {
    console.error("加载分类统计失败", e);
  } finally {
    loading.value = false;
  }
}

function getBarColor(stat: CategoryStat): string {
  return stat.categoryColor || "#6B7280";
}

onMounted(() => {
  loadData();
});
</script>

<template>
  <div class="category-page">
    <div class="page-header">
      <h2 class="page-title">今日分类统计</h2>
      <p class="page-subtitle">按类别查看你的时间分布</p>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else-if="categoryStats.length === 0" class="empty-state">
      <BarChart3 class="empty-icon" :size="48" :stroke-width="1.2" />
      <div class="empty-text">暂无数据</div>
      <div class="empty-desc">使用一会儿电脑后再来看看吧</div>
    </div>

    <div v-else class="category-list">
      <div
        v-for="(stat, index) in categoryStats"
        :key="stat.categoryId"
        class="category-item"
      >
        <div class="category-header">
          <div class="category-left">
            <span class="category-rank">{{ index + 1 }}</span>
            <component :is="getCategoryIcon(stat.categoryIcon)" class="category-icon" :size="18" :stroke-width="1.8" :style="{ color: getBarColor(stat) }" />
            <span class="category-name">{{ stat.categoryName }}</span>
          </div>
          <div class="category-right">
            <span class="category-duration">{{ formatDuration(stat.totalSeconds) }}</span>
            <span class="category-percent">{{ stat.percentage.toFixed(1) }}%</span>
          </div>
        </div>
        <div class="progress-bar">
          <div
            class="progress-fill"
            :style="{
              width: stat.percentage + '%',
              background: `linear-gradient(90deg, ${getBarColor(stat)}88, ${getBarColor(stat)})`,
              boxShadow: `0 0 10px ${getBarColor(stat)}44`,
            }"
          ></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.category-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  margin-bottom: 16px;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 4px 0;
}

.page-subtitle {
  font-size: 12px;
  color: var(--text-tertiary);
  margin: 0;
}

.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
  font-size: 13px;
}

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
  font-size: 40px;
  opacity: 0.5;
}

.empty-text {
  font-size: 14px;
  color: var(--text-secondary);
}

.empty-desc {
  font-size: 12px;
  color: var(--text-tertiary);
}

.category-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.category-item {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-glass);
  border-radius: 12px;
  padding: 12px 14px;
  transition: all 0.2s ease;
}

.category-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: var(--border-glass-strong);
}

.category-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.category-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.category-rank {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  background: rgba(255, 255, 255, 0.08);
  border-radius: 6px;
}

.category-icon {
  font-size: 18px;
}

.category-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.category-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.category-duration {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.category-percent {
  font-size: 11px;
  color: var(--text-tertiary);
  min-width: 40px;
  text-align: right;
}

.progress-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease;
}
</style>
