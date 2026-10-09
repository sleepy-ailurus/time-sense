<script setup lang="ts">
import { onBeforeUnmount, watch } from "vue";
import { useI18n } from "vue-i18n";
import { AlertTriangle } from "lucide-vue-next";
import { confirmState, resolveConfirm } from "../composables/useConfirm";

const { t } = useI18n();

function onKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  // 只关弹窗，不让 Esc 继续冒泡到「收起面板」的全局快捷键
  e.stopPropagation();
  resolveConfirm(false);
}

watch(
  () => confirmState.show,
  (show) => {
    if (show) document.addEventListener("keydown", onKeydown);
    else document.removeEventListener("keydown", onKeydown);
  },
);

onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition name="gc-fade">
      <div v-if="confirmState.show" class="gc-overlay">
        <div class="gc-box">
          <div class="gc-title">
            <span class="gc-icon" :class="{ danger: confirmState.danger }">
              <AlertTriangle :size="15" :stroke-width="2.2" />
            </span>
            <span>{{ confirmState.title || t('common.confirm') }}</span>
          </div>
          <p class="gc-text">{{ confirmState.message }}</p>
          <div class="gc-actions">
            <button class="btn-secondary" @click="resolveConfirm(false)">
              {{ confirmState.cancelText || t('common.cancel') }}
            </button>
            <button
              :class="confirmState.danger ? 'btn-danger' : 'btn-primary'"
              @click="resolveConfirm(true)"
            >
              {{ confirmState.confirmText || t('common.confirm') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.gc-overlay {
  position: fixed;
  inset: 0;
  z-index: 2600;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(8, 10, 18, 0.38);
  backdrop-filter: blur(8px) saturate(120%);
  -webkit-backdrop-filter: blur(8px) saturate(120%);
}

.gc-box {
  position: relative;
  width: 300px;
  padding: 22px 20px 18px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-glass-strong);
  background: var(--bg-glass);
  backdrop-filter: blur(60px) saturate(190%);
  -webkit-backdrop-filter: blur(60px) saturate(190%);
  box-shadow: var(--shadow-glass), 0 0 0 1px rgba(139, 92, 246, 0.08);
  overflow: hidden;
}

.gc-box::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.28) 50%,
    transparent 100%
  );
  pointer-events: none;
}

.gc-title {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  padding-bottom: 12px;
  margin-bottom: 12px;
  border-bottom: 1px solid var(--border-glass);
}

.gc-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 7px;
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.14);
  flex-shrink: 0;
}

.gc-icon.danger {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.14);
}

.gc-text {
  position: relative;
  z-index: 1;
  font-size: 13px;
  line-height: 1.6;
  color: var(--text-secondary);
  margin: 0;
}

.gc-actions {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
}

.btn-secondary {
  padding: 8px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-secondary:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-glass-strong);
  color: var(--text-primary);
}

.btn-danger {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #fff;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(239, 68, 68, 0.25);
  transition: all 0.15s ease;
}

.btn-danger:hover {
  box-shadow: 0 6px 18px rgba(239, 68, 68, 0.45);
  transform: translateY(-1px);
}

.btn-primary {
  padding: 8px 18px;
  border: none;
  border-radius: 8px;
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
  color: #fff;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(139, 92, 246, 0.3);
  transition: all 0.15s ease;
}

.btn-primary:hover {
  box-shadow: 0 6px 18px rgba(139, 92, 246, 0.5);
  transform: translateY(-1px);
}

.gc-fade-enter-active,
.gc-fade-leave-active {
  transition: opacity 0.16s ease;
}

.gc-fade-enter-active .gc-box,
.gc-fade-leave-active .gc-box {
  transition: transform 0.16s ease;
}

.gc-fade-enter-from,
.gc-fade-leave-to {
  opacity: 0;
}

.gc-fade-enter-from .gc-box,
.gc-fade-leave-to .gc-box {
  transform: translateY(6px) scale(0.98);
}
</style>
