use std::fs;
use std::path::Path;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

/// Steam 路径信息
#[derive(Debug, Clone)]
pub struct SteamPaths {
    /// Steam 安装根目录
    pub steam_path: String,
    /// SteamVR 启动程序完整路径
    pub steamvr_exe: String,
}

/// 检测 Steam 安装路径的错误类型
#[derive(Debug)]
pub enum SteamPathError {
    /// 注册表读取失败
    RegistryError(String),
    /// Steam 路径未在注册表中发现
    SteamNotFound,
}

impl std::fmt::Display for SteamPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamPathError::RegistryError(msg) => write!(f, "注册表读取失败: {}", msg),
            SteamPathError::SteamNotFound => write!(f, "未在注册表中找到 Steam 安装路径"),
        }
    }
}

impl std::error::Error for SteamPathError {}

/// SteamVR 相对于 Steam 安装目录的路径
const STEAMVR_EXE_REL: &str = "steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe";

/// 从 HKEY_CURRENT_USER 读取 Steam 路径
fn detect_from_hkcu() -> Result<String, SteamPathError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags("Software\\Valve\\Steam", KEY_READ)
        .map_err(|e| SteamPathError::RegistryError(format!("HKCU 子键打开失败: {}", e)))?;
    key.get_value("SteamPath")
        .map_err(|e| SteamPathError::RegistryError(format!("HKCU SteamPath 读取失败: {}", e)))
}

/// 从 HKEY_LOCAL_MACHINE (WOW6432Node) 读取 Steam 路径
fn detect_from_hklm_wow64() -> Result<String, SteamPathError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey_with_flags("SOFTWARE\\WOW6432Node\\Valve\\SteamInstall", KEY_READ)
        .map_err(|e| SteamPathError::RegistryError(format!("HKLM WOW6432Node 子键打开失败: {}", e)))?;
    key.get_value("InstallPath")
        .map_err(|e| SteamPathError::RegistryError(format!("HKLM WOW6432Node InstallPath 读取失败: {}", e)))
}

/// 从 HKEY_LOCAL_MACHINE 读取 Steam 路径
fn detect_from_hklm() -> Result<String, SteamPathError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey_with_flags("SOFTWARE\\Valve\\SteamInstall", KEY_READ)
        .map_err(|e| SteamPathError::RegistryError(format!("HKLM 子键打开失败: {}", e)))?;
    key.get_value("InstallPath")
        .map_err(|e| SteamPathError::RegistryError(format!("HKLM InstallPath 读取失败: {}", e)))
}

/// 从注册表检测 Steam 安装路径
///
/// 按优先级依次尝试:
/// 1. `HKEY_CURRENT_USER\Software\Valve\Steam\SteamPath`
/// 2. `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\SteamInstall\InstallPath`
/// 3. `HKEY_LOCAL_MACHINE\SOFTWARE\Valve\SteamInstall\InstallPath`
fn detect_steam_install_path() -> Result<String, SteamPathError> {
    detect_from_hkcu()
        .or_else(|_| detect_from_hklm_wow64())
        .or_else(|_| detect_from_hklm())
}

/// 递归搜索指定目录及其子目录，查找 `vrstartup.exe`
///
/// 返回 `Some(SteamPaths)` 表示找到了文件，`None` 表示未找到
pub fn find_vrstartup_in_dir(dir: &str) -> Option<SteamPaths> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return None;
    }

    search_vrstartup(dir_path)
}

  /// 递归搜索目录下是否存在 `vrstartup.exe`，找到后推导 Steam 路径
fn search_vrstartup(dir: &Path) -> Option<SteamPaths> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name == "vrstartup.exe" {
                    // 从 exe 路径向上推导 Steam 根目录
                    let full_exe = path.to_string_lossy().to_string();
                    if let Some(steam_root) = extract_steam_root(&path) {
                        return Some(SteamPaths {
                            steam_path: steam_root,
                            steamvr_exe: full_exe,
                        });
                    }
                }
            }
        } else if path.is_dir() {
            // 递归搜索子目录
            if let Some(result) = search_vrstartup(&path) {
                return Some(result);
            }
        }
    }

    None
}

/// 从 vrstartup.exe 的路径推导 Steam 根目录
/// 期望路径包含 steamapps/common/SteamVR/bin/win64
fn extract_steam_root(vrstartup_path: &Path) -> Option<String> {
    let mut current = vrstartup_path.parent()?;
    // bin/win64 -> SteamVR -> common -> steamapps -> steam root
    for _ in 0..4 {
        current = current.parent()?;
    }

    let steam_root = current.to_string_lossy().to_string();
    // 验证：该目录下应该有 steamapps 文件夹
    if Path::new(&steam_root).join("steamapps").is_dir() {
        Some(steam_root)
    } else {
        None
    }
}

/// 自动检测 Steam 安装路径和 SteamVR 启动程序
///
/// 返回 `Some(SteamPaths)` 表示检测成功，`None` 表示未找到 Steam 或 vrstartup.exe
pub fn detect_steam_path() -> Option<SteamPaths> {
    let steam_path = detect_steam_install_path().ok()?;

    // 拼接 SteamVR 启动程序路径
    let steamvr_exe = format!("{}\\{}", steam_path, STEAMVR_EXE_REL);

    // 验证 vrstartup.exe 文件是否存在
    if !Path::new(&steamvr_exe).exists() {
        return None;
    }

    Some(SteamPaths {
        steam_path,
        steamvr_exe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_steam_path_returns_correct_type() {
        // 仅验证函数调用和返回类型，不要求 Steam 一定安装
        let result = detect_steam_path();
        match result {
            Some(paths) => {
                assert!(!paths.steam_path.is_empty());
                assert!(paths.steamvr_exe.contains("vrstartup.exe"));
            }
            None => {
                // Steam 未安装或未检测到，测试通过
            }
        }
    }
}
