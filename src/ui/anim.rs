use std::sync::Mutex;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    KillTimer, SendMessageW, SetTimer,
};

pub const ID_TIMER_PROGRESS: usize = 901;
pub const PBM_SETPOS: u32 = 1026; // WM_USER + 2

pub struct ProgressAnimator {
    pub target_current: usize,
    pub displayed_current: usize,
    pub target_overall: usize,
    pub displayed_overall: usize,
    pub is_active: bool,
    pub ticks: usize,
}

impl ProgressAnimator {
    pub const fn new() -> Self {
        ProgressAnimator {
            target_current: 0,
            displayed_current: 0,
            target_overall: 0,
            displayed_overall: 0,
            is_active: false,
            ticks: 0,
        }
    }
}

pub static ANIMATOR: Mutex<ProgressAnimator> = Mutex::new(ProgressAnimator::new());

/// Starts the animation timer on the window (~60fps for smooth updates)
pub fn start_anim(hwnd: HWND) {
    if let Ok(mut anim) = ANIMATOR.lock() {
        anim.target_current = 10;
        anim.displayed_current = 0;
        anim.target_overall = 0;
        anim.displayed_overall = 0;
        anim.is_active = true;
        anim.ticks = 0;
    }
    unsafe {
        // 16ms ≈ 60fps - Windows checks for hung apps after ~5 seconds of no messages
        // Timer keeps message queue active without excessive CPU usage
        SetTimer(hwnd, ID_TIMER_PROGRESS, 16, None);
    }
}

/// Sets target percentage for the current file
pub fn set_target_percent(pct: usize) {
    if let Ok(mut anim) = ANIMATOR.lock() {
        anim.target_current = pct.min(100);
    }
}

/// Sets overall file progress target
pub fn set_overall_target(pct: usize) {
    if let Ok(mut anim) = ANIMATOR.lock() {
        anim.target_overall = pct.min(100);
    }
}

// Called on every WM_TIMER tick: smoothly animates the progress bars
// Key principle: Only animate when there's actual work happening.
// Don't fake progress - users notice and distrust moving bars without real work.
pub fn on_timer_tick(hwnd_current: HWND, hwnd_overall: HWND) {
    let mut updated_current = None;
    let mut updated_overall = None;

    if let Ok(mut anim) = ANIMATOR.lock() {
        if !anim.is_active {
            return;
        }

        anim.ticks += 1;

        // 1. Smoothly step current file progress towards target
        // Adaptive stepping: bigger steps when far behind, smaller when close
        if anim.displayed_current < anim.target_current {
            let diff = anim.target_current - anim.displayed_current;
            let step = if diff > 50 { 8 } else if diff > 20 { 5 } else if diff > 5 { 3 } else { 1 };
            anim.displayed_current += step;
            updated_current = Some(anim.displayed_current);
        }

        // 2. Smoothly step overall progress towards target
        if anim.displayed_overall < anim.target_overall {
            anim.displayed_overall += 1;
            updated_overall = Some(anim.displayed_overall);
        }
    }

    unsafe {
        if let Some(pos) = updated_current {
            SendMessageW(hwnd_current, PBM_SETPOS, pos, 0);
        }
        if let Some(pos) = updated_overall {
            SendMessageW(hwnd_overall, PBM_SETPOS, pos, 0);
        }
    }
}

/// Stops animation timer and snaps to final 100% state
pub fn stop_anim(hwnd: HWND, hwnd_current: HWND, hwnd_overall: HWND) {
    unsafe {
        KillTimer(hwnd, ID_TIMER_PROGRESS);
        SendMessageW(hwnd_current, PBM_SETPOS, 100, 0);
        SendMessageW(hwnd_overall, PBM_SETPOS, 100, 0);
    }
    if let Ok(mut anim) = ANIMATOR.lock() {
        anim.is_active = false;
        anim.displayed_current = 100;
        anim.displayed_overall = 100;
    }
}
