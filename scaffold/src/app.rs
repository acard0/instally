use iced::{
    alignment::{Horizontal, Vertical},
    application, event,
    futures::{self},
    mouse,
    widget::{
        self, button, column, container, horizontal_rule, horizontal_space, progress_bar, row,
        scrollable, stack, text, vertical_space,
    },
    window::{self},
    Alignment, Background, Border, Color, Element, Event, Length, Point, Renderer, Size,
    Subscription, Task, Theme,
};
use instally_core::{
    _rust_i18n_translate,
    definitions::{
        app::InstallyApp, 
        context::AppContextNotifiable,
    },
    once_cell::sync::Lazy,
};
use rust_i18n::t;
use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Win(Event),
    StateChanged,
    Abort,
    Quit,
}

static mut CURSOR: Option<Point> = None;

pub static CH: Lazy<(watch::Sender<()>, watch::Receiver<()>)> = Lazy::new(|| watch::channel(()));

pub fn create(app: InstallyApp) -> iced::Result {
    app.get_context().lock().subscribe(Box::new(|_update| {
        let _ = CH.0.send(());
    }));

    let initial_size = desired_window_size(&app);

    application(|ctx: &InstallyApp| ctx.get_product().title.clone(), update, view)
        .subscription(subscription)
        .window(window::Settings {
            decorations: false,
            transparent: true,
            position: window::Position::Centered,
            size: initial_size,
            ..window::Settings::default()
        })
        .run_with(|| (app, Task::none()))
}

fn subscription(_: &InstallyApp) -> Subscription<Msg> {
    let rx = CH.1.clone();
    let backend_stream = Subscription::run_with_id(
        "ctx",
        futures::stream::unfold(rx, |mut rx| async {
            rx.changed().await.ok()?;
            Some((Msg::StateChanged, rx))
        }),
    );

    let ui_events = event::listen().map(Msg::Win);
    Subscription::batch(vec![backend_stream, ui_events])
}

fn update(app: &mut InstallyApp, msg: Msg) -> Task<Msg> {
    match msg {
        Msg::StateChanged => {
            let size = desired_window_size(app);
            return window::get_oldest().and_then(move |id| window::resize(id, size));
        },
        Msg::Win(Event::Mouse(ev)) => match ev {
            mouse::Event::CursorMoved { position } => unsafe {
                CURSOR = Some(position);
            },
            mouse::Event::ButtonPressed(mouse::Button::Left) => unsafe {
                if let Some(p) = CURSOR {
                    if p.y < 60.0 {
                        return window::get_oldest().and_then(window::drag);
                    }
                }
            },
            _ => {}
        },
        Msg::Abort | Msg::Quit => std::process::exit(0),
        _ => {}
    }

    Task::none()
}

fn view(app: &InstallyApp) -> Element<'_, Msg> {
    let product = app.get_product();
    let binding = app.get_context();
    let ctx = binding.lock();
    let prog = ctx.get_progress();
    let state = ctx.get_state_information();
    let has_error = ctx.get_result().is_some_and(|r| r.is_ok() == false);
    let is_complete = ctx.is_complete();

    let header = text(if product.title.is_empty() { "Setup" } else { &product.title })
        .size(28)
        .color(Color::WHITE)
        .width(Length::Fill)
        .align_x(Horizontal::Center);

    let divider = container(horizontal_rule(1).style(|_| widget::rule::Style {
        color: Color::from_rgb8(65, 65, 82),
        width: 1,
        radius: 0.0.into(),
        fill_mode: widget::rule::FillMode::Full,
    }))
    .width(Length::Fill);

    let window_width = desired_window_width(product.product_url.as_str());
    let status = status_section(&state, has_error, window_width);

    let bottom = row![
        text(product.product_url.to_owned())
            .size(14)
            .color(Color::from_rgb8(215, 215, 225))
            .align_x(Horizontal::Left),
        horizontal_space().width(Length::Fill),
        action_button(is_complete),
    ]
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .spacing(12);

    let content = if let Some(err_result) = ctx.get_result().filter(|r| r.is_ok() == false) {
        let err = err_result.get_error().unwrap();

        let error_message = text::<Theme, Renderer>(err.message)
            .size(12)
            .color(Color::from_rgb8(255, 190, 115))
            .width(Length::Fill)
            .align_x(Horizontal::Left);

        let mut error_group = column![error_message]
            .width(Length::Fill)
            .spacing(8)
            .align_x(Alignment::Start);

        if let Some(suggestion) = err.suggestion {
            let suggestion = container(
                text::<Theme, Renderer>(suggestion)
                    .size(12)
                    .color(Color::from_rgb8(165, 225, 245))
                    .width(Length::Fill)
                    .align_x(Horizontal::Left),
            )
            .padding(9)
            .width(Length::Fill)
            .style(|_| widget::container::Style {
                background: Some(Background::Color(Color::from_rgb8(31, 47, 62))),
                border: Border::default().width(1).rounded(8).color(Color::from_rgb8(60, 95, 120)),
                ..Default::default()
            });

            error_group = error_group.push(suggestion);
        }

        let error_panel = container(error_group)
            .padding(10)
            .width(Length::Fill)
            .style(|_| widget::container::Style {
                background: Some(Background::Color(Color::from_rgb8(44, 32, 31))),
                border: Border::default().width(1).rounded(12).color(Color::from_rgb8(120, 70, 48)),
                ..Default::default()
            });

        column![
            header,
            divider,
            status,
            scrollable(error_panel).width(Length::Fill).height(Length::Fill),
            bottom
        ]
    } else {
        column![
            header,
            divider,
            status,
            progress_section(prog),
            vertical_space().height(Length::Fill),
            bottom
        ]
    }
    .padding(20)
    .spacing(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Start);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(36, 38, 59))),
            border: Border::default().width(1).color(Color::from_rgb8(55, 55, 55)),
            ..Default::default()
        })
        .into()
}

