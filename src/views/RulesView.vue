<script setup lang="ts">
import { ref, onMounted, computed, markRaw } from "vue";
import { useI18n } from "vue-i18n";
import { getRules, getCategories, createRule, deleteRule, toggleRule, updateRule, reorderRules } from "../api";
import type { AppRule, Category, MatchMode, MatchType } from "../api/types";
import { categoryLabel } from "../utils/categoryName";
import GlassSelect from "../components/GlassSelect.vue";
import { confirmDialog } from "../composables/useConfirm";
import {
  Briefcase,
  BookOpen,
  Gamepad2,
  MessageCircle,
  MoreHorizontal,
  Folder,
  Plus,
  Trash2,
  Power,
  Pencil,
  X,
  ChevronUp,
  ChevronDown,
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

const { t } = useI18n();

const rules = ref<AppRule[]>([]);
const categories = ref<Category[]>([]);
const loading = ref(false);
const showAddModal = ref(false);
const showEditModal = ref(false);
const editingId = ref<number | null>(null);

const newRule = ref({
  categoryId: 0,
  matchType: "process" as MatchType,
  matchValue: "",
  matchMode: "contains" as MatchMode,
  label: "",
});

const editRule = ref({
  categoryId: 0,
  matchType: "process" as MatchType,
  matchValue: "",
  matchMode: "contains" as MatchMode,
  label: "",
});

const filteredRules = computed(() => rules.value);

// 下拉选项（跟随语言 / 分类列表变化）
const matchModeOptions = computed(() => [
  { value: "contains", label: t("rules.contains") },
  { value: "exact", label: t("rules.exactMatch") },
  { value: "regex", label: t("rules.regexMatch") },
]);

const categoryOptions = computed(() =>
  categories.value.map((c) => ({ value: String(c.id), label: categoryLabel(c.name) })),
);

function setMatchMode(target: "new" | "edit", value: string) {
  if (target === "new") newRule.value.matchMode = value as MatchMode;
  else editRule.value.matchMode = value as MatchMode;
}

function setCategoryId(target: "new" | "edit", value: string) {
  if (target === "new") newRule.value.categoryId = Number(value);
  else editRule.value.categoryId = Number(value);
}

async function loadData() {
  loading.value = true;
  try {
    const [rulesData, catsData] = await Promise.all([
      getRules(),
      getCategories(),
    ]);
    rules.value = rulesData;
    categories.value = catsData;
    if (catsData.length > 0 && newRule.value.categoryId === 0) {
      newRule.value.categoryId = catsData[0].id;
    }
  } catch (e) {
    console.error("加载规则失败", e);
  } finally {
    loading.value = false;
  }
}

async function handleAddRule() {
  if (!newRule.value.matchValue.trim()) return;
  try {
    await createRule({
      categoryId: newRule.value.categoryId,
      matchType: newRule.value.matchType,
      matchValue: newRule.value.matchValue.trim(),
      matchMode: newRule.value.matchMode,
      label: newRule.value.label.trim() || null,
    });
    newRule.value.matchValue = "";
    newRule.value.label = "";
    showAddModal.value = false;
    await loadData();
  } catch (e) {
    console.error("添加规则失败", e);
  }
}

async function handleToggle(id: number) {
  try {
    await toggleRule(id);
    await loadData();
  } catch (e) {
    console.error("切换规则失败", e);
  }
}

function handleEdit(rule: AppRule) {
  editingId.value = rule.id;
  editRule.value = {
    categoryId: rule.categoryId,
    matchType: rule.matchType,
    matchValue: rule.matchValue,
    matchMode: rule.matchMode,
    label: rule.label || "",
  };
  showEditModal.value = true;
}

async function handleSaveEdit() {
  if (editingId.value === null) return;
  try {
    await updateRule(editingId.value, {
      categoryId: editRule.value.categoryId,
      matchType: editRule.value.matchType,
      matchValue: editRule.value.matchValue.trim(),
      matchMode: editRule.value.matchMode,
      label: editRule.value.label.trim() || null,
    });
    showEditModal.value = false;
    editingId.value = null;
    await loadData();
  } catch (e) {
    console.error("更新规则失败", e);
  }
}


async function requestDelete(id: number) {
  const ok = await confirmDialog({
    title: t("rules.deleteRule"),
    message: t("rules.deleteConfirm"),
    confirmText: t("common.delete"),
  });
  if (!ok) return;
  try {
    await deleteRule(id);
    await loadData();
  } catch (e) {
    console.error("删除规则失败", e);
  }
}

function getCategoryColor(catId: number): string {
  return categories.value.find((c) => c.id === catId)?.color || "#6B7280";
}

function getCategoryName(catId: number): string {
  return categories.value.find((c) => c.id === catId)?.name || t("common.unknown");
}

function getMatchTypeLabel(type: string): string {
  return type === "process" ? t("rules.process") : t("rules.windowTitle");
}

function getMatchModeLabel(mode: string): string {
  switch (mode) {
    case "exact":
      return t("rules.exact");
    case "contains":
      return t("rules.contains");
    case "regex":
      return t("rules.regex");
    default:
      return mode;
  }
}

async function moveRuleUp(index: number) {
  if (index <= 0) return;
  const ids = filteredRules.value.map((r) => r.id);
  [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]];
  try {
    await reorderRules(ids);
    await loadData();
  } catch (e) {
    console.error("移动规则失败", e);
  }
}

