//
//
use anyhow::{Result};
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_idf_svc::hal::{gpio, i2c};
use std::net::TcpStream;
use std::sync::{ Arc, Mutex};

use rust_kasa::kasa_protocol;
use sh1106::{prelude::*, Builder};



#[toml_cfg::toml_config]
pub struct Config {
    #[default("blah")]
    wifi_ssid: &'static str,
    #[default("blah")]
    wifi_psk: &'static str,
    #[default("127.0.0.1")]
    target_ip: &'static str,
}

#[derive(Clone)]
enum Mode {
    Monitor,
    Totals,
    Info,
}

struct Update {
    idx: Option<u8>,
    mode: Option<Mode>,
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
    stats: Vec<kasa_protocol::Realtime>,
}

impl Totals {
    pub fn new() -> Option<Self> {
        Some(Self {
            stats: Self::get_stats()?,
        })
    }

    pub fn update(&mut self) {
        if let Some(stat) = Self::get_stats() {
            self.stats = stat;
        }
    }

    pub fn get_stats() -> Option<Vec<kasa_protocol::Realtime>> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        kasa_protocol::get_all_realtime(&mut stream)
    }
}

#[derive(Clone)]
pub struct RemoteState {
    mode: Mode,
    monitor: Monitor,
    totals: Option<Totals>,
}
// this wont work at all as it currently sits
impl RemoteState {
    pub fn new() -> Self {
        println!("new remote state initialized");
        Self {
            mode: Mode::Monitor,
            monitor: Monitor::new(),
            totals: None,
        }
    }
    //this is absolutely disgusting
    pub fn update_from_encoder(&mut self, dir: Direction) {
        match self.mode {
            Mode::Monitor => {
                match dir {
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
                    _ => (),
                }
            }
            Mode::Totals => {}
            Mode::Info => {}
        }
    }
}

pub fn display_service(i2c: i2c::I2cDriver, rs: Arc<Mutex<RemoteState>>) -> Result<()> {
    println!("display_service hit");

    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    display.init().unwrap();
    display.flush().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let text_small = MonoTextStyleBuilder::new()
        .font(&FONT_5X8)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    //let info = format!("ip: {:?}",(*_wifi).sta_netif().get_ip_info()?.ip);
    //
    //Text::with_baseline(&info, Point::new(0, 16), text_style, Baseline::Top)
    //    .draw(&mut display)
    //    .unwrap();

    display.flush().unwrap();

    loop {
        match rs.lock() {
            Ok(msg) => {
                //log::info!("got message");
                display.clear();
                let mode1 = "Monitor";
                let mode2 = "Totals";
                let mode3 = "Settings";

                Text::with_baseline(mode1, Point::new(0, 4), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                Text::with_baseline(
                    mode2,
                    Point::new((6 * mode1.len() as i32) + 4, 2),
                    text_small,
                    Baseline::Top,
                )
                .draw(&mut display)
                .unwrap();

                Text::with_baseline(
                    mode3,
                    Point::new((6 * mode1.len() + 6 * mode2.len()) as i32 + 4, 2),
                    text_small,
                    Baseline::Top,
                )
                .draw(&mut display)
                .unwrap();

                //msg.monitor.update();
                let ma = format!(
                    "I: {:?}mA   Idx: {:?}",
                    match msg.monitor.stats {
                        Some(stat) => stat.current_ma,
                        _ => 4242,
                    },
                    msg.monitor.idx
                );
                Text::with_baseline(&ma, Point::new(0, 26), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();

                //let outlet = " 1 * * * * * ";

                let mut outlet = String::from("");

                for i in 1..7 {
                    if msg.monitor.idx + 1 == i {
                        outlet.push_str(format!(" {:?}",i).as_str());
                    } else {
                        outlet.push_str(" *");
                    }
                }
                let outlet = outlet.as_str();
                //let outlet = " ";
                //for i in in 1..6 {
                //
                //}
                Text::with_baseline(outlet, Point::new(28, 57), text_small, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                display.flush().unwrap();
            }
            _ => (),
        };
        //time for ~24fps
        std::thread::sleep(std::time::Duration::from_millis((1000 / 24) as u64));
    }
}
//https://leshow.github.io/post/rotary_encoder_hal/ thank u sir
#[derive(PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
    None,
}

struct Rotary {
    state: u8,
}
impl Rotary {
    pub fn new() -> Self {
        Self { state: 0u8 }
    }

    pub fn update(&mut self, clk: bool, dt: bool) -> Option<Direction> {
        match self.state {
            0 => {
                if !clk {
                    self.state = 1;
                } else if !dt {
                    self.state = 4;
                }
            }
            1 => {
                if !dt {
                    self.state = 2;
                }
            }
            2 => {
                if clk {
                    self.state = 3;
                }
            }
            3 => {
                if clk && dt {
                    self.state = 0;
                    return Some(Direction::Clockwise);
                }
            }
            4 => {
                if !clk {
                    self.state = 5;
                }
            }
            5 => {
                if dt {
                    self.state = 6;
                }
            }
            6 => {
                if clk && dt {
                    self.state = 0;
                    println!("counter");
                    return Some(Direction::CounterClockwise);
                }
            }
            _ => (),
        }

        None
    }
}

pub fn encoder_service(
    dt: gpio::PinDriver<'static, gpio::Gpio26, gpio::Input>,
    clk: gpio::PinDriver<'static, gpio::Gpio27, gpio::Input>,
    rs: Arc<Mutex<RemoteState>>,
) {
    let mut rot = Rotary::new();

    loop {
        match rot.update(clk.is_high(), dt.is_high()) {
            Some(dir) => {
                if dir != Direction::None {
                    match rs.lock() {
                        Ok(mut state) => {
                            state.update_from_encoder(dir);
                            //std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        };

        std::thread::sleep(std::time::Duration::from_millis(5));
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
                        ){
                                Some(rt) => Some(rt),
                                _ => None,

                        }
                    }
                    Mode::Totals => {}
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
