<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getPomodoroSettings, updatePomodoroSettings, getGeneralSettings, updateGeneralSettings, getAppVersion, checkUpdateInfo, getGoals, createGoal, updateGoal, deleteGoal, getCategories } from "../api";
import type { PomodoroSettings, GeneralSettings, Goal, GoalStatus, NewGoal, Category } from "../api/types";
import { Timer, Settings, Info, Check, Loader2, ExternalLink, Target, Plus, Pencil, Trash2 } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";
import { formatDuration } from "../utils/format";
import { categoryLabel } from "../utils/categoryName";
import { confirmDialog } from "../composables/useConfirm";
import {
  DEFAULT_SHORTCUT_TOGGLE_POMODORO,
  DEFAULT_SHORTCUT_TOGGLE_WINDOW,
  refreshGlobalShortcuts,
} from "../composables/useGlobalShortcuts";
import ShortcutInput from "../components/ShortcutInput.vue";
import { setLocale, getLocale } from "../i18n";
import GlassSelect from "../components/GlassSelect.vue";

const { t } = useI18n();

// ---------- Tab ----------
const activeTab = ref<"pomodoro" | "general" | "goals" | "about">("pomodoro");

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

const fieldRules: Record<NumericField, { min: number; max: number; labelKey: string }> = {
  focusMinutes: { min: 1, max: 120, labelKey: "settings.focusMinutes" },
  shortBreakMinutes: { min: 1, max: 30, labelKey: "settings.shortBreak" },
  longBreakMinutes: { min: 1, max: 60, labelKey: "settings.longBreak" },
  longBreakInterval: { min: 1, max: 10, labelKey: "settings.longBreakInterval" },
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
    errors[field] = t("settings.errorRequired", { label: t(rule.labelKey) });
    return false;
  }

  const num = Number(trimmed);
  if (isNaN(num) || !Number.isInteger(num)) {
    errors[field] = t("settings.errorInteger");
    return false;
  }

  if (num < rule.min || num > rule.max) {
    errors[field] = t("settings.errorRange", { min: rule.min, max: rule.max });
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
  shortcutToggleWindow: DEFAULT_SHORTCUT_TOGGLE_WINDOW,
  shortcutTogglePomodoro: DEFAULT_SHORTCUT_TOGGLE_POMODORO,
});

// 快捷键注册失败提示（通常是被其它软件占用）
const shortcutError = ref("");

// 闲置阈值输入值
const idleInputValue = ref("3");
const idleError = ref("");

