<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed, watch } from "vue";
import { getPomodoroSettings, updatePomodoroSettings, getGeneralSettings, updateGeneralSettings, getAppVersion, checkUpdateInfo } from "../api";
import type { PomodoroSettings, GeneralSettings } from "../api/types";
import { Timer, Settings, Info, Check, Loader2, ExternalLink } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";

// ---------- Tab ----------
const activeTab = ref<"pomodoro" | "general" | "about">("pomodoro");

// ---------- 番茄钟设置 ----------
const pomodoroSettings = ref<PomodoroSettings>({
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

// 字段边界配置
const numericFields = ["focusMinutes", "shortBreakMinutes", "longBreakMinutes", "longBreakInterval"] as const;
type NumericField = typeof numericFields[number];

const fieldRules: Record<NumericField, { min: number; max: number; label: string }> = {
  focusMinutes: { min: 1, max: 120, label: "专注时长" },
  shortBreakMinutes: { min: 1, max: 30, label: "短休息时长" },
  longBreakMinutes: { min: 1, max: 60, label: "长休息时长" },
  longBreakInterval: { min: 1, max: 10, label: "长休息间隔" },
};

// 输入框显示的字符串
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

// 是否有任何错误
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
  const valid = validateField(field, target.value);
  if (valid) {
    const num = parseInt(target.value, 10);
    (pomodoroSettings.value as unknown as Record<NumericField, number>)[field] = num;
  }
}

function onBlur(field: NumericField) {
  const valid = validateField(field, inputValues[field]);
  if (valid) {
    const num = parseInt(inputValues[field], 10);
    (pomodoroSettings.value as unknown as Record<NumericField, number>)[field] = num;
    inputValues[field] = String(num);
  }
}

// ---------- 自动保存 ----------
type SaveStatus = "idle" | "saving" | "saved" | "error";
const saveStatus = ref<SaveStatus>("idle");
let saveTimer: ReturnType<typeof setTimeout> | null = null;

async function savePomodoroSettings() {
  if (hasErrors.value) return;
  saveStatus.value = "saving";
  try {
    await updatePomodoroSettings(pomodoroSettings.value);
    saveStatus.value = "saved";
    setTimeout(() => {
      if (saveStatus.value === "saved") saveStatus.value = "idle";
    }, 1500);
  } catch (e) {
    console.error("保存设置失败", e);
    saveStatus.value = "error";
  }
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    savePomodoroSettings();
  }, 500);
}

// 监听番茄钟设置变化，自动保存
watch(
  pomodoroSettings,
  () => {
    if (loading.value) return; // 初始加载不触发保存
    scheduleSave();
  },
  { deep: true }
);

// ---------- 常规设置 ----------
const generalSettings = ref<GeneralSettings>({
  autoStart: false,
  notificationEnabled: true,
  idleThresholdMinutes: 3,
});

// 闲置阈值输入值
const idleInputValue = ref("3");
const idleError = ref("");

function validateIdle(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed === "") {
    idleError.value = "请输入阈值";
    return false;
  }
  const num = Number(trimmed);
  if (isNaN(num) || !Number.isInteger(num)) {
    idleError.value = "请输入整数";
    return false;
  }
  if (num < 1 || num > 60) {
    idleError.value = "范围：1 ~ 60";
    return false;
  }
  idleError.value = "";
  return true;
}

function onIdleInput(e: Event) {
  const target = e.target as HTMLInputElement;
  idleInputValue.value = target.value;
  const valid = validateIdle(target.value);
  if (valid) {
    generalSettings.value.idleThresholdMinutes = parseInt(target.value, 10);
  }
}

function onIdleBlur() {
  const valid = validateIdle(idleInputValue.value);
  if (valid) {
    const num = parseInt(idleInputValue.value, 10);
    generalSettings.value.idleThresholdMinutes = num;
    idleInputValue.value = String(num);
  }
}

let generalSaveTimer: ReturnType<typeof setTimeout> | null = null;

async function saveGeneralSettings() {
  if (idleError.value) return;
  saveStatus.value = "saving";
  try {
    await updateGeneralSettings(generalSettings.value);
    saveStatus.value = "saved";
    setTimeout(() => {
      if (saveStatus.value === "saved") saveStatus.value = "idle";
    }, 1500);
  } catch (e) {
    console.error("保存常规设置失败", e);
    saveStatus.value = "error";
  }
}

function scheduleGeneralSave() {
  if (generalSaveTimer) clearTimeout(generalSaveTimer);
  generalSaveTimer = setTimeout(() => {
    saveGeneralSettings();
  }, 500);
}

watch(
  generalSettings,
  () => {
    if (loading.value) return;
    scheduleGeneralSave();
  },
  { deep: true }
);

