<script setup lang="ts">
import { ref, onMounted, computed, markRaw } from "vue";
import { getRules, getCategories, createRule, deleteRule, toggleRule, updateRule } from "../api";
import type { AppRule, Category, MatchMode, MatchType } from "../api/types";
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
});

const editRule = ref({
  categoryId: 0,
  matchType: "process" as MatchType,
  matchValue: "",
  matchMode: "contains" as MatchMode,
});

const filteredRules = computed(() => rules.value);

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
    });
    newRule.value.matchValue = "";
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
  };
  showEditModal.value = true;
}

async function handleSaveEdit() {
  if (editingId.value === null) return;
  try {
    await updateRule(editingId.value, editRule.value);
    showEditModal.value = false;
    editingId.value = null;
    await loadData();
  } catch (e) {
    console.error("更新规则失败", e);
  }
}

async function handleDelete(id: number) {
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
  return categories.value.find((c) => c.id === catId)?.name || "未知";
}

function getMatchTypeLabel(type: string): string {
  return type === "process" ? "进程名" : "窗口标题";
}

function getMatchModeLabel(mode: string): string {
  switch (mode) {
    case "exact":
      return "精确";
    case "contains":
      return "包含";
    case "regex":
      return "正则";
    default:
      return mode;
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
        <h2 class="page-title">规则管理</h2>
        <p class="page-subtitle">共 {{ rules.length }} 条规则</p>
      </div>
      <button class="btn-primary" @click="showAddModal = true">
        <span>+</span> 新增规则
      </button>
    </div>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else class="rules-list">
      <div
        v-for="rule in filteredRules"
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
            {{ rule.categoryName || getCategoryName(rule.categoryId) }}
          </span>
        </div>
        <div class="rule-actions">
          <button class="action-btn edit" @click="handleEdit(rule)" title="编辑">
            <Pencil :size="14" :stroke-width="1.8" />
          </button>
          <button class="action-btn toggle" @click="handleToggle(rule.id)">
            {{ rule.enabled ? "停用" : "启用" }}
          </button>
          <button class="action-btn delete" @click="handleDelete(rule.id)" title="删除">
            <Trash2 :size="14" :stroke-width="1.8" />
          </button>
        </div>
      </div>
    </div>

    <!-- 新增规则弹窗 -->
    <div v-if="showAddModal" class="modal-overlay" @click.self="showAddModal = false">
      <div class="modal-content">
        <h3 class="modal-title">新增规则</h3>

        <div class="form-group">
          <label>匹配类型</label>
          <div class="radio-group">
            <label class="radio-item">
              <input type="radio" v-model="newRule.matchType" value="process" />
              <span class="radio-label">进程名</span>
            </label>
            <label class="radio-item">
              <input type="radio" v-model="newRule.matchType" value="title" />
              <span class="radio-label">窗口标题</span>
            </label>
          </div>
        </div>

        <div class="form-group">
          <label>匹配方式</label>
          <select v-model="newRule.matchMode" class="form-select">
            <option value="contains">包含</option>
            <option value="exact">精确匹配</option>
            <option value="regex">正则表达式</option>
          </select>
        </div>

        <div class="form-group">
          <label>匹配值</label>
          <input
            v-model="newRule.matchValue"
            type="text"
            class="form-input"
            placeholder="如：Code.exe 或 掘金"
          />
        </div>

        <div class="form-group">
          <label>归属分类</label>
          <select v-model="newRule.categoryId" class="form-select">
            <option v-for="cat in categories" :key="cat.id" :value="cat.id">
              {{ cat.name }}
            </option>
          </select>
        </div>

        <div class="modal-actions">
          <button class="btn-secondary" @click="showAddModal = false">取消</button>
          <button class="btn-primary" @click="handleAddRule">添加</button>
        </div>
      </div>
    </div>

    <!-- 编辑规则弹窗 -->
    <div v-if="showEditModal" class="modal-overlay" @click.self="showEditModal = false">
      <div class="modal-content">
        <h3 class="modal-title">编辑规则</h3>

        <div class="form-group">
          <label>匹配类型</label>
          <div class="radio-group">
            <label class="radio-item">
              <input type="radio" v-model="editRule.matchType" value="process" />
              <span class="radio-label">进程名</span>
            </label>
            <label class="radio-item">
              <input type="radio" v-model="editRule.matchType" value="title" />
              <span class="radio-label">窗口标题</span>
            </label>
          </div>
        </div>

        <div class="form-group">
          <label>匹配方式</label>
          <select v-model="editRule.matchMode" class="form-select">
            <option value="contains">包含</option>
            <option value="exact">精确匹配</option>
            <option value="regex">正则表达式</option>
          </select>
        </div>

        <div class="form-group">
          <label>匹配值</label>
          <input
            v-model="editRule.matchValue"
            type="text"
            class="form-input"
            placeholder="如：Code.exe 或 掘金"
          />
        </div>

        <div class="form-group">
          <label>归属分类</label>
          <select v-model="editRule.categoryId" class="form-select">
            <option v-for="cat in categories" :key="cat.id" :value="cat.id">
              {{ cat.name }}
            </option>
          </select>
        </div>

        <div class="modal-actions">
          <button class="btn-secondary" @click="showEditModal = false">取消</button>
          <button class="btn-primary" @click="handleSaveEdit">保存</button>
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
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 4px;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.4);
}

.btn-secondary {
  padding: 8px 14px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-secondary);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.12);
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

.action-btn.edit:hover {
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
  border-color: rgba(59, 130, 246, 0.3);
}

/* 弹窗 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-content {
  width: 320px;
  background: var(--bg-glass);
  backdrop-filter: blur(50px) saturate(200%);
  border: 1px solid var(--border-glass-strong);
  border-radius: 14px;
  padding: 20px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}

.modal-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 16px 0;
}

.form-group {
  margin-bottom: 14px;
}

/* 仅作用于表单字段标题，避免覆盖 .radio-item 的 flex 布局 */
.form-group > label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.form-input,
.form-select {
  width: 100%;
  padding: 8px 10px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--border-glass);
  border-radius: 8px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: all 0.15s ease;
  box-sizing: border-box;
}

.form-select option {
  background: #1e1b2e;
  color: #e5e7eb;
}

.form-input:focus,
.form-select:focus {
  border-color: rgba(139, 92, 246, 0.5);
  background: rgba(255, 255, 255, 0.08);
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
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}
</style>
