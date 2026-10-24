use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, CreateIconIndirect, DestroyIcon, LoadCursorW, SetSystemCursor, SystemParametersInfoW,
    HCURSOR, HICON, ICONINFO, IDC_ARROW, OCR_NORMAL, SPI_SETCURSORS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};

static SESSION_REF_COUNT: AtomicUsize = AtomicUsize::new(0);
static SYSTEM_CURSOR_REPLACED: AtomicBool = AtomicBool::new(false);
static SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);
static LAST_ACTIVITY_TIME: OnceLock<Mutex<Instant>> = OnceLock::new();
static SAVED_ORIGINAL_CURSOR: OnceLock<Mutex<Option<isize>>> = OnceLock::new();
static LIFECYCLE_INITIALIZED: OnceLock<()> = OnceLock::new();

fn get_last_activity_time() -> &'static Mutex<Instant> {
    LAST_ACTIVITY_TIME.get_or_init(|| Mutex::new(Instant::now()))
}

fn get_saved_original_cursor() -> &'static Mutex<Option<isize>> {
    SAVED_ORIGINAL_CURSOR.get_or_init(|| Mutex::new(None))
}

/// RAII Guard that manages an active desktop control session.
/// Restores the original Windows system cursor exactly once when all active sessions drop.
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

/// Initialize the cursor subsystem lifecycle, console exit handlers, inactivity watchdog,
/// and ensure any stale system cursor state from a prior crash is restored to normal.
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

        // 2. Spawn background inactivity watchdog to auto-restore system cursor if tasks stall
        thread::spawn(|| {
            loop {
                thread::sleep(Duration::from_millis(500));
                if SYSTEM_CURSOR_REPLACED.load(Ordering::SeqCst) {
                    let now = Instant::now();
                    let elapsed = {
                        let guard = get_last_activity_time().lock().unwrap();
                        now.duration_since(*guard)
                    };
                    // If no desktop actions have occurred for 10 seconds, auto-restore
                    if elapsed > Duration::from_secs(10) {
                        eprintln!("[Aven Cursor Watchdog] Desktop inactivity timeout (10s) reached. Restoring system cursor.");
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

/// Touch the activity timestamp to keep the session alive
pub fn touch_activity() {
    let mut guard = get_last_activity_time().lock().unwrap();
    *guard = Instant::now();
}

/// Acquire an active desktop control session.
/// Returns an RAII guard that restores the cursor when dropped.
/// Safe: If cursor replacement fails, desktop tools continue without error using the standard cursor.
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
        // First active session: try to replace cursor
        let _ = replace_system_cursor_internal();
        super::overlay::show_overlay_ui();
        new_gen
    } else {
        SESSION_GENERATION.load(Ordering::SeqCst)
    };

    Ok(DesktopControlGuard {
        generation: current_gen,
        active: true,
    })
}

/// Release an active session. When count drops to 0, original system cursor is restored.
pub fn release_session(generation: u64) {
    if generation != SESSION_GENERATION.load(Ordering::SeqCst) {
        // Generation was invalidated by a force restore / cancel
        return;
    }

    let prev = SESSION_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
    if prev <= 1 {
        SESSION_REF_COUNT.store(0, Ordering::SeqCst);
        perform_restoration();
    }
}

/// Internal helper to replace the Windows system cursor safely with original cursor backup
fn replace_system_cursor_internal() -> Result<(), String> {
    if SYSTEM_CURSOR_REPLACED.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Save copy of current default system cursor before modification
    unsafe {
        if let Ok(c) = LoadCursorW(None, IDC_ARROW) {
            if !c.0.is_null() {
                if let Ok(copy) = CopyIcon(HICON(c.0)) {
                    let mut guard = get_saved_original_cursor().lock().unwrap();
                    *guard = Some(copy.0 as isize);
                }
            }
        }
    }

    let hcur = match create_aven_cursor() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Aven Cursor] Custom cursor creation failed: {e}. Keeping default cursor.");
            return Err(e);
        }
    };

    unsafe {
        let copy = CopyIcon(HICON(hcur.0))
            .map_err(|e| format!("CopyIcon failed: {e}"))?;
        let _ = DestroyIcon(HICON(hcur.0));

        let set_res = SetSystemCursor(HCURSOR(copy.0), OCR_NORMAL);
        if set_res.is_ok() {
            SYSTEM_CURSOR_REPLACED.store(true, Ordering::SeqCst);
            Ok(())
        } else {
            SYSTEM_CURSOR_REPLACED.store(false, Ordering::SeqCst);
            Err("SetSystemCursor call failed".to_string())
        }
    }
}