// ---------- 加载 ----------
async function loadSettings() {
  loading.value = true;
  try {
    const [pomodoroData, generalData] = await Promise.all([
      getPomodoroSettings(),
      getGeneralSettings(),
    ]);
    pomodoroSettings.value = pomodoroData;
    inputValues.focusMinutes = String(pomodoroData.focusMinutes);
    inputValues.shortBreakMinutes = String(pomodoroData.shortBreakMinutes);
    inputValues.longBreakMinutes = String(pomodoroData.longBreakMinutes);
    inputValues.longBreakInterval = String(pomodoroData.longBreakInterval);

    generalSettings.value = generalData;
    idleInputValue.value = String(generalData.idleThresholdMinutes);
  } catch (e) {
    console.error("加载设置失败", e);
  } finally {
    loading.value = false;
  }
}

// ---------- 关于页面 ----------
const appVersion = ref("");
const repoUrl = "https://github.com/sleepy-ailurus/time-sense";
const issuesUrl = "https://github.com/sleepy-ailurus/time-sense/issues";

const checkingUpdate = ref(false);
const updateResult = ref<"none" | "new" | "error" | null>(null);
const latestVersion = ref("");
const updateMessage = ref("");
const downloadUrl = ref("");

function openUrl(url: string) {
  invoke("open_url", { url }).catch(() => {});
}

async function loadVersion() {
  appVersion.value = await getAppVersion();
}

/** 比较版本号：v1 > v2 返回 1，相等返回 0，小于返回 -1 */
function compareVersions(v1: string, v2: string): number {
  const a = v1.replace(/^v/, "").split(".").map(Number);
  const b = v2.replace(/^v/, "").split(".").map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] || 0;
    const y = b[i] || 0;
    if (x > y) return 1;
    if (x < y) return -1;
  }
  return 0;
}

async function checkUpdate() {
  checkingUpdate.value = true;
  updateResult.value = null;
  updateMessage.value = "";
  latestVersion.value = "";
  downloadUrl.value = "";
  try {
    const info = await checkUpdateInfo();
    latestVersion.value = info.version;
    downloadUrl.value = info.url;
    const current = appVersion.value.replace(/^v/, "");
    if (compareVersions(info.version, current) > 0) {
      updateResult.value = "new";
      updateMessage.value = `发现新版本 v${info.version}，点击右侧图标前往下载`;
    } else {
      updateResult.value = "none";
      updateMessage.value = "已是最新版本";
    }
  } catch (e: any) {
    updateResult.value = "error";
    const reason = typeof e === "string" && e ? e : "网络异常";
    updateMessage.value = `检查更新失败（${reason}）`;
  } finally {
    checkingUpdate.value = false;
  }
}

onMounted(() => {
  loadSettings();
  loadVersion();
});

