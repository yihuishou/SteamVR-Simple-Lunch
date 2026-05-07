use std::process::Command;
use std::path::Path;

use eframe::egui;

use crate::steam_path::{self, SteamPaths};
use crate::steam_language::{self, LANGUAGES};
use crate::shortcut_manager;

/// Toast 通知状态
struct Toast {
    /// 消息内容
    message: String,
    /// 是否成功
    success: bool,
    /// 剩余显示时间 (秒)
    remaining: f64,
}

/// 主应用状态
pub struct SteamVrApp {
    /// SteamVR 路径检测结果
    steam_paths: Option<SteamPaths>,
    /// 当前 Steam 语言值 (注册表原始值)
    current_language: String,
    /// 下拉框选中索引
    selected_language: usize,
    /// Toast 通知 (Some=显示中, None=无消息)
    toast: Option<Toast>,
  
    /// 操作是否进行中 (防重复点击)
    is_working: bool,
    /// 上一帧时间戳
    last_time: Option<f64>,
}

impl SteamVrApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // 启动时自动检测 SteamVR 路径
        let steam_paths = steam_path::detect_steam_path();

        // 启动时自动读取当前语言
        let current_language = steam_language::read_steam_language()
            .unwrap_or_else(|_| "english".to_string());

        // 根据当前语言匹配下拉框选中索引
        let selected_language = LANGUAGES
            .iter()
            .position(|(_, val)| *val == current_language)
            .unwrap_or(0);

        Self {
            steam_paths,
            current_language,
            selected_language,
            toast: None,
            is_working: false,
            last_time: None,
        }
    }

    /// 显示 Toast 消息 (持续 6 秒)
    fn show_toast(&mut self, message: String, success: bool) {
        self.toast = Some(Toast {
            message,
            success,
            remaining: 6.0,
        });
    }

    /// 更新 Toast 计时器
    fn update_toast(&mut self, dt: f64) {
        if let Some(ref mut t) = self.toast {
            t.remaining -= dt;
            if t.remaining <= 0.0 {
                self.toast = None;
            }
        }
    }

    /// 重新检测 SteamVR 路径
    fn detect_steam(&mut self) {
        self.is_working = true;
        self.steam_paths = steam_path::detect_steam_path();
        self.is_working = false;

        if self.steam_paths.is_some() {
            self.show_toast("✅ 检测到 SteamVR 路径".to_string(), true);
        } else {
            self.show_toast(
                "❌ 未检测到 SteamVR，请手动指定路径".to_string(),
                false,
            );
        }
    }

    /// 应用手动选择的 SteamVR 路径
    fn apply_manual_path(&mut self, path: &str) {
        self.is_working = true;

        // 校验输入路径为绝对路径，防止路径注入
        if !Path::new(path).is_absolute() {
            self.show_toast("❌ 请选择有效的目录".to_string(), false);
            self.is_working = false;
            return;
        }

        // 先尝试直接拼接路径验证
        let steamvr_exe = format!(
            "{}\\steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe",
            path
        );

        if Path::new(&steamvr_exe).exists() {
            self.steam_paths = Some(SteamPaths {
                steamvr_path: path.to_string(),
                steamvr_exe,
            });
            self.show_toast("✅ 路径验证成功".to_string(), true);
        } else {
            // 在选定目录及其子目录中递归搜索 vrstartup.exe
            if let Some(found_paths) = steam_path::find_vrstartup_in_dir(path) {
                self.steam_paths = Some(found_paths);
                self.show_toast("✅ 在子目录中找到 SteamVR".to_string(), true);
            } else {
                self.show_toast("❌ 未找到 SteamVR，请确认目录正确".to_string(), false);
            }
        }

        self.is_working = false;
    }

    /// 创建桌面快捷方式
    fn create_shortcut(&mut self) {
        if let Some(ref paths) = self.steam_paths {
            self.is_working = true;
            let working_dir = shortcut_manager::get_working_dir_from_exe(&paths.steamvr_exe);

            // 获取自定义图标路径（相对于可执行文件目录）
            let icon_path = std::env::current_exe()
                .ok()
                .and_then(|exe_path| {
                    let icon = exe_path.parent()?.join("assets").join("SteamVRIcon.ico");
                    if icon.exists() {
                        Some(icon.to_string_lossy().to_string())
                    } else {
                        None
                    }
                });

            match shortcut_manager::create_desktop_shortcut(
                &paths.steamvr_exe,
                &working_dir,
                icon_path.as_deref(),
            ) {
                Ok(()) => self.show_toast("✅ 桌面快捷方式创建成功".to_string(), true),
                Err(e) => {
                    self.show_toast(format!("❌ 创建快捷方式失败: {}", e), false);
                }
            }
            self.is_working = false;
        }
    }

    /// 应用语言更改
    fn apply_language(&mut self) {
        let lang_value = LANGUAGES[self.selected_language].1;
        self.is_working = true;

        match steam_language::write_steam_language(lang_value) {
            Ok(()) => {
                self.current_language = lang_value.to_string();
                self.show_toast("✅ 语言已更改，需重启 Steam 生效".to_string(), true);
            }
            Err(e) => {
                self.show_toast(format!("❌ 写入语言失败: {}", e), false);
            }
        }
        self.is_working = false;
    }

    /// 启动 SteamVR
    fn launch_steamvr(&mut self) {
        if let Some(ref paths) = self.steam_paths {
            self.is_working = true;

            // 校验 exe 路径为绝对路径，防止路径注入
            if !Path::new(&paths.steamvr_exe).is_absolute() {
                self.show_toast("❌ 路径不安全，拒绝启动".to_string(), false);
                self.is_working = false;
                return;
            }

            match Command::new(&paths.steamvr_exe).spawn() {
                Ok(_) => self.show_toast("✅ SteamVR 启动中...".to_string(), true),
                Err(e) => self.show_toast(format!("❌ 启动失败: {}", e), false),
            }
            self.is_working = false;
        }
    }
}

