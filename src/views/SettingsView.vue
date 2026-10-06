<script setup lang="ts">
import { ref, reactive, onMounted, computed } from "vue";
import { getPomodoroSettings, updatePomodoroSettings } from "../api";
import type { PomodoroSettings } from "../api/types";
import { Timer, Save, Bell, Coffee, Brain } from "lucide-vue-next";

const settings = ref<PomodoroSettings>({
  focusMinutes: 45,
  shortBreakMinutes: 5,
  longBreakMinutes: 15,
  longBreakInterval: 4,
  autoStartBreak: true,
  autoStartFocus: false,
  autoMode: false,
  enabled: true,
});

const loading = ref(false);
const saved = ref(false);

// 字段边界配置
const numericFields = ["focusMinutes", "shortBreakMinutes", "longBreakMinutes", "longBreakInterval"] as const;
type NumericField = typeof numericFields[number];

const fieldRules: Record<NumericField, { min: number; max: number; label: string }> = {
  focusMinutes: { min: 1, max: 120, label: "专注时长" },
  shortBreakMinutes: { min: 1, max: 30, label: "短休息时长" },
  longBreakMinutes: { min: 1, max: 60, label: "长休息时长" },
  longBreakInterval: { min: 1, max: 10, label: "长休息间隔" },
};

// 输入框显示的字符串（可以为空，失焦时校验并修正）
const inputValues = reactive<Record<NumericField, string>>({
  focusMinutes: "45",
  shortBreakMinutes: "5",
  longBreakMinutes: "15",
  longBreakInterval: "4",
});

// 每个字段的错误信息
const errors = reactive<Record<NumericField, string>>({
  focusMinutes: "",
  shortBreakMinutes: "",
  longBreakMinutes: "",
  longBreakInterval: "",
});

// 是否有任何错误（用于禁用保存按钮）
const hasErrors = computed(() =>
  Object.values(errors).some((e) => e !== "")
);

function validateField(field: NumericField, value: string): boolean {
  const rule = fieldRules[field];
  const trimmed = value.trim();

  if (trimmed === "") {
    errors[field] = `请输入${rule.label}`;
    return false;
  }

  const num = Number(trimmed);
  if (isNaN(num) || !Number.isInteger(num)) {
    errors[field] = "请输入整数";
    return false;
  }

  if (num < rule.min || num > rule.max) {
    errors[field] = `范围：${rule.min} ~ ${rule.max}`;
    return false;
  }

  errors[field] = "";
  return true;
}

function onInput(field: NumericField, e: Event) {
  const target = e.target as HTMLInputElement;
  inputValues[field] = target.value;
  // 输入时实时校验，但不修正值
  validateField(field, target.value);
}

function onBlur(field: NumericField) {
  const valid = validateField(field, inputValues[field]);
  if (valid) {
    // 校验通过：同步到 settings
    const num = parseInt(inputValues[field], 10);
    (settings.value as unknown as Record<NumericField, number>)[field] = num;
    // 规范化显示（去掉前导零等）
    inputValues[field] = String(num);
  }
}

async function loadSettings() {
  loading.value = true;
  try {
    const data = await getPomodoroSettings();
    settings.value = data;
    // 同步到输入字符串
    inputValues.focusMinutes = String(data.focusMinutes);
    inputValues.shortBreakMinutes = String(data.shortBreakMinutes);
    inputValues.longBreakMinutes = String(data.longBreakMinutes);
    inputValues.longBreakInterval = String(data.longBreakInterval);
  } catch (e) {
    console.error("加载设置失败", e);
  } finally {
    loading.value = false;
  }
}

async function handleSave() {
  if (hasErrors.value) return;
  try {
    await updatePomodoroSettings(settings.value);
    saved.value = true;
    setTimeout(() => {
      saved.value = false;
    }, 2000);
  } catch (e) {
    console.error("保存设置失败", e);
  }
}

