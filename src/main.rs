use anyhow::{bail, Result};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::{gpio, i2c};
use esp_idf_svc::hal::prelude::*;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use core::fmt::Write;
use sh1106::{prelude::*, Builder};
//use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306, mode::TerminalMode};

use rust_kasa::kasa_protocol;
use std::net::TcpStream;
use std::{thread, sync::Arc};
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use std::sync::mpsc;
use wifi::wifi;


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

struct remote_state {
    ps_info:  kasa_protocol::SysInfo,
    realtime: Vec<kasa_protocol::Realtime>,
    switches: Vec<bool>,
}

//static FLAG: AtomicBool = AtomicBool::new(false);
//
//fn gpio_int_callback() {
//    FLAG.store(true, Ordering::Relaxed);
//    println!("callback hit");
//}
//
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
fn get_ma() -> Option<kasa_protocol::Realtime> {
    let app_config = CONFIG; 
    let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
    let rt = kasa_protocol::get_realtime(&mut stream);
    
    return rt
    //return Some(99)
}



fn display_service(i2c: i2c::I2cDriver, rx: mpsc::Receiver<u32>)  -> Result<()>{
    println!("display_service hit");


    let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    display.init().unwrap();
    display.flush().unwrap();

    
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
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
            Ok(msg) => {
                log::info!("got message");
                display.clear();
                let ma = format!("current ma: {:?}", msg);
                Text::with_baseline(&ma, Point::new(0, 26), text_style, Baseline::Top)
                    .draw(&mut display)
                    .unwrap();
                display.flush().unwrap();
            }
            _ => (),
        };
        
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

}

fn current_service(tx: mpsc::Sender<u32>) {
    
    let app_config = CONFIG; 
    loop {

        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok().unwrap();
        let rt = kasa_protocol::get_realtime(&mut stream);
        match rt {
            Some(i) => {
                //log::info!("sent message");
                tx.send(i.current_ma);
        }
        _ => (),
        };

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
    

    let _wifi = match wifi(
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

    //let mut rs = remote_state {
    //    ps_info: get_sys().unwrap(),
    //    realtime: get_allrt().unwrap(),
    //    switches: vec![false;5]
    //};

    let mut button = gpio::PinDriver::input(peripherals.pins.gpio19).unwrap();
    button.set_pull(gpio::Pull::Up).unwrap();

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio22;
    let scl = peripherals.pins.gpio21;

    let config = i2c::I2cConfig::new().baudrate(400.kHz().into());
    let mut i2c = i2c::I2cDriver::new(i2c, sda, scl, &config)?;
    
    let (tx,rx) = mpsc::channel();
   
    ThreadSpawnConfiguration {
         name: Some("mid-lvl-thread\0".as_bytes()),
         stack_size: 10000,
         priority: 15,
         ..Default::default()
     }
     .set()
     .unwrap(); 

     let i_thread = thread::Builder::new()
         .stack_size(5000)
         .spawn(move || {
             current_service(tx);
         });


    //thread::spawn(move || {
    //    button_service(button);
    //});

    thread::spawn( move || {
        display_service(i2c, rx);
    });
    

    log::info!("Hello, after thread spawn");

    loop {
        //if FLAG.load(Ordering::Relaxed) {
            //FLAG.store(false, Ordering::Relaxed);
        if button.is_low() {
            let _ = toggle();
            println!("hit");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }


        //match get_ma() {
        //    Some(i) => {
        //        //log::info!("sent message");
        //        tx.send(i.current_ma);
        //}
        //_ => (),
        //};
        //std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
