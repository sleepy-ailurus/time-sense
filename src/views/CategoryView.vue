<script setup lang="ts">
import { ref, onMounted, markRaw, computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  getCategories,
  getTodayCategoryStats,
  createCategory,
  updateCategory,
  deleteCategory,
} from "../api";
import type { Category, CategoryStat } from "../api/types";
import { formatDuration } from "../utils/format";
import { categoryLabel } from "../utils/categoryName";
import { confirmDialog } from "../composables/useConfirm";
import {
  BarChart3,
  Briefcase,
  BookOpen,
  Gamepad2,
  MessageCircle,
  MoreHorizontal,
  Folder,
  Plus,
  Pencil,
  Trash2,
} from "lucide-vue-next";

const iconMap: Record<string, any> = {
  Briefcase: markRaw(Briefcase),
  BookOpen: markRaw(BookOpen),
  Gamepad2: markRaw(Gamepad2),
  MessageCircle: markRaw(MessageCircle),
  MoreHorizontal: markRaw(MoreHorizontal),
  Folder: markRaw(Folder),
};

const { t } = useI18n();

const iconOptions = ["Briefcase", "BookOpen", "Gamepad2", "MessageCircle", "MoreHorizontal", "Folder"];

function getCategoryIcon(name?: string | null) {
  return iconMap[name || ""] || Folder;
}

// 预设颜色
const colorOptions = [
  "#3B82F6", "#10B981", "#EF4444", "#8B5CF6", "#F59E0B",
  "#EC4899", "#06B6D4", "#84CC16", "#F97316", "#6366F1",
  "#14B8A6", "#E11D48", "#A855F7", "#0EA5E9", "#D946EF",
];

// 数据
const categories = ref<Category[]>([]);
const categoryStats = ref<CategoryStat[]>([]);
const loading = ref(false);
const activeTab = ref<"stats" | "manage">("stats");

// 弹窗
const showAddModal = ref(false);
const showEditModal = ref(false);
const editingId = ref<number | null>(null);

const addForm = ref({ name: "", color: "#3B82F6", icon: "Folder", isFocus: false });
const editForm = ref({ name: "", color: "#3B82F6", icon: "Folder", isFocus: false });

// 分类统计合并（给管理页用：显示每个分类的今日时长）
const categoryMap = computed(() => {
  const map = new Map<number, CategoryStat>();
  for (const s of categoryStats.value) {
    map.set(s.categoryId, s);
  }
  return map;
});

async function loadData() {
  loading.value = true;
  try {
    const [cats, stats] = await Promise.all([
      getCategories(),
      getTodayCategoryStats(),
    ]);
    categories.value = cats;
    categoryStats.value = stats;
  } catch (e) {
    console.error("加载分类数据失败", e);
  } finally {
    loading.value = false;
  }
}

function getBarColor(stat: CategoryStat): string {
  return stat.categoryColor || "#6B7280";
}

// ---- CRUD ----

function openAddModal() {
  addForm.value = { name: "", color: "#3B82F6", icon: "Folder", isFocus: false };
  showAddModal.value = true;
}

async function handleAdd() {
  if (!addForm.value.name.trim()) return;
  try {
    await createCategory({
      name: addForm.value.name.trim(),
      color: addForm.value.color,
      icon: addForm.value.icon,
      isFocus: addForm.value.isFocus,
    });
    showAddModal.value = false;
    await loadData();
  } catch (e) {
    console.error("新增分类失败", e);
  }
}

function openEditModal(cat: Category) {
  editingId.value = cat.id;
  editForm.value = {
    name: cat.name,
    color: cat.color,
    icon: cat.icon || "Folder",
    isFocus: !!cat.isFocus,
  };
  showEditModal.value = true;
}

async function handleSaveEdit() {
  if (editingId.value === null || !editForm.value.name.trim()) return;
  try {
    await updateCategory({
      id: editingId.value,
      name: editForm.value.name.trim(),
      color: editForm.value.color,
      icon: editForm.value.icon,
      isFocus: editForm.value.isFocus,
    });
    showEditModal.value = false;
    editingId.value = null;
    await loadData();
  } catch (e) {
    console.error("更新分类失败", e);
  }
}

async function handleDelete(cat: Category) {
  if (cat.isDefault) return;
  const ok = await confirmDialog({
    title: t("category.delete"),
    message: t("category.deleteConfirm", { name: categoryLabel(cat.name) }),
    confirmText: t("category.delete"),
  });
  if (!ok) return;
  try {
    await deleteCategory(cat.id);
    await loadData();
  } catch (e) {
    console.error("删除分类失败", e);
  }
}

