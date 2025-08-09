use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND, SMTO_ABORTIFHUNG,
};

/// 控制显示器电源状态
/// 
/// 该函数通过 Windows API 向所有窗口广播显示器电源控制消息，
/// 实现屏幕的开启和关闭功能。
/// 
/// # Arguments
/// * `on` - 显示器状态：`true` 表示开启屏幕，`false` 表示关闭屏幕
/// 
/// # Safety
/// 此函数包含 unsafe 代码块，因为调用了 Windows API。
/// 在 Windows 系统上调用是安全的。
pub fn set_display(on: bool) {
    unsafe {
        // 根据开启/关闭状态设置显示器电源参数
        // -1: 显示器开启
        // 2: 显示器关闭
        let state = if on { -1 } else { 2 };
        // 使用 SendMessageTimeoutW 防止 HWND_BROADCAST 导致阻塞
        // 设置较短的超时（例如 500ms），并在窗口挂起时中止
        let mut _unused: usize = 0;
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SYSCOMMAND,
            WPARAM(SC_MONITORPOWER as usize),
            LPARAM(state),
            SMTO_ABORTIFHUNG,
            500,
            Some(&mut _unused as *mut usize),
        );
        
        // 操作结果已在调用方记录日志
        
        // 注意：SendMessageW 的返回值在此上下文中通常不需要检查
        // 因为显示器电源控制是一个广播消息，没有特定的返回值含义
    }
}
