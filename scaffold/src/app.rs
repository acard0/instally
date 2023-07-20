
use std::process;

use eframe::egui;
use egui::ProgressBar;
use instally_core::{workloads::abstraction::InstallyApp, *};

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

        let binding = self.app.get_context();
        let app = binding.lock();

        custom_window_frame(ctx, frame, format!("instally - {}", self.app.get_product().name).as_ref(), |ui| {

            ui.add_space(15.0);

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {

                // state information
                ui.label(app.get_state_information());

                // state progress
                let value = match app.is_completed() {
                    true => 0.999,
                    _ => {  // at 1.0 ui stops updating itself, bug
                        let q = app.get_progress() / 100.0;
                        f32::min(q, 0.999)
                    }
                };
                
                // progress bar
                let progress_bar = ui.add(ProgressBar::new(value)
                    .animate(!app.is_completed())
                );
            
                // progress bar text, centered
                let mut progress_text = egui::text::LayoutJob::simple_singleline(
                    format!("%{:.1}", ((value + 0.001) * 100.0)),
                    egui::FontId::new(11.0, egui::FontFamily::default()),
                    egui::Color32::WHITE
                );
                progress_text.halign = egui::Align::Center;
                ui.put(progress_bar.rect, egui::Label::new(progress_text));
                
            });
        
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                if ui.button(t!(if app.is_completed() { "ok" } else { "abort" })).clicked() {
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
    
    set_theme(&ctx, MOCHA);

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

// catppuccin-egui = "3.0.0"

use egui::{epaint, style, Color32};

/// Apply the given theme to a [`Context`](egui::Context).
fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();
    ctx.set_visuals(egui::Visuals {
        override_text_color: Some(theme.text),
        hyperlink_color: theme.rosewater,
        faint_bg_color: theme.surface0,
        extreme_bg_color: theme.crust,
        code_bg_color: theme.mantle,
        warn_fg_color: theme.peach,
        error_fg_color: theme.maroon,
        window_fill: theme.base,
        panel_fill: theme.base,
        window_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.window_stroke
        },
        widgets: style::Widgets {
            noninteractive: make_widget_visual(old.widgets.noninteractive, &theme, theme.base),
            inactive: make_widget_visual(old.widgets.inactive, &theme, theme.surface0),
            hovered: make_widget_visual(old.widgets.hovered, &theme, theme.surface2),
            active: make_widget_visual(old.widgets.active, &theme, theme.surface1),
            open: make_widget_visual(old.widgets.open, &theme, theme.surface0),
        },
        selection: style::Selection {
            bg_fill: theme
                .blue
                .linear_multiply(if theme == LATTE { 0.4 } else { 0.2 }),
            stroke: egui::Stroke {
                color: theme.overlay1,
                ..old.selection.stroke
            },
        },
        window_shadow: epaint::Shadow {
            color: theme.base,
            ..old.window_shadow
        },
        popup_shadow: epaint::Shadow {
            color: theme.base,
            ..old.popup_shadow
        },
        ..old
    });
}

fn make_widget_visual(
    old: style::WidgetVisuals,
    theme: &Theme,
    bg_fill: egui::Color32,
) -> style::WidgetVisuals {
    style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke {
            color: theme.overlay1,
            ..old.bg_stroke
        },
        fg_stroke: egui::Stroke {
            color: theme.text,
            ..old.fg_stroke
        },
        ..old
    }
}

// FIXME: Theme should be `Copy` since it isn't big enough to generate a call to `memcpy`,
// do this when egui releases a minor version
/// The colors for a theme variant.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct Theme {
    pub rosewater: Color32,
    pub flamingo: Color32,
    pub pink: Color32,
    pub mauve: Color32,
    pub red: Color32,
    pub maroon: Color32,
    pub peach: Color32,
    pub yellow: Color32,
    pub green: Color32,
    pub teal: Color32,
    pub sky: Color32,
    pub sapphire: Color32,
    pub blue: Color32,
    pub lavender: Color32,
    pub text: Color32,
    pub subtext1: Color32,
    pub subtext0: Color32,
    pub overlay2: Color32,
    pub overlay1: Color32,
    pub overlay0: Color32,
    pub surface2: Color32,
    pub surface1: Color32,
    pub surface0: Color32,
    pub base: Color32,
    pub mantle: Color32,
    pub crust: Color32,
}

const LATTE: Theme = Theme {
    rosewater: Color32::from_rgb(220, 138, 120),
    flamingo: Color32::from_rgb(221, 120, 120),
    pink: Color32::from_rgb(234, 118, 203),
    mauve: Color32::from_rgb(136, 57, 239),
    red: Color32::from_rgb(210, 15, 57),
    maroon: Color32::from_rgb(230, 69, 83),
    peach: Color32::from_rgb(254, 100, 11),
    yellow: Color32::from_rgb(223, 142, 29),
    green: Color32::from_rgb(64, 160, 43),
    teal: Color32::from_rgb(23, 146, 153),
    sky: Color32::from_rgb(4, 165, 229),
    sapphire: Color32::from_rgb(32, 159, 181),
    blue: Color32::from_rgb(30, 102, 245),
    lavender: Color32::from_rgb(114, 135, 253),
    text: Color32::from_rgb(76, 79, 105),
    subtext1: Color32::from_rgb(92, 95, 119),
    subtext0: Color32::from_rgb(108, 111, 133),
    overlay2: Color32::from_rgb(124, 127, 147),
    overlay1: Color32::from_rgb(140, 143, 161),
    overlay0: Color32::from_rgb(156, 160, 176),
    surface2: Color32::from_rgb(172, 176, 190),
    surface1: Color32::from_rgb(188, 192, 204),
    surface0: Color32::from_rgb(204, 208, 218),
    base: Color32::from_rgb(239, 241, 245),
    mantle: Color32::from_rgb(230, 233, 239),
    crust: Color32::from_rgb(220, 224, 232),
};