async function moveRuleDown(index: number) {
  if (index >= filteredRules.value.length - 1) return;
  const ids = filteredRules.value.map((r) => r.id);
  [ids[index], ids[index + 1]] = [ids[index + 1], ids[index]];
  try {
    await reorderRules(ids);
    await loadData();
  } catch (e) {
    console.error("移动规则失败", e);
  }
}

onMounted(() => {
  loadData();
});
</script>

<template>
  <div class="rules-page">
    <div class="page-header">
      <div>
        <h2 class="page-title">{{ t('rules.title') }}</h2>
        <p class="page-subtitle">{{ t('rules.total', { count: rules.length }) }}</p>
      </div>
      <button class="btn-primary" @click="showAddModal = true">
        <span>+</span> {{ t('rules.addRule') }}
      </button>
    </div>

    <div v-if="loading" class="loading">{{ t('common.loading') }}</div>

    <div v-else class="rules-list">
      <div
        v-for="(rule, index) in filteredRules"
        :key="rule.id"
        class="rule-item"
        :class="{ disabled: !rule.enabled }"
      >
        <div class="rule-main">
          <div class="rule-match">
            <span class="match-type">{{ getMatchTypeLabel(rule.matchType) }}</span>
            <span class="match-mode">{{ getMatchModeLabel(rule.matchMode) }}</span>
            <span class="match-value">{{ rule.matchValue }}</span>
          </div>
          <span
            class="cat-tag"
            :style="{ background: getCategoryColor(rule.categoryId) + '33', color: getCategoryColor(rule.categoryId) }"
          >
            {{ categoryLabel(rule.categoryName || getCategoryName(rule.categoryId)) }}
          </span>
        </div>
        <div class="rule-actions">
          <button class="action-btn move" @click="moveRuleUp(index)" :disabled="index === 0" :title="t('common.moveUp')">
            <ChevronUp :size="14" :stroke-width="1.8" />
          </button>
          <button class="action-btn move" @click="moveRuleDown(index)" :disabled="index === filteredRules.length - 1" :title="t('common.moveDown')">
            <ChevronDown :size="14" :stroke-width="1.8" />
          </button>
          <button class="action-btn edit" @click="handleEdit(rule)" :title="t('rules.edit')">
            <Pencil :size="14" :stroke-width="1.8" />
          </button>
          <button class="action-btn toggle" @click="handleToggle(rule.id)">
            {{ rule.enabled ? t('rules.disable') : t('rules.enable') }}
          </button>
          <button class="action-btn delete" @click="requestDelete(rule.id)" :title="t('rules.delete')">
            <Trash2 :size="14" :stroke-width="1.8" />
          </button>
        </div>
      </div>
    </div>

    <!-- 新增规则弹窗 -->
    <div v-if="showAddModal" class="modal-overlay">
      <div class="modal-content">
        <h3 class="modal-title">{{ t('rules.addRule') }}</h3>

        <div class="form-group">
          <label>{{ t('rules.matchType') }}</label>
          <div class="radio-group">
            <label class="radio-item">
              <input type="radio" v-model="newRule.matchType" value="process" />
              <span class="radio-label">{{ t('rules.process') }}</span>
            </label>
            <label class="radio-item">
              <input type="radio" v-model="newRule.matchType" value="title" />
              <span class="radio-label">{{ t('rules.windowTitle') }}</span>
            </label>
          </div>
        </div>

        <div class="form-group">
          <label>{{ t('rules.matchMode') }}</label>
          <GlassSelect
            :model-value="newRule.matchMode"
            :options="matchModeOptions"
            block
            @update:model-value="(v) => setMatchMode('new', v)"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.matchValue') }}</label>
          <input
            v-model="newRule.matchValue"
            type="text"
            class="form-input"
            :placeholder="t('rules.matchValuePlaceholder')"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.displayName') }}</label>
          <input
            v-model="newRule.label"
            type="text"
            class="form-input"
            :placeholder="t('rules.displayNameHint')"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.assignCategory') }}</label>
          <GlassSelect
            :model-value="String(newRule.categoryId)"
            :options="categoryOptions"
            block
            @update:model-value="(v) => setCategoryId('new', v)"
          />
        </div>

        <div class="modal-actions">
          <button class="btn-secondary" @click="showAddModal = false">{{ t('common.cancel') }}</button>
          <button class="btn-primary" @click="handleAddRule">{{ t('common.add') }}</button>
        </div>
      </div>
    </div>

    <!-- 编辑规则弹窗 -->
    <div v-if="showEditModal" class="modal-overlay">
      <div class="modal-content">
        <h3 class="modal-title">{{ t('rules.editRule') }}</h3>

        <div class="form-group">
          <label>{{ t('rules.matchType') }}</label>
          <div class="radio-group">
            <label class="radio-item">
              <input type="radio" v-model="editRule.matchType" value="process" />
              <span class="radio-label">{{ t('rules.process') }}</span>
            </label>
            <label class="radio-item">
              <input type="radio" v-model="editRule.matchType" value="title" />
              <span class="radio-label">{{ t('rules.windowTitle') }}</span>
            </label>
          </div>
        </div>

        <div class="form-group">
          <label>{{ t('rules.matchMode') }}</label>
          <GlassSelect
            :model-value="editRule.matchMode"
            :options="matchModeOptions"
            block
            @update:model-value="(v) => setMatchMode('edit', v)"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.matchValue') }}</label>
          <input
            v-model="editRule.matchValue"
            type="text"
            class="form-input"
            :placeholder="t('rules.matchValuePlaceholder')"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.displayName') }}</label>
          <input
            v-model="editRule.label"
            type="text"
            class="form-input"
            :placeholder="t('rules.displayNameHint')"
          />
        </div>

        <div class="form-group">
          <label>{{ t('rules.assignCategory') }}</label>
          <GlassSelect
            :model-value="String(editRule.categoryId)"
            :options="categoryOptions"
            block
            @update:model-value="(v) => setCategoryId('edit', v)"
          />
        </div>

        <div class="modal-actions">
          <button class="btn-secondary" @click="showEditModal = false">{{ t('common.cancel') }}</button>
          <button class="btn-primary" @click="handleSaveEdit">{{ t('common.save') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rules-page {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
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

.btn-primary {
  padding: 8px 14px;
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(139, 92, 246, 0.3);
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 4px;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 18px rgba(139, 92, 246, 0.5);
}

.btn-secondary {
  padding: 8px 16px;
  background: var(--bg-card);
  color: var(--text-secondary);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-secondary:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-glass-strong);
  color: var(--text-primary);
}

.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
  font-size: 13px;
}

.rules-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.rule-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-glass);
  border-radius: 10px;
  transition: all 0.15s ease;
}