onMounted(() => {
  loadData();
});
</script>

<template>
  <div class="category-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('category.title') }}</h2>
      <div class="tab-bar">
        <button class="tab-btn" :class="{ active: activeTab === 'stats' }" @click="activeTab = 'stats'">
          {{ t('category.todayStats') }}
        </button>
        <button class="tab-btn" :class="{ active: activeTab === 'manage' }" @click="activeTab = 'manage'">
          {{ t('category.manage') }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="loading">{{ t('common.loading') }}</div>

    <!-- ====== 统计视图 ====== -->
    <template v-else-if="activeTab === 'stats'">
      <div v-if="categoryStats.length === 0" class="empty-state">
        <BarChart3 class="empty-icon" :size="48" :stroke-width="1.2" />
        <div class="empty-text">{{ t('common.noData') }}</div>
        <div class="empty-desc">{{ t('category.useForAWhile') }}</div>
      </div>

      <div v-else class="category-list">
        <div
          v-for="(stat, index) in categoryStats"
          :key="stat.categoryId"
          class="category-item"
        >
          <div class="category-header">
            <div class="category-left">
              <span class="category-rank">{{ index + 1 }}</span>
              <component :is="getCategoryIcon(stat.categoryIcon)" class="category-icon" :size="18" :stroke-width="1.8" :style="{ color: getBarColor(stat) }" />
        <span class="category-name">{{ categoryLabel(stat.categoryName) }}</span>
            </div>
            <div class="category-right">
              <span class="category-duration">{{ formatDuration(stat.totalSeconds) }}</span>
              <span class="category-percent">{{ stat.percentage.toFixed(1) }}%</span>
            </div>
          </div>
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{
                width: stat.percentage + '%',
                background: `linear-gradient(90deg, ${getBarColor(stat)}88, ${getBarColor(stat)})`,
                boxShadow: `0 0 10px ${getBarColor(stat)}44`,
              }"
            ></div>
          </div>
        </div>
      </div>
    </template>

    <!-- ====== 管理视图 ====== -->
    <template v-else>
      <button class="add-btn" @click="openAddModal">
        <Plus :size="16" :stroke-width="2" />
        {{ t('category.addCategory') }}
      </button>

      <div v-if="categories.length === 0" class="empty-state">
        <Folder class="empty-icon" :size="48" :stroke-width="1.2" />
        <div class="empty-text">{{ t('category.noData') }}</div>
      </div>

      <div v-else class="manage-list">
        <div
          v-for="cat in categories"
          :key="cat.id"
          class="manage-item"
        >
          <div class="manage-left">
            <span class="manage-color-dot" :style="{ background: cat.color }"></span>
            <component :is="getCategoryIcon(cat.icon)" class="manage-icon" :size="16" :stroke-width="1.8" :style="{ color: cat.color }" />
            <span class="manage-name">{{ categoryLabel(cat.name) }}</span>
            <span v-if="cat.isDefault" class="manage-badge">{{ t('category.system') }}</span>
            <span v-else class="manage-badge custom">{{ t('category.custom') }}</span>
          </div>
          <div class="manage-right">
            <span class="manage-time">
              {{ formatDuration(categoryMap.get(cat.id)?.totalSeconds || 0) }}
            </span>
            <button class="manage-btn edit" @click="openEditModal(cat)" :title="t('category.edit')">
              <Pencil :size="14" :stroke-width="1.8" />
            </button>
            <button
              v-if="!cat.isDefault"
              class="manage-btn delete"
              @click="handleDelete(cat)"
              :title="t('category.delete')"
            >
              <Trash2 :size="14" :stroke-width="1.8" />
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- ====== 新增弹窗 ====== -->
    <Teleport to="body">
      <div v-if="showAddModal" class="modal-overlay">
        <div class="modal">
          <h3 class="modal-title">{{ t('category.addCategory') }}</h3>
          <div class="form-group">
            <label>{{ t('category.name') }}</label>
            <input v-model="addForm.name" type="text" class="form-input" :placeholder="t('category.namePlaceholder')" maxlength="20" />
          </div>
          <div class="form-group">
            <label>
              {{ t('category.color') }}
              <span class="color-hex">{{ addForm.color.toUpperCase() }}</span>
            </label>
            <div class="color-grid">
              <button
                v-for="c in colorOptions"
                :key="c"
                class="color-swatch"
                :class="{ active: addForm.color === c }"
                :style="{ background: c }"
                @click="addForm.color = c"
              ></button>
              <!-- 自定义取色：点击弹出系统取色器 -->
              <label
                class="color-swatch custom"
                :class="{ active: !colorOptions.includes(addForm.color) }"
                :style="{ '--picked': addForm.color }"
                :title="t('category.customColor')"
              >
                <input type="color" v-model="addForm.color" class="color-input" />
              </label>
            </div>
          </div>
          <div class="form-group">
            <label>{{ t('category.icon') }}</label>
            <div class="icon-grid">
              <button
                v-for="ic in iconOptions"
                :key="ic"
                class="icon-swatch"
                :class="{ active: addForm.icon === ic }"
                @click="addForm.icon = ic"
              >
                <component :is="getCategoryIcon(ic)" :size="18" :stroke-width="1.8" />
              </button>
            </div>
          </div>
          <div class="form-group">
            <div class="switch-row">
              <div class="switch-info">
                <span class="switch-name">{{ t('category.focusCounted') }}</span>
                <span class="switch-desc">{{ t('category.focusCountedDesc') }}</span>
              </div>
              <label class="switch">
                <input type="checkbox" v-model="addForm.isFocus" />
                <span class="slider"></span>
              </label>
            </div>
          </div>
          <div class="modal-actions">
            <button class="btn-secondary" @click="showAddModal = false">{{ t('category.cancel') }}</button>
            <button class="btn-primary" @click="handleAdd">{{ t('category.add') }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ====== 编辑弹窗 ====== -->
    <Teleport to="body">
      <div v-if="showEditModal" class="modal-overlay">
        <div class="modal">
          <h3 class="modal-title">{{ t('category.editCategory') }}</h3>
          <div class="form-group">
            <label>{{ t('category.name') }}</label>
            <input v-model="editForm.name" type="text" class="form-input" maxlength="20" />
          </div>
          <div class="form-group">
            <label>
              {{ t('category.color') }}
              <span class="color-hex">{{ editForm.color.toUpperCase() }}</span>
            </label>
            <div class="color-grid">
              <button
                v-for="c in colorOptions"
                :key="c"
                class="color-swatch"
                :class="{ active: editForm.color === c }"
                :style="{ background: c }"
                @click="editForm.color = c"
              ></button>
              <!-- 自定义取色：点击弹出系统取色器 -->
              <label
                class="color-swatch custom"
                :class="{ active: !colorOptions.includes(editForm.color) }"
                :style="{ '--picked': editForm.color }"
                :title="t('category.customColor')"
              >
                <input type="color" v-model="editForm.color" class="color-input" />
              </label>
            </div>
          </div>
          <div class="form-group">
            <label>{{ t('category.icon') }}</label>
            <div class="icon-grid">
              <button
                v-for="ic in iconOptions"
                :key="ic"
                class="icon-swatch"
                :class="{ active: editForm.icon === ic }"
                @click="editForm.icon = ic"
              >
                <component :is="getCategoryIcon(ic)" :size="18" :stroke-width="1.8" />
              </button>
            </div>
          </div>
          <div class="form-group">
            <div class="switch-row">
              <div class="switch-info">
                <span class="switch-name">{{ t('category.focusCounted') }}</span>
                <span class="switch-desc">{{ t('category.focusCountedDesc') }}</span>
              </div>
              <label class="switch">
                <input type="checkbox" v-model="editForm.isFocus" />
                <span class="slider"></span>
              </label>
            </div>
          </div>
          <div class="modal-actions">
            <button class="btn-secondary" @click="showEditModal = false">{{ t('category.cancel') }}</button>
            <button class="btn-primary" @click="handleSaveEdit">{{ t('category.save') }}</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.category-page {
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
  margin: 0 0 10px 0;
}

/* Tab bar */
.tab-bar {
  display: flex;
  gap: 4px;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 8px;
  padding: 3px;
  width: fit-content;
}

.tab-btn {
  padding: 5px 14px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.tab-btn:hover {
  color: var(--text-secondary);
}

.tab-btn.active {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}

/* Loading / Empty */
.loading {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
  font-size: 13px;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 40px;
  opacity: 0.5;
}

.empty-text {
  font-size: 14px;
  color: var(--text-secondary);
}

.empty-desc {
  font-size: 12px;
  color: var(--text-tertiary);
}

/* ====== Stats View ====== */
.category-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.category-item {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--border-glass);
  border-radius: 12px;
  padding: 12px 14px;
  transition: all 0.2s ease;
}

.category-item:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: var(--border-glass-strong);
}

.category-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.category-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.category-rank {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  background: rgba(255, 255, 255, 0.08);
  border-radius: 6px;
}

.category-icon {
  font-size: 18px;
}

.category-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.category-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.category-duration {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.category-percent {
  font-size: 11px;
  color: var(--text-tertiary);
  min-width: 40px;
  text-align: right;
}

.progress-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.5s ease;
}

/* ====== Manage View ====== */
.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: 1px dashed var(--border-glass-strong);
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s ease;
  margin-bottom: 12px;
}

