use std::fmt;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

/// 语言错误类型
#[derive(Debug)]
pub enum LanguageError {
    /// 注册表读取错误
    RegistryRead(String),
    /// 注册表写入错误
    RegistryWrite(String),
}

impl fmt::Display for LanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageError::RegistryRead(msg) => write!(f, "读取注册表失败: {}", msg),
            LanguageError::RegistryWrite(msg) => write!(f, "写入注册表失败: {}", msg),
        }
    }
}

impl std::error::Error for LanguageError {}

impl From<std::io::Error> for LanguageError {
    fn from(err: std::io::Error) -> Self {
        LanguageError::RegistryRead(err.to_string())
    }
}

/// Steam 支持的语言列表 (显示名称, 注册表值)
pub const LANGUAGES: &[(&str, &str)] = &[
    ("English", "english"),
    ("简体中文", "schinese"),
    ("繁體中文", "tchinese"),
    ("日本語", "japanese"),
    ("한국어", "koreana"),
    ("Русский", "russian"),
    ("Deutsch", "german"),
    ("Français", "french"),
    ("Español", "spanish"),
    ("Italiano", "italian"),
    ("Português", "portuges"),
    ("ภาษาไทย", "thai"),
    ("Polski", "polish"),
];

/// 从注册表读取当前 Steam 语言值
/// 键路径: HKEY_CURRENT_USER\Software\Valve\Steam\Language
/// 如果键不存在，返回默认值 "english"
pub fn read_steam_language() -> Result<String, LanguageError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(steam_key) = hkcu.open_subkey_with_flags("Software\\Valve\\Steam", winreg::enums::KEY_READ) {
        if let Ok(lang) = steam_key.get_value::<String, _>("Language") {
            return Ok(lang);
        }
    }

    // 键不存在或读取失败，返回默认值
    Ok("english".to_string())
}

/// 写入新的 Steam 语言值到注册表
/// 键路径: HKEY_CURRENT_USER\Software\Valve\Steam\Language
/// 如果键不存在则自动创建
pub fn write_steam_language(lang: &str) -> Result<(), LanguageError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _disp) = hkcu
        .create_subkey_with_flags("Software\\Valve\\Steam", KEY_WRITE)
        .map_err(|e| LanguageError::RegistryWrite(e.to_string()))?;
    key.set_value("Language", &lang)
        .map_err(|e| LanguageError::RegistryWrite(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_languages_not_empty() {
        assert!(!LANGUAGES.is_empty());
        assert!(LANGUAGES.len() >= 10);
    }

    #[test]
    fn test_languages_no_duplicate_values() {
        let mut values: Vec<&str> = LANGUAGES.iter().map(|(_, v)| *v).collect();
        values.sort();
        values.dedup();
        assert_eq!(values.len(), LANGUAGES.len());
    }
}