onUnmounted(() => {
  if (saveTimer) clearTimeout(saveTimer);
  if (generalSaveTimer) clearTimeout(generalSaveTimer);
});
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
      <h2 class="page-title">设置</h2>
      <p class="page-subtitle">个性化你的 TimeSense</p>
    </div>

    <!-- 顶部 Tab -->
    <div class="tabs">
      <div
        class="tab"
        :class="{ active: activeTab === 'pomodoro' }"
        @click="activeTab = 'pomodoro'"
      >
        <Timer :size="16" :stroke-width="1.8" />
        <span>番茄钟</span>
      </div>
      <div
        class="tab"
        :class="{ active: activeTab === 'general' }"
        @click="activeTab = 'general'"
      >
        <Settings :size="16" :stroke-width="1.8" />
        <span>常规</span>
      </div>
      <div
        class="tab"
        :class="{ active: activeTab === 'about' }"
        @click="activeTab = 'about'"
      >
        <Info :size="16" :stroke-width="1.8" />
        <span>关于</span>
      </div>
    </div>

    <!-- 保存状态提示 -->
    <div class="save-status" v-if="saveStatus !== 'idle'">
      <Loader2 v-if="saveStatus === 'saving'" class="spin" :size="14" />
      <Check v-else-if="saveStatus === 'saved'" :size="14" />
      <span v-if="saveStatus === 'saving'">保存中...</span>
      <span v-else-if="saveStatus === 'saved'">已保存</span>
      <span v-else-if="saveStatus === 'error'">保存失败</span>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <template v-else>
      <!-- 番茄钟设置 -->
      <div v-show="activeTab === 'pomodoro'" class="settings-section">
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">启用番茄钟</span>
            <span class="setting-desc">开启后将自动计时并提醒</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.enabled" />
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
            <input type="checkbox" v-model="pomodoroSettings.autoStartBreak" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">自动开始专注</span>
            <span class="setting-desc">休息结束后自动开始下一个番茄</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.autoStartFocus" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">自动番茄钟</span>
            <span class="setting-desc">使用工作/学习类应用时自动开始专注，切走自动暂停</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.autoMode" />
            <span class="slider"></span>
          </label>
        </div>
      </div>

      <!-- 常规设置 -->
      <div v-show="activeTab === 'general'" class="settings-section">
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">开机自启</span>
            <span class="setting-desc">系统启动时自动运行 TimeSense</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="generalSettings.autoStart" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">系统通知</span>
            <span class="setting-desc">番茄钟阶段切换时发送系统通知</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="generalSettings.notificationEnabled" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">闲置检测阈值</span>
            <span class="setting-desc">无操作多久后判定为闲置（1~60 分钟）</span>
          </div>
          <div class="setting-value-col">
            <div class="setting-value">
              <input
                type="text"
                :value="idleInputValue"
                @input="onIdleInput($event)"
                @blur="onIdleBlur"
                class="num-input"
                :class="{ error: idleError }"
              />
              <span class="unit">分钟</span>
            </div>
            <div v-if="idleError" class="field-error">{{ idleError }}</div>
          </div>
        </div>
      </div>

      <!-- 关于页面 -->
      <div v-show="activeTab === 'about'" class="about-section">
        <div class="about-app-info">
          <img src="../assets/icon.png" alt="TimeSense" class="about-icon" @error="e => (e.target as HTMLImageElement).style.display = 'none'" />
          <div class="about-name">TimeSense</div>
          <div class="about-version">v{{ appVersion }}</div>
        </div>

        <div class="about-actions">
          <div class="about-action-item" @click="checkUpdate">
            <div class="about-action-info">
              <span class="about-action-name">检查更新</span>
              <span class="about-action-desc">
                <template v-if="checkingUpdate">正在检查...</template>
                <template v-else-if="updateResult === 'new'" class="update-new">{{ updateMessage }}</template>
                <template v-else-if="updateResult === 'none'">{{ updateMessage }}</template>
                <template v-else-if="updateResult === 'error'" class="update-error">{{ updateMessage }}</template>
                <template v-else>点击检查最新版本</template>
              </span>
            </div>
            <Loader2 v-if="checkingUpdate" class="spin" :size="18" :stroke-width="1.8" />
            <ExternalLink v-else-if="updateResult === 'new'" :size="18" :stroke-width="1.8" @click.stop="openUrl(downloadUrl || repoUrl + '/releases')" />
            <ExternalLink v-else :size="18" :stroke-width="1.8" />
          </div>
          <div class="about-action-item" @click="openUrl(issuesUrl)">
            <div class="about-action-info">
              <span class="about-action-name">问题反馈</span>
              <span class="about-action-desc">提交 Bug 或建议</span>
            </div>
            <ExternalLink :size="18" :stroke-width="1.8" />
          </div>
          <div class="about-action-item" @click="openUrl(repoUrl)">
            <div class="about-action-info">
              <span class="about-action-name">GitHub 项目</span>
              <span class="about-action-desc">查看源代码</span>
            </div>
            <ExternalLink :size="18" :stroke-width="1.8" />
          </div>
        </div>
      </div>
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
  margin-bottom: 12px;
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

/* Tabs */
.tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border-glass);
}

.tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  transition: all 0.15s ease;
}

.tab:hover {
  color: var(--text-primary);
}

.tab.active {
  color: var(--text-primary);
  border-bottom-color: #8b5cf6;
}

/* 保存状态 */
.save-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: 12px;
  animation: fadeIn 0.2s ease;
}

.save-status svg {
  color: #10b981;
}

.save-status .spin {
  animation: spin 0.8s linear infinite;
  color: var(--text-tertiary);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
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
  padding: 4px 12px;
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
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

.section-tip {
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary);
  padding: 16px 0 8px;
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

/* ---------- 关于页面 ---------- */
.about-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 4px 20px;
}

.about-app-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 30px;
  padding: 20px 0;
}

.about-icon {
  width: 80px;
  height: 80px;
  border-radius: 18px;
  margin-bottom: 14px;
  object-fit: cover;
  box-shadow: 0 6px 20px rgba(139, 92, 246, 0.25);
}

.about-name {
  font-size: 22px;
  font-weight: 600;
  color: #ffffff;
  margin-bottom: 4px;
}

.about-version {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.7);
}

.about-actions {
  width: 100%;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  overflow: hidden;
  backdrop-filter: blur(10px);
}

.about-action-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  cursor: pointer;
  transition: background 0.15s;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.about-action-item:last-child {
  border-bottom: none;
}

.about-action-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.about-action-item:active {
  background: rgba(255, 255, 255, 0.1);
}

.about-action-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about-action-name {
  font-size: 15px;
  font-weight: 500;
  color: #ffffff;
}

.about-action-desc {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
}

.update-new {
  color: #34d399 !important;
}

.update-error {
  color: #f87171 !important;
}
</style>
