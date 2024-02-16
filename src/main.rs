use anyhow::{bail, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::{gpio, i2c};
use esp_idf_svc::hal::prelude::*;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_5X8, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use std::net::TcpStream;
use std::{thread};
use std::sync::mpsc;
use wifi::wifi;

use sh1106::{prelude::*, Builder};
use rust_kasa::kasa_protocol;

/// This configuration is picked up at compile time by `build.rs` from the
/// file `cfg.toml`.
#[toml_cfg::toml_config]
pub struct Config {
    #[default("blah")]
    wifi_ssid: &'static str,
    #[default("blah")]
    wifi_psk: &'static str,
    #[default("127.0.0.1")]
    target_ip: &'static str,
}


fn toggle() -> Result<()>{
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip))?;
    //let mut stream = TcpStream::connect("10.20.10.155:9999").ok().unwrap();
    kasa_protocol::toggle_relay_by_idx(&mut stream, 0);
    Ok(())
}

fn get_sys() -> Option<kasa_protocol::SysInfo> {
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
    let sys : Option<kasa_protocol::SysInfo> = kasa_protocol::get_sys_info(&mut stream);
    return sys
}

fn get_allrt() -> Option<Vec<kasa_protocol::Realtime>> {
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
    let all_rt: Option<Vec<kasa_protocol::Realtime>> = kasa_protocol::get_all_realtime(&mut stream);
    return all_rt
}

fn get_rt(idx: u8) -> Option<kasa_protocol::Realtime> {
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
    let children = kasa_protocol::get_children(&mut stream)?;

    let rt = kasa_protocol::get_realtime_by_id(&mut stream, &children[idx as usize].id);

    return rt

}

fn get_ma() -> Option<kasa_protocol::Realtime> {
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
    let rt = kasa_protocol::get_realtime(&mut stream);
    return rt
}

#[derive(Clone)]
enum Mode {
    Monitor,
    Totals,
    Info
}

struct Update {
    idx: Option<u8>,
    mode: Option<Mode>
}

#[derive(Clone)]
struct Monitor {
    idx: u8,
    stats: Option<kasa_protocol::Realtime>,
}
impl Monitor {
    pub fn new() -> Self{
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
    stats: Vec<kasa_protocol::Realtime>
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
struct RemoteState {
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
                        //println!("c");
                        if self.monitor.idx < 5 {
                            self.monitor.idx+=1;
                            println!("{:?}", self.monitor.idx);
                        }
                    }
                    Direction::CounterClockwise => {
                        //println!("cc");
                        if self.monitor.idx > 0 {
                            //println!("subbin");
                            self.monitor.idx-=1;
                            println!("{:?}", self.monitor.idx);
                        }
                    }
                    _ => (),
                }
            }
            Mode::Totals => {

            }
            Mode::Info => {
            }
        }
    }


}



