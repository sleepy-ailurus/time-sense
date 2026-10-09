<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { refreshGlobalShortcuts, suspendGlobalShortcuts } from "../composables/useGlobalShortcuts";

const props = defineProps<{ modelValue?: string }>();
const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

const { t } = useI18n();
const recording = ref(false);

const MODIFIER_KEYS = ["Control", "Shift", "Alt", "Meta", "OS"];

async function startRecording() {
  recording.value = true;
  // 录制期间先注销全局快捷键，否则按下已绑定的组合键会直接触发（窗口会被隐藏等）
  await suspendGlobalShortcuts();
}

async function stopRecording() {
  recording.value = false;
  await refreshGlobalShortcuts();
}

function keyName(e: KeyboardEvent): string {
  const code = e.code;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  const map: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Minus: "-",
    Equal: "=",
    BracketLeft: "[",
    BracketRight: "]",
    Semicolon: ";",
    Quote: "'",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Backslash: "\\",
    Backquote: "`",
  };
  return map[code] || e.key.toUpperCase();
}

async function onKeydown(e: KeyboardEvent) {
  if (!recording.value) return;
  e.preventDefault();
  e.stopPropagation();

  if (e.key === "Escape") {
    await stopRecording();
    return;
  }
  if (e.key === "Backspace" || e.key === "Delete") {
    emit("update:modelValue", "");
    await stopRecording();
    return;
  }
  // 只按修饰键时继续等待
  if (MODIFIER_KEYS.includes(e.key)) return;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Super");
  // 必须带修饰键，避免占用普通按键
  if (parts.length === 0) return;

  parts.push(keyName(e));
  emit("update:modelValue", parts.join("+"));
  await stopRecording();
}
</script>

<template>
  <button
    type="button"
    class="shortcut-input"
    :class="{ recording }"
    @click="startRecording"
    @keydown="onKeydown"
    @blur="recording && stopRecording()"
  >
    <span v-if="recording" class="recording-text">{{ t('settings.shortcutRecording') }}</span>
    <template v-else>
      <kbd v-for="part in (modelValue || '').split('+').filter(Boolean)" :key="part">{{ part }}</kbd>
      <span v-if="!modelValue" class="empty">{{ t('settings.shortcutEmpty') }}</span>
    </template>
  </button>
</template>

<style scoped>
.shortcut-input {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 132px;
  justify-content: center;
  padding: 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.shortcut-input:hover {
  background: var(--bg-card-hover);
}

.shortcut-input.recording {
  border-color: rgba(139, 92, 246, 0.55);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

kbd {
  padding: 1px 5px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid var(--border-glass);
  font-family: inherit;
  font-size: 11px;
}

.empty,
.recording-text {
  font-size: 11px;
  color: var(--text-tertiary);
}
</style>
