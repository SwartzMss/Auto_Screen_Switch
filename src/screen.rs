use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND, SMTO_ABORTIFHUNG,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// 屏幕状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenState {
    On,     // 屏幕开启
    Off,    // 屏幕关闭
    Unknown, // 状态未知
}

/// 全局屏幕状态跟踪器
static SCREEN_STATE: AtomicBool = AtomicBool::new(true); // 默认认为屏幕是开启的

/// 检测当前屏幕状态
/// 
/// 该函数通过内存状态跟踪来检测屏幕状态：
/// 由于 Windows API 检测屏幕状态比较复杂且不可靠，
/// 我们使用内部状态跟踪来记录最后一次操作的结果。
/// 
/// # Returns
/// * `ScreenState` - 当前屏幕状态
pub fn get_display_state() -> ScreenState {
    let current_state = SCREEN_STATE.load(Ordering::Relaxed);
    if current_state {
        ScreenState::On
    } else {
        ScreenState::Off
    }
}

/// 智能屏幕控制函数
/// 
/// 该函数会先检测当前屏幕状态，避免重复操作：
/// - 如果当前屏幕已开启且收到开启指令，则不执行操作
/// - 如果当前屏幕已关闭且收到关闭指令，则不执行操作
/// 
/// # Arguments
/// * `target_state` - 目标屏幕状态：`true` 表示开启屏幕，`false` 表示关闭屏幕
/// 
/// # Returns
/// * `bool` - 是否执行了操作：`true` 表示执行了操作，`false` 表示无需操作
pub fn set_display_smart(target_state: bool) -> bool {
    let current_state = get_display_state();
    let target_screen_state = if target_state { ScreenState::On } else { ScreenState::Off };
    
    // 检查是否需要执行操作
    match (current_state, target_screen_state) {
        (ScreenState::On, ScreenState::On) => {
            // 屏幕已经开启，无需重复操作
            false
        }
        (ScreenState::Off, ScreenState::Off) => {
            // 屏幕已经关闭，无需重复操作
            false
        }
        _ => {
            // 需要执行操作
            set_display(target_state);
            true
        }
    }
}

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
        
        // 更新内部状态跟踪
        SCREEN_STATE.store(on, Ordering::Relaxed);
        
        // 操作结果已在调用方记录日志
        
        // 注意：SendMessageW 的返回值在此上下文中通常不需要检查
        // 因为显示器电源控制是一个广播消息，没有特定的返回值含义
    }
}
