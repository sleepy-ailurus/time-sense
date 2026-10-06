use std::sync::Arc;
use chrono::Utc;
use parking_lot::Mutex;

use crate::db::{Database, PomodoroDao, PomodoroSettings, PomodoroStatus};

/// 番茄钟状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PomodoroPhase {
    Idle,
    Focus,
    ShortBreak,
    LongBreak,
}

/// 当前墙钟时间（毫秒）——计时统一用毫秒，避免秒级截断带来的 ±1 秒误差
fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// 已用毫秒数：暂停时返回冻结值，运行中按墙钟推进
fn elapsed_ms_at(session_start_ms: i64, is_paused: bool, paused_elapsed_ms: i64, now_ms: i64) -> i64 {
    if is_paused {
        paused_elapsed_ms.max(0)
    } else {
        (now_ms - session_start_ms).max(0)
    }
}

/// 番茄钟状态机
pub struct PomodoroEngine {
    db: Arc<Database>,
    inner: Mutex<PomodoroInner>,
}

struct PomodoroInner {
    phase: PomodoroPhase,
    current_session_id: Option<i64>,
    /// 本阶段的开始时刻（墙钟毫秒）
    session_start_ms: i64,
    /// 是否暂停
    is_paused: bool,
    /// 暂停时冻结的已用时长（毫秒）
    paused_elapsed_ms: i64,
    /// 上次会话的信息（停止后展示用）
    last_session_type: Option<String>,
    last_session_duration: i64,
    last_session_target: i64,
    /// 设置
    settings: PomodoroSettings,
    /// 番茄计数（用于判断何时长休息，会话级别，不持久化）
    focus_session_counter: i32,
    /// 自动模式暂停防抖：检测到"应该暂停"的起始时间戳（秒）
    /// 持续 AUTO_PAUSE_DEBOUNCE_SECS 秒后才真正暂停
    auto_pause_pending_since: Option<i64>,
    /// 当前会话是否是用户手动启动的
    /// 手动启动的会话，自动模式不会自动暂停（尊重用户手动操作的意图）
    is_manually_started: bool,
}

impl PomodoroEngine {
    pub fn new(db: Arc<Database>) -> Self {
        // 启动时修复未闭合的番茄钟
        {
            let conn = db.conn().lock();
            match PomodoroDao::fix_open_sessions(&conn) {
                Ok(count) if count > 0 => {
                    tracing::info!("Recovered from unclean shutdown: fixed {} pomodoro sessions", count);
                }
                Err(e) => {
                    tracing::error!("Failed to fix open pomodoro sessions: {}", e);
                }
                _ => {}
            }
        }

        // 从数据库加载设置
        let settings = {
            let conn = db.conn().lock();
            PomodoroDao::load_settings(&conn)
        };
        tracing::info!("Pomodoro settings loaded: auto_mode={}", settings.auto_mode);

        let inner = PomodoroInner {
            phase: PomodoroPhase::Idle,
            current_session_id: None,
            session_start_ms: 0,
            is_paused: false,
            paused_elapsed_ms: 0,
            last_session_type: None,
            last_session_duration: 0,
            last_session_target: 0,
            settings,
            focus_session_counter: 0,
            auto_pause_pending_since: None,
            is_manually_started: false,
        };

        Self {
            db,
            inner: Mutex::new(inner),
        }
    }

    /// 获取设置
    pub fn get_settings(&self) -> PomodoroSettings {
        self.inner.lock().settings.clone()
    }

    /// 更新设置
    pub fn update_settings(&self, settings: PomodoroSettings) {
        let mut inner = self.inner.lock();
        inner.settings = settings.clone();
        // 持久化到数据库
        let conn = self.db.conn().lock();
        if let Err(e) = PomodoroDao::save_settings(&conn, &settings) {
            tracing::error!("Failed to save pomodoro settings: {}", e);
        }
        tracing::info!("Pomodoro settings saved: auto_mode={}", settings.auto_mode);
    }

    /// 自动模式暂停防抖阈值（秒）
    /// 检测到非工作类应用后，持续这么久才真正暂停，避免短暂切换误判
    const AUTO_PAUSE_DEBOUNCE_SECS: i64 = 3;

