mod app;
mod shortcut_manager;
mod steam_language;
mod steam_path;

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 450.0])
            .with_title("SteamVR 快捷启动器"),
        ..Default::default()
    };

    eframe::run_native(
        "SteamVR 快捷启动器",
        options,
        Box::new(|cc| {
            // 配置 CJK 中文字体，解决中文乱码问题
            let mut fonts = FontDefinitions::default();
            load_cjk_fonts(&mut fonts);
            cc.egui_ctx.set_fonts(fonts);
            
            // 全局 UI 缩放 1.2 倍
            cc.egui_ctx.set_zoom_factor(1.2);

            Ok(Box::new(app::SteamVrApp::new(cc)))
        }),
    )
}

/// 从系统字体目录加载 CJK 中文字体
fn load_cjk_fonts(fonts: &mut FontDefinitions) {
    let cjk_proportional: Vec<&str> = if cfg!(target_os = "windows") {
        // Windows 中文字体文件路径
        vec![
            ("C:\\Windows\\Fonts\\msyh.ttc", "Microsoft YaHei"),       // 微软雅黑
            ("C:\\Windows\\Fonts\\msyhbd.ttc", "Microsoft YaHei Bold"), // 微软雅黑粗体
            ("C:\\Windows\\Fonts\\simhei.ttf", "SimHei"),               // 黑体
            ("C:\\Windows\\Fonts\\simsun.ttc", "SimSun"),               // 宋体
        ]
        .into_iter()
        .filter_map(|(path, name)| {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(data)));
                Some(name)
            } else {
                None
            }
        })
        .collect()
    } else if cfg!(target_os = "macos") {
        vec![
            ("/System/Library/Fonts/PingFang.ttc", "PingFang SC"),
            ("/System/Library/Fonts/STHeiti Lite.ttc", "STHeiti"),
            ("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", "Arial Unicode"),
        ]
        .into_iter()
        .filter_map(|(path, name)| {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(data)));
                Some(name)
            } else {
                None
            }
        })
        .collect()
    } else {
        // Linux
        vec![
            ("/usr/share/fonts/truetype/noto/NotoSansSC-Regular.ttf", "Noto Sans SC"),
            ("/usr/share/fonts/opentype/noto/NotoSansSC-Regular.otf", "Noto Sans SC"),
            (
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                "WenQuanYi Micro Hei",
            ),
        ]
        .into_iter()
        .filter_map(|(path, name)| {
            if let Ok(data) = std::fs::read(path) {
                fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(data)));
                Some(name)
            } else {
                None
            }
        })
        .collect()
    };

    // 将 CJK 字体添加到 Proportional 和 Monospace 家族，作为 fallback
    if !cjk_proportional.is_empty() {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_insert_with(|| {
                let mut vec = Vec::new();
                vec.push("Ubuntu-Light".to_owned());
                vec
            })
            .extend(cjk_proportional.iter().map(|s| s.to_string()));

        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_insert_with(|| {
                let mut vec = Vec::new();
                vec.push("Hack".to_owned());
                vec
            })
            .extend(cjk_proportional.iter().map(|s| s.to_string()));
    }
}
