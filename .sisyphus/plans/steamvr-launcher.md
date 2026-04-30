# SteamVR 快捷启动器 - 实施计划

## 项目概述

使用 **Rust + egui** 构建一个轻量级 Windows GUI 工具，实现：
1. 创建桌面快捷方式指向 `SteamVR\bin\win64\vrstartup.exe`
2. 读取/修改 Steam 注册表 `Language` 值

### 技术选型

| 决策 | 选择 | 理由 |
|------|------|------|
| 语言 | Rust 1.90 | 单文件编译, ~5MB, 原生 Windows 支持 |
| GUI | egui + eframe | 轻量, 现代 UI, 无需额外依赖 |
| 注册表 | winreg crate | 原生 Windows 注册表操作, 支持 WOW6432Node |
| 快捷方式 | shortcut crate | 创建 .lnk 文件的标准库 |
| 打包 | cargo build --release | 单文件 .exe, 无需安装包 |

### 核心依赖

```toml
[dependencies]
eframe = "0.29"
egui = "0.29"
winreg = "0.52"
shortcut = "0.2"
dirs = "5"
serde = { version = "1", features = ["derive"] }
```

---

## TODOs

- [x] **T1: 初始化 Rust 项目** ✅
  - 创建 Cargo project (`cargo init`)
  - 添加所有依赖到 `Cargo.toml`
  - 配置 `cargo build --release` 输出单文件 exe
  - 验证: `cargo check` 通过, 无编译错误

- [x] **T2: 实现 Steam 路径自动检测模块** ✅
  - 读取 `HKEY_CURRENT_USER\Software\Valve\Steam\SteamPath` (优先)
  - Fallback: `HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Valve\SteamInstall\InstallPath`
  - 拼接完整 SteamVR 路径: `{SteamPath}\steamapps\common\SteamVR\bin\win64\vrstartup.exe`
  - 验证 `vrstartup.exe` 文件存在
  - 返回 `Option<(String, String)>` (Steam路径, SteamVR exe路径)
  - 验证: 单元测试覆盖注册表存在/不存在两种场景

- [x] **T3: 实现 Steam 语言读写模块** ✅
  - 读取 `HKEY_CURRENT_USER\Software\Valve\Steam\Language`
  - 写入 `HKEY_CURRENT_USER\Software\Valve\Steam\Language`
  - 定义语言枚举: english, 简体中文(schinese), 繁体中文(tchinese), 日本語(japanese), 한국어(koreana), Русский(russian), Deutsch(german), Français(french), Español(spanish), Italiano(italian), Português(portuges), 日本語(thai), Polski(polish)
  - 返回 `Result<String, Error>` (读) / `Result<(), Error>` (写)
  - 验证: 单元测试覆盖读写成功/失败场景

- [x] **T4: 实现桌面快捷方式创建模块** ✅
  - 使用 `lnks` crate 创建 `.lnk` 文件
  - Target: `vrstartup.exe` 完整路径
  - WorkingDirectory: `SteamVR\bin\win64\`
  - IconLocation: `vrstartup.exe,0`
  - Description: "SteamVR"
  - 检测桌面是否已存在同名快捷方式 (覆盖/跳过逻辑)
  - 返回 `Result<(), Error>`
  - 验证: 创建后验证 `.lnk` 文件存在, 属性正确

- [x] **T5: 实现 egui GUI 主界面** ✅
  - 窗口标题: "SteamVR 快捷启动器"
  - 窗口大小: 600x450
  - **区域 1 - Steam 路径**: 显示检测到的 Steam 路径 + 手动输入
  - **区域 2 - 快捷方式**: 显示 SteamVR exe 路径, "创建桌面快捷方式" 按钮, 状态提示
  - **区域 3 - 语言设置**: 下拉框 (语言列表), 显示当前语言, "应用更改" 按钮, ⚠️ 提示"需重启 Steam 生效"
  - **区域 4 - 启动按钮**: "启动 SteamVR" 大按钮 (执行 vrstartup.exe)
  - 所有操作有 Toast 提示 (成功/失败)
  - 深色主题
  - 验证: 窗口正常显示, 所有交互元素可用

- [x] **T6: 集成所有模块 + 错误处理 + 最终构建** ✅
  - 整合 T2-T5 到 `main.rs`
  - 完整的错误处理和用户友好的错误消息
  - 程序启动时自动检测 Steam 路径和当前语言
  - `cargo build --release` 成功, 产物 2.99MB
  - 验证: 完整流程测试 - 打开程序 → 检测路径 → 创建快捷方式 → 修改语言

---

## Final Verification Wave

- [x] **F1: Oracle 代码审查** ✅ APPROVE - 代码质量、架构、安全性审查通过。P0已修复 (移除shortcut依赖 + 路径注入防护)。
- [x] **F2: Oracle 构建验证** ✅ APPROVE - `cargo build --release` 成功, 产物 2.99MB, 2个dead_code警告(预期)。
- [x] **F3: Oracle 功能验证** ✅ APPROVE - Steam路径检测/语言读写/快捷方式创建/GUI集成全部通过。
- [x] **F4: Oracle UI/UX 审查** ✅ APPROVE - 界面布局合理, 交互流畅, 错误提示友好, 符合简洁理念。
- [x] **T7: 中文字体修复** ✅ - 运行时加载 Windows 系统 CJK 字体 (`msyh.ttc` → `simsun.ttc` → `simhei.ttf`)，使用 `FontData::from_owned()` 保持小体积。产物 3.14MB。

---

## 关键设计决策

### Steam 路径检测优先级
1. `HKCU\Software\Valve\Steam\SteamPath` (字符串值, 最常见)
2. `HKLM\SOFTWARE\WOW6432Node\Valve\SteamInstall\InstallPath` (64位系统)
3. `HKLM\SOFTWARE\Valve\SteamInstall\InstallPath` (32位系统)
4. 用户手动浏览选择

### 语言列表

| 显示名称 | 注册表值 |
|----------|----------|
| English | `english` |
| 简体中文 | `schinese` |
| 繁体中文 | `tchinese` |
| 日本語 | `japanese` |
| 한국어 | `koreana` |
| Русский | `russian` |
| Deutsch | `german` |
| Français | `french` |
| Español | `spanish` |
| Italiano | `italian` |
| Português | `portuges` |
| ภาษาไทย | `thai` |
| Polski | `polish` |

### 注册表路径
- **语言读取/写入**: `HKEY_CURRENT_USER\Software\Valve\Steam\Language` (REG_SZ)
- **Steam 路径**: `HKEY_CURRENT_USER\Software\Valve\Steam\SteamPath` (REG_SZ)

### 快捷方式属性
- **文件名**: `SteamVR.lnk`
- **位置**: 用户桌面 (`dirs::desktop_dir()`)
- **Target**: `{SteamPath}\steamapps\common\SteamVR\bin\win64\vrstartup.exe`
- **WorkingDirectory**: `{SteamPath}\steamapps\common\SteamVR\bin\win64\`
- **Icon**: `{SteamPath}\steamapps\common\SteamVR\bin\win64\vrstartup.exe,0`
