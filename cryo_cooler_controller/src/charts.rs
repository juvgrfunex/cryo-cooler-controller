use std::collections::VecDeque;
use std::time::Duration;

use chrono::{DateTime, Utc};
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        canvas::{Cache, Frame, Geometry},
        Column, Container, Row,
    },
    Alignment, Element, Length, Size,
};
use plotters::{
    prelude::ChartBuilder,
    series::AreaSeries,
    style::{Color, IntoFont, RGBAColor, RGBColor, ShapeStyle},
};
use plotters_backend::{DrawingBackend, FontTransform};
use plotters_iced::{Chart, ChartWidget, Renderer};

use crate::Message;

const PLOT_LINE_COLOR: RGBColor = RGBColor(0, 175, 255);
const GRID_BOLD_COLOR: RGBAColor = RGBAColor(100, 100, 100, 0.5);

pub struct ChartGroup {
    tec_temp_chart: MonitoringChartf32,
    pcb_temp_chart: MonitoringChartf32,
    humidty_chart: MonitoringChartf32,
    dew_point_chart: MonitoringChartf32,
    tec_voltage_chart: MonitoringChartf32,
    tec_current_chart: MonitoringChartf32,
    tec_power_chart: MonitoringChartf32,
    chart_height: f32,
}

impl Default for ChartGroup {
    fn default() -> Self {
        Self {
            tec_temp_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "TEC Temp".to_owned(),
                0.0,
                20.0,
                "C".to_owned(),
            ),
            pcb_temp_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "PCB Temp".to_owned(),
                20.0,
                30.0,
                "C".to_owned(),
            ),
            humidty_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "Humidity".to_owned(),
                45.0,
                55.0,
                "%".to_owned(),
            ),
            dew_point_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "Dew Point".to_owned(),
                10.0,
                20.0,
                "C".to_owned(),
            ),
            tec_voltage_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "TEC Voltage".to_owned(),
                11.0,
                13.0,
                "V".to_owned(),
            ),
            tec_current_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "TEC Current".to_owned(),
                0.0,
                10.0,
                "A".to_owned(),
            ),
            tec_power_chart: MonitoringChartf32::new(
                Vec::new().into_iter(),
                "TEC Power Level".to_owned(),
                0.0,
                100.0,
                "%".to_owned(),
            ),
            chart_height: 140.0,
        }
    }
}

impl ChartGroup {
    pub fn update(&mut self, data: cryo_cooler_controller_lib::MonitoringData) {
        self.tec_temp_chart
            .push_data(data.timestamp, data.tec_temperature);
        self.pcb_temp_chart
            .push_data(data.timestamp, data.pcb_temperature);
        self.humidty_chart.push_data(data.timestamp, data.humidity);
        self.dew_point_chart
            .push_data(data.timestamp, data.dew_point_temperature);
        self.tec_voltage_chart
            .push_data(data.timestamp, data.tec_voltage);
        self.tec_current_chart
            .push_data(data.timestamp, data.tec_current);
        self.tec_power_chart
            .push_data(data.timestamp, data.tec_power_level as f32);
    }

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(self.new_row().push(self.tec_temp_chart.view()))
            .push(self.new_row().push(self.tec_voltage_chart.view()))
            .push(self.new_row().push(self.tec_current_chart.view()))
            .push(self.new_row().push(self.tec_power_chart.view()))
            .push(self.new_row().push(self.humidty_chart.view()))
            .push(self.new_row().push(self.dew_point_chart.view()))
            .push(self.new_row().push(self.pcb_temp_chart.view()))
            .into()
    }

    pub fn new_row(&self) -> Row<Message> {
        Row::new()
            .spacing(0)
            .padding(0)
            .width(Length::Fill)
            .height(Length::Fixed(self.chart_height))
            .align_items(Alignment::Center)
    }
}

struct MonitoringChartf32 {
    title: String,
    min: f32,
    max: f32,
    unit: String,
    cache: Cache,
    data_points: VecDeque<(DateTime<Utc>, f32)>,
    limit: Duration,
}

impl MonitoringChartf32 {
    fn new(
        data: impl Iterator<Item = (DateTime<Utc>, f32)>,
        title: String,
        min: f32,
        max: f32,
        unit: String,
    ) -> Self {
        let data_points: VecDeque<_> = data.collect();
        Self {
            title,
            min,
            max,
            unit,
            cache: Cache::new(),
            data_points,
            limit: Duration::from_secs(300),
        }
    }

    fn push_data(&mut self, time: DateTime<Utc>, value: f32) {
        let cur_ms = time.timestamp_millis();
        if value > self.max {
            self.max = (value - self.min) * 0.05 + value;
        }
        if value < self.min {
            self.min = value - (self.min - value) * 0.05;
        }

        self.data_points.push_front((time, value));
        loop {
            if let Some((time, _)) = self.data_points.back() {
                let diff = Duration::from_millis((cur_ms - time.timestamp_millis()) as u64);
                if diff > self.limit {
                    self.data_points.pop_back();
                    continue;
                }
            }
            break;
        }
        self.cache.clear();
    }

