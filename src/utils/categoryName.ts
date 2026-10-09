import i18n from "../i18n";

/**
 * 内置分类名 → 词条 key。
 * 后端预置分类固定以中文名落库（见 migration.rs 的 preset_categories），
 * 所以这里按中文名映射，展示时再跟随界面语言。
 */
const BUILTIN_CATEGORY_KEYS: Record<string, string> = {
  工作: "category.work",
  学习: "category.study",
  娱乐: "category.entertainment",
  社交: "category.social",
  其他: "category.other",
  游戏: "category.game",
  未分类: "category.uncategorized",
};

/**
 * 展示用分类名：
 * - 内置分类（预置的 5 个 + 游戏/未分类）跟随界面语言；
 * - 用户自建分类保持用户填写的原名。
 */
export function categoryLabel(name?: string | null): string {
  if (!name) return "";
  const key = BUILTIN_CATEGORY_KEYS[name];
  return key ? i18n.global.t(key) : name;
}
