#[cfg(windows)]
use windows::Win32::Foundation::{LPARAM, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND};

/// Control the display power state. Only effective on Windows.
#[cfg(windows)]
pub fn set_display(on: bool) {
    unsafe {
        let state = if on { -1 } else { 2 };
        SendMessageW(
            HWND_BROADCAST,
            WM_SYSCOMMAND,
            WPARAM(SC_MONITORPOWER as usize),
            LPARAM(state),
        );
    }
}

#[cfg(not(windows))]
pub fn set_display(_on: bool) {
    // Non-Windows platforms are not supported; this is a no-op placeholder.
}