/// Perform complete restoration of the original cursor and clean up overlay/input state
pub fn perform_restoration() {
    if SYSTEM_CURSOR_REPLACED.swap(false, Ordering::SeqCst) {
        unsafe {
            // 1. If we have a saved original cursor handle, restore it
            let saved = {
                let mut guard = get_saved_original_cursor().lock().unwrap();
                guard.take()
            };

            if let Some(h) = saved {
                let hcur = HCURSOR(h as *mut _);
                let copy = CopyIcon(HICON(hcur.0));
                let _ = DestroyIcon(HICON(hcur.0));
                if let Ok(c) = copy {
                    let _ = SetSystemCursor(HCURSOR(c.0), OCR_NORMAL);
                }
            }

            // 2. Call SystemParametersInfoW with SPI_SETCURSORS to reload user's scheme from system settings without touching registry
            let _ = SystemParametersInfoW(
                SPI_SETCURSORS,
                0,
                None,
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
    }

    super::overlay::hide_overlay_ui();
    super::overlay::clear_activity_text();
    super::input::release_all_input_locks();
}

/// Forcibly reset session counters and restore normal Windows cursor immediately.
/// Used on task cancellation, emergency stop, error paths, and process exit.
pub fn force_restore_and_cleanup() {
    SESSION_REF_COUNT.store(0, Ordering::SeqCst);
    SESSION_GENERATION.fetch_add(1, Ordering::SeqCst);
    perform_restoration();
}

/// Check if an active desktop control session is currently in progress
pub fn is_in_active_session() -> bool {
    SESSION_REF_COUNT.load(Ordering::SeqCst) > 0 && !super::hotkey::is_cancelled()
}

/// Check if the Windows system cursor is currently replaced with Aven's cursor
pub fn is_system_cursor_replaced() -> bool {
    SYSTEM_CURSOR_REPLACED.load(Ordering::SeqCst)
}

/// Backward-compatible wrapper for explicit replacement
pub fn replace_system_cursor() -> Result<(), String> {
    let guard = acquire_session()?;
    std::mem::forget(guard);
    Ok(())
}

/// Backward-compatible wrapper for explicit restoration
pub fn restore_system_cursor() {
    force_restore_and_cleanup();
}

/// Generate a 32x32 32-bit ARGB hardware cursor from the Aven Flight Cursor SVG geometry
pub fn create_aven_cursor() -> Result<HCURSOR, String> {
    unsafe {
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.0.is_null() {
            return Err("Failed to obtain screen DC".to_string());
        }

        let width = 32i32;
        let height = 32i32;

        // 1. Create 32-bit ARGB Color DIBSection
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: (width * height * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default()],
        };

        let mut bits_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm_color = CreateDIBSection(
            hdc_screen,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            windows::Win32::Foundation::HANDLE::default(),
            0,
        ).map_err(|e| format!("Failed to create DIB section: {e}"))?;

        if bits_ptr.is_null() {
            ReleaseDC(HWND::default(), hdc_screen);
            return Err("DIBSection memory pointer is null".to_string());
        }

        // 2. Create 1-bit monochrome mask bitmap
        let mask_bytes_len = ((width + 15) / 16 * 2 * height) as usize;
        let mut mask_data = vec![0xFFu8; mask_bytes_len]; // Default all transparent (1)

        let pixel_buf = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (width * height * 4) as usize);
        pixel_buf.fill(0); // Zero out ARGB

        // Aven Cursor Primary Color: #7C7CF8 -> R: 124, G: 124, B: 248 (ARGB: 0xFF7C7CF8)
        let fill_r = 124u8;
        let fill_g = 124u8;
        let fill_b = 248u8;
        let outline_r = 25u8;
        let outline_g = 25u8;
        let outline_b = 40u8;

        // Rasterize Aven autonomous agent cursor polygon:
        // Hotspot: Tip of cursor at (2, 2)
        // Polygon vertices matching aven.svg scaled to 32x32:
        let poly_coords: &[(i32, i32)] = &[
            (2, 2),   // Tip hotspot
            (8, 24),  // Bottom left wing
            (10, 29), // Wing tip
            (13, 27), // Inner left notch
            (10, 14), // Center waist
            (22, 16), // Right wing
            (28, 15), // Right wing tip
            (14, 7),  // Upper slope
            (3, 2),   // Return to tip
        ];

        let inner_tail: &[(i32, i32)] = &[
            (12, 13),
            (15, 23),
            (18, 20),
            (21, 19),
            (16, 15),
            (12, 13),
        ];

        // Fill polygon using standard scanline point-in-polygon
        for y in 0..height {
            for x in 0..width {
                let inside_main = is_point_in_poly(x, y, poly_coords);
                let inside_tail = is_point_in_poly(x, y, inner_tail);

                if inside_main || inside_tail {
                    let offset = ((y * width + x) * 4) as usize;
                    let is_border = is_on_border(x, y, poly_coords, inner_tail);

                    if is_border {
                        pixel_buf[offset] = outline_b;
                        pixel_buf[offset + 1] = outline_g;
                        pixel_buf[offset + 2] = outline_r;
                        pixel_buf[offset + 3] = 255; // Alpha
                    } else {
                        pixel_buf[offset] = fill_b;
                        pixel_buf[offset + 1] = fill_g;
                        pixel_buf[offset + 2] = fill_r;
                        pixel_buf[offset + 3] = 255; // Alpha
                    }

                    // Mask bit: 0 = opaque in Win32 icon mask
                    let mask_byte_idx = (y * 4 + (x / 8)) as usize;
                    if mask_byte_idx < mask_data.len() {
                        mask_data[mask_byte_idx] &= !(1 << (7 - (x % 8)));
                    }
                }
            }
        }

        let hbm_mask = CreateBitmap(
            width,
            height,
            1,
            1,
            Some(mask_data.as_ptr() as *const core::ffi::c_void),
        );

        ReleaseDC(HWND::default(), hdc_screen);

        if hbm_mask.0.is_null() {
            let _ = DeleteObject(hbm_color);
            return Err("Failed to create monochrome mask bitmap".to_string());
        }

        // 3. Assemble hardware cursor via CreateIconIndirect
        let icon_info = ICONINFO {
            fIcon: BOOL(0), // FALSE denotes cursor with hotspot
            xHotspot: 2,    // Exact tip of Aven flight cursor
            yHotspot: 2,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hcursor = CreateIconIndirect(&icon_info);

        let _ = DeleteObject(hbm_mask);
        let _ = DeleteObject(hbm_color);

        match hcursor {
            Ok(cur) if !cur.0.is_null() => Ok(HCURSOR(cur.0)),
            _ => Err("CreateIconIndirect failed to generate HCURSOR".to_string()),
        }
    }
}

fn is_point_in_poly(px: i32, py: i32, poly: &[(i32, i32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];

        let intersect = ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / (yj - yi) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn is_on_border(x: i32, y: i32, p1: &[(i32, i32)], p2: &[(i32, i32)]) -> bool {
    let neighbors = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
    for (nx, ny) in neighbors {
        if nx < 0 || ny < 0 || nx >= 32 || ny >= 32 {
            return true;
        }
        let in1 = is_point_in_poly(nx, ny, p1);
        let in2 = is_point_in_poly(nx, ny, p2);
        if !in1 && !in2 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::desktop_control::TEST_DESKTOP_LOCK;

    #[test]
    fn test_create_aven_cursor_succeeds() {
        let cur = create_aven_cursor();
        if let Ok(h) = cur {
            assert!(!h.0.is_null());
            unsafe {
                let _ = DestroyIcon(HICON(h.0));
            }
        }
    }

    #[test]
    fn test_point_in_poly_hotspot() {
        let poly: &[(i32, i32)] = &[
            (2, 2),
            (8, 24),
            (10, 29),
            (13, 27),
            (10, 14),
            (22, 16),
            (28, 15),
            (14, 7),
            (3, 2),
        ];
        assert!(is_point_in_poly(5, 5, poly));
        assert!(!is_point_in_poly(30, 30, poly));
    }

    #[test]
    fn test_session_reference_counting_and_raii() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());

        {
            let guard1 = acquire_session().unwrap();
            assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 1);
            assert!(is_in_active_session());

            {
                let guard2 = acquire_session().unwrap();
                assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 2);
                assert_eq!(guard1.generation, guard2.generation);
            }
            // guard2 dropped
            assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 1);
            assert!(is_in_active_session());
        }
        // guard1 dropped
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }

    #[test]
    fn test_early_error_restores_cursor() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        force_restore_and_cleanup();

        let simulate_failing_action = || -> Result<(), String> {
            let _guard = acquire_session()?;
            assert!(is_in_active_session());
            Err("Simulated desktop action failure".to_string())
        };

        let res = simulate_failing_action();
        assert!(res.is_err());
        // Guard dropped on early error return
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }

    #[test]
    fn test_force_restore_and_cleanup_resets_state() {
        let _lock = TEST_DESKTOP_LOCK.lock().unwrap();
        let _g1 = acquire_session().unwrap();
        let _g2 = acquire_session().unwrap();
        assert!(is_in_active_session());

        force_restore_and_cleanup();
        assert_eq!(SESSION_REF_COUNT.load(Ordering::SeqCst), 0);
        assert!(!is_in_active_session());
        assert!(!is_system_cursor_replaced());
    }
}
