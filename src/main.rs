mod watermark_utils;

use bitvec::prelude::*; // 引入位操作
use blind_watermark::prelude::*; // 引入盲水印 API
use eframe::egui;
use watermark_utils::{FIXED_PAYLOAD_SIZE, pack_watermark, unpack_watermark};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        // 设定一个轻量化工具的标准窗口大小
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 380.0]),
        ..Default::default()
    };

    eframe::run_native(
        "极简图片隐形水印工具",
        options,
        Box::new(|cc| {
            // 自动向 egui 的 Context 注入当前操作系统的默认中文/CJK 字体
            // 加上 expect，一旦加载字体失败，会在终端打印出清晰的错误原因
            egui_chinese_font::setup_chinese_fonts(&cc.egui_ctx).expect("系统缺少中文字体支持！");

            Box::new(WatermarkApp::default())
        }),
    )
}

#[derive(PartialEq)]
enum AppMode {
    Embed,
    Extract,
}

/// 维护 GUI 界面的状态数据
struct WatermarkApp {
    mode: AppMode,
    input_path: String,
    output_path: String,
    watermark_text: String,
    seed_password: i32,     // 界面暴露给用户的密码 (底层对应 Seed)
    result_message: String, // 用于向用户展示操作结果或报错信息
}

impl Default for WatermarkApp {
    fn default() -> Self {
        Self {
            mode: AppMode::Embed,
            input_path: String::new(),
            output_path: String::new(),
            watermark_text: "版权所有，盗图必究".to_string(),
            seed_password: 114514, // 预设默认密码
            result_message: String::new(),
        }
    }
}

impl eframe::App for WatermarkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("隐形水印");
            ui.separator();

            // === 模式切换 Tab ===
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, AppMode::Embed, "嵌入水印");
                ui.selectable_value(&mut self.mode, AppMode::Extract, "提取水印");
            });
            ui.separator();
            ui.add_space(10.0);

            // 嵌入模式 UI
            if self.mode == AppMode::Embed {
                ui.horizontal(|ui| {
                    if ui.button("选择原图...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.input_path = path.display().to_string();
                            self.output_path = format!("{}_watermarked.png", self.input_path);
                        }
                    }
                    ui.label(if self.input_path.is_empty() {
                        "未选择文件"
                    } else {
                        &self.input_path
                    });
                });

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("水印内容:");
                    ui.text_edit_singleline(&mut self.watermark_text);
                });

                ui.horizontal(|ui| {
                    ui.label("提取密码:");
                    ui.add(egui::DragValue::new(&mut self.seed_password));
                    ui.label(egui::RichText::new("(提取时需保持一致)").weak());
                });

                ui.add_space(15.0);

                if ui.button("注入隐形水印并保存").clicked() {
                    if self.input_path.is_empty() {
                        self.result_message = "请先选择要处理的图片！".to_string();
                    } else {
                        match pack_watermark(&self.watermark_text) {
                            Ok(payload) => {
                                let payload_bits = payload.view_bits::<Lsb0>();
                                let seed_opt = Some(self.seed_password as u64);

                                let res = embed_watermark_bits(
                                    &self.input_path,
                                    &self.output_path,
                                    payload_bits,
                                    seed_opt,
                                );

                                if res.is_ok() {
                                    self.result_message =
                                        format!("成功！已保存至:\n{}", self.output_path);
                                } else {
                                    self.result_message =
                                        "注入失败，可能是图片格式或尺寸不支持。".to_string();
                                }
                            }
                            Err(e) => self.result_message = e,
                        }
                    }
                }
            }

            // 提取模式 UI
            if self.mode == AppMode::Extract {
                ui.horizontal(|ui| {
                    if ui.button("选择带水印图片...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.input_path = path.display().to_string();
                        }
                    }
                    ui.label(if self.input_path.is_empty() {
                        "未选择文件"
                    } else {
                        &self.input_path
                    });
                });

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("提取密码:");
                    ui.add(egui::DragValue::new(&mut self.seed_password));
                });

                ui.add_space(15.0);

                if ui.button("尝试提取隐藏信息").clicked() {
                    if self.input_path.is_empty() {
                        self.result_message = "请先选择图片！".to_string();
                    } else {
                        let extract_bit_len = FIXED_PAYLOAD_SIZE * 8;
                        let seed_opt = Some(self.seed_password as u64);

                        match extract_watermark_bits(&self.input_path, extract_bit_len, seed_opt) {
                            Ok(raw_bits) => {
                                let raw_payload = raw_bits.into_vec();
                                match unpack_watermark(&raw_payload) {
                                    Ok(text) => {
                                        self.result_message =
                                            format!("提取成功：\n\n【 {} 】", text)
                                    }
                                    Err(e) => self.result_message = format!("{}", e),
                                }
                            }
                            Err(_) => {
                                self.result_message = "提取崩溃！请检查文件是否为图片。".to_string()
                            }
                        }
                    }
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            // 状态反馈输出面板
            let color = if self.result_message.contains("❌") || self.result_message.contains("⚠️")
            {
                egui::Color32::from_rgb(250, 100, 100)
            } else {
                egui::Color32::from_rgb(100, 250, 100)
            };

            ui.label(
                egui::RichText::new(&self.result_message)
                    .color(color)
                    .size(14.0),
            );
        });
    }
}
