<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

async function hideToTray() {
  try {
    await invoke("hide_main_window");
  } catch (e) {
    console.error("hide failed:", e);
  }
}
</script>

<template>
  <div class="app-shell">
    <!-- 玻璃主体 -->
    <div class="glass-body">
      <!-- 标题栏（可拖拽） -->
      <div class="title-bar">
        <div class="drag-region" data-tauri-drag-region>
          <span class="app-title">TimeSense</span>
        </div>
        <div class="window-controls">
          <button class="win-btn collapse" @click.stop="hideToTray" title="收起面板">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path
                d="M3 5L7 9L11 5"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>
      </div>

      <!-- 内容区 -->
      <div class="content-area">
        <router-view />
      </div>
    </div>
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
  padding: 0 16px;
  -webkit-app-region: drag;
  app-region: drag;
  user-select: none;
}

.app-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px 0 6px;
  flex-shrink: 0;
  -webkit-app-region: no-drag;
  app-region: no-drag;
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

.content-area {
  flex: 1;
  overflow: hidden;
  padding: 16px;
}
</style>
