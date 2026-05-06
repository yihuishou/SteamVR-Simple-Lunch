use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// 快捷方式创建相关的错误类型
#[derive(Debug)]
pub enum ShortcutError {
    /// 无法获取桌面路径
    DesktopPathNotFound,
    /// I/O 错误
    IoError(io::Error),
    /// 快捷方式 crate 内部错误
    ShortcutCreateError(String),
}

impl fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShortcutError::DesktopPathNotFound => write!(f, "无法找到桌面路径"),
            ShortcutError::IoError(e) => write!(f, "I/O 错误: {}", e),
            ShortcutError::ShortcutCreateError(msg) => write!(f, "创建快捷方式失败: {}", msg),
        }
    }
}

impl std::error::Error for ShortcutError {}

impl From<io::Error> for ShortcutError {
    fn from(e: io::Error) -> Self {
        ShortcutError::IoError(e)
    }
}

/// 从 exe 路径提取其所在目录作为工作目录
pub fn get_working_dir_from_exe(exe_path: &str) -> String {
    let path = Path::new(exe_path);
    match path.parent() {
        Some(parent) => parent.to_string_lossy().to_string(),
        None => exe_path.to_string(),
    }
}

/// 获取用户桌面路径
/// 优先使用 dirs crate，fallback 到 %USERPROFILE%\Desktop
fn get_desktop_path() -> Result<PathBuf, ShortcutError> {
    if let Some(desktop) = dirs::desktop_dir() {
        return Ok(desktop);
    }
    // Fallback 到用户配置文件夹中的 Desktop
    if let Some(home) = dirs::home_dir() {
        let fallback = home.join("Desktop");
        return Ok(fallback);
    }
    Err(ShortcutError::DesktopPathNotFound)
}

/// 在用户桌面上创建 SteamVR 快捷方式
///
/// # Arguments
/// * `target_path` - 目标可执行文件的完整路径（如 vrstartup.exe）
/// * `working_dir` - 工作目录（如 SteamVR\bin\win64\）
/// * `icon_path` - 自定义图标路径（可选），未提供则使用目标 exe 图标
///
/// # Returns
/// 成功返回 Ok(())，失败返回 ShortcutError
pub fn create_desktop_shortcut(
    target_path: &str,
    working_dir: &str,
    icon_path: Option<&str>,
) -> Result<(), ShortcutError> {
    let desktop = get_desktop_path()?;
    let lnk_path = desktop.join("SteamVR.lnk");

    // 如果已存在同名 .lnk，静默删除
    if lnk_path.exists() {
        std::fs::remove_file(&lnk_path)?;
    }

    // 使用 lnks crate 构建并创建快捷方式
    let mut shortcut = lnks::Shortcut::new(target_path);
    shortcut.working_dir = Some(PathBuf::from(working_dir));
    shortcut.description = Some("SteamVR".to_string());
    shortcut.icon = Some(match icon_path {
        Some(path) if Path::new(path).exists() => lnks::Icon::with_index(PathBuf::from(path), 0),
        _ => lnks::Icon::with_index(PathBuf::from(target_path), 0),
    });

    shortcut
        .save(&lnk_path)
        .map_err(|e| ShortcutError::ShortcutCreateError(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_working_dir_from_exe() {
        let dir = get_working_dir_from_exe("C:\\Steam\\steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe");
        assert_eq!(dir, "C:\\Steam\\steamapps\\common\\SteamVR\\bin\\win64");
    }

    #[test]
    fn test_get_working_dir_from_exe_no_parent() {
        let dir = get_working_dir_from_exe("vrstartup.exe");
        assert_eq!(dir, "vrstartup.exe");
    }

    #[test]
    fn test_get_working_dir_from_exe_empty_parent() {
        let dir = get_working_dir_from_exe("\\");
        assert!(dir.is_empty() || dir == "\\");
    }

    #[test]
    fn test_shortcut_error_display() {
        let err = ShortcutError::DesktopPathNotFound;
        assert!(!format!("{}", err).is_empty());

        let io_err = ShortcutError::IoError(io::Error::new(io::ErrorKind::NotFound, "文件未找到"));
        assert!(format!("{}", io_err).contains("I/O 错误"));

        let create_err = ShortcutError::ShortcutCreateError("测试错误".to_string());
        assert!(format!("{}", create_err).contains("测试错误"));
    }
}
