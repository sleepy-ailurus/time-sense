<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { Check, ChevronDown } from "lucide-vue-next";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    options: Array<{ value: string; label: string }>;
    minWidth?: number;
    /** 撑满父容器宽度（用于表单里替代原生 select） */
    block?: boolean;
  }>(),
  { minWidth: 96, block: false },
);

const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

const triggerRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const open = ref(false);
const menuStyle = ref<Record<string, string>>({});

const currentLabel = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? "",
);

function updatePosition() {
  const el = triggerRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const width = Math.max(rect.width, props.minWidth);
  const height = props.options.length * 34 + 14;
  const spaceBelow = window.innerHeight - rect.bottom;
  const top = spaceBelow < height + 8 ? rect.top - height - 6 : rect.bottom + 6;
  const left = Math.min(
    Math.max(8, rect.right - width),
    Math.max(8, window.innerWidth - width - 8),
  );
  menuStyle.value = {
    top: `${Math.max(8, top)}px`,
    left: `${left}px`,
    width: `${width}px`,
  };
}

function onDocumentMouseDown(e: MouseEvent) {
  const target = e.target as Node;
  if (triggerRef.value?.contains(target)) return;
  if (menuRef.value?.contains(target)) return;
  close();
}

function onDocumentKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") close();
}

function openMenu() {
  updatePosition();
  open.value = true;
  window.addEventListener("scroll", updatePosition, true);
  window.addEventListener("resize", updatePosition);
  document.addEventListener("mousedown", onDocumentMouseDown);
  document.addEventListener("keydown", onDocumentKeydown);
}

function close() {
  if (!open.value) return;
  open.value = false;
  window.removeEventListener("scroll", updatePosition, true);
  window.removeEventListener("resize", updatePosition);
  document.removeEventListener("mousedown", onDocumentMouseDown);
  document.removeEventListener("keydown", onDocumentKeydown);
}

function toggle() {
  if (open.value) close();
  else openMenu();
}

function select(value: string) {
  emit("update:modelValue", value);
  close();
}

onBeforeUnmount(close);
</script>

<template>
  <div ref="triggerRef" class="glass-select" :class="{ block }">
    <button
      type="button"
      class="gs-trigger"
      :class="{ open }"
      :aria-expanded="open"
      @click="toggle"
    >
      <span class="gs-value">{{ currentLabel }}</span>
      <ChevronDown class="gs-arrow" :size="14" :stroke-width="2" />
    </button>

    <Teleport to="body">
      <Transition name="gs-fade">
        <div v-if="open" ref="menuRef" class="gs-menu" :style="menuStyle">
          <button
            v-for="opt in options"
            :key="opt.value"
            type="button"
            class="gs-item"
            :class="{ active: opt.value === modelValue }"
            @click="select(opt.value)"
          >
            <span>{{ opt.label }}</span>
            <Check v-if="opt.value === modelValue" :size="14" :stroke-width="2.2" />
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.glass-select {
  display: inline-flex;
}

.glass-select.block {
  display: flex;
  width: 100%;
}

.glass-select.block .gs-trigger {
  width: 100%;
}

.gs-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 96px;
  padding: 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  outline: none;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.gs-trigger:hover {
  background: var(--bg-card-hover);
}

.gs-trigger.open {
  border-color: rgba(139, 92, 246, 0.55);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

.gs-arrow {
  flex-shrink: 0;
  color: var(--text-tertiary);
  transition: transform 0.18s ease;
}

.gs-trigger.open .gs-arrow {
  transform: rotate(180deg);
}

/* 亚克力弹出层：半透明 + 背景模糊 + 高光边 */
.gs-menu {
  position: fixed;
  z-index: 3000;
  padding: 6px;
  border-radius: 12px;
  border: 1px solid var(--border-glass-strong);
  background: var(--bg-glass-strong);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  box-shadow: var(--shadow-glass), 0 14px 34px rgba(0, 0, 0, 0.35);
  overflow: hidden;
}

.gs-menu::before {
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

.gs-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12.5px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.gs-item:hover {
  background: rgba(139, 92, 246, 0.16);
  color: var(--text-primary);
}

.gs-item.active {
  color: var(--accent, #8b5cf6);
  font-weight: 500;
}

.gs-fade-enter-active,
.gs-fade-leave-active {
  transition: opacity 0.14s ease, transform 0.14s ease;
}

.gs-fade-enter-from,
.gs-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