onMounted(() => {
  loadSettings();
});
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <h2 class="page-title">设置</h2>
      <p class="page-subtitle">个性化你的 TimeSense</p>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <template v-else>
      <!-- 番茄钟设置 -->
      <div class="settings-section">
        <div class="section-header">
          <Timer class="section-icon" :size="18" :stroke-width="1.8" />
          <span class="section-title">番茄钟</span>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">启用番茄钟</span>
            <span class="setting-desc">开启后将自动计时并提醒</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.enabled" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">专注时长</span>
            <span class="setting-desc">每个番茄的专注时间</span>
          </div>
          <div class="setting-value-col">
            <div class="setting-value">
              <input
                type="text"
                :value="inputValues.focusMinutes"
                @input="onInput('focusMinutes', $event)"
                @blur="onBlur('focusMinutes')"
                class="num-input"
                :class="{ error: errors.focusMinutes }"
              />
              <span class="unit">分钟</span>
            </div>
            <div v-if="errors.focusMinutes" class="field-error">{{ errors.focusMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">短休息时长</span>
            <span class="setting-desc">番茄之间的短暂休息</span>
          </div>
          <div class="setting-value-col">
            <div class="setting-value">
              <input
                type="text"
                :value="inputValues.shortBreakMinutes"
                @input="onInput('shortBreakMinutes', $event)"
                @blur="onBlur('shortBreakMinutes')"
                class="num-input"
                :class="{ error: errors.shortBreakMinutes }"
              />
              <span class="unit">分钟</span>
            </div>
            <div v-if="errors.shortBreakMinutes" class="field-error">{{ errors.shortBreakMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">长休息时长</span>
            <span class="setting-desc">每 N 个番茄后的长休息</span>
          </div>
          <div class="setting-value-col">
            <div class="setting-value">
              <input
                type="text"
                :value="inputValues.longBreakMinutes"
                @input="onInput('longBreakMinutes', $event)"
                @blur="onBlur('longBreakMinutes')"
                class="num-input"
                :class="{ error: errors.longBreakMinutes }"
              />
              <span class="unit">分钟</span>
            </div>
            <div v-if="errors.longBreakMinutes" class="field-error">{{ errors.longBreakMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">长休息间隔</span>
            <span class="setting-desc">每几个番茄后进入长休息</span>
          </div>
          <div class="setting-value-col">
            <div class="setting-value">
              <input
                type="text"
                :value="inputValues.longBreakInterval"
                @input="onInput('longBreakInterval', $event)"
                @blur="onBlur('longBreakInterval')"
                class="num-input"
                :class="{ error: errors.longBreakInterval }"
              />
              <span class="unit">个</span>
            </div>
            <div v-if="errors.longBreakInterval" class="field-error">{{ errors.longBreakInterval }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">自动开始休息</span>
            <span class="setting-desc">专注结束后自动进入休息</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.autoStartBreak" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">自动开始专注</span>
            <span class="setting-desc">休息结束后自动开始下一个番茄</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.autoStartFocus" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">自动番茄钟</span>
            <span class="setting-desc">使用工作/学习类应用时自动开始专注，切走自动暂停</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="settings.autoMode" />
            <span class="slider"></span>
          </label>
        </div>
      </div>

      <button class="save-btn" @click="handleSave" :class="{ saved }">
        {{ saved ? "✓ 已保存" : "保存设置" }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.settings-page {
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

.settings-section {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-glass);
  border-radius: 12px;
  padding: 12px;
  margin-bottom: 16px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-glass);
}

.section-icon {
  font-size: 18px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.setting-name {
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
}

.setting-desc {
  font-size: 11px;
  color: var(--text-tertiary);
}

.setting-value {
  display: flex;
  align-items: center;
  gap: 6px;
}

.setting-value-col {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  min-width: 80px;
}

.num-input {
  width: 60px;
  padding: 6px 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border-glass);
  border-radius: 6px;
  color: #fff !important;
  font-size: 12px;
  text-align: center;
  outline: none;
  /* 禁用 number 上下箭头（如果浏览器还是当 number 处理） */
  appearance: textfield;
  -moz-appearance: textfield;
}

.num-input::-webkit-outer-spin-button,
.num-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.num-input:focus {
  border-color: rgba(139, 92, 246, 0.5);
}

.num-input.error {
  border-color: rgba(239, 68, 68, 0.6);
  background: rgba(239, 68, 68, 0.08);
}

.field-error {
  font-size: 10px;
  color: #ef4444;
  line-height: 1;
}

.unit {
  font-size: 11px;
  color: var(--text-tertiary);
}

/* Switch */
.switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(255, 255, 255, 0.15);
  transition: 0.2s;
  border-radius: 22px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  transition: 0.2s;
  border-radius: 50%;
}

input:checked + .slider {
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
}

input:checked + .slider:before {
  transform: translateX(18px);
}

.save-btn {
  margin-top: auto;
  padding: 12px;
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
  color: white;
  border: none;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.save-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.4);
}

.save-btn.saved {
  background: linear-gradient(135deg, #10b981, #059669);
}
</style>