fn status_section(state: &str, has_error: bool, window_width: f32) -> Element<'static, Msg> {
    let color = if has_error { Color::from_rgb8(255, 178, 88) } else { Color::WHITE };
    let max_chars = status_chars_per_line(window_width);
    let lines = wrap_text_lines(state, max_chars);

    let mut content = widget::Column::new()
        .width(Length::Fill)
        .spacing(2)
        .align_x(Alignment::Center);

    for line in lines {
        content = content.push(
            text::<Theme, Renderer>(line)
                .size(16)
                .color(color)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        );
    }

    container(content)
        .width(Length::Fill)
        .padding([0, 2])
        .into()
}

fn progress_section(progress: f32) -> Element<'static, Msg> {
    let bar = progress_bar(0.0..=100.0, progress)
        .height(20)
        .style(|_| widget::progress_bar::Style {
            background: Background::Color(Color::from_rgb8(51, 51, 61)),
            bar: Background::Color(Color::from_rgb8(76, 204, 76)),
            border: Border::default().rounded(12),
        });

    let percent = text(format!("{:.1}%", progress))
        .size(16)
        .color(Color::WHITE)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .width(Length::Fill);

    container(stack![bar.width(Length::Fill), percent].width(Length::Fill))
        .width(Length::Fill)
        .into()
}

fn action_button(is_complete: bool) -> button::Button<'static, Msg> {
    let button = button(text(if is_complete { t!("ok") } else { t!("abort") })).style(rounded_primary);

    if is_complete {
        button.on_press(Msg::Quit)
    } else {
        button.on_press(Msg::Abort)
    }
}

pub fn rounded_primary(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: Border::default().rounded(12),
        ..button::primary(theme, status)
    }
}

fn desired_window_size(app: &InstallyApp) -> Size {
    let product = app.get_product();
    let binding = app.get_context();
    let ctx = binding.lock();
    let state = ctx.get_state_information();

    let width = desired_window_width(product.product_url.as_str());
    let mut height = 210.0_f32;

    height += estimated_text_extra_height(&state, status_chars_per_line(width), 18.0);

    if let Some(err_result) = ctx.get_result().filter(|r| r.is_ok() == false) {
        if let Some(err) = err_result.get_error() {
            height += estimated_text_extra_height(&err.message, chars_per_line(width, 64, 84), 17.0);

            if let Some(suggestion) = &err.suggestion {
                height += 36.0;
                height += estimated_text_extra_height(suggestion, chars_per_line(width, 60, 80), 17.0);
            }
        }
    }

    Size::new(width, height.clamp(200.0, 420.0))
}

fn desired_window_width(product_url: &str) -> f32 {
    if product_url.len() > 48 {
        720.0
    } else {
        600.0
    }
}

fn status_chars_per_line(window_width: f32) -> usize {
    if window_width > 600.0 {
        56
    } else {
        42
    }
}

fn wrap_text_lines(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(8);
    let mut lines = Vec::new();

    for source_line in text.lines() {
        let mut current = String::new();

        for word in source_line.split_whitespace() {
            let word_chars = word.chars().count();
            let current_chars = current.chars().count();
            let separator = if current.is_empty() { 0 } else { 1 };

            if word_chars > max_chars {
                if !current.is_empty() {
                    lines.push(current);
                    current = String::new();
                }

                lines.extend(split_long_word(word, max_chars));
                continue;
            }

            if current_chars + separator + word_chars > max_chars {
                if !current.is_empty() {
                    lines.push(current);
                }

                current = word.to_owned();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }

                current.push_str(word);
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn split_long_word(word: &str, max_chars: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in word.chars() {
        if current.chars().count() >= max_chars {
            parts.push(current);
            current = String::new();
        }

        current.push(ch);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

fn chars_per_line(width: f32, compact: usize, wide: usize) -> usize {
    if width > 600.0 {
        wide
    } else {
        compact
    }
}

fn estimated_text_extra_height(text: &str, chars_per_line: usize, line_height: f32) -> f32 {
    let chars = text.chars().count();
    let lines = chars.div_ceil(chars_per_line).max(1);

    (lines.saturating_sub(1) as f32) * line_height
}
