/**
 * 番茄钟倒计时的时间轴计算（纯函数，便于单测）。
 *
 * 旧实现的两个 ±1 秒 bug：
 * 1. 刷新用的是「上一次 1 秒 tick 的时间戳」tickNow，而重新设定基准用的是 Date.now()。
 *    基准刚重置、下一次 tick 还没来时，`tickNow - baselineAt` 是负数，
 *    `Math.floor(-0.5)` === -1 → 屏幕上凭空多出 1 秒（暂停后点继续就会出现）。
 * 2. 暂停时用 Date.now() 重新算剩余，而显示用的是过期的 tickNow，
 *    两边一减就少 1 秒。
 *
 * 现在统一成：锚点（某时刻的已用毫秒）+ 本地真实流逝时间，显示层只做一次取整。
 * 这样开始、暂停、恢复、阶段切换之间都不会出现 ±1 秒的跳变。
 */

export interface PomodoroTimeline {
  /** 锚点时刻已经过的毫秒数 */
  elapsedMs: number;
  /** 锚点对应的本地时间戳（Date.now()） */
  atMs: number;
  /** 暂停时冻结的已用毫秒数 */
  frozenMs: number;
}

export function createTimeline(): PomodoroTimeline {
  return { elapsedMs: 0, atMs: 0, frozenMs: 0 };
}

/** 把时间轴重新锚定到「此刻已经过 elapsedMs 毫秒」 */
export function anchorTimeline(
  timeline: PomodoroTimeline,
  elapsedMs: number,
  nowMs: number,
): void {
  const safeElapsed = Math.max(Number.isFinite(elapsedMs) ? elapsedMs : 0, 0);
  timeline.elapsedMs = safeElapsed;
  timeline.atMs = nowMs;
  timeline.frozenMs = safeElapsed;
}

/** 当前已用毫秒；paused 为 true 时返回冻结值，不再随时间增长 */
export function elapsedAt(
  timeline: PomodoroTimeline,
  nowMs: number,
  paused: boolean,
): number {
  if (paused) return timeline.frozenMs;
  // 用 max(0, ...) 兜住「采样时间早于锚点」的情况（旧 bug 的正负号问题）
  return timeline.elapsedMs + Math.max(nowMs - timeline.atMs, 0);
}

/** 冻结在当前时刻（暂停用）——冻结值与屏幕上正在显示的数字一致 */
export function freezeTimeline(timeline: PomodoroTimeline, nowMs: number): void {
  timeline.frozenMs = elapsedAt(timeline, nowMs, false);
}

/** 剩余毫秒 */
export function remainingMs(elapsedMs: number, targetMs: number): number {
  return Math.max(targetMs - elapsedMs, 0);
}

/** 显示用的剩余秒：向上取整，起点显示满值、到点显示 0 */
export function displaySeconds(remainMs: number): number {
  return Math.ceil(Math.max(remainMs, 0) / 1000);
}

/**
 * 距离显示数字下一次变化还有多少毫秒。
 * 用来把刷新定时器对齐到秒边界，而不是固定 1 秒轮询，
 * 这样「暂停」抓到的数字和屏幕上的数字才是同一个。
 */
export function msUntilNextTick(remainMs: number): number {
  if (remainMs <= 0) return 1000;
  const mod = remainMs % 1000;
  return mod === 0 ? 1000 : mod;
}

/** 剩余毫秒格式化成 mm:ss */
export function formatRemaining(remainMs: number): string {
  const total = displaySeconds(remainMs);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