    /// 自动模式：根据当前活动分类自动开始/暂停番茄钟
    /// 返回是否触发了状态变化（需要通知前端）
    pub fn auto_mode_update(&self, is_focus_activity: bool, is_idle: bool) -> Option<PomodoroStatus> {
        let settings = self.get_settings();
        if !settings.auto_mode || !settings.enabled {
            return None;
        }

        let status = self.get_status();
        let in_focus_phase = matches!(status.session_type.as_deref(), Some("focus"));
        let is_running = status.is_running;
        let is_paused = status.is_paused;

        tracing::info!(
            "Auto mode check: is_focus={}, is_idle={}, running={}, paused={}, phase={:?}, manual={}",
            is_focus_activity, is_idle, is_running, is_paused, status.session_type, status.is_manually_started
        );

        // 空闲或非专注类活动 → 考虑暂停（带防抖）
        // 注意：只有自动启动的会话才会被自动暂停，手动启动的会话尊重用户意图
        let should_pause = !status.is_manually_started
            && (is_idle || (!is_focus_activity && in_focus_phase && is_running && !is_paused));

        if should_pause {
            let now_secs = Utc::now().timestamp();
            let mut inner = self.inner.lock();

            // 第一次检测到应该暂停 → 记录时间，不立刻暂停
            if inner.auto_pause_pending_since.is_none() {
                inner.auto_pause_pending_since = Some(now_secs);
                tracing::info!("Auto mode: pause pending (debounce {}s)", Self::AUTO_PAUSE_DEBOUNCE_SECS);
                drop(inner);
                return None;
            }

            // 已经在等待中 → 检查是否超过阈值
            let pending_since = inner.auto_pause_pending_since.unwrap();
            if now_secs - pending_since >= Self::AUTO_PAUSE_DEBOUNCE_SECS {
                // 超过阈值 → 真正暂停
                inner.auto_pause_pending_since = None;
                drop(inner);
                tracing::info!("Auto mode: debounce elapsed, pausing focus");
                return Some(self.pause());
            }

            // 还在防抖期内 → 什么都不做
            tracing::info!(
                "Auto mode: still in debounce ({}s remaining)",
                Self::AUTO_PAUSE_DEBOUNCE_SECS - (now_secs - pending_since)
            );
            return None;
        }

        // 专注类活动 + 非空闲 → 清除防抖 + 考虑开始/恢复
        if is_focus_activity && !is_idle {
            // 清除暂停防抖
            let mut inner = self.inner.lock();
            if inner.auto_pause_pending_since.is_some() {
                inner.auto_pause_pending_since = None;
                tracing::info!("Auto mode: focus activity detected, cleared pause debounce");
            }
            drop(inner);

            if !is_running {
                // 空闲 → 自动开始专注（标记为非手动）
                tracing::info!("Auto mode: focus activity detected, auto-starting focus");
                return Some(self.auto_start_focus());
            } else if is_paused && in_focus_phase {
                // 暂停中 → 恢复
                tracing::info!("Auto mode: focus activity detected, resuming focus");
                return Some(self.resume(false));
            }
        }

        None
    }

    /// 开始专注（手动）
    pub fn start_focus(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        self._start_phase(&mut inner, PomodoroPhase::Focus, true)
    }

    /// 自动模式开始专注（自动启动，标记为非手动）
    pub fn auto_start_focus(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        self._start_phase(&mut inner, PomodoroPhase::Focus, false)
    }

    /// 开始短休息（手动）
    pub fn start_short_break(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        self._start_phase(&mut inner, PomodoroPhase::ShortBreak, true)
    }

    /// 开始长休息（手动）
    pub fn start_long_break(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        self._start_phase(&mut inner, PomodoroPhase::LongBreak, true)
    }

    fn _start_phase(&self, inner: &mut PomodoroInner, phase: PomodoroPhase, is_manual: bool) -> PomodoroStatus {
        // 如果正在进行中，先结束当前的
        if inner.phase != PomodoroPhase::Idle {
            self._end_current_session(inner, true);
        }

        let now_ms = now_millis();
        let session_type = match phase {
            PomodoroPhase::Focus => "focus",
            PomodoroPhase::ShortBreak => "short_break",
            PomodoroPhase::LongBreak => "long_break",
            PomodoroPhase::Idle => "idle",
        };

        // 写入数据库（数据库字段保持秒精度）
        let session_id = {
            let conn = self.db.conn().lock();
            match PomodoroDao::create_session(&conn, session_type, now_ms / 1000) {
                Ok(id) => Some(id),
                Err(e) => {
                    tracing::error!("Failed to create pomodoro session: {}", e);
                    None
                }
            }
        };

        inner.phase = phase;
        inner.session_start_ms = now_ms;
        inner.current_session_id = session_id;
        inner.is_paused = false;
        inner.paused_elapsed_ms = 0;
        inner.is_manually_started = is_manual;

        tracing::info!("Pomodoro started: {:?}, manual={}", phase, is_manual);
        let mut status = self._build_status(inner);
        status.is_phase_transition = true;
        status
    }

