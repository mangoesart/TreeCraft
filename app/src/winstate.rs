//! The window opens maximized, or at the size it had when it was last closed un-maximized
//! (picture 23). Windows only; elsewhere these do nothing.

#[cfg(windows)]
mod sys {
    #[repr(C)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        pub fn EnumWindows(cb: extern "system" fn(isize, isize) -> i32, lp: isize) -> i32;
        pub fn GetWindowThreadProcessId(h: isize, pid: *mut u32) -> u32;
        pub fn IsWindowVisible(h: isize) -> i32;
        pub fn ShowWindow(h: isize, cmd: i32) -> i32;
        pub fn IsZoomed(h: isize) -> i32;
        pub fn IsIconic(h: isize) -> i32;
        pub fn GetClientRect(h: isize, r: *mut Rect) -> i32;
    }
}

/// This program's visible top-level window.
#[cfg(windows)]
fn own_window() -> Option<isize> {
    use std::sync::atomic::{AtomicIsize, Ordering};
    static FOUND: AtomicIsize = AtomicIsize::new(0);
    extern "system" fn each(h: isize, _: isize) -> i32 {
        let mut pid = 0u32;
        unsafe { sys::GetWindowThreadProcessId(h, &mut pid) };
        if pid == std::process::id() && unsafe { sys::IsWindowVisible(h) } != 0 {
            FOUND.store(h, Ordering::SeqCst);
            return 0;
        }
        1
    }
    FOUND.store(0, Ordering::SeqCst);
    unsafe { sys::EnumWindows(each, 0) };
    let h = FOUND.load(Ordering::SeqCst);
    (h != 0).then_some(h)
}

/// Maximize the window (first launch, or it was maximized when last closed). True once done.
pub fn maximize() -> bool {
    #[cfg(windows)]
    {
        if let Some(h) = own_window() {
            unsafe { sys::ShowWindow(h, 3) }; // SW_MAXIMIZE
            return true;
        }
        false
    }
    #[cfg(not(windows))]
    true
}

/// Remember how the window is as the program closes: maximized, or its inside size in pixels.
pub fn remember(prefs: &mut crate::prefs::Prefs) {
    #[cfg(windows)]
    {
        let Some(h) = own_window() else { return };
        unsafe {
            if sys::IsIconic(h) != 0 {
                return;
            }
            if sys::IsZoomed(h) != 0 {
                prefs.window_max = true;
            } else {
                let mut r = sys::Rect { left: 0, top: 0, right: 0, bottom: 0 };
                if sys::GetClientRect(h, &mut r) != 0 && r.right - r.left >= 400 && r.bottom - r.top >= 300 {
                    prefs.window_max = false;
                    prefs.window_w = r.right - r.left;
                    prefs.window_h = r.bottom - r.top;
                }
            }
        }
        prefs.save();
    }
    #[cfg(not(windows))]
    let _ = prefs;
}
