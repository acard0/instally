use iced::{
    alignment::{Horizontal, Vertical}, application, event, futures::{self}, mouse, widget::{
        self, button, column, container, horizontal_rule, progress_bar, row, stack, text, vertical_space
    }, window::{self}, Alignment, Background, Border, Color, Element, Event, Length, Point, Size, Subscription, Task, Theme
};
use instally_core::{definitions::{app::InstallyApp, context::AppContextNotifiable}, once_cell::sync::Lazy};
use rust_i18n::t;
use instally_core::_rust_i18n_translate;
use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Win(Event),
    StateChanged,
    Abort,
    Quit
}

static mut CURSOR: Option<Point> = None;

pub static CH: Lazy<(watch::Sender<()>, watch::Receiver<()>)> =
    Lazy::new(|| watch::channel(()));

pub fn create(app: InstallyApp) -> iced::Result {
    app.get_context()
        .lock()
        .subscribe(Box::new(|_update| {
            let _ = CH.0.send(());
        }));

    application(|ctx: &InstallyApp| ctx.get_product().title.clone(), update, view)
        .subscription(subscription)
        .window(window::Settings {
            decorations: false,
            transparent: true,
            position: window::Position::Centered,
            size: Size::new(600.0, 200.0),
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

fn update(_: &mut InstallyApp, msg: Msg) -> Task<Msg> {
    match msg {
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

fn view(app: &InstallyApp) -> Element<Msg> {
    let product = app.get_product();
    let binding = app.get_context();
    let ctx = binding.lock();
    let prog    = ctx.get_progress();
    let state   = ctx.get_state_information();

    let header = text(if product.title.is_empty() { "Untitled" } else { &product.title })
        .size(28)
        .color(Color::WHITE);

    let divider = container(
        horizontal_rule(1).style(|_| widget::rule::Style {
            color: Color::from_rgb(0.35, 0.35, 0.45),
            width: 1,
            radius: 0.0.into(),
            fill_mode: widget::rule::FillMode::Full,
        }),
    )
    .width(Length::Fill);

    let status = text(state)
        .size(16)
        .color(Color::WHITE);

    let bar = progress_bar(0.0..=100.0, prog)
        .height(20)
        .style(|_| widget::progress_bar::Style {
            background: Background::Color(Color::from_rgb(0.20, 0.20, 0.24)),
            bar:        Background::Color(Color::from_rgb(0.30, 0.80, 0.30)),
            border:     Border::default().rounded(12),
        });

    let percent = text(format!("{:.1}%", prog))
        .size(16)
        .color(Color::WHITE)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .width(Length::Fill);
    
    let bar_container = container(
            stack![
                bar.width(Length::Fill),
                percent,
            ]
            .width(Length::Fill),
        );

        let product_url = text(product.product_url.to_owned())
            .size(14)
            .color(Color::WHITE)
            .align_x(Horizontal::Left);

        let abort = {
            let base = button(text(if ctx.is_completed() { t!("ok") } else { t!("abort") })).style(rounded_primary);
            match ctx.is_completed() {
                true => base.on_press(Msg::Quit),
                false => base.on_press(Msg::Abort),
            }
        };

    let bottom = row![
        product_url,
        vertical_space().width(Length::Fill),
        abort,
    ]
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let content = column![header, divider, status, bar_container, bottom]
        .padding(20)
        .spacing(12)
        .align_x(Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| widget::container::Style {
            background: Some(Background::Color(Color::from_rgb(0.14, 0.15, 0.23))),
            border: Border::default().width(1).color(Color::from_rgb8(55, 55, 55)),
            ..Default::default()
        })
        .into()
}

pub fn rounded_primary(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: Border::default().rounded(12),
        ..button::primary(theme, status)
    }
}