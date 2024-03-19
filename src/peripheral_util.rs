//
//
use anyhow::{Result};
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, ascii::FONT_6X10, MonoTextStyleBuilder, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_idf_svc::hal::{gpio, i2c};
use std::net::TcpStream;
use std::sync::{ Arc, Mutex};

use rust_kasa::kasa_protocol;
use sh1106::{prelude::*, Builder, displayrotation::DisplayRotation};



#[toml_cfg::toml_config]
pub struct Config {
    #[default("blah")]
    wifi_ssid: &'static str,
    #[default("blah")]
    wifi_psk: &'static str,
    #[default("127.0.0.1")]
    target_ip: &'static str,
}

#[derive(Clone, Copy)]
#[derive(PartialEq)]
#[derive(Debug)]
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
            voltage_mv: (stats_vec.iter().fold(0u32, |sum, rt| sum + rt.voltage_mv)) / stats_vec.len() as u32,
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
                        Direction::Press => {
                            self.select_mode = !self.select_mode;
                        }
                        _ => (),
                    }
                }
                Mode::Totals => {}
                Mode::Info => {}
            }
        }
    }
}


pub struct Display<'a> {
    text_normal: MonoTextStyle<'a,BinaryColor>,
    text_small: MonoTextStyle<'a,BinaryColor>,
}

impl<'a> Display<'a> {
    pub fn new() -> Self{
        
        let text_normal = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        let text_small = MonoTextStyleBuilder::new()
            .font(&FONT_5X8)
            .text_color(BinaryColor::On)
            .build();

        Self {
            text_normal: text_normal,
            text_small: text_small,
        }
    }

    fn lazy_menu_setup(&mut self, cur_mode: Mode, drawing_mode: Mode) -> 
    (i32,  MonoTextStyle<'a,BinaryColor>) {
        if cur_mode == drawing_mode {

            return (4, self.text_normal)
        } else {
            return (2, self.text_small)
        }

    }


    pub fn display_service(&mut self,i2c: i2c::I2cDriver, rs: Arc<Mutex<RemoteState>>) -> Result<()> {
        println!("display_service hit");
        //this Builder is the specific SH1106 builder 
        let mut display: GraphicsMode<_> = Builder::new().with_rotation(DisplayRotation::Rotate180).connect_i2c(i2c).into();
    
        display.init().unwrap();
        display.flush().unwrap();
    
        //let text_style = MonoTextStyleBuilder::new()
        //    .font(&FONT_6X10)
        //    .text_color(BinaryColor::On)
        //    .build();
        //let text_small = MonoTextStyleBuilder::new()
        //    .font(&FONT_5X8)
        //    .text_color(BinaryColor::On)
        //    .build();
        Text::with_baseline("Hello world!", Point::zero(), self.text_normal, Baseline::Top)
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
                    display.clear();
                    let modes =  ["Monitor", "Totals", "Settings"];
                    let mode1 = "Monitor";
                    let mode2 = "Totals";
                    let mode3 = "Info";
                    
    
                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Monitor);
                    Text::with_baseline(mode1, Point::new(0, y), text, Baseline::Top)
                        .draw(&mut display)
                        .unwrap();

                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Totals);
                    Text::with_baseline(
                        mode2,
                        Point::new((6 * mode1.len() as i32) + 6, y),
                        text,
                        Baseline::Top,
                    )
                    .draw(&mut display)
                    .unwrap();
    
                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Info);
                    Text::with_baseline(
                        mode3,
                        Point::new((6 * mode1.len() + 6 * mode2.len()) as i32 + 12, y),
                        text,
                        Baseline::Top,
                    )
                    .draw(&mut display)
                    .unwrap();
    
                    //msg.monitor.update();
                    match msg.mode {
                        Mode::Monitor => {
                            if let Some(stats) = msg.monitor.stats {
                                let ma = format!(
                                    "I:{:>4}mA   P: {:>4}mW\r\n\r\nPt: {:>3}Wh",
                                    stats.current_ma,
                                    stats.power_mw,
                                    stats.total_wh,
                                );
                                Text::with_baseline(&ma, Point::new(0, 18), self.text_normal, Baseline::Top)
                                    .draw(&mut display)
                                    .unwrap();

                                let mut outlet = String::from("");
                                for i in 1..7 {
                                    if msg.monitor.idx + 1 == i {
                                        outlet.push_str(format!(" {:?}",i).as_str());
                                    } else {
                                        outlet.push_str(" *");
                                    }
                                }
                                let outlet = outlet.as_str();
                                
                                Text::with_baseline(outlet, Point::new(28, 57), self.text_small, Baseline::Top)
                                    .draw(&mut display)
                                    .unwrap();
                            };
                        }
                        Mode::Totals => {

                            if let Some(stats) = msg.totals.stats {
                                let ma = format!(
                                    "I:{:>4}mA   P: {:>4}mW\r\n\r\nPt: {:>3}Wh",
                                    stats.current_ma,
                                    stats.power_mw,
                                    stats.total_wh,
                                );
                                Text::with_baseline(&ma, Point::new(0, 18), self.text_normal, Baseline::Top)
                                    .draw(&mut display)
                                    .unwrap();
                            } else {
                                println!("stats none in totals");
                            }
                        }
                        _ => (),
                    }
    
                    display.flush().unwrap();
    
    
                }
                _ => (),
            };
            //time for ~24fps
            std::thread::sleep(std::time::Duration::from_millis((1000 / 12) as u64));
        }
    }

}

//https://leshow.github.io/post/rotary_encoder_hal/ thank u sir
#[derive(PartialEq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
    Press,
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
    dt: impl gpio::IOPin + 'static,
    clk: impl gpio::IOPin + 'static,
    btn: impl gpio::IOPin + 'static,
    rs: Arc<Mutex<RemoteState>>,
) {
    //https://github.com/esp-rs/esp-idf-hal/issues/221#issuecomment-1483905314
    //this is so I don't have to hard-specify the pin in the function signature
    let mut dt = gpio::PinDriver::input(dt.downgrade()).unwrap();
    let mut clk = gpio::PinDriver::input(clk.downgrade()).unwrap();
    let mut btn = gpio::PinDriver::input(btn.downgrade()).unwrap();
    dt.set_pull(gpio::Pull::Up).unwrap();
    clk.set_pull(gpio::Pull::Up).unwrap();
    btn.set_pull(gpio::Pull::Up).unwrap();

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
        
        if btn.is_low() {
            match rs.lock() {
                Ok(mut state) => {
                    state.update_from_encoder(Direction::Press);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                _ => (),
            }
        }
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
