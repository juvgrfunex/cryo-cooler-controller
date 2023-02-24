#![windows_subsystem = "windows"]
#![forbid(unsafe_code)]
#![warn(
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::todo,
    clippy::unimplemented,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug
)]


extern crate iced;
extern crate plotters;

mod charts;
mod running;

use iced::{
    alignment, executor,
    widget::{Column, Container, Row, Text},
    Application, Color, Command, Element, Font, Length, Settings, Subscription, Theme,
};

use running::RunningState;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};

const FONT_REGULAR: Font = Font::External {
    name: "sans-serif-regular",
    bytes: include_bytes!("../fonts/notosans-regular.ttf"),
};

const FONT_BOLD: Font = Font::External {
    name: "sans-serif-bold",
    bytes: include_bytes!("../fonts/notosans-bold.ttf"),
};

const ICON: &[u8; 0x4000] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.bin"));

fn main() {
    let icon =
        tray_icon::icon::Icon::from_rgba(ICON.to_vec(), 64, 64).expect("Failed to open icon");

    let tray_menu = Menu::new();

    let quit_i = MenuItem::new("Quit", true, None);
    tray_menu.append_items(&[
        &MenuItem::new("Show", true, None),
        &PredefinedMenuItem::separator(),
        &quit_i,
    ]);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Cryo Cooler Controller")
        .with_icon(icon)
        .build();

    if let Err(error) = tray_icon {
        match error {
            tray_icon::Error::OsError(err) => std::process::exit(err.raw_os_error().unwrap_or(-1)),
            _ => std::process::exit(-1),
        }
    }

    let _ = CryoCoolerController::run(Settings {
        antialiasing: true,
        default_font: Some(include_bytes!("../fonts/notosans-regular.ttf")),
        window: iced::window::Settings {
            size: (550, 350),
            resizable: true,
            decorations: true,
            icon: Some(
                iced::window::icon::Icon::from_rgba(ICON.to_vec(), 64, 64)
                    .expect("icon.bin contains valid rgba"),
            ),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    });
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    CloseModal,
    Enable,
    Disable,
    UpdatePCoef(f32),
    UpdateICoef(f32),
    UpdateDCoef(f32),
    UpdateSetpoint(f32),
    UpdateMaxPower(u8),

    PortSelected(PortIdent),
    Open,
    ChangeState,
    Hide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortIdent {
    path: std::path::PathBuf,
}

impl std::fmt::Display for PortIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[derive(Default)]
struct HomeState {
    selected_port: Option<PortIdent>,
    error_text: Option<String>,
}

impl HomeState {
    pub fn new() -> Self {
        let mut ports: Vec<PortIdent> = serial2::SerialPort::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|path| PortIdent { path })
            .collect();

        Self {
            selected_port: ports.pop(),
            error_text: None,
        }
    }
    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PortSelected(port) => {
                self.selected_port = Some(port);
            }
            Message::CloseModal => {
                self.error_text = None;
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let label = Text::new("Select Serial Port").size(48);

        let options: Vec<PortIdent> = serial2::SerialPort::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|path| PortIdent { path })
            .collect();
        let pick_list =
            iced::widget::pick_list(options, self.selected_port.clone(), Message::PortSelected)
                .width(Length::Fixed(250.0));

        let button = |label| {
            iced::widget::button(
                iced::widget::text(label)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .vertical_alignment(alignment::Vertical::Center),
            )
            .padding(10)
        };
        let open_btn = button("Connect")
            .style(iced::theme::Button::Primary)
            .on_press(Message::Open);

        let content = Column::new()
            .align_items(iced::Alignment::Center)
            .push(Row::new().spacing(20).push(label))
            .push(iced::widget::vertical_space(Length::Fixed(50.0)))
            .push(
                Row::new()
                    .spacing(20)
                    .push(Column::new().push(pick_list))
                    .push(Column::new().push(open_btn)),
            )
            .push(Row::new().push(Text::new(format!("Version {}", env!("CARGO_PKG_VERSION")))));

        let content = iced_aw::Modal::new(self.error_text.is_some(), content, || {
            iced_aw::Card::new(
                Text::new("Failed to connect to cooler"),
                Text::new(self.error_text.clone().unwrap_or_else(|| "".to_owned())),
            )
            .foot(
                Column::new().padding(5).width(Length::Fill).push(
                    iced::widget::Button::new(
                        Text::new("Ok").horizontal_alignment(alignment::Horizontal::Center),
                    )
                    .width(Length::Fixed(100.0))
                    .on_press(Message::CloseModal),
                ).push(iced::widget::horizontal_rule(20)).push(Text::new("If you are unsure which port belongs to the cooler, replug it and see which port temporarily disappears")),
            )
            .max_width(300.0)
            .on_close(Message::CloseModal)
            .into()

        })
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal);

        Container::new(content)
            //.style(style::Container)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(2)
            .center_x()
            .center_y()
            .into()
    }
}

struct CryoCoolerController {
    state: State,
}

enum State {
    Home(HomeState),
    Running(RunningState),
}

impl Application for CryoCoolerController {
    type Message = self::Message;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn theme(&self) -> Self::Theme {
        Theme::custom(iced::theme::Palette {
            background: Color::from_rgb(
                0x20 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x25 as f32 / 255.0,
            ),
            text: Color::WHITE,
            primary: Color::from_rgb(
                0x5E as f32 / 255.0,
                0x7C as f32 / 255.0,
                0xE2 as f32 / 255.0,
            ),
            success: Color::from_rgb(
                0x12 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0x4F as f32 / 255.0,
            ),
            danger: Color::from_rgb(
                0xC3 as f32 / 255.0,
                0x42 as f32 / 255.0,
                0x3F as f32 / 255.0,
            ),
        })
    }

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            CryoCoolerController {
                state: State::Home(HomeState::new()),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Cryo Cooler Controller".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id {
                1000 => {
                    return Command::single(iced_native::command::Action::Window(
                        iced_native::window::Action::Close,
                    ));
                }
                1001 => {
                    return Command::single(iced_native::command::Action::Window(
                        iced_native::window::Action::ChangeMode(iced::window::Mode::Windowed),
                    ));
                }
                _ => {}
            }
        }

        match message {
            Message::Open => {
                if let State::Home(ref mut home) = &mut self.state {
                    if let Some(port) = &home.selected_port {
                        match RunningState::new(&port.path) {
                            Ok(running_state) => {
                                self.state = State::Running(running_state);
                            }
                            Err(error) => {
                                home.error_text =
                                    Some(format!("Error connecting to Port {port} ({error})"));
                                return iced_native::Command::none();
                            }
                        }

                        return Command::single(iced_native::command::Action::Window(
                            iced_native::window::Action::Resize {
                                width: 1400,
                                height: 1000,
                            },
                        ));
                    }
                }
            }
            Message::Hide => {
                return Command::single(iced_native::command::Action::Window(
                    iced_native::window::Action::ChangeMode(iced::window::Mode::Hidden),
                ));
            }
            _ => {}
        }
        match &mut self.state {
            State::Home(state) => state.update(message),
            State::Running(state) => state.update(message),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        match &self.state {
            State::Home(state) => state.view(),
            State::Running(state) => state.view(),
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        const FPS: u64 = 50;
        iced::time::every(Duration::from_millis(1000 / FPS)).map(|_| Message::Tick)
    }
}