.rule-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.rule-item.disabled {
  opacity: 0.5;
}

.rule-main {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

.rule-match {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.match-type {
  font-size: 10px;
  padding: 2px 6px;
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
  border-radius: 4px;
  font-weight: 500;
}

.match-mode {
  font-size: 10px;
  padding: 2px 6px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-tertiary);
  border-radius: 4px;
}

.match-value {
  font-size: 13px;
  color: var(--text-primary);
  font-family: monospace;
  font-weight: 500;
}

.cat-tag {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  flex-shrink: 0;
  margin-right: 8px;
}

.rule-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.action-btn {
  padding: 4px 10px;
  font-size: 11px;
  border: 1px solid var(--border-glass);
  background: transparent;
  color: var(--text-secondary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 26px;
}

.action-btn.edit,
.action-btn.delete {
  width: 26px;
  padding: 0;
}

.action-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.action-btn.delete:hover {
  background: rgba(239, 68, 68, 0.15);
  color: #f87171;
  border-color: rgba(239, 68, 68, 0.3);
}

.action-btn.move {
  width: 26px;
  padding: 0;
  color: var(--text-tertiary);
}

.action-btn.move:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.action-btn.move:not(:disabled):hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.action-btn.edit:hover {
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
  border-color: rgba(59, 130, 246, 0.3);
}

/* 弹窗 */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(8, 10, 18, 0.38);
  backdrop-filter: blur(8px) saturate(120%);
  -webkit-backdrop-filter: blur(8px) saturate(120%);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-content {
  position: relative;
  width: 320px;
  background: var(--bg-glass);
  backdrop-filter: blur(60px) saturate(190%);
  -webkit-backdrop-filter: blur(60px) saturate(190%);
  border: 1px solid var(--border-glass-strong);
  border-radius: var(--radius-lg);
  padding: 22px 20px 18px;
  box-shadow: var(--shadow-glass), 0 0 0 1px rgba(139, 92, 246, 0.08);
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
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
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

/* 仅作用于表单字段标题，避免覆盖 .radio-item 的 flex 布局 */
.form-group > label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.form-input {
  width: 100%;
  padding: 9px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
  box-sizing: border-box;
}

.form-input:hover {
  background: var(--bg-card-hover);
}

.form-input:focus {
  border-color: rgba(139, 92, 246, 0.55);
  background: var(--bg-card-hover);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

.radio-group {
  display: flex;
  align-items: center;
  gap: 16px;
}

.radio-item {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 12px;
  line-height: 1;
  color: var(--text-secondary);
  cursor: pointer;
}

.radio-item input[type="radio"] {
  margin: 0;
  display: block;
  flex-shrink: 0;
  cursor: pointer;
}

.radio-item .radio-label {
  line-height: 1.2;
  white-space: nowrap;
}

.modal-actions {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
}

.confirm-text {
  position: relative;
  z-index: 1;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
  margin: 0;
}

.btn-danger {
  padding: 8px 16px;
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 14px rgba(239, 68, 68, 0.25);
  transition: all 0.2s ease;
}

.btn-danger:hover {
  box-shadow: 0 6px 18px rgba(239, 68, 68, 0.45);
  transform: translateY(-1px);
}
</style>