const FRAPPE: Theme = Theme {
    rosewater: Color32::from_rgb(242, 213, 207),
    flamingo: Color32::from_rgb(238, 190, 190),
    pink: Color32::from_rgb(244, 184, 228),
    mauve: Color32::from_rgb(202, 158, 230),
    red: Color32::from_rgb(231, 130, 132),
    maroon: Color32::from_rgb(234, 153, 156),
    peach: Color32::from_rgb(239, 159, 118),
    yellow: Color32::from_rgb(229, 200, 144),
    green: Color32::from_rgb(166, 209, 137),
    teal: Color32::from_rgb(129, 200, 190),
    sky: Color32::from_rgb(153, 209, 219),
    sapphire: Color32::from_rgb(133, 193, 220),
    blue: Color32::from_rgb(140, 170, 238),
    lavender: Color32::from_rgb(186, 187, 241),
    text: Color32::from_rgb(198, 208, 245),
    subtext1: Color32::from_rgb(181, 191, 226),
    subtext0: Color32::from_rgb(165, 173, 206),
    overlay2: Color32::from_rgb(148, 156, 187),
    overlay1: Color32::from_rgb(131, 139, 167),
    overlay0: Color32::from_rgb(115, 121, 148),
    surface2: Color32::from_rgb(98, 104, 128),
    surface1: Color32::from_rgb(81, 87, 109),
    surface0: Color32::from_rgb(65, 69, 89),
    base: Color32::from_rgb(48, 52, 70),
    mantle: Color32::from_rgb(41, 44, 60),
    crust: Color32::from_rgb(35, 38, 52),
};

const MACCHIATO: Theme = Theme {
    rosewater: Color32::from_rgb(244, 219, 214),
    flamingo: Color32::from_rgb(240, 198, 198),
    pink: Color32::from_rgb(245, 189, 230),
    mauve: Color32::from_rgb(198, 160, 246),
    red: Color32::from_rgb(237, 135, 150),
    maroon: Color32::from_rgb(238, 153, 160),
    peach: Color32::from_rgb(245, 169, 127),
    yellow: Color32::from_rgb(238, 212, 159),
    green: Color32::from_rgb(166, 218, 149),
    teal: Color32::from_rgb(139, 213, 202),
    sky: Color32::from_rgb(145, 215, 227),
    sapphire: Color32::from_rgb(125, 196, 228),
    blue: Color32::from_rgb(138, 173, 244),
    lavender: Color32::from_rgb(183, 189, 248),
    text: Color32::from_rgb(202, 211, 245),
    subtext1: Color32::from_rgb(184, 192, 224),
    subtext0: Color32::from_rgb(165, 173, 203),
    overlay2: Color32::from_rgb(147, 154, 183),
    overlay1: Color32::from_rgb(128, 135, 162),
    overlay0: Color32::from_rgb(110, 115, 141),
    surface2: Color32::from_rgb(91, 96, 120),
    surface1: Color32::from_rgb(73, 77, 100),
    surface0: Color32::from_rgb(54, 58, 79),
    base: Color32::from_rgb(36, 39, 58),
    mantle: Color32::from_rgb(30, 32, 48),
    crust: Color32::from_rgb(24, 25, 38),
};

const MOCHA: Theme = Theme {
    rosewater: Color32::from_rgb(245, 224, 220),
    flamingo: Color32::from_rgb(242, 205, 205),
    pink: Color32::from_rgb(245, 194, 231),
    mauve: Color32::from_rgb(203, 166, 247),
    red: Color32::from_rgb(243, 139, 168),
    maroon: Color32::from_rgb(235, 160, 172),
    peach: Color32::from_rgb(250, 179, 135),
    yellow: Color32::from_rgb(249, 226, 175),
    green: Color32::from_rgb(166, 227, 161),
    teal: Color32::from_rgb(148, 226, 213),
    sky: Color32::from_rgb(137, 220, 235),
    sapphire: Color32::from_rgb(116, 199, 236),
    blue: Color32::from_rgb(137, 180, 250),
    lavender: Color32::from_rgb(180, 190, 254),
    text: Color32::from_rgb(205, 214, 244),
    subtext1: Color32::from_rgb(186, 194, 222),
    subtext0: Color32::from_rgb(166, 173, 200),
    overlay2: Color32::from_rgb(147, 153, 178),
    overlay1: Color32::from_rgb(127, 132, 156),
    overlay0: Color32::from_rgb(108, 112, 134),
    surface2: Color32::from_rgb(88, 91, 112),
    surface1: Color32::from_rgb(69, 71, 90),
    surface0: Color32::from_rgb(49, 50, 68),
    base: Color32::from_rgb(30, 30, 46),
    mantle: Color32::from_rgb(24, 24, 37),
    crust: Color32::from_rgb(17, 17, 27),
};