.add-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: rgba(139, 92, 246, 0.06);
}

.manage-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.manage-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border-glass);
  border-radius: 10px;
  transition: all 0.15s ease;
}

.manage-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.manage-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.manage-color-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.manage-icon {
  flex-shrink: 0;
}

.manage-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.manage-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-tertiary);
  flex-shrink: 0;
}

/* 自建分类：紫色描边样式，和系统预置的灰色徽标区分开 */
.manage-badge.custom {
  background: rgba(139, 92, 246, 0.16);
  border: 1px solid rgba(139, 92, 246, 0.35);
  color: var(--accent, #8b5cf6);
}

.manage-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.manage-time {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: monospace;
  min-width: 40px;
  text-align: right;
}

.manage-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.manage-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.manage-btn.delete:hover {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

/* ====== Modal ====== */
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

.modal {
  position: relative;
  background: var(--bg-glass);
  backdrop-filter: blur(60px) saturate(190%);
  -webkit-backdrop-filter: blur(60px) saturate(190%);
  border: 1px solid var(--border-glass-strong);
  border-radius: var(--radius-lg);
  padding: 22px 20px 18px;
  width: 320px;
  max-width: 90vw;
  box-shadow: var(--shadow-glass), 0 0 0 1px rgba(139, 92, 246, 0.08);
  overflow: hidden;
}

/* 顶部高光，跟主面板保持同一套玻璃质感 */
.modal::before {
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
.modal::after {
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

.color-hex {
  margin-left: 6px;
  font-size: 10px;
  color: var(--text-tertiary);
  font-family: monospace;
  letter-spacing: 0.5px;
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
}

.form-input:hover {
  background: var(--bg-card-hover);
}

.form-input:focus {
  border-color: rgba(139, 92, 246, 0.55);
  background: var(--bg-card-hover);
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15);
}

.color-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.color-swatch {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  border: 2px solid transparent;
  cursor: pointer;
  transition: all 0.15s ease;
}

.color-swatch:hover {
  transform: scale(1.15);
}

.color-swatch.active {
  border-color: #fff;
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.3);
}

/* 自定义取色器：外圈彩虹环，内圈显示当前颜色 */
.color-swatch.custom {
  position: relative;
  padding: 3px;
  background: conic-gradient(
    from 0deg,
    #f87171,
    #fbbf24,
    #a3e635,
    #34d399,
    #22d3ee,
    #60a5fa,
    #a78bfa,
    #f472b6,
    #f87171
  );
}

.color-swatch.custom::after {
  content: "";
  position: absolute;
  inset: 3px;
  border-radius: 4px;
  background: var(--picked, var(--bg-card));
}

.color-input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  padding: 0;
  border: none;
  opacity: 0;
  cursor: pointer;
}

/* 计入专注时长开关 */
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.switch-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.switch-name {
  font-size: 12px;
  color: var(--text-primary);
}

.switch-desc {
  font-size: 10px;
  color: var(--text-tertiary);
  line-height: 1.4;
}

.switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  inset: 0;
  background-color: rgba(255, 255, 255, 0.15);
  border-radius: 22px;
  transition: 0.2s;
}

.slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 3px;
  bottom: 3px;
  background-color: white;
  border-radius: 50%;
  transition: 0.2s;
}

input:checked + .slider {
  background: linear-gradient(135deg, #8b5cf6, #ec4899);
}

input:checked + .slider:before {
  transform: translateX(18px);
}

.icon-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.icon-swatch {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-glass);
  background: var(--bg-card);
  border-radius: 8px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
}

.icon-swatch:hover {
  background: var(--bg-card-hover);
  color: var(--text-primary);
}

.icon-swatch.active {
  border-color: var(--accent, #8B5CF6);
  background: rgba(139, 92, 246, 0.15);
  color: var(--accent, #8B5CF6);
}

.modal-actions {
  position: relative;
  z-index: 1;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  margin-top: 20px;
}

.btn-secondary {
  padding: 8px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-glass);
  color: var(--text-secondary);
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
