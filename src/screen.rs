use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND,
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
        
        // 发送显示器电源控制消息到所有窗口
        // HWND_BROADCAST: 广播到所有顶级窗口
        // WM_SYSCOMMAND: 系统命令消息
        // SC_MONITORPOWER: 显示器电源控制命令
        let _result = SendMessageW(
            HWND_BROADCAST,
            WM_SYSCOMMAND,
            WPARAM(SC_MONITORPOWER as usize),
            LPARAM(state),
        );
        
        // 记录操作结果（可选，用于调试）
        if on {
            println!("已发送开启屏幕指令");
        } else {
            println!("已发送关闭屏幕指令");
        }
        
        // 注意：SendMessageW 的返回值在此上下文中通常不需要检查
        // 因为显示器电源控制是一个广播消息，没有特定的返回值含义
    }
}
