use std::time::Duration;
use std::time::Instant;

use iced::{
    alignment,
    widget::{horizontal_rule, horizontal_space, vertical_space, Column, Container, Row, Text},
    Alignment, Command, Element, Length,
};
use iced_aw::NumberInput;

use cryo_cooler_controller_lib::TecStatus;

use crate::settings;
use crate::{charts::ChartGroup, Message};

pub struct RunningState {
    last_sample_time: Instant,
    tec: cryo_cooler_controller_lib::Tec,
    tec_status: TecStatus,
    firmware_version_major: u8,
    firmware_version_minor: u8,
    hardware_version: u32,
    chart: ChartGroup,
    error_text: Option<String>,
    update_intervall: Duration,
    app_settings: settings::AppSettings,
}

impl RunningState {
    pub fn new<T>(
        serial_port: &T,
        app_settings: settings::AppSettings,
    ) -> Result<Self, std::io::Error>
    where
        T: AsRef<std::path::Path> + std::fmt::Debug,
    {
        let mut tec = cryo_cooler_controller_lib::Tec::new(&serial_port.as_ref().as_os_str())?;
        let fw_version = tec.fw_version()?;
        let firmware_version_major = fw_version.0;
        let firmware_version_minor = fw_version.1;
        let hardware_version = tec.hw_version()?;
        let tec_status = tec.heart_beat()?;
        let mut error_text = None;
        if app_settings.get_enable_on_startup(){
            if let Err(err) = tec.enable(
                app_settings.get_p_coef(),
                app_settings.get_i_coef(),
                app_settings.get_d_coef(),
                app_settings.get_max_power(),
                app_settings.get_set_point(),
            ) {
                error_text = Some(format!("Failed to enable TEC ({err})"));
            }
        }
        Ok(RunningState {
            last_sample_time: Instant::now(),
            tec,
            tec_status,
            firmware_version_major,
            firmware_version_minor,
            hardware_version,
            chart: Default::default(),
            error_text,
            update_intervall: Duration::from_millis(500),
            app_settings,
        })
    }
    #[inline]
    pub fn should_update(&self) -> bool {
        self.last_sample_time.elapsed() > self.update_intervall
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                if !self.should_update() {
                    return Command::none();
                }

                match self.tec.heart_beat() {
                    Ok(status) => self.tec_status = status,
                    Err(err) => {
                        if let Err(e) = self.tec.reset_connection() {
                            self.error_text =
                                Some(format!("Failed to communicate with coooler ({e:?})"));
                            return Command::none();
                        } else {
                            self.error_text =
                                Some(format!("Failed to communicate with coooler ({err:?})"))
                        }
                    }
                }
                self.last_sample_time = Instant::now();
                match self.tec.monitor() {
                    Ok(data) => self.chart.update(data),
                    Err(err) => {
                        self.error_text = Some(format!("Failed to get data from coooler ({err})"));
                    }
                }
            }
            Message::Enable => {
                if let Err(err) = self.tec.enable(
                    self.app_settings.get_p_coef(),
                    self.app_settings.get_i_coef(),
                    self.app_settings.get_d_coef(),
                    self.app_settings.get_max_power(),
                    self.app_settings.get_set_point(),
                ) {
                    self.error_text = Some(format!("Failed to enable TEC ({err})"));
                }
            }
            Message::Disable => {
                if let Err(err) = self.tec.disable() {
                    self.error_text = Some(format!("Failed to disable TEC ({err})"));
                }
            }
            Message::UpdatePCoef(input) => {
                if let Err(e) = self.app_settings.set_p_coef(input) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            Message::UpdateICoef(input) => {
                if let Err(e) = self.app_settings.set_i_coef(input) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            Message::UpdateDCoef(input) => {
                if let Err(e) = self.app_settings.set_d_coef(input) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            Message::UpdateSetpoint(input) => {
                if let Err(e) = self.app_settings.set_set_point(input) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            Message::UpdateMaxPower(input) => {
                if let Err(e) = self.app_settings.set_max_power(input) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            Message::CloseModal => {
                self.error_text = None;
            }
            Message::ApplyStartupCheckboxToggled(checked) => {
                if let Err(e) = self.app_settings.set_enable_on_startup(checked) {
                    self.error_text = Some(format!("Failed to save settings ({e})"));
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = Row::new().spacing(20);

        let content = content
            .push(self.view_left_column())
            .push(self.view_right_column());

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(2)
            .center_x()
            .center_y()
            .into()
    }

    pub fn view_left_column(&self) -> Element<'_, Message> {
        let button = |label| {
            iced::widget::button(
                iced::widget::text(label).horizontal_alignment(alignment::Horizontal::Center),
            )
            .padding(10)
            .width(Length::Fixed(110.0))
        };

        let en_button = if !self.tec_status.contains(TecStatus::LOW_POWER_MODE_ACTIVE) {
            button("Disable TEC")
                .style(iced::theme::Button::Secondary)
                .on_press(Message::Disable)
        } else {
            button("Enable TEC")
                .style(iced::theme::Button::Primary)
                .on_press(Message::Enable)
        };

        let hide_button = button("Hide Window")
            .style(iced::theme::Button::Primary)
            .on_press(Message::Hide)
            .width(Length::Fixed(150.0));

        let content = Column::new()
            .spacing(5)
            .width(Length::Fixed(280.0))
            .push(
                Row::new()
                    .push(
                        Column::new()
                            .push(
                                Row::new().push(
                                    Text::new(format!(
                                        "Firmware Version: {:X}.{:X}",
                                        self.firmware_version_major, self.firmware_version_minor
                                    ))
                                    .size(28),
                                ),
                            )
                            .push(
                                Row::new().push(
                                    Text::new(format!(
                                        "Hardware Version: {}",
                                        self.hardware_version
                                    ))
                                    .size(28),
                                ),
                            ),
                    )
                    .padding(15),
            )
            .push(horizontal_rule(20))
            .push(
                Row::new()
                    .push(Text::new("Offset"))
                    .push(horizontal_space(Length::Fill))
                    .push(
                        NumberInput::new(
                            self.app_settings.get_set_point(),
                            50.0,
                            Message::UpdateSetpoint,
                        )
                        .style(iced_aw::style::NumberInputStyles::Default)
                        .step(1.0)
                        .min(-50.0),
                    )
                    .padding(5)
                    .spacing(5),
            )
            .push(
                Row::new()
                    .push(Text::new("Max. Power"))
                    .push(horizontal_space(Length::Fill))
                    .push(
                        NumberInput::new(
                            self.app_settings.get_max_power(),
                            100,
                            Message::UpdateMaxPower,
                        )
                        .style(iced_aw::style::NumberInputStyles::Default)
                        .step(1)
                        .min(0),
                    )
                    .padding(5)
                    .spacing(5),
            )
            .push(
                Column::new()
                    .push(en_button)
                    .padding(15)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(horizontal_rule(20))
            .push(
                Row::new()
                    .push(Text::new("P Coefficient"))
                    .push(horizontal_space(Length::Fill))
                    .push(
                        NumberInput::new(
                            self.app_settings.get_p_coef(),
                            1000.0,
                            Message::UpdatePCoef,
                        )
                        .style(iced_aw::style::NumberInputStyles::Default)
                        .step(0.1)
                        .min(-1000.0),
                    )
                    .padding(5)
                    .spacing(5),
            )
            .push(
                Row::new()
                    .push(Text::new("I Coefficient"))
                    .push(horizontal_space(Length::Fill))
                    .push(
                        NumberInput::new(
                            self.app_settings.get_i_coef(),
                            1000.0,
                            Message::UpdateICoef,
                        )
                        .style(iced_aw::style::NumberInputStyles::Default)
                        .step(0.1)
                        .min(-1000.0),
                    )
                    .padding(5)
                    .spacing(5),
            )
            .push(
                Row::new()
                    .push(Text::new("D Coefficient"))
                    .push(horizontal_space(Length::Fill))
                    .push(
                        NumberInput::new(
                            self.app_settings.get_d_coef(),
                            1000.0,
                            Message::UpdateDCoef,
                        )
                        .style(iced_aw::style::NumberInputStyles::Default)
                        .step(0.1)
                        .min(-1000.0),
                    )
                    .padding(5)
                    .spacing(5),
            )
            .push(horizontal_rule(20))
            .push(view_badges(&self.tec_status))
            .push(vertical_space(Length::Fill))
            .push(horizontal_rule(20))
            .push(
                Column::new()
                    .push(iced::widget::checkbox(
                        "Enable TEC on Startup",
                        self.app_settings.get_enable_on_startup(),
                        Message::ApplyStartupCheckboxToggled,
                    ))
                    .push(hide_button)
                    .padding(15)
                    .spacing(15)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            );

        iced_aw::Modal::new(
            self.error_text.is_some(),
            content,
            iced_aw::Card::new(
                Text::new("Error"),
                Text::new(self.error_text.clone().unwrap_or_else(|| "".to_owned())),
            )
            .foot(
                Column::new().padding(5).width(Length::Fill).push(
                    iced::widget::Button::new(
                        Text::new("Ok").horizontal_alignment(alignment::Horizontal::Center),
                    )
                    .width(Length::Fixed(100.0))
                    .on_press(Message::CloseModal),
                ),
            )
            .max_width(300.0)
            .on_close(Message::CloseModal),
        )
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal)
        .into()
    }

    pub fn view_right_column(&self) -> Element<'_, Message> {
        Column::new()
            .spacing(5)
            .align_items(Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
            .push(iced::widget::vertical_space(Length::Fixed(5.0)))
            .push(self.chart.view())
            .into()
    }
}

fn add_badge_if_flag_missing<'a, T>(
    mut column: Column<'a, Message, iced::Renderer<T>>,
    status: &'a TecStatus,
    flag: TecStatus,
    text: &'static str,
) -> Column<'a, Message, iced::Renderer<T>>
where
    T: iced::widget::text::StyleSheet
        + iced_aw::badge::StyleSheet<Style = iced_aw::style::BadgeStyles>
        + 'a,
{
    if !status.contains(flag) {
        column = column.push(
            iced_aw::Badge::new(Text::new(text).size(20).width(Length::Fill))
                .style(iced_aw::style::BadgeStyles::Danger),
        )
    }
    column
}

fn add_badge_if_flag_set<'a, T>(
    mut column: Column<'a, Message, iced::Renderer<T>>,
    status: &'a TecStatus,
    flag: TecStatus,
    text: &'static str,
) -> Column<'a, Message, iced::Renderer<T>>
where
    T: iced::widget::text::StyleSheet
        + iced_aw::badge::StyleSheet<Style = iced_aw::style::BadgeStyles>
        + 'a,
{
    if status.contains(flag) {
        column = column.push(
            iced_aw::Badge::new(Text::new(text).size(20).width(Length::Fill))
                .style(iced_aw::style::BadgeStyles::Danger),
        )
    }
    column
}

pub fn view_badges(status: &TecStatus) -> Element<'_, Message> {
    let mut col = Column::new()
        .spacing(12)
        .align_items(Alignment::Center)
        .width(Length::Fill);

    col = add_badge_if_flag_missing(col, status, TecStatus::POWER_OK, "TEC NO POWER");
    col = add_badge_if_flag_missing(col, status, TecStatus::TEMP_SENSE_OK, "TEMP SENSOR ERROR");
    col = add_badge_if_flag_missing(col, status, TecStatus::HUM_SENSE_OK, "HUM SENSOR ERROR");

    col = add_badge_if_flag_set(col, status, TecStatus::PID_OUT_OF_RANGE, "PID OUT OF RANGE");
    col = add_badge_if_flag_set(col, status, TecStatus::PID_INVALID, "PID INVALID");
    col = add_badge_if_flag_set(col, status, TecStatus::OCP_ACTIVE, "OCP ACTIVE");

    if status.contains(TecStatus::LOW_POWER_MODE_ACTIVE) {
        col = col.push(
            iced_aw::Badge::new(Text::new("TEC DISABLED").size(20).width(Length::Fill))
                .style(iced_aw::style::BadgeStyles::Primary),
        )
    } else {
        col = col.push(
            iced_aw::Badge::new(Text::new("TEC ENABLED").size(20).width(Length::Fill))
                .style(iced_aw::style::BadgeStyles::Primary),
        )
    }

    col.into()
}
