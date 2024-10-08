use crate::peripheral_util::{
    battery_monitor::BatteryMonitor,
    buttons,
    display::{display_error, Display, DisplayMessage},
    wifi,
};
use anyhow::{bail, Result};
use embedded_hal_bus::i2c::MutexDevice;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::hal::{gpio, i2c};
use std::sync::{mpsc, Mutex};
use std::thread;

pub mod module_runner;
pub mod modules;
pub mod peripheral_util;
use crate::modules::{kasa_control, snake, test};

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

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    unsafe {
        esp_idf_svc::sys::nvs_flash_init();
    }
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // The constant `CONFIG` is auto-generated by `toml_config`.
    let app_config = CONFIG;

    let buttons: Vec<gpio::AnyIOPin> = vec![
        peripherals.pins.gpio46.into(), //1
        peripherals.pins.gpio9.into(),
        peripherals.pins.gpio11.into(),
        peripherals.pins.gpio12.into(),
        peripherals.pins.gpio13.into(),
        peripherals.pins.gpio14.into(),
        peripherals.pins.gpio21.into(),
        peripherals.pins.gpio47.into(),
        peripherals.pins.gpio48.into(), //9
    ];

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio17;
    let scl = peripherals.pins.gpio18;

    let config = i2c::I2cConfig::new().baudrate(400.kHz().into());
    let i2c = i2c::I2cDriver::new(i2c, sda, scl, &config)?;
    let i2c_mutex = Box::new(Mutex::new(i2c));
    //this next line was tricky, we're leaking the box to avoid deconstructing,
    //and reborrowing via &* to get a place expression from the box
    //https://haibane-tenshi.github.io/rust-reborrowing/
    let bus = &*Box::leak(i2c_mutex);
    let device1 = MutexDevice::new(bus);
    let device2 = MutexDevice::new(bus);

    let (but_tx, but_rx) = mpsc::channel();
    let (disp_tx, disp_rx) = mpsc::channel::<DisplayMessage>();

    let mut _wifi = match wifi::wifi(
        app_config.wifi_ssid,
        app_config.wifi_psk,
        peripherals.modem,
        sysloop,
        false,
    ) {
        Ok(inner) => inner,
        Err(err) => {
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };
    //this apparently works for the anteceding thread builder call
    //https://github.com/esp-rs/esp-idf-hal/issues/228#issuecomment-1676035648
    ThreadSpawnConfiguration {
        name: Some("display_service\0".as_bytes()),
        stack_size: 32000,
        priority: 13,
        ..Default::default()
    }
    .set()
    .unwrap();

    let _d_thread = thread::Builder::new().stack_size(32000).spawn(move || {
        let _ = Display::new().display_service(device1, disp_rx);
    });

    ThreadSpawnConfiguration {
        name: Some("runner_service\0".as_bytes()),
        stack_size: 10000,
        priority: 14,
        ..Default::default()
    }
    .set()
    .unwrap();
    let runner_dtx = disp_tx.clone();
    let mut md = crate::module_runner::ModuleRunner::new(
        but_rx,
        disp_tx.clone(),
        vec![
            Box::new(snake::Snake::new()),
            Box::new(kasa_control::KasaControl::new()),
            Box::new(test::TestModule::new()),
        ],
    );
    let _e_thread = thread::Builder::new().stack_size(10000).spawn(move || {
        module_runner::runner_service(&mut md);
        //if module_runner is dying, will it kill child threads?
        display_error(runner_dtx, "Module_Runner\r\nExited".to_string());
    });

    ThreadSpawnConfiguration {
        name: Some("button_service\0".as_bytes()),
        stack_size: 4000,
        priority: 15,
        ..Default::default()
    }
    .set()
    .unwrap();

    let _e_thread = thread::Builder::new().stack_size(4000).spawn(move || {
        buttons::button_service(buttons, but_tx.clone());
    });

    ThreadSpawnConfiguration {
        name: Some("battery_service\0".as_bytes()),
        stack_size: 2000,
        priority: 17,
        ..Default::default()
    }
    .set()
    .unwrap();

    let _e_thread = thread::Builder::new().stack_size(2000).spawn(move || {
        let _ = BatteryMonitor::new().battery_service(device2, disp_tx.clone());
    });

    log::info!("Hello, after thread spawn");

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));

        if !_wifi.is_connected().unwrap() {
            log::info!("wifi disconnected");
            std::thread::sleep(std::time::Duration::from_secs(1)); //sleep a bit
            if let Err(_status) = _wifi.connect() {
                std::thread::sleep(std::time::Duration::from_secs(2)); //sleep a bit
            }
        }
    }
}