impl eframe::App for SteamVrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 计算帧间隔时间
        let current_time = ctx.input(|i| i.time);
        let dt = self
            .last_time
            .map(|t| current_time - t)
            .unwrap_or(0.016); // 默认约 60fps
        self.last_time = Some(current_time);

        self.update_toast(dt);

        egui::CentralPanel::default().show(ctx, |ui| {
 

            // ========== 区域 1: SteamVR 路径 ==========
            egui::Frame::NONE
                .fill(egui::Color32::from_black_alpha(40))
                .inner_margin(egui::Margin::same(12))
                .corner_radius(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                .show(ui, |ui| {
                    ui.strong("SteamVR 路径");
                    ui.separator();

                    // 显示检测到的路径
                    if let Some(ref paths) = self.steam_paths {
                        let green = egui::Color32::from_rgb(80, 200, 120);
                        ui.colored_label(green, &paths.steamvr_path);
                        ui.separator();
                        ui.label(format!("SteamVR: {}", paths.steamvr_exe));
                    } else {
                        let red = egui::Color32::from_rgb(220, 80, 80);
                        ui.colored_label(red, "未检测到 SteamVR");
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.is_working, egui::Button::new("重新检测").min_size(egui::vec2(200.0, 0.0)))
                            .clicked()
                        {
                            self.detect_steam();
                        }
                    });

                    // 选择 SteamVR 安装路径
                    ui.separator();
                    if ui
                        .add_enabled(!self.is_working, egui::Button::new("📂 选择 SteamVR 安装路径").min_size(egui::vec2(200.0, 0.0)))
                        .clicked()
                    {
                        // 打开文件夹选择对话框
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            let path_str = path.to_string_lossy().to_string();
                            self.apply_manual_path(&path_str);
                        }
                    }
                });

            ui.spacing();

            // ========== 区域 2: 桌面快捷方式 ==========
            egui::Frame::NONE
                .fill(egui::Color32::from_black_alpha(40))
                .inner_margin(egui::Margin::same(12))
                .corner_radius(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                .show(ui, |ui| {
                    ui.strong("桌面快捷方式");
                    ui.separator();

                    if let Some(ref paths) = self.steam_paths {
                        ui.label(format!("目标: {}", paths.steamvr_exe));
                     ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !self.is_working,
                                    egui::Button::new("创建桌面快捷方式").min_size(egui::vec2(200.0, 0.0)),
                                )
                                .clicked()
                            {
                                self.create_shortcut();
                            }
                        });
                    } else {
                        ui.label("检测到 SteamVR 后可创建快捷方式");
                    }
                });

            ui.spacing();

            // ========== 区域 3: 语言设置 ==========
            egui::Frame::NONE
                .fill(egui::Color32::from_black_alpha(40))
                .inner_margin(egui::Margin::same(12))
                .corner_radius(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                .show(ui, |ui| {
                    ui.strong("语言设置");
                    ui.separator();

                    // 下拉框
                    let selected_name = LANGUAGES[self.selected_language].0;
                    egui::ComboBox::from_label("选择语言")
                        .selected_text(selected_name)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for (index, (display, _)) in LANGUAGES.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected_language,
                                    index,
                                    *display,
                                );
                            }
                        });

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.is_working, egui::Button::new("应用更改").min_size(egui::vec2(200.0, 0.0)))
                            .clicked()
                        {
                            self.apply_language();
                        }
                    });

                    let warning = egui::Color32::from_rgb(220, 180, 60);
                    ui.colored_label(warning, "⚠️ 需重启 Steam 生效");
                });

            ui.spacing();

            // ========== 区域 4: 启动 ==========
            let has_steam = self.steam_paths.is_some();
            if ui
                .add_enabled(
                    has_steam && !self.is_working,
                    egui::Button::new(egui::RichText::new("🚀 启动 SteamVR").color(egui::Color32::BLACK))
                        .min_size(egui::vec2(200.0, 0.0))
                        .fill(if has_steam {
                            egui::Color32::from_rgb(40, 120, 200)
                        } else {
                            egui::Color32::from_gray(60)
                        }),
                )
                .clicked()
            {
                self.launch_steamvr();
            }

            ui.separator();

            // ========== Toast 通知 ==========
            if let Some(ref toast) = self.toast {
                let color = if toast.success {
                    egui::Color32::from_rgb(80, 200, 120)
                } else {
                    egui::Color32::from_rgb(220, 80, 80)
                };

                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(80))
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.colored_label(color, &toast.message);
                    });
            }
        });
    }
}