function validateIdle(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed === "") {
    idleError.value = t("settings.errorRequired", { label: t("settings.idleThreshold") });
    return false;
  }
  const num = Number(trimmed);
  if (isNaN(num) || !Number.isInteger(num)) {
    idleError.value = t("settings.errorInteger");
    return false;
  }
  if (num < 1 || num > 60) {
    idleError.value = t("settings.errorRange", { min: 1, max: 60 });
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

const localeOptions = [
  { value: "zh", label: "中文" },
  { value: "en", label: "English" },
];

function handleLocaleChange(value: string) {
  setLocale(value);
}

let generalSaveTimer: ReturnType<typeof setTimeout> | null = null;

async function saveGeneralSettings() {
  if (idleError.value) return;
  saveStatus.value = "saving";
  try {
    await updateGeneralSettings(generalSettings.value);
    // 快捷键改完立刻重新注册（失败的会在这里提示）
    const failed = await refreshGlobalShortcuts();
    shortcutError.value = failed.length
      ? t("settings.shortcutConflict", { list: failed.join("、") })
      : "";
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

    generalSettings.value = {
      ...generalData,
      shortcutToggleWindow: generalData.shortcutToggleWindow || DEFAULT_SHORTCUT_TOGGLE_WINDOW,
      shortcutTogglePomodoro: generalData.shortcutTogglePomodoro || DEFAULT_SHORTCUT_TOGGLE_POMODORO,
    };
    idleInputValue.value = String(generalData.idleThresholdMinutes);
  } catch (e) {
    console.error("加载设置失败", e);
  } finally {
    loading.value = false;
  }
}

// ---------- 目标预算 ----------
const goals = ref<Goal[]>([]);
const goalCategories = ref<Category[]>([]);
const showAddGoalModal = ref(false);
const showEditGoalModal = ref(false);
const editingGoalId = ref<number | null>(null);
const addGoalForm = ref<NewGoal>({ categoryId: 0, dailyLimitMinutes: 60 });
const editGoalForm = ref<NewGoal>({ categoryId: 0, dailyLimitMinutes: 60 });

async function loadGoals() {
  try {
    const [goalsData, catsData] = await Promise.all([getGoals(), getCategories()]);
    goals.value = goalsData;
    goalCategories.value = catsData;
    if (catsData.length > 0 && addGoalForm.value.categoryId === 0) {
      addGoalForm.value.categoryId = catsData[0].id;
    }
  } catch (e) {
    console.error("加载目标失败", e);
  }
}

async function handleAddGoal() {
  try {
    await createGoal(addGoalForm.value);
    showAddGoalModal.value = false;
    addGoalForm.value = { categoryId: goalCategories.value[0]?.id || 0, dailyLimitMinutes: 60 };
    await loadGoals();
  } catch (e) {
    console.error("新增目标失败", e);
  }
}

function openEditGoalModal(goal: Goal) {
  editingGoalId.value = goal.id;
  editGoalForm.value = { categoryId: goal.categoryId, dailyLimitMinutes: goal.dailyLimitMinutes, enabled: goal.enabled };
  showEditGoalModal.value = true;
}

async function handleSaveEditGoal() {
  if (editingGoalId.value === null) return;
  try {
    await updateGoal(editingGoalId.value, editGoalForm.value);
    showEditGoalModal.value = false;
    editingGoalId.value = null;
    await loadGoals();
  } catch (e) {
    console.error("更新目标失败", e);
  }
}

async function handleDeleteGoal(id: number) {
  const ok = await confirmDialog({
    title: t("settings.deleteBudget"),
    message: t("settings.deleteBudgetConfirm"),
    confirmText: t("common.delete"),
  });
  if (!ok) return;
  try {
    await deleteGoal(id);
    await loadGoals();
  } catch (e) {
    console.error("删除目标失败", e);
  }
}

async function handleGoalToggle(goal: Goal) {
  try {
    await updateGoal(goal.id, {
      categoryId: goal.categoryId,
      dailyLimitMinutes: goal.dailyLimitMinutes,
      enabled: !goal.enabled,
    });
    await loadGoals();
  } catch (e) {
    console.error("切换目标状态失败", e);
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
      updateMessage.value = t("settings.newVersion", { version: info.version });
    } else {
      updateResult.value = "none";
      updateMessage.value = t("settings.upToDate");
    }
  } catch (e: any) {
    updateResult.value = "error";
    const reason = typeof e === "string" && e ? e : t("settings.networkError");
    updateMessage.value = t("settings.checkFailed", { reason });
  } finally {
    checkingUpdate.value = false;
  }
}

onMounted(() => {
  loadSettings();
  loadVersion();
  loadGoals();
});

onUnmounted(() => {
  if (saveTimer) clearTimeout(saveTimer);
  if (generalSaveTimer) clearTimeout(generalSaveTimer);
});
</script>

