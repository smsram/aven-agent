use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::BOOL;

static SESSION_REF_COUNT: AtomicUsize = AtomicUsize::new(0);
static SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);
static LAST_ACTIVITY_TIME: OnceLock<Mutex<Instant>> = OnceLock::new();
static LIFECYCLE_INITIALIZED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct TaskActivityTracker {
    pub session_id: String,
    pub last_heartbeat: Instant,
    pub last_progress: Instant,
    pub session_state: super::overlay::AutomationSessionState,
    pub timed_out: bool,
}

static TASK_ACTIVITY_TRACKER: OnceLock<Mutex<TaskActivityTracker>> = OnceLock::new();

fn get_task_tracker() -> &'static Mutex<TaskActivityTracker> {
    TASK_ACTIVITY_TRACKER.get_or_init(|| {
        Mutex::new(TaskActivityTracker {
            session_id: String::new(),
            last_heartbeat: Instant::now(),
            last_progress: Instant::now(),
            session_state: super::overlay::AutomationSessionState::Idle,
            timed_out: false,
        })
    })
}

fn get_last_activity_time() -> &'static Mutex<Instant> {
    LAST_ACTIVITY_TIME.get_or_init(|| Mutex::new(Instant::now()))
}

/// RAII Guard that manages an active desktop control session.
/// Cleans up overlay state when dropped.
#[derive(Debug)]
pub struct DesktopControlGuard {
    pub generation: u64,
    active: bool,
}

impl Drop for DesktopControlGuard {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            release_session(self.generation);
        }
    }
}

/// Initialize the automation lifecycle and ensure clean initial state.
pub fn init_lifecycle_and_restore() {
    ensure_lifecycle_initialized();
    force_restore_and_cleanup();
}

fn ensure_lifecycle_initialized() {
    LIFECYCLE_INITIALIZED.get_or_init(|| {
        // 1. Register Win32 console control handler for terminal / process termination
        unsafe {
            let _ = windows::Win32::System::Console::SetConsoleCtrlHandler(
                Some(console_ctrl_handler),
                true,
            );
        }

        // 2. Background inactivity watchdog to auto-clean overlay if tasks genuinely stall
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_millis(500));
                if is_in_active_session() {
                    let now = Instant::now();
                    let (sid, state, elapsed_heartbeat, already_timed_out) = {
                        let guard = get_task_tracker().lock().unwrap();
                        (
                            guard.session_id.clone(),
                            guard.session_state,
                            now.duration_since(guard.last_heartbeat),
                            guard.timed_out,
                        )
                    };

                    if already_timed_out {
                        // Already handled timeout for this stall event; do not log or clean again
                        continue;
                    }

                    // Define state-aware timeout thresholds:
                    // - WaitingForModel: allow up to 90s (AI API call / model inference wait)
                    // - ExecutingTool / Verifying / Recovering: allow up to 60s
                    // - Idle / Running with no input or heartbeat: 45s threshold
                    let timeout_threshold = match state {
                        super::overlay::AutomationSessionState::WaitingForModel => Duration::from_secs(90),
                        super::overlay::AutomationSessionState::ExecutingTool
                        | super::overlay::AutomationSessionState::Verifying
                        | super::overlay::AutomationSessionState::Recovering => Duration::from_secs(60),
                        _ => Duration::from_secs(45),
                    };

                    if elapsed_heartbeat > timeout_threshold {
                        // Mark as timed out so we log and clean up EXACTLY ONCE
                        {
                            let mut guard = get_task_tracker().lock().unwrap();
                            guard.timed_out = true;
                            guard.last_heartbeat = now;
                        }

                        eprintln!(
                            "[Aven Watchdog] Inactivity timeout reached for session '{}' (state: {:?}, elapsed: {:.1?}s). Cleaning up desktop overlay.",
                            sid, state, elapsed_heartbeat
                        );
                        force_restore_and_cleanup();
                    }
                }
            }
        });
    });
}

unsafe extern "system" fn console_ctrl_handler(_ctrl_type: u32) -> BOOL {
    force_restore_and_cleanup();
    BOOL(0)
}

/// Touch the task heartbeat and record current state
pub fn touch_task_heartbeat(session_id: &str, state: super::overlay::AutomationSessionState) {
    let now = Instant::now();
    {
        let mut tracker = get_task_tracker().lock().unwrap();
        tracker.session_id = session_id.to_string();
        tracker.last_heartbeat = now;
        tracker.session_state = state;
        tracker.timed_out = false;
        match state {
            super::overlay::AutomationSessionState::ExecutingTool
            | super::overlay::AutomationSessionState::Verifying
            | super::overlay::AutomationSessionState::Recovering => {
                tracker.last_progress = now;
            }
            _ => {}
        }
    }
    {
        let mut guard = get_last_activity_time().lock().unwrap();
        *guard = now;
    }
}