    /// 停止番茄钟（结束当前会话）
    pub fn stop(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        self._end_current_session(&mut inner, true);
        inner.phase = PomodoroPhase::Idle;
        inner.current_session_id = None;
        inner.is_paused = false;
        inner.paused_elapsed_ms = 0;
        inner.is_manually_started = false;
        inner.auto_pause_pending_since = None;
        let mut status = self._build_status(&inner);
        status.is_phase_transition = true;
        status
    }

    /// 暂停番茄钟
    pub fn pause(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        if inner.phase == PomodoroPhase::Idle || inner.is_paused {
            return self._build_status(&inner);
        }
        // 冻结此刻的已用时长：暂停后 remaining / elapsed 都按这个值算，
        // 与暂停前屏幕上显示的数字完全一致
        inner.paused_elapsed_ms = self._elapsed_ms(&inner);
        inner.is_paused = true;
        tracing::info!("Pomodoro paused");
        self._build_status(&inner)
    }

    /// 继续/恢复番茄钟
    /// mark_as_manual: 是否标记为手动启动（手动继续且当前不在工作区时传 true）
    pub fn resume(&self, mark_as_manual: bool) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        if inner.phase == PomodoroPhase::Idle || !inner.is_paused {
            return self._build_status(&inner);
        }
        // 把开始时间往前推，使得恢复后 "now - start == 暂停时的已用时长"，
        // 既不少算也不多算那一秒
        inner.session_start_ms = now_millis() - inner.paused_elapsed_ms;
        inner.is_paused = false;
        inner.paused_elapsed_ms = 0;
        // 如果标记为手动，升级会话为手动启动（自动模式不再自动暂停）
        if mark_as_manual {
            inner.is_manually_started = true;
        }
        tracing::info!("Pomodoro resumed, manual={}", mark_as_manual);
        self._build_status(&inner)
    }

    /// 跳过当前阶段，进入下一个
    pub fn skip(&self) -> PomodoroStatus {
        let mut inner = self.inner.lock();
        let current_phase = inner.phase.clone();
        self._end_current_session(&mut inner, true);

        let next_phase = match current_phase {
            PomodoroPhase::Focus => {
                // 完成一个专注番茄
                inner.focus_session_counter += 1;
                if inner.focus_session_counter >= inner.settings.long_break_interval {
                    inner.focus_session_counter = 0;
                    PomodoroPhase::LongBreak
                } else {
                    PomodoroPhase::ShortBreak
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => PomodoroPhase::Focus,
            PomodoroPhase::Idle => PomodoroPhase::Focus,
        };

        self._start_phase(&mut inner, next_phase, true)
    }

    /// Tick：检查是否到时。
    /// 返回 Some(status) 表示发生了阶段切换（调用方需要发通知 / 刷新托盘），
    /// 没到点时不做任何多余工作（心跳频率很高，不能每次都查库）。
    pub fn tick(&self) -> Option<PomodoroStatus> {
        let mut inner = self.inner.lock();

        if inner.phase == PomodoroPhase::Idle || inner.is_paused {
            return None;
        }

        let elapsed = self._elapsed_ms(&inner);
        let target = self._target_ms(&inner);

        if elapsed < target {
            return None;
        }

        // 时间到了，切换到下一阶段
        let current_phase = inner.phase.clone();
        self._end_current_session(&mut inner, false);

        let next_phase = match current_phase {
            PomodoroPhase::Focus => {
                inner.focus_session_counter += 1;
                if inner.focus_session_counter >= inner.settings.long_break_interval {
                    inner.focus_session_counter = 0;
                    if inner.settings.auto_start_break {
                        PomodoroPhase::LongBreak
                    } else {
                        PomodoroPhase::Idle
                    }
                } else if inner.settings.auto_start_break {
                    PomodoroPhase::ShortBreak
                } else {
                    PomodoroPhase::Idle
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
                if inner.settings.auto_start_focus {
                    PomodoroPhase::Focus
                } else {
                    PomodoroPhase::Idle
                }
            }
            PomodoroPhase::Idle => PomodoroPhase::Idle,
        };

        if next_phase != PomodoroPhase::Idle {
            self._start_phase(&mut inner, next_phase, false);
        } else {
            inner.phase = PomodoroPhase::Idle;
            inner.current_session_id = None;
            inner.is_paused = false;
            inner.paused_elapsed_ms = 0;
            inner.is_manually_started = false;
            inner.auto_pause_pending_since = None;
            let mut status = self._build_status(&inner);
            status.is_phase_transition = true;
            tracing::info!("Pomodoro phase switched, next: Idle");
            return Some(status);
        }

        tracing::info!("Pomodoro phase switched, next: {:?}", next_phase);
        Some(self._build_status(&inner))
    }

    fn _end_current_session(&self, inner: &mut PomodoroInner, interrupted: bool) {
        if inner.phase == PomodoroPhase::Idle {
            return;
        }

        let now_ms = now_millis();
        // 用「实际计时时长」而不是「墙钟差值」：暂停期间的时间不计入
        let elapsed_ms = self._elapsed_ms(inner);
        let duration = (elapsed_ms + 500) / 1000;
        let target = self._target_ms(inner) / 1000;

        // 保存上次会话信息，供 Idle 时展示
        inner.last_session_type = match inner.phase {
            PomodoroPhase::Focus => Some("focus".to_string()),
            PomodoroPhase::ShortBreak => Some("short_break".to_string()),
            PomodoroPhase::LongBreak => Some("long_break".to_string()),
            PomodoroPhase::Idle => None,
        };
        inner.last_session_duration = duration;
        inner.last_session_target = target;

        if let Some(session_id) = inner.current_session_id {
            let conn = self.db.conn().lock();
            if let Err(e) = PomodoroDao::complete_session(&conn, session_id, now_ms / 1000, duration, interrupted) {
                tracing::error!("Failed to complete pomodoro session: {}", e);
            }
        }
    }

    /// 当前已用毫秒（暂停时返回冻结值）
    fn _elapsed_ms(&self, inner: &PomodoroInner) -> i64 {
        elapsed_ms_at(
            inner.session_start_ms,
            inner.is_paused,
            inner.paused_elapsed_ms,
            now_millis(),
        )
    }

    fn _target_ms(&self, inner: &PomodoroInner) -> i64 {
        let minutes = match inner.phase {
            PomodoroPhase::Focus => inner.settings.focus_minutes,
            PomodoroPhase::ShortBreak => inner.settings.short_break_minutes,
            PomodoroPhase::LongBreak => inner.settings.long_break_minutes,
            PomodoroPhase::Idle => 0,
        };
        minutes as i64 * 60_000
    }

    fn _build_status(&self, inner: &PomodoroInner) -> PomodoroStatus {
        let now_ms = now_millis();

        let (elapsed_ms, target_ms, session_type, start_time) = if inner.phase != PomodoroPhase::Idle {
            // 运行中或暂停中
            let st = match inner.phase {
                PomodoroPhase::Focus => Some("focus".to_string()),
                PomodoroPhase::ShortBreak => Some("short_break".to_string()),
                PomodoroPhase::LongBreak => Some("long_break".to_string()),
                PomodoroPhase::Idle => None,
            };
            (
                self._elapsed_ms(inner),
                self._target_ms(inner),
                st,
                Some(inner.session_start_ms / 1000),
            )
        } else {
            // 空闲：显示专注时长，让用户知道设置的是多久
            (0, inner.settings.focus_minutes as i64 * 60_000, None, None)
        };

        let remaining_ms = (target_ms - elapsed_ms).max(0);

        let today_focus_seconds = {
            let conn = self.db.conn().lock();
            PomodoroDao::get_today_focus_seconds(&conn).unwrap_or(0)
        };

        let today_focus_count = {
            let conn = self.db.conn().lock();
            PomodoroDao::get_today_focus_count(&conn).unwrap_or(0)
        };

        PomodoroStatus {
            is_running: inner.phase != PomodoroPhase::Idle,
            is_paused: inner.is_paused,
            session_type,
            start_time,
            elapsed_ms,
            remaining_ms,
            server_time_ms: now_ms,
            elapsed_seconds: elapsed_ms / 1000,
            target_seconds: target_ms / 1000,
            // 与前端显示一致：剩余秒向上取整（起点显示满值、到点显示 0）
            remaining_seconds: (remaining_ms + 999) / 1000,
            today_focus_count,
            today_focus_seconds,
            is_phase_transition: false,
            is_manually_started: inner.is_manually_started,
        }
    }

    /// 获取当前状态
    pub fn get_status(&self) -> PomodoroStatus {
        let inner = self.inner.lock();
        self._build_status(&inner)
    }

    /// 当前阶段（供外部判断是否发通知）
    pub fn current_phase(&self) -> PomodoroPhase {
        self.inner.lock().phase.clone()
    }
}