fn display_service(i2c: i2c::I2cDriver, rx: mpsc::Receiver<RemoteState>)  -> Result<()>{
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
        
        match rx.try_recv() {
            Ok(mut msg) => {
                //log::info!("got message");
                display.clear();
                let mode1 = "Monitor";
                let mode2 = "Totals";
                let mode3 = "Settings";

                Text::with_baseline(mode1, Point::new(0, 4), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                Text::with_baseline(mode2, Point::new((6 * mode1.len() as i32) + 4, 2), text_small, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();

                Text::with_baseline(mode3, Point::new((6 * mode1.len() + 6 * mode2.len()) as i32 + 4, 2), text_small, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                
                msg.monitor.update();
                let ma = format!("I: {:?}mA   Idx: {:?}", 
                    match msg.monitor.stats {
                        Some(stat) => stat.current_ma,
                        _ => 4242,
                    },
                    msg.monitor.idx
                );
                Text::with_baseline(&ma, Point::new(0, 26), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                
                let outlet = " 1 * * * * * ";
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
        
        std::thread::sleep(std::time::Duration::from_millis(1000));
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
        Self {
            state: 0u8,
        }
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
                    return Some(Direction::CounterClockwise);
                }
            }
            _ => (),
        }

        None
    }

}

fn encoder_service(dt: gpio::PinDriver<'static, gpio::Gpio26, gpio::Input>,
        clk: gpio::PinDriver<'static,gpio::Gpio27, gpio::Input>,
        tx: mpsc::Sender<Direction>) {
    
    let mut rot = Rotary::new();


    loop {

        match rot.update(clk.is_low(), dt.is_low()) {
            Some(dir) => {
                if dir != Direction::None {
                    let _ = tx.send(dir);
                    //std::thread::sleep(std::time::Duration::from_millis(200));
                }
            },
            _ => (),
        };
            
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn current_service(tx: mpsc::Sender<kasa_protocol::Realtime>) {
    
    let app_config = CONFIG; 
    loop {

        if let Ok(mut stream) = TcpStream::connect(format!("{:}:9999", app_config.target_ip)) {
            //got a crash here with a None Value
            let rt = kasa_protocol::get_realtime(&mut stream);
            match rt {
                Some(i) => {
                    //log::info!("sent message");
                    let _ = tx.send(i);
                }
                _ => (),
            };
        } else {
            log::info!("failed to connect tcp stream");
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));

    }

}


fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // The constant `CONFIG` is auto-generated by `toml_config`.
    let app_config = CONFIG;
    

    let mut _wifi = match wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
    ) {
        Ok(inner) => inner,
        Err(err) => {
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };


    let mut button = gpio::PinDriver::input(peripherals.pins.gpio19).unwrap();
    button.set_pull(gpio::Pull::Up).unwrap();

    let mut encoder_switch = gpio::PinDriver::input(peripherals.pins.gpio14).unwrap();
    encoder_switch.set_pull(gpio::Pull::Up).unwrap();

    let mut enc_a = gpio::PinDriver::input(peripherals.pins.gpio26).unwrap();
    let mut enc_b = gpio::PinDriver::input(peripherals.pins.gpio27).unwrap();
    enc_a.set_pull(gpio::Pull::Up).unwrap();
    enc_b.set_pull(gpio::Pull::Up).unwrap();
    
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio22;
    let scl = peripherals.pins.gpio21;

    let config = i2c::I2cConfig::new().baudrate(400.kHz().into());
    let  i2c = i2c::I2cDriver::new(i2c, sda, scl, &config)?;
    
    let (i_tx,i_rx) = mpsc::channel();
    let (enc_tx,enc_rx) = mpsc::channel();
    let (rs_tx, rs_rx) = mpsc::channel();
    //this apparently works for the anteceding thread builder call
    //https://github.com/esp-rs/esp-idf-hal/issues/228#issuecomment-1676035648
    ThreadSpawnConfiguration {
         name: Some("current_service\0".as_bytes()),
         stack_size: 8000,
         priority: 13,
         ..Default::default()
     }
     .set()
     .unwrap(); 

     let _i_thread = thread::Builder::new()
         .stack_size(5000)
         .spawn(move || {
             let _ = current_service(i_tx);
         });
    
    ThreadSpawnConfiguration {
         name: Some("display_service\0".as_bytes()),
         stack_size: 10000,
         priority: 14,
         ..Default::default()
     }
     .set()
     .unwrap(); 

    let _d_thread = thread::Builder::new()
        .stack_size(10000)
        .spawn( move || {
        let _ = display_service(i2c, rs_rx);
    });

    ThreadSpawnConfiguration {
         name: Some("encoder_service\0".as_bytes()),
         stack_size: 3000,
         priority: 15,
         ..Default::default()
     }
     .set()
     .unwrap(); 

    let _e_thread = thread::Builder::new()
        .stack_size(3000)
        .spawn(move || {
            let _ = encoder_service(enc_a.into(), enc_b.into(), enc_tx);
        });

    log::info!("Hello, after thread spawn");
    
    let mut rs = RemoteState::new();
    //send the inital state for the display
    rs_tx.send(rs.clone());

    loop {
        if button.is_low() {
            let _ = toggle();
            println!("hit");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        
        match enc_rx.try_recv() {
            Ok(msg) => {
                rs.update_from_encoder(msg);
                rs_tx.send(rs.clone());
            }
            _ => (),
        };



        if !_wifi.is_connected().unwrap() {
            log::info!("wifi disconnected");
            std::thread::sleep(std::time::Duration::from_secs(1)); //sleep a bit
            match _wifi.connect() {
                Err(_status) => {
                    std::thread::sleep(std::time::Duration::from_secs(2)); //sleep a bit
                }
                _ => (),
            }


        }
    }
}
