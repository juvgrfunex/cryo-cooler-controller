#![cfg_attr(not(test), windows_subsystem = "windows")]
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
//TODO bump versions
extern crate iced;
extern crate plotters;

mod charts;
mod running;
mod settings;

use iced::{
    alignment, executor,
    widget::{Column, Container, Row, Text},
    Application, Color, Command, Element, Length, Settings, Size, Subscription, Theme,
};

use running::RunningState;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};

const ICON: &[u8; 0x4000] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.bin"));

fn main() {
    let settings = settings::AppSettings::new();
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
        window: iced::window::Settings {
            size: (550, 350),
            resizable: true,
            decorations: true,
            icon: Some(
                iced::window::icon::from_rgba(ICON.to_vec(), 64, 64)
                    .expect("icon.bin contains valid rgba"),
            ),
            ..iced::window::Settings::default()
        },
        flags: settings,
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
    ApplyStartupCheckboxToggled(bool),

    PortSelected(PortIdent),
    Open,
    ChangeState,
    Hide,
    FontLoaded,
    FontLoadingFailed,
    OpenCheckboxToggled(bool),
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
    app_settings: settings::AppSettings,
}

impl HomeState {
    pub fn new(app_settings: settings::AppSettings) -> Self {
        let mut ports: Vec<PortIdent> = serial2::SerialPort::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|path| PortIdent { path })
            .collect();
        let port = match app_settings.get_last_port_ident() {
            Some(path) => Some(PortIdent { path: path.clone() }),
            None => ports.pop(),
        };

        Self {
            selected_port: port,
            error_text: None,
            app_settings,
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
            Message::OpenCheckboxToggled(checked) => {
                if let Err(e) = self.app_settings.set_open_port_on_startup(checked) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
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
            .push(iced::widget::vertical_space(Length::Fixed(15.0)))
            .push(
                Row::new()
                    .spacing(80)
                    .push(
                        Row::new()
                            .push(Text::new(format!("Version {}", env!("CARGO_PKG_VERSION")))),
                    )
                    .push(iced::widget::checkbox(
                        "Connect on Startup",
                        self.app_settings.get_open_port_on_startup(),
                        Message::OpenCheckboxToggled,
                    )),
            );

        let content = iced_aw::Modal::new(self.error_text.is_some(), content,
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

        )
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
    type Flags = settings::AppSettings;
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

    fn new(settings: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut commands = vec![
            iced::font::load(iced_aw::graphics::icons::ICON_FONT_BYTES).map(|ret| match ret {
                Ok(_) => Message::FontLoaded,
                Err(_) => Message::FontLoadingFailed,
            }),
        ];

        let state = {
            if let (Some(p), true) = (
                settings.get_last_port_ident(),
                settings.get_open_port_on_startup(),
            ) {
                match RunningState::new(p, settings.clone()) {
                    Ok(running_state) => {
                        commands.push(Command::single(iced_runtime::command::Action::Window(
                            iced_runtime::window::Action::Resize(Size::new(1400, 1000)),
                        )));
                        State::Running(running_state)
                    }
                    Err(error) => {
                        let mut home = HomeState::new(settings.clone());
                        home.error_text = Some(format!(
                            "Error connecting to Port {} ({error})",
                            PortIdent { path: p.clone() }
                        ));
                        State::Home(home)
                    }
                }
            } else {
                State::Home(HomeState::new(settings))
            }
        };
        (CryoCoolerController { state }, Command::batch(commands))
    }

    fn title(&self) -> String {
        "Cryo Cooler Controller".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id {
                1000 => {
                    return Command::single(iced_runtime::command::Action::Window(
                        iced_runtime::window::Action::Close,
                    ));
                }
                1001 => {
                    return Command::single(iced_runtime::command::Action::Window(
                        iced_runtime::window::Action::ChangeMode(iced::window::Mode::Windowed),
                    ));
                }
                _ => {}
            }
        }

        match message {
            Message::Open => {
                if let State::Home(ref mut home) = self.state {
                    if let Some(port) = &home.selected_port {
                        let _ = home
                            .app_settings
                            .set_last_port_ident(Some(port.path.clone()));
                        let cloned_settings = home.app_settings.clone();
                        match RunningState::new(&port.path, cloned_settings) {
                            Ok(running_state) => {
                                self.state = State::Running(running_state);
                            }
                            Err(error) => {
                                home.error_text =
                                    Some(format!("Error connecting to Port {port} ({error})"));
                                return iced_runtime::Command::none();
                            }
                        }

                        return Command::single(iced_runtime::command::Action::Window(
                            iced_runtime::window::Action::Resize(Size::new(1400, 1000)),
                        ));
                    }
                }
            }
            Message::Hide => {
                return Command::single(iced_runtime::command::Action::Window(
                    iced_runtime::window::Action::ChangeMode(iced::window::Mode::Hidden),
                ));
            }
            Message::FontLoadingFailed => {
                if let State::Home(ref mut home) = &mut self.state {
                    home.error_text = Some("Failed to load some icon, the software will still function but some UI elemnts may be missing".to_owned());
                    return iced_runtime::Command::none();
                }
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
        const FPS: u64 = 100;
        iced::time::every(Duration::from_millis(1000 / FPS)).map(|_| Message::Tick)
    }
}
