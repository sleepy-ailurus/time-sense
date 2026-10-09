import { reactive } from "vue";

export interface ConfirmOptions {
  /** 标题，缺省用「确定」 */
  title?: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  /** 危险操作（红色确认按钮），默认 true */
  danger?: boolean;
}

/** 全局确认弹窗状态（由 App.vue 里挂载的 GlassConfirm 渲染） */
export const confirmState = reactive({
  show: false,
  title: "",
  message: "",
  confirmText: "",
  cancelText: "",
  danger: true,
});

let resolver: ((ok: boolean) => void) | null = null;

/**
 * 应用内确认弹窗，替代浏览器原生 confirm：
 *
 * ```ts
 * if (!(await confirmDialog({ title: "删除规则", message: "确定删除？" }))) return;
 * ```
 */
export function confirmDialog(options: ConfirmOptions): Promise<boolean> {
  confirmState.title = options.title ?? "";
  confirmState.message = options.message;
  confirmState.confirmText = options.confirmText ?? "";
  confirmState.cancelText = options.cancelText ?? "";
  confirmState.danger = options.danger ?? true;
  confirmState.show = true;

  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

/** 关闭弹窗并回传结果 */
export function resolveConfirm(ok: boolean) {
  confirmState.show = false;
  const fn = resolver;
  resolver = null;
  fn?.(ok);
}
