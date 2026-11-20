//! The window's title bar (picture 49): the open scene's name and where it is, as Blender shows
//! it ("* name [path] - TreeCraft 0.1.0 Beta"); a star while it has unsaved changes.

/// The title for a scene at `path` (None: not saved yet).
pub fn title_for(path: Option<&std::path::Path>, modified: bool) -> String {
    let app = format!("TreeCraft {} Beta", env!("CARGO_PKG_VERSION"));
    let star = if modified { "* " } else { "" };
    match path {
        Some(p) => {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            format!("{}{} [{}] - {}", star, name, p.display(), app)
        }
        None => format!("{}(Unsaved) - {}", star, app),
    }
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    type Hwnd = isize;
    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
        fn IsWindowVisible(h: Hwnd) -> i32;
        fn GetWindow(h: Hwnd, cmd: u32) -> Hwnd;
        fn SetWindowTextW(h: Hwnd, text: *const u16) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcessId() -> u32;
    }
    static FOUND: AtomicIsize = AtomicIsize::new(0);

    extern "system" fn each(h: Hwnd, _: isize) -> i32 {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(h, &mut pid);
            // the program's own visible top-level window (no owner)
            if pid == GetCurrentProcessId() && IsWindowVisible(h) != 0 && GetWindow(h, 4) == 0 {
                FOUND.store(h, Ordering::Relaxed);
                return 0;
            }
        }
        1
    }

    pub fn set(title: &str) -> bool {
        let mut h = FOUND.load(Ordering::Relaxed);
        if h == 0 {
            unsafe {
                EnumWindows(each, 0);
            }
            h = FOUND.load(Ordering::Relaxed);
        }
        if h == 0 {
            return false;
        }
        let w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { SetWindowTextW(h, w.as_ptr()) != 0 }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set(_title: &str) -> bool {
        false
    }
}

/// Put `title` on the window's title bar; false when the window couldn't be found.
pub fn set(title: &str) -> bool {
    imp::set(title)
}