<template>
  <div class="settings-page">
    <div class="page-header">
        <h2 class="page-title">{{ t('settings.title') }}</h2>
        <p class="page-subtitle">{{ t('settings.subtitle') }}</p>
    </div>

    <!-- 顶部 Tab -->
    <div class="tabs">
      <div
        class="tab"
        :class="{ active: activeTab === 'pomodoro' }"
        @click="activeTab = 'pomodoro'"
      >
        <Timer :size="16" :stroke-width="1.8" />
          <span>{{ t('settings.pomodoro') }}</span>
      </div>
      <div
        class="tab"
        :class="{ active: activeTab === 'general' }"
        @click="activeTab = 'general'"
      >
        <Settings :size="16" :stroke-width="1.8" />
          <span>{{ t('settings.general') }}</span>
      </div>
      <div
        class="tab"
        :class="{ active: activeTab === 'goals' }"
        @click="activeTab = 'goals'"
      >
        <Target :size="16" :stroke-width="1.8" />
          <span>{{ t('settings.goals') }}</span>
      </div>
      <div
        class="tab"
        :class="{ active: activeTab === 'about' }"
        @click="activeTab = 'about'"
      >
        <Info :size="16" :stroke-width="1.8" />
          <span>{{ t('settings.about') }}</span>
      </div>
    </div>

    <!-- 保存状态提示 -->
    <div class="save-status" v-if="saveStatus !== 'idle'">
      <Loader2 v-if="saveStatus === 'saving'" class="spin" :size="14" />
      <Check v-else-if="saveStatus === 'saved'" :size="14" />
        <span v-if="saveStatus === 'saving'">{{ t('settings.saving') }}</span>
        <span v-else-if="saveStatus === 'saved'">{{ t('settings.saved') }}</span>
        <span v-else-if="saveStatus === 'error'">{{ t('settings.saveFailed') }}</span>
    </div>

        <div v-if="loading" class="loading">{{ t('common.loading') }}</div>

    <template v-else>
      <!-- 番茄钟设置 -->
      <div v-show="activeTab === 'pomodoro'" class="settings-section">
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.enablePomodoro') }}</span>
            <span class="setting-desc">{{ t('settings.enablePomodoroDesc') }}</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.enabled" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.focusMinutes') }}</span>
            <span class="setting-desc">{{ t('settings.focusMinutesDesc') }}</span>
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
              <span class="unit">{{ t('common.minute') }}</span>
            </div>
            <div v-if="errors.focusMinutes" class="field-error">{{ errors.focusMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.shortBreak') }}</span>
            <span class="setting-desc">{{ t('settings.shortBreakDesc') }}</span>
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
              <span class="unit">{{ t('common.minute') }}</span>
            </div>
            <div v-if="errors.shortBreakMinutes" class="field-error">{{ errors.shortBreakMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.longBreak') }}</span>
            <span class="setting-desc">{{ t('settings.longBreakDesc') }}</span>
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
              <span class="unit">{{ t('common.minute') }}</span>
            </div>
            <div v-if="errors.longBreakMinutes" class="field-error">{{ errors.longBreakMinutes }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.longBreakInterval') }}</span>
            <span class="setting-desc">{{ t('settings.longBreakIntervalDesc') }}</span>
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
              <span class="unit">{{ t('common.unit') }}</span>
            </div>
            <div v-if="errors.longBreakInterval" class="field-error">{{ errors.longBreakInterval }}</div>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.autoStartBreak') }}</span>
            <span class="setting-desc">{{ t('settings.autoStartBreakDesc') }}</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.autoStartBreak" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.autoStartFocus') }}</span>
            <span class="setting-desc">{{ t('settings.autoStartFocusDesc') }}</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="pomodoroSettings.autoStartFocus" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.autoMode') }}</span>
            <span class="setting-desc">{{ t('settings.autoModeDesc') }}</span>
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
            <span class="setting-name">{{ t('settings.autoStart') }}</span>
            <span class="setting-desc">{{ t('settings.autoStartDesc') }}</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="generalSettings.autoStart" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.notification') }}</span>
            <span class="setting-desc">{{ t('settings.notificationDesc') }}</span>
          </div>
          <label class="switch">
            <input type="checkbox" v-model="generalSettings.notificationEnabled" />
            <span class="slider"></span>
          </label>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.idleThreshold') }}</span>
            <span class="setting-desc">{{ t('settings.idleThresholdDesc') }}</span>
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
              <span class="unit">{{ t('common.minute') }}</span>
            </div>
            <div v-if="idleError" class="field-error">{{ idleError }}</div>
          </div>
        </div>

        <!-- 全局快捷键 -->
        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.shortcuts') }}</span>
            <span class="setting-desc">{{ t('settings.shortcutsDesc') }}</span>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.shortcutToggleWindow') }}</span>
            <span class="setting-desc">{{ t('settings.shortcutToggleWindowDesc') }}</span>
          </div>
          <ShortcutInput v-model="generalSettings.shortcutToggleWindow" />
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.shortcutTogglePomodoro') }}</span>
            <span class="setting-desc">{{ t('settings.shortcutTogglePomodoroDesc') }}</span>
          </div>
          <ShortcutInput v-model="generalSettings.shortcutTogglePomodoro" />
        </div>

        <div v-if="shortcutError" class="setting-item">
          <div class="setting-info">
            <span class="setting-desc field-error">{{ shortcutError }}</span>
          </div>
        </div>

        <div class="setting-item">
          <div class="setting-info">
            <span class="setting-name">{{ t('settings.language') }}</span>
            <span class="setting-desc">{{ t('settings.languageDesc') }}</span>
          </div>
          <div class="setting-value">
            <GlassSelect
              :model-value="getLocale()"
              :options="localeOptions"
              :min-width="96"
              @update:model-value="handleLocaleChange"
            />
          </div>
        </div>
      </div>

      <!-- 目标预算设置 -->
      <div v-show="activeTab === 'goals'" class="settings-section goal-section">
        <div class="goal-header">
            <span class="setting-name">{{ t('settings.dailyBudget') }}</span>
          <button class="goal-add-btn" @click="showAddGoalModal = true">
            <Plus :size="14" :stroke-width="2" /> {{ t('settings.addBudget') }}
          </button>
        </div>
        <div v-if="goals.length === 0" class="goal-empty">
          <span>{{ t('settings.noBudget') }}</span>
        </div>
        <div v-else class="goal-list">
          <div v-for="goal in goals" :key="goal.id" class="goal-row">
            <span class="goal-dot" :style="{ background: goal.categoryColor || '#6B7280' }"></span>
            <span class="goal-cat-name">{{ goal.categoryName ? categoryLabel(goal.categoryName) : t('common.unknown') }}</span>
            <span class="goal-limit">{{ goal.dailyLimitMinutes }} {{ t('settings.minutesPerDay') }}</span>
            <label class="switch mini">
              <input type="checkbox" :checked="goal.enabled" @change="handleGoalToggle(goal)" />
              <span class="slider"></span>
            </label>
            <button class="goal-action" @click="openEditGoalModal(goal)" :title="t('common.edit')">
              <Pencil :size="13" :stroke-width="1.8" />
            </button>
            <button class="goal-action delete" @click="handleDeleteGoal(goal.id)" :title="t('common.delete')">
              <Trash2 :size="13" :stroke-width="1.8" />
            </button>
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
              <span class="about-action-name">{{ t('settings.checkUpdate') }}</span>
              <span class="about-action-desc">
                <template v-if="checkingUpdate">{{ t('settings.checking') }}</template>
                <template v-else-if="updateResult === 'new'" class="update-new">{{ updateMessage }}</template>
                <template v-else-if="updateResult === 'none'">{{ updateMessage }}</template>
                <template v-else-if="updateResult === 'error'" class="update-error">{{ updateMessage }}</template>
                <template v-else>{{ t('settings.clickToCheck') }}</template>
              </span>
            </div>
            <Loader2 v-if="checkingUpdate" class="spin" :size="18" :stroke-width="1.8" />
            <ExternalLink v-else-if="updateResult === 'new'" :size="18" :stroke-width="1.8" @click.stop="openUrl(downloadUrl || repoUrl + '/releases')" />
            <ExternalLink v-else :size="18" :stroke-width="1.8" />
          </div>
          <div class="about-action-item" @click="openUrl(issuesUrl)">
            <div class="about-action-info">
              <span class="about-action-name">{{ t('settings.feedback') }}</span>
              <span class="about-action-desc">{{ t('settings.feedbackDesc') }}</span>
            </div>
            <ExternalLink :size="18" :stroke-width="1.8" />
          </div>
          <div class="about-action-item" @click="openUrl(repoUrl)">
            <div class="about-action-info">
              <span class="about-action-name">{{ t('settings.github') }}</span>
              <span class="about-action-desc">{{ t('settings.githubDesc') }}</span>
            </div>
            <ExternalLink :size="18" :stroke-width="1.8" />
          </div>
        </div>
      </div>
    </template>

    <!-- 新增目标弹窗 -->
    <div v-if="showAddGoalModal" class="modal-overlay">
      <div class="modal-content">
        <h3 class="modal-title">{{ t('settings.addGoalTitle') }}</h3>
        <div class="form-group">
          <label>{{ t('settings.category') }}</label>
          <select v-model="addGoalForm.categoryId" class="form-select goal-form-select">
            <option v-for="cat in goalCategories" :key="cat.id" :value="cat.id">{{ categoryLabel(cat.name) }}</option>
          </select>
        </div>
        <div class="form-group">
          <label>{{ t('settings.dailyLimit') }}</label>
          <input v-model.number="addGoalForm.dailyLimitMinutes" type="number" class="num-input goal-num-input" min="1" max="1440" />
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" @click="showAddGoalModal = false">{{ t('common.cancel') }}</button>
          <button class="btn-primary" @click="handleAddGoal">{{ t('common.add') }}</button>
        </div>
      </div>
    </div>

    <!-- 编辑目标弹窗 -->
    <div v-if="showEditGoalModal" class="modal-overlay">
      <div class="modal-content">
        <h3 class="modal-title">{{ t('settings.editGoalTitle') }}</h3>
        <div class="form-group">
          <label>{{ t('settings.category') }}</label>
          <select v-model="editGoalForm.categoryId" class="form-select goal-form-select">
            <option v-for="cat in goalCategories" :key="cat.id" :value="cat.id">{{ categoryLabel(cat.name) }}</option>
          </select>
        </div>
        <div class="form-group">
          <label>{{ t('settings.dailyLimit') }}</label>
          <input v-model.number="editGoalForm.dailyLimitMinutes" type="number" class="num-input goal-num-input" min="1" max="1440" />
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" @click="showEditGoalModal = false">{{ t('common.cancel') }}</button>
          <button class="btn-primary" @click="handleSaveEditGoal">{{ t('common.save') }}</button>
        </div>
      </div>
    </div>
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

/* ---------- 目标预算 ---------- */
.goal-section {
  padding: 12px;
}

.goal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.goal-add-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 10px;
  border: 1px dashed var(--border-glass-strong);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.goal-add-btn:hover {
  border-color: var(--accent, #8B5CF6);
  color: var(--accent, #8B5CF6);
}

.goal-empty {
  text-align: center;
  padding: 20px 0;
  font-size: 12px;
  color: var(--text-tertiary);
}

.goal-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.goal-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
}

.goal-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.goal-cat-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  flex: 1;
}

.goal-limit {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: monospace;
}

.switch.mini {
  width: 34px;
  height: 18px;
}

.switch.mini .slider:before {
  height: 12px;
  width: 12px;
}

.switch.mini input:checked + .slider:before {
  transform: translateX(16px);
}

.goal-action {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
}

.goal-action:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.goal-action.delete:hover {
  color: #ef4444;
}

.goal-form-select {
  width: 100%;
  padding: 9px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  box-sizing: border-box;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.goal-form-select:hover {
  background: var(--bg-card-hover);
}

.goal-form-select:focus {
  border-color: rgba(139, 92, 246, 0.55);
  background: var(--bg-card-hover);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

.goal-form-select option {
  background: var(--bg-primary);
  color: var(--text-primary);
}

.goal-num-input {
  width: 100% !important;
  padding: 9px 12px !important;
  text-align: left !important;
  background: var(--bg-card) !important;
  border-radius: 8px !important;
  /* 覆盖 .num-input 里写死的白色字，统一用主题文字色 */
  color: var(--text-primary) !important;
}

.goal-num-input:hover {
  background: var(--bg-card-hover) !important;
}

.goal-num-input:focus {
  border-color: rgba(139, 92, 246, 0.55) !important;
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

/* 弹窗复用 */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(8, 10, 18, 0.38);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(8px) saturate(120%);
  -webkit-backdrop-filter: blur(8px) saturate(120%);
}

.modal-content {
  position: relative;
  width: 320px;
  background: var(--bg-glass);
  backdrop-filter: blur(60px) saturate(190%);
  -webkit-backdrop-filter: blur(60px) saturate(190%);
  border: 1px solid var(--border-glass-strong);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-glass), 0 0 0 1px rgba(139, 92, 246, 0.08);
  padding: 22px 20px 18px;
  overflow: hidden;
}

/* 顶部高光，跟主面板保持同一套玻璃质感 */
.modal-content::before {
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

/* 左上角紫色柔光，呼应主面板卡片的 glow-orb */
.modal-content::after {
  content: "";
  position: absolute;
  top: -70px;
  left: -50px;
  width: 190px;
  height: 190px;
  background: radial-gradient(circle, rgba(139, 92, 246, 0.3) 0%, transparent 70%);
  pointer-events: none;
}

.modal-title {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.2px;
  margin: 0 0 16px 0;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-glass);
}

/* 标题前的渐变小竖条，跟侧边栏选中态、主按钮同一套配色 */
.modal-title::before {
  content: "";
  width: 3px;
  height: 14px;
  border-radius: 2px;
  background: linear-gradient(180deg, #8b5cf6, #ec4899);
}

.form-group {
  position: relative;
  z-index: 1;
  margin-bottom: 14px;
}

.form-group label {
  display: block;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 6px;
  letter-spacing: 0.3px;
}

.modal-actions {
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
  color: var(--text-secondary);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-secondary:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-glass-strong);
  color: var(--text-primary);
}

.btn-primary {
  padding: 8px 18px;
  background: linear-gradient(135deg, #8B5CF6, #EC4899);
  border: none;
  color: white;
  border-radius: 8px;
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
</style>
