//
//
//use anyhow::Result;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::peripheral_util::rotary::Direction;
use rust_kasa::kasa_protocol;

pub mod buttons;
pub mod display;
pub mod rotary;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("blah")]
    wifi_ssid: &'static str,
    #[default("blah")]
    wifi_psk: &'static str,
    #[default("127.0.0.1")]
    target_ip: &'static str,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Monitor,
    Totals,
    Info,
}

#[derive(Clone)]
struct Monitor {
    idx: u8,
    stats: Option<kasa_protocol::Realtime>,
}
impl Monitor {
    pub fn new() -> Self {
        Self {
            idx: 0u8,
            stats: Self::get_current_stat(1),
        }
    }

    pub fn update(&mut self) {
        if let Some(stat) = Self::get_current_stat(self.idx) {
            self.stats = Some(stat);
        }
    }
    pub fn get_current_stat(idx: u8) -> Option<kasa_protocol::Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        kasa_protocol::get_realtime_by_idx(&mut stream, idx.into())
    }
}

#[derive(Clone)]
struct Totals {
    stats: Option<kasa_protocol::Realtime>,
}

impl Totals {
    pub fn new() -> Self {
        Self {
            stats: Self::get_stats(),
        }
    }

    pub fn update(&mut self) {
        if let Some(stat) = Self::get_stats() {
            self.stats = Some(stat);
        }
    }

    pub fn get_stats() -> Option<kasa_protocol::Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        let stats_vec = kasa_protocol::get_all_realtime(&mut stream)?;
        Some(kasa_protocol::Realtime {
            current_ma: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.current_ma),
            err_code: 0,
            power_mw: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.power_mw),
            slot_id: 0,
            total_wh: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.total_wh),
            voltage_mv: (stats_vec.iter().fold(0u32, |sum, rt| sum + rt.voltage_mv))
                / stats_vec.len() as u32,
        })
    }
}

#[derive(Clone)]
pub struct RemoteState {
    mode: Mode,
    select_mode: bool,
    monitor: Monitor,
    totals: Totals,
}
// this wont work at all as it currently sits
impl RemoteState {
    pub fn new() -> Self {
        println!("new remote state initialized");
        Self {
            mode: Mode::Monitor,
            select_mode: false,
            monitor: Monitor::new(),
            totals: Totals::new(),
        }
    }

    fn update_mode(&mut self, dir: Direction) {
        match dir {
            Direction::Clockwise => {
                self.mode = match self.mode {
                    Mode::Monitor => Mode::Totals,
                    Mode::Totals => Mode::Info,
                    _ => self.mode,
                };
            }
            Direction::CounterClockwise => {
                self.mode = match self.mode {
                    Mode::Info => Mode::Totals,
                    Mode::Totals => Mode::Monitor,
                    _ => self.mode,
                };
            }
            Direction::Press => {
                self.select_mode = !self.select_mode;
            }
            _ => (),
        };
        println!("{:?}", self.mode);
    }

    //this is absolutely disgusting
    pub fn update_from_encoder(&mut self, dir: Direction) {
        if self.select_mode {
            self.update_mode(dir);
        } else {
            match self.mode {
                Mode::Monitor => match dir {
                    Direction::Clockwise => {
                        if self.monitor.idx < 5 {
                            self.monitor.idx += 1;
                            println!("{:?}", self.monitor.idx);
                        }
                    }
                    Direction::CounterClockwise => {
                        if self.monitor.idx > 0 {
                            self.monitor.idx -= 1;
                            println!("{:?}", self.monitor.idx);
                        }
                    }
                    Direction::Press => {
                        self.select_mode = !self.select_mode;
                    }
                    _ => (),
                },
                Mode::Totals => {}
                Mode::Info => {}
            }
        }
    }
}

pub fn statistics_service(rs: Arc<Mutex<RemoteState>>) {
    let app_config = CONFIG;
    loop {
        if let Ok(mut stream) = TcpStream::connect(format!("{:}:9999", app_config.target_ip)) {
            match rs.lock() {
                Ok(mut state) => match state.mode {
                    Mode::Monitor => {
                        state.monitor.stats = match kasa_protocol::get_realtime_by_idx(
                            &mut stream,
                            state.monitor.idx as usize,
                        ) {
                            Some(rt) => Some(rt),
                            _ => None,
                        };
                    }
                    Mode::Totals => {
                        state.totals.update();
                        //longer wait since this takes longer and is bogging up the thread doing it
                        //too often
                        //std::thread::sleep(std::time::Duration::from_millis(5000));
                    }
                    Mode::Info => {}
                },
                _ => (),
            }
        } else {
            log::info!("failed to connect tcp stream");
        }

        //update every second
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
