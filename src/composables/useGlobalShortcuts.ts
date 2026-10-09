import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { invoke } from "@tauri-apps/api/core";
import { getGeneralSettings } from "../api";

/** 默认快捷键（与后端 GeneralSettings 的默认值保持一致） */
export const DEFAULT_SHORTCUT_TOGGLE_WINDOW = "Ctrl+Shift+T";
export const DEFAULT_SHORTCUT_TOGGLE_POMODORO = "Ctrl+Shift+P";

/** 已注册的快捷键，便于切换配置时先注销 */
let registeredAccelerators: string[] = [];

// 走后端命令（和托盘左键同一套逻辑），避免前端窗口 API 权限不足导致静默失败
function toggleMainWindow() {
  invoke("toggle_main_window").catch(() => {});
}

function togglePomodoro() {
  invoke("toggle_pomodoro_focus").catch(() => {});
}

async function apply(bindings: Array<{ accelerator: string; handler: () => void }>) {
  // 先注销旧的，避免「HotKey already registered」
  for (const acc of registeredAccelerators) {
    try {
      await unregister(acc);
    } catch {
      // 未注册过/已被系统释放，忽略
    }
  }
  registeredAccelerators = [];

  const failed: string[] = [];
  for (const { accelerator, handler } of bindings) {
    const acc = (accelerator || "").trim();
    if (!acc) continue; // 留空表示不注册该快捷键
    try {
      await register(acc, (event) => {
        if (event.state === "Pressed") handler();
      });
      registeredAccelerators.push(acc);
    } catch (e) {
      failed.push(acc);
      console.warn(`注册全局快捷键失败：${acc}`, e);
    }
  }
  return failed;
}

/**
 * 按当前设置注册全局快捷键，返回注册失败的快捷键列表（例如被其它软件占用）。
 * 应用启动与「设置」保存后都会调用。
 */
export async function refreshGlobalShortcuts(): Promise<string[]> {
  const settings = await getGeneralSettings().catch(() => null);
  return apply([
    {
      accelerator: settings?.shortcutToggleWindow || DEFAULT_SHORTCUT_TOGGLE_WINDOW,
      handler: toggleMainWindow,
    },
    {
      accelerator: settings?.shortcutTogglePomodoro || DEFAULT_SHORTCUT_TOGGLE_POMODORO,
      handler: togglePomodoro,
    },
  ]);
}

/**
 * 录制快捷键时临时注销，避免按下已占用的组合键触发 OS 级热键（例如录制时窗口突然被隐藏）。
 */
export async function suspendGlobalShortcuts() {
  for (const acc of registeredAccelerators) {
    try {
      await unregister(acc);
    } catch {
      // 忽略
    }
  }
  registeredAccelerators = [];
}
