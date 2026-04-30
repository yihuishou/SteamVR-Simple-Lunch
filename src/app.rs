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
    /// Steam 路径检测结果
    steam_paths: Option<SteamPaths>,
    /// 当前 Steam 语言值 (注册表原始值)
    current_language: String,
    /// 下拉框选中索引
    selected_language: usize,
    /// Toast 通知 (Some=显示中, None=无消息)
    toast: Option<Toast>,
    /// 手动输入的 Steam 路径
    manual_steam_path: String,
    /// 操作是否进行中 (防重复点击)
    is_working: bool,
    /// 上一帧时间戳
    last_time: Option<f64>,
}

impl SteamVrApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // 启动时自动检测 Steam 路径
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
            manual_steam_path: String::new(),
            is_working: false,
            last_time: None,
        }
    }

    /// 显示 Toast 消息 (持续 3 秒)
    fn show_toast(&mut self, message: String, success: bool) {
        self.toast = Some(Toast {
            message,
            success,
            remaining: 3.0,
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

    /// 重新检测 Steam 路径
    fn detect_steam(&mut self) {
        self.is_working = true;
        self.steam_paths = steam_path::detect_steam_path();
        self.is_working = false;

        if self.steam_paths.is_some() {
            self.show_toast("✅ 检测到 Steam 路径".to_string(), true);
        } else {
            self.show_toast(
                "❌ 未检测到 Steam，请手动输入路径".to_string(),
                false,
            );
        }
    }

    /// 应用手动输入的 Steam 路径
    fn apply_manual_path(&mut self, path: &str) {
        self.is_working = true;

        // 校验输入路径为绝对路径，防止路径注入
        if !Path::new(path).is_absolute() {
            self.show_toast("❌ 请输入绝对路径".to_string(), false);
            self.is_working = false;
            return;
        }

        // 拼接并验证 SteamVR exe 路径
        let steamvr_exe = format!(
            "{}\\steamapps\\common\\SteamVR\\bin\\win64\\vrstartup.exe",
            path
        );

        if Path::new(&steamvr_exe).exists() {
            self.steam_paths = Some(SteamPaths {
                steam_path: path.to_string(),
                steamvr_exe,
            });
            self.show_toast("✅ 手动路径验证成功".to_string(), true);
        } else {
            self.show_toast("❌ 路径无效，找不到 vrstartup.exe".to_string(), false);
        }

        self.is_working = false;
    }

    /// 创建桌面快捷方式
    fn create_shortcut(&mut self) {
        if let Some(ref paths) = self.steam_paths {
            self.is_working = true;
            let working_dir = shortcut_manager::get_working_dir_from_exe(&paths.steamvr_exe);

            match shortcut_manager::create_desktop_shortcut(&paths.steamvr_exe, &working_dir) {
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
            // 标题
            ui.heading("SteamVR 快捷启动器");
            ui.spacing();

            // ========== 区域 1: Steam 路径 ==========
            egui::Frame::NONE
                .fill(egui::Color32::from_black_alpha(40))
                .inner_margin(egui::Margin::same(12))
                .corner_radius(8.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                .show(ui, |ui| {
                    ui.strong("Steam 路径");
                    ui.separator();

                    // 显示检测到的路径
                    if let Some(ref paths) = self.steam_paths {
                        let green = egui::Color32::from_rgb(80, 200, 120);
                        ui.colored_label(green, &paths.steam_path);
                        ui.separator();
                        ui.label(format!("SteamVR: {}", paths.steamvr_exe));
                    } else {
                        let red = egui::Color32::from_rgb(220, 80, 80);
                        ui.colored_label(red, "未检测到 Steam");
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.is_working, egui::Button::new("重新检测"))
                            .clicked()
                        {
                            self.detect_steam();
                        }
                    });

                    // 手动路径输入
                    ui.separator();
                    ui.label("手动输入 Steam 安装路径:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.manual_steam_path);
                        if ui
                            .add_enabled(!self.is_working, egui::Button::new("应用"))
                            .clicked()
                        {
                            let path = self.manual_steam_path.trim().to_string();
                            if !path.is_empty() {
                                self.apply_manual_path(&path);
                            }
                        }
                    });
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
                                    egui::Button::new("创建桌面快捷方式"),
                                )
                                .clicked()
                            {
                                self.create_shortcut();
                            }
                        });
                    } else {
                        ui.label("检测到 Steam 后可创建快捷方式");
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
                            .add_enabled(!self.is_working, egui::Button::new("应用更改"))
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
                    egui::Button::new("🚀 启动 SteamVR").fill(if has_steam {
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
