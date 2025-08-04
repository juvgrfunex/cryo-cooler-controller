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
    clippy::unwrap_used,
    clippy::use_debug
)]

use chrono::Utc;
use serial::SerialPort;
use std::{
    convert::TryInto,
    io::{Read, Write},
};
const CRC_16_XMODEM: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_XMODEM);
pub struct Tec {
    port: serial::SystemPort,
    port_ident: std::ffi::OsString,
}

impl Tec {
    fn send_cmd(&mut self, request: &Request) -> Result<Response, std::io::Error> {
        self.port.write_all(&request.as_bytes())?;
        let mut buffer = [0u8; 8];
        self.port.read_exact(&mut buffer)?;
        if buffer[1] != { request.op_code + 127 } {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Response contained incorrect op code",
            ));
        }

        let crc = CRC_16_XMODEM.checksum(&buffer[0..6]);
        let response = Response::from_bytes(buffer);
        if response.crc == crc {
            Ok(response)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Response contained incorrect crc",
            ))
        }
    }

    pub fn reset(&mut self) -> Result<(), std::io::Error> {
        self.send_cmd(&Request::new(commands::set::RESET_BOARD, [0; 4]))?;
        Ok(())
    }

    fn set_pid(&mut self, p: f32, i: f32, d: f32) -> Result<(), std::io::Error> {
        self.send_cmd(&Request::new(commands::set::P_COEFFICIENT, p.to_le_bytes()))?;
        self.send_cmd(&Request::new(commands::set::I_COEFFICIENT, i.to_le_bytes()))?;
        self.send_cmd(&Request::new(commands::set::D_COEFFICIENT, d.to_le_bytes()))?;

        Ok(())
    }
}

fn open_serial_port<T: AsRef<std::ffi::OsStr>>(
    serial_port: &T,
) -> Result<serial::SystemPort, std::io::Error> {
    let mut port = serial::open(serial_port)?;
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud115200)?;
        settings.set_char_size(serial::Bits8);
        settings.set_stop_bits(serial::Stop1);
        settings.set_parity(serial::ParityNone);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    })?;
    Ok(port)
}
impl Tec {
    pub fn reset_connection(&mut self) -> Result<(), std::io::Error> {
        self.port = open_serial_port(&self.port_ident)?;
        Ok(())
    }

    pub fn new<T: AsRef<std::ffi::OsStr>>(serial_port: &T) -> Result<Self, std::io::Error> {
        let port = open_serial_port(serial_port)?;
        let mut tec = Tec {
            port,
            port_ident: serial_port.into(),
        };

        let status = tec.heart_beat()?;
        if !status.contains(TecStatus::BOARD_INIT) {
            tec.reset()?;
        }

        Ok(tec)
    }

