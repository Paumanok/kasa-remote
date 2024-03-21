
use crate::CONFIG;
use crate::kasa_protocol;
use crate::peripheral_util::RemoteState;
use crate::peripheral_util::Direction;
use esp_idf_svc::hal::gpio;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub fn button_service(btn_gpio: Vec<impl gpio::IOPin + 'static>, rs: Arc<Mutex<RemoteState>>) {
    let app_config = CONFIG;
    if btn_gpio.len() != 9 {
        return;
    }

    let mut buttons: Vec<_> = btn_gpio
        .into_iter()
        .map(|x| {
            gpio::PinDriver::input(x.downgrade())
                .unwrap()
        })
        .collect::<Vec<_>>();

    for b in &mut buttons {
        b.set_pull(gpio::Pull::Up).unwrap();
    }
    log::info!("after pull up set");
    loop {
//      for (idx, b) in buttons.iter().enumerate() {
        for idx in 0..buttons.len() {
            //log::info!("idx at {:}", idx);
            if buttons[idx].is_low() {
                if idx <= 2 {
                   match rs.lock() {
                        Ok(mut state) => {
                            state.update_from_encoder( match idx {
                                0 => Direction::CounterClockwise,
                                1 => Direction::Press,
                                2 => Direction::Clockwise,
                                _ => Direction::None,
                            })
                        },
                        _ => (),
                    };
                    // do control button stuff
                } else {
                    if let Ok(mut stream) = TcpStream::connect(format!("{:}:9999", app_config.target_ip)) {
                        let _res = kasa_protocol::toggle_relay_by_idx(&mut stream, idx - 3);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

    }
}
