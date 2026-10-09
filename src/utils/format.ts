/**
 * 时间格式化工具
 */

import i18n from "../i18n";

export function formatDuration(seconds: number): string {
  const t = i18n.global.t.bind(i18n.global);
  if (seconds < 60) {
    return t("common.durationSeconds", { n: seconds });
  }
  if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return secs > 0
      ? t("common.durationMinSec", { m: mins, s: secs })
      : t("common.durationMinutes", { m: mins });
  }
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return mins > 0
    ? t("common.durationHourMin", { h: hours, m: mins })
    : t("common.durationHours", { h: hours });
}

/**
 * HTML 转义：进程名/窗口标题等外部输入拼进 ECharts tooltip（innerHTML 渲染）前必须转义
 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