    pub fn heart_beat(&mut self) -> Result<TecStatus, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::HEART_BEAT, [0; 4]))?;
        let status_code = u32::from_le_bytes(response.data);
        TecStatus::from_bits(status_code & 0b111111111111111111).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Tecstatus bit pattern invalid: {status_code:b} "),
            )
        })
    }

    pub fn monitor(&mut self) -> Result<MonitoringData, std::io::Error> {
        Ok(MonitoringData {
            timestamp: Utc::now(),
            tec_temperature: self.tec_temperature()?,
            pcb_temperature: self.board_temperature()?,
            humidity: self.humidity()?,
            dew_point_temperature: self.dew_point_temperature()?,
            tec_voltage: self.tec_voltage()?,
            tec_current: self.tec_current()?,
            tec_power_level: self.tec_power_level()?,
        })
    }
    pub fn humidity(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::HUMIDITY, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn tec_temperature(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::TEC_TEMPERATURE, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn board_temperature(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::BOARD_TEMP, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn dew_point_temperature(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::DEW_POINT, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn tec_voltage(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::TEC_VOLTAGE, [0; 4]))?;
        Ok(u32::from_le_bytes(response.data) as f32 / 21.1)
    }

    pub fn tec_current(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::TEC_CURRENT, [0; 4]))?;
        Ok(u32::from_le_bytes(response.data) as f32 / 4.6545)
    }

    pub fn tec_power_level(&mut self) -> Result<u8, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::TEC_POWERLEVEL, [0; 4]))?;
        Ok(response.data[0])
    }

    pub fn p_coefficient(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::P_COEFFICIENT, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn i_coefficient(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::I_COEFFICIENT, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn d_coefficient(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::D_COEFFICIENT, [0; 4]))?;
        Ok(f32::from_le_bytes(response.data))
    }

    pub fn set_setpoint_offset(&mut self, setpoint: f32) -> Result<(), std::io::Error> {
        self.send_cmd(&Request::new(
            commands::set::POINT_OFFSET,
            setpoint.to_le_bytes(),
        ))?;

        Ok(())
    }

    pub fn setpoint_offset(&mut self) -> Result<f32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::SET_POINT_OFFSET, [0; 4]))?;
        Ok(u32::from_le_bytes(response.data) as f32)
    }

    pub fn hw_version(&mut self) -> Result<u32, std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::HW_VERSION, [0; 4]))?;
        Ok(u32::from_le_bytes(response.data))
    }

    pub fn fw_version(&mut self) -> Result<(u8, u8, u8, u8), std::io::Error> {
        let response = self.send_cmd(&Request::new(commands::get::FW_VERSION, [0; 4]))?;
        Ok((
            response.data[0],
            response.data[1],
            response.data[2],
            response.data[3],
        ))
    }

    pub fn set_power_level(&mut self, power_level: u8) -> Result<(), std::io::Error> {
        //! Does not work currently
        self.send_cmd(&Request::new(
            commands::set::TEC_POWER_LEVEL,
            [power_level, 0, 0, 0],
        ))?;
        Ok(())
    }

    pub fn enable(
        &mut self,
        p: f32,
        i: f32,
        d: f32,
        power_level: u8,
        setpoint: f32,
    ) -> Result<(), std::io::Error> {
        self.set_power_level(power_level)?;
        self.set_setpoint_offset(setpoint)?;
        self.set_pid(p, i, d)?;

        self.send_cmd(&Request::new(
            commands::set::DISABLE_NOT_ENABLE,
            [0, 0, 0, 0],
        ))?;

        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), std::io::Error> {
        self.send_cmd(&Request::new(
            commands::set::DISABLE_NOT_ENABLE,
            [1, 0, 0, 0],
        ))?;
        Ok(())
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct TecStatus: u32{
        const BOARD_INIT = 1 << 0;
        const POWER_OK = 1 << 1;
        const TEMP_SENSE_OK = 1 << 2;
        const HUM_SENSE_OK = 1 << 3;
        const LAST_CMD_OK = 1 << 4;
        const LAST_CMD_BAD_CRC = 1 << 5;
        const LAST_CMD_INCOMPLETE = 1 << 6;
        const FAILSAFE_ACTIVE = 1 << 7;
        const PID_READY = 1 << 8;
        const PID_INVALID = 1 << 9;
        const PID_OUT_OF_RANGE = 1 << 10;
        const PID_DEFAULT = 1 << 11;
        const PID_RUNNING = 1 << 12;
        const OCP_ACTIVE = 1 << 13;
        const BOARD_TEMP_OK = 1 << 14;
        const TEC_CONN_OK = 1 << 15;
        const LOW_POWER_MODE_ACTIVE = 1 << 16;
        const TEMP_MODE = 1 << 17;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Request {
    fixed: u8,
    op_code: u8,
    data: [u8; 4],
    crc: u16,
}

impl Request {
    pub const fn new(op_code: u8, data: [u8; 4]) -> Self {
        let buffer = [0xAA, op_code, data[0], data[1], data[2], data[3]];
        let crc = CRC_16_XMODEM.checksum(&buffer);
        Request {
            fixed: 0xAA,
            op_code,
            data,
            crc,
        }
    }

    const fn as_bytes(&self) -> [u8; 8] {
        [
            self.fixed,
            self.op_code,
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            (self.crc & 0xFF) as u8,
            (self.crc >> 8) as u8,
        ]
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Response {
    fixed: u8,
    op_code: u8,
    data: [u8; 4],
    crc: u16,
}

impl Response {
    fn from_bytes(bytes: [u8; 8]) -> Self {
        Response {
            fixed: bytes[0],
            op_code: bytes[1],
            data: bytes[2..6]
                .try_into()
                .expect("Constant slice size will not fail"),
            crc: u16::from_le_bytes(
                bytes[6..8]
                    .try_into()
                    .expect("Constant slice size will not fail"),
            ),
        }
    }
}
#[allow(dead_code)]
mod commands {
    pub const HEART_BEAT: u8 = 0x00;
    pub mod get {
        pub const TEC_TEMPERATURE: u8 = 0x01;
        pub const HUMIDITY: u8 = 0x02;
        pub const DEW_POINT: u8 = 0x03;
        pub const SET_POINT_OFFSET: u8 = 0x04;
        pub const P_COEFFICIENT: u8 = 0x05;
        pub const I_COEFFICIENT: u8 = 0x06;
        pub const D_COEFFICIENT: u8 = 0x07;
        pub const TEC_POWERLEVEL: u8 = 0x08;
        pub const HW_VERSION: u8 = 0x09;
        pub const FW_VERSION: u8 = 0x0A;
        pub const NTC_COEFFICIENT: u8 = 0x1B;
        pub const BOARD_TEMP: u8 = 0x1F;
        pub const VOLTAGE_AND_CURRENT: u8 = 0x22;
        pub const TEC_VOLTAGE: u8 = 0x23;
        pub const TEC_CURRENT: u8 = 0x24;
    }
    pub mod set {
        pub const POINT_OFFSET: u8 = 0x14;
        pub const P_COEFFICIENT: u8 = 0x15;
        pub const I_COEFFICIENT: u8 = 0x16;
        pub const D_COEFFICIENT: u8 = 0x17;
        pub const DISABLE_NOT_ENABLE: u8 = 0x18;
        pub const CPU_TEMP: u8 = 0x19; // unclear how to use
        pub const NTC_COEFFICIENT: u8 = 0x20;
        pub const TEMP_SENSOR: u8 = 0x1C;
        pub const TEC_POWER_LEVEL: u8 = 0x1D;
        pub const RESET_BOARD: u8 = 0x1E;
    }
}

pub struct MonitoringData {
    pub timestamp: chrono::DateTime<Utc>,
    pub tec_temperature: f32,
    pub pcb_temperature: f32,
    pub humidity: f32,
    pub dew_point_temperature: f32,
    pub tec_voltage: f32,
    pub tec_current: f32,
    pub tec_power_level: u8,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, unused)]
mod tests {

    use super::*;
    const PORT_NAME: &str = "COM3";

    #[test]
    fn open() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
    }

    #[test]
    fn heart_beat() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
        let beat = tec.heart_beat();
    }

    #[test]
    fn humidity() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
        let hum = tec.humidity();
    }

    #[test]
    fn board_temp() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
        let hum = tec.board_temperature();
    }

    #[test]
    fn hw_version() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
        let hw_version = tec.hw_version();
    }

    #[test]
    fn fw_version() {
        let mut tec = Tec::new(&PORT_NAME).unwrap();
        let fw_version = tec.fw_version().unwrap();
    }
}
