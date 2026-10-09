<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, markRaw } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { hideMainWindow } from "./api";
import { refreshGlobalShortcuts } from "./composables/useGlobalShortcuts";
import { LayoutDashboard, PieChart, Filter, Settings, ChevronDown, Activity, BarChart3, Grid3x3 } from "lucide-vue-next";
import { NConfigProvider, darkTheme } from "naive-ui";
import GlassConfirm from "./components/GlassConfirm.vue";

const route = useRoute();
const router = useRouter();
const win = getCurrentWindow();
const { t } = useI18n();
let unlistenMoved: (() => void) | null = null;

const navItems = computed(() => [
  { path: "/", title: t('nav.overview'), icon: markRaw(LayoutDashboard) },
  { path: "/categories", title: t('nav.category'), icon: markRaw(PieChart) },
  { path: "/timeline", title: t('nav.timeline'), icon: markRaw(Activity) },
  { path: "/statistics", title: t('nav.statistics'), icon: markRaw(BarChart3) },
  { path: "/heatmap", title: t('nav.heatmap'), icon: markRaw(Grid3x3) },
  { path: "/rules", title: t('nav.rules'), icon: markRaw(Filter) },
  { path: "/settings", title: t('nav.settings'), icon: markRaw(Settings) },
]);

function navigate(path: string) {
  void router.push(path);
}

// 位置记忆：保存窗口位置到 localStorage
function saveWindowPosition() {
  win.innerPosition().then((pos) => {
    localStorage.setItem("timesense_window_x", String(pos.x));
    localStorage.setItem("timesense_window_y", String(pos.y));
  }).catch(() => {
    // 忽略获取位置失败
  });
}

// 位置记忆：从 localStorage 还原窗口位置
async function restoreWindowPosition() {
  const x = localStorage.getItem("timesense_window_x");
  const y = localStorage.getItem("timesense_window_y");
  if (x && y) {
    try {
      await win.setPosition(new LogicalPosition(Number(x), Number(y)));
    } catch {
      // 位置无效时忽略（比如屏幕分辨率变了）
    }
  }
}

// Esc 键隐藏面板
function handleKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  // 有弹窗打开时（新增/编辑/确认），Esc 不收起整个面板，避免弹窗还没处理完窗口就没了
  if (document.querySelector(".modal-overlay, .gc-overlay")) return;
  hideMainWindow();
}

onMounted(async () => {
  // 恢复窗口位置
  await restoreWindowPosition();

  // 普通启动：显示主窗口；开机自启（--autostart）：静默驻留托盘
  try {
    const shouldShow = await invoke<boolean>("should_show_window_on_start");
    if (shouldShow) {
      await win.show();
    }
  } catch {
    // 查询失败时保持隐藏，仍可从托盘唤出
  }

  // 监听窗口移动，保存位置
  try {
    unlistenMoved = await win.onMoved(() => {
      saveWindowPosition();
    });
  } catch {
    // 监听移动事件失败，不影响主功能
  }

  // 注意：这里**不做**「失去焦点自动隐藏」。
  // 任务栏点击/右键会让窗口短暂失焦，一旦失焦就 hide()，
  // 任务栏上的程序图标会跟着消失（窗口既看不见、也无法从任务栏唤回）。
  // 现在只保留显式关闭入口：Esc、标题栏「收起面板」、托盘图标/菜单、Alt+F4（关闭即收进托盘）。

  // Esc 键监听
  window.addEventListener("keydown", handleKeydown);

  // 注册全局快捷键（默认 Ctrl+Shift+T 显示/隐藏主面板、Ctrl+Shift+P 开始/暂停番茄钟，
  // 可在「设置 → 常规 → 快捷键」里修改）
  const failed = await refreshGlobalShortcuts();
  if (failed.length === 0) {
    console.log("Global shortcuts registered");
  } else {
    console.warn("部分全局快捷键注册失败（可能被其它软件占用）：", failed.join(", "));
  }
});

onBeforeUnmount(() => {
  unlistenMoved?.();
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div class="app-shell">
    <div class="glass-body">
      <!-- 标题栏 -->
      <div class="title-bar">
        <div class="drag-region" data-tauri-drag-region>
          <span class="app-title">TimeSense</span>
          <span class="page-title">{{ route.meta.title ? t(route.meta.title) : "" }}</span>
        </div>
        <div class="window-controls">
          <button class="win-btn collapse" @click.stop="hideMainWindow" :title="t('common.collapse')">
            <ChevronDown :size="16" :stroke-width="2.2" />
          </button>
        </div>
      </div>

      <!-- 主体 -->
      <div class="main-layout">
        <!-- 侧边导航 -->
        <nav class="sidebar">
          <div
            v-for="item in navItems"
            :key="item.path"
            class="nav-item"
            :class="{ active: route.path === item.path }"
            @click="navigate(item.path)"
          >
            <component :is="item.icon" class="nav-icon" :size="20" :stroke-width="1.8" />
            <span class="nav-label">{{ item.title }}</span>
          </div>
        </nav>

        <!-- 内容区 -->
        <div class="content-area">
          <n-config-provider :theme="darkTheme">
            <router-view />
          </n-config-provider>
        </div>
      </div>
    </div>
    <!-- 全局确认弹窗（替代浏览器原生 confirm） -->
    <GlassConfirm />
  </div>
</template>

<style scoped>
.app-shell {
  width: 100%;
  height: 100vh;
  padding: 12px;
  background: transparent;
}

.glass-body {
  width: 100%;
  height: 100%;
  background: var(--bg-glass);
  backdrop-filter: blur(50px) saturate(200%);
  -webkit-backdrop-filter: blur(50px) saturate(200%);
  border: 1px solid var(--border-glass-strong);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-glass);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
}

.glass-body::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.2) 50%,
    transparent 100%
  );
  pointer-events: none;
  z-index: 10;
}

.title-bar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--border-glass);
  position: relative;
}

.drag-region {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 16px;
  -webkit-app-region: drag;
  user-select: none;
}

.app-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.page-title {
  font-size: 12px;
  color: var(--text-tertiary);
  padding-left: 10px;
  border-left: 1px solid var(--border-glass);
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px 0 6px;
  flex-shrink: 0;
  -webkit-app-region: no-drag;
  z-index: 5;
}

.win-btn {
  width: 32px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  pointer-events: auto;
}

.win-btn:hover {
  background: rgba(255, 255, 255, 0.12);
  color: var(--text-primary);
}

.main-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.sidebar {
  width: 110px;
  flex-shrink: 0;
  padding: 12px 8px;
  border-right: 1px solid var(--border-glass);
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: rgba(255, 255, 255, 0.02);
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 6px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s ease;
  color: var(--text-secondary);
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-primary);
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.2), rgba(236, 72, 153, 0.15));
  color: var(--text-primary);
  border: 1px solid rgba(139, 92, 246, 0.3);
}

.nav-icon {
  font-size: 20px;
}

.nav-label {
  font-size: 11px;
  font-weight: 500;
}

.content-area {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
</style>
