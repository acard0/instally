use std::{process};

use egui::{ProgressBar};
use instally_core::{workloads::abstraction::{InstallyApp}};

pub struct AppWrapper {
    app: InstallyApp,
}

impl AppWrapper {
    pub fn new(app: InstallyApp) -> AppWrapper {

        AppWrapper { app }
    }
}

impl eframe::App for AppWrapper{
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        let binding = self.app.context.clone();
        let app = binding.lock();

        custom_window_frame(ctx, frame, format!("instally - {}", self.app.product.name).as_ref(), |ui| {
            
            ui.add_space(15.0);

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {

                // state information
                ui.label(app.get_state_information());

                // state progress
                let mut value = (app.get_progress() / 100.0) + 0.06;
                value = f32::min(value, 0.999); // at 1.0 ui stops updating itself, bug
                
                // progress bar
                let progress_bar = ui.add(ProgressBar::new(value)
                    .animate(!app.is_completed())
                );
            
                // progress bar text, centered
                let mut progress_text = egui::text::LayoutJob::simple_singleline(
                    format!("%{:.1}", app.get_progress()),
                    egui::FontId::new(11.0, egui::FontFamily::default()),
                    egui::Color32::WHITE
                );
                progress_text.halign = egui::Align::Center;
                ui.put(progress_bar.rect, egui::Label::new(progress_text));
                
            });
        
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                if ui.button("Abort").clicked() {
                    process::exit(1);
                }
            });

            // 

        });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }
}

fn custom_window_frame(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    use egui::*;
    let text_color = ctx.style().visuals.text_color();

    let mut style = (*ctx.style()).clone();
    style.spacing.interact_size.y = 25.0;
    ctx.set_style(style);
    
    catppuccin_egui::set_theme(&ctx, catppuccin_egui::MACCHIATO);

    // Height of the title bar
    let height = 28.0;

    CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();

            // Paint the frame:
            painter.rect(
                rect.shrink(1.0),
                10.0,
                Color32::from_rgb(36, 39, 58),
                Stroke::new(0.1, text_color),
            );

            // Paint the title:
            let _ = painter.text(
                rect.center_top() + vec2(0.0, height + 5.0 / 2.0),
                Align2::CENTER_CENTER,
                title,
                FontId::proportional(height * 0.8),
                text_color,
            );

            // Interact with the title bar (drag to move window):
            let title_bar_rect = {
                let mut rect = rect;
                rect.max.y = rect.min.y + height;
                rect
            };
            let title_bar_response =
                ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click());
            if title_bar_response.is_pointer_button_down_on() {
                frame.drag_window();
            }

            // Add the contents:
            let content_rect = {
                let mut rect = rect;
                rect.min.y = title_bar_rect.max.y + 10.0;
                rect
            }
            .shrink2(emath::vec2(10.0, 10.0));

            let mut content_ui = ui.child_ui(content_rect, *ui.layout());
            add_contents(&mut content_ui);
        });
    }