/// Touch the activity timestamp to keep the session alive
pub fn touch_activity() {
    let now = Instant::now();
    {
        let mut guard = get_last_activity_time().lock().unwrap();
        *guard = now;
    }
    {
        let mut tracker = get_task_tracker().lock().unwrap();
        tracker.last_heartbeat = now;
        tracker.last_progress = now;
        tracker.timed_out = false;
    }
}

/// Acquire an active desktop control session.
/// The real Windows mouse cursor is never modified or moved.
pub fn acquire_session() -> Result<DesktopControlGuard, String> {
    if super::hotkey::is_cancelled() {
        force_restore_and_cleanup();
        return Err("Operation cancelled by user (emergency stop)".to_string());
    }

    ensure_lifecycle_initialized();
    touch_activity();

    let prev = SESSION_REF_COUNT.fetch_add(1, Ordering::SeqCst);
    let current_gen = if prev == 0 {
        let new_gen = SESSION_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        super::overlay::start_desktop_automation();
        new_gen
    } else {
        SESSION_GENERATION.load(Ordering::SeqCst)
    };

    Ok(DesktopControlGuard {
        generation: current_gen,
        active: true,
    })
}

/// Release an active session ref count.
pub fn release_session(generation: u64) {
    if generation != SESSION_GENERATION.load(Ordering::SeqCst) {
        return;
    }

    let prev = SESSION_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
    if prev <= 1 {
        SESSION_REF_COUNT.store(0, Ordering::SeqCst);
        // Only stop if overlay is not held by the overarching task orchestration session
        if !super::overlay::is_automation_active() {
            perform_restoration();
        }
    }
}

/// Clean up overlay and input state.
pub fn perform_restoration() {
    super::overlay::stop_desktop_automation();
    super::input::release_all_input_locks();
}

/// Forcibly reset session counters and clean up overlay immediately.
/// Used on task cancellation, emergency stop, error paths, and process exit.
pub fn force_restore_and_cleanup() {
    SESSION_REF_COUNT.store(0, Ordering::SeqCst);
    SESSION_GENERATION.fetch_add(1, Ordering::SeqCst);
    {
        let mut tracker = get_task_tracker().lock().unwrap();
        tracker.timed_out = true;
        tracker.session_state = super::overlay::AutomationSessionState::Idle;
        tracker.last_heartbeat = Instant::now();
    }
    super::overlay::stop_desktop_automation();
    super::input::release_all_input_locks();
}

/// Check if an active desktop session is running
pub fn is_in_active_session() -> bool {
    SESSION_REF_COUNT.load(Ordering::SeqCst) > 0 || super::overlay::is_automation_active()
}

/// We never replace or modify the real Windows system cursor, so this always returns false.
pub fn is_system_cursor_replaced() -> bool {
    false
}

/// Activate overlay session
pub fn replace_system_cursor() -> Result<(), String> {
    super::overlay::start_desktop_automation();
    Ok(())
}

/// Restore overlay state
pub fn restore_system_cursor() {
    super::overlay::stop_desktop_automation();
    perform_restoration();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::desktop_control::TEST_DESKTOP_LOCK;

    #[test]
    fn test_system_cursor_is_never_modified() {
        assert!(!is_system_cursor_replaced());
    }

    #[test]
    fn test_session_reference_counting_and_raii() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());

        {
            let guard1 = acquire_session().unwrap();
            assert!(SESSION_REF_COUNT.load(Ordering::SeqCst) >= 1);
            assert!(is_in_active_session());

            {
                let guard2 = acquire_session().unwrap();
                assert!(SESSION_REF_COUNT.load(Ordering::SeqCst) >= 2);
                assert_eq!(guard1.generation, guard2.generation);
            }
            assert!(SESSION_REF_COUNT.load(Ordering::SeqCst) >= 1);
            assert!(is_in_active_session());
        }
        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }

    #[test]
    fn test_early_error_restores_cursor() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        force_restore_and_cleanup();

        let simulate_failing_action = || -> Result<(), String> {
            let _guard = acquire_session()?;
            assert!(is_in_active_session());
            Err("Simulated desktop action failure".to_string())
        };

        let res = simulate_failing_action();
        assert!(res.is_err());
        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }

    #[test]
    fn test_force_restore_and_cleanup_resets_state() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g1 = acquire_session().unwrap();
        let _g2 = acquire_session().unwrap();
        assert!(is_in_active_session());

        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }
}
