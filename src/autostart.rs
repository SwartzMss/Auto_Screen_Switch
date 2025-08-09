use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, KEY_QUERY_VALUE, REG_SZ, REG_VALUE_TYPE,
};

const STARTUP_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "AutoScreenSwitch";

/// 将字符串转换为 Windows 宽字符格式
fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 检查是否已设置开机启动
pub fn is_autostart_enabled() -> bool {
    unsafe {
        let mut key: HKEY = HKEY::default();
        let key_name = to_wide_string(STARTUP_KEY);
        
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_name.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        ).is_err() {
            return false;
        }

        let app_name = to_wide_string(APP_NAME);
        let mut data_type = REG_VALUE_TYPE(0);
        let mut data_size = 0u32;
        
        let result = windows::Win32::System::Registry::RegQueryValueExW(
            key,
            PCWSTR(app_name.as_ptr()),
            None,
            Some(&mut data_type),
            None,
            Some(&mut data_size),
        );

        let _ = RegCloseKey(key);
        result.is_ok()
    }
}

/// 启用开机启动
pub fn enable_autostart() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("无法获取程序路径: {}", e))?;
    
    let exe_path_str = exe_path.to_string_lossy();
    
    unsafe {
        let mut key: HKEY = HKEY::default();
        let key_name = to_wide_string(STARTUP_KEY);
        
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_name.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        ).map_err(|e| format!("无法打开注册表项: {:?}", e))?;

        let app_name = to_wide_string(APP_NAME);
        let exe_path_wide = to_wide_string(&exe_path_str);
        
        let data = exe_path_wide.as_ptr() as *const u8;
        let data_slice = std::slice::from_raw_parts(data, exe_path_wide.len() * 2);
        
        let result = RegSetValueExW(
            key,
            PCWSTR(app_name.as_ptr()),
            0,
            REG_SZ,
            Some(data_slice),
        );

        let _ = RegCloseKey(key);
        
        result.map_err(|e| format!("设置注册表值失败: {:?}", e))?;
    }
    
    Ok(())
}

/// 禁用开机启动
pub fn disable_autostart() -> Result<(), String> {
    unsafe {
        let mut key: HKEY = HKEY::default();
        let key_name = to_wide_string(STARTUP_KEY);
        
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_name.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        ).map_err(|e| format!("无法打开注册表项: {:?}", e))?;

        let app_name = to_wide_string(APP_NAME);
        let result = RegDeleteValueW(key, PCWSTR(app_name.as_ptr()));

        let _ = RegCloseKey(key);
        
        result.map_err(|e| format!("删除注册表值失败: {:?}", e))?;
    }
    
    Ok(())
}

/// 切换开机启动状态
pub fn toggle_autostart() -> Result<bool, String> {
    if is_autostart_enabled() {
        disable_autostart()?;
        Ok(false)
    } else {
        enable_autostart()?;
        Ok(true)
    }
}