    fn view(&self) -> Element<Message> {
        Container::new(
            Column::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(ChartWidget::new(self).height(Length::Fill)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
    }
}

#[derive(Default)]
struct ChartState {
    mouse_x_position: Option<f32>,
    bounds: iced::Rectangle,
}
impl Chart<Message> for MonitoringChartf32 {
    type State = ChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: iced::widget::canvas::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> (iced::event::Status, Option<Message>) {
        if let iced::widget::canvas::Event::Mouse(mouse_event) = event {
            if mouse_event == iced::mouse::Event::CursorLeft {
                state.mouse_x_position = None;
                return (iced::event::Status::Captured, None);
            }
        }
        if let iced::mouse::Cursor::Available(point) = cursor {
            if point.x >= bounds.x && point.x <= bounds.x + bounds.width {
                state.mouse_x_position = Some(point.x);
                state.bounds = bounds;
                self.cache.clear();
            } else {
                state.mouse_x_position = None;
            }
        }
        (iced::event::Status::Ignored, None)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if state.mouse_x_position.is_some() {
            iced::mouse::Interaction::Crosshair
        } else {
            iced::mouse::Interaction::Idle
        }
    }

    #[inline]
    fn draw<R: Renderer, F: Fn(&mut Frame)>(
        &self,
        renderer: &R,
        bounds: Size,
        draw_fn: F,
    ) -> Geometry {
        renderer.draw_cache(&self.cache, bounds, draw_fn)
    }

    fn build_chart<DB: DrawingBackend>(&self, state: &Self::State, mut chart: ChartBuilder<DB>) {
        //! This silently ignores error because there is nothing usefull that can be done about them.

        let newest_time = self
            .data_points
            .front()
            .unwrap_or(&(chrono::DateTime::<Utc>::MIN_UTC, 0.0))
            .0;
        let oldest_time = self
            .data_points
            .back()
            .unwrap_or(&(chrono::DateTime::<Utc>::MIN_UTC, 0.0))
            .0;

        let hover_index = calc_hover_index(
            state.mouse_x_position,
            self.data_points.len(),
            state.bounds.width,
            state.bounds.x,
        );
        let caption = if let Some(idx) = hover_index {
            format!(
                "{}  -  {:.2} {}",
                self.title, self.data_points[idx].1, self.unit
            )
        } else {
            self.title.clone()
        };

        let mut chart = match chart
            .caption(caption, ("sans-serif", 22, &plotters::style::colors::WHITE))
            .x_label_area_size(14)
            .y_label_area_size(28)
            .margin(10)
            .build_cartesian_2d(oldest_time..newest_time, self.min..self.max)
        {
            Ok(chart) => chart,
            Err(_) => return,
        };

        let _ = chart
            .configure_mesh()
            .bold_line_style(GRID_BOLD_COLOR)
            .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.90)).stroke_width(0))
            .y_labels(10)
            .x_labels(5)
            .y_label_style(
                ("sans-serif", 15)
                    .into_font()
                    .color(&plotters::style::colors::WHITE)
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&|y| format!("{} {}", y, self.unit))
            .x_label_style(
                ("sans-serif", 15)
                    .into_font()
                    .color(&plotters::style::colors::WHITE),
            )
            .x_label_formatter(&|x| format!("{} ", x.time()))
            .draw();

        let _ = chart.draw_series(
            AreaSeries::new(
                self.data_points.iter().map(|x| (x.0, x.1)),
                self.min,
                PLOT_LINE_COLOR.mix(0.175),
            )
            .border_style(ShapeStyle::from(PLOT_LINE_COLOR).stroke_width(2)),
        );

        if let Some(idx) = hover_index {
            let _ = chart.draw_series(std::iter::once(plotters::prelude::Circle::new(
                (self.data_points[idx].0, self.data_points[idx].1),
                5_i32,
                PLOT_LINE_COLOR.filled(),
            )));
        }
    }
}

fn calc_hover_index(
    x_pos_option: Option<f32>,
    data_size: usize,
    bounds_width: f32,
    bounds_x: f32,
) -> Option<usize> {
    if data_size == 0{
        return None;
    }
    if let Some(x_pos) = x_pos_option{
        let translation_factor = (data_size as f32) / bounds_width;
        let mut hover_index =
            data_size.saturating_sub(((x_pos - bounds_x) * translation_factor).round() as usize);
        if hover_index >= data_size{
            hover_index = data_size - 1;
        }
        return Some(hover_index);
    }
    None
}
