
use crate::CONFIG;
use crate::kasa_protocol;
use crate::peripheral_util::RemoteState;
use crate::peripheral_util::Direction;
use esp_idf_svc::hal::gpio;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

#[derive(Copy)]
#[derive(Clone)]
struct Button {
    last_state: bool,
}

struct Buttons {
    btns: [Button; 9],
}

impl Buttons {
    pub fn new() -> Self {
        Self {
            btns: [ Button{last_state: false} ; 9],
        }
    }
}

fn button_action(btn_idx: usize, rs: &Arc<Mutex<RemoteState>>) {
    let app_config = CONFIG;
    if btn_idx <= 2 {
       match rs.lock() {
            Ok(mut state) => {
                state.update_from_encoder( match btn_idx {
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
            let _res = kasa_protocol::toggle_relay_by_idx(&mut stream, btn_idx - 3);
        }
    }
}

pub fn button_service(btn_gpio: Vec<impl gpio::IOPin + 'static>, rs: Arc<Mutex<RemoteState>>) {

    let mut btns = Buttons::new();

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

    loop {
        for idx in 0..buttons.len() {
            if buttons[idx].is_low() {
                
                if !btns.btns[idx].last_state {
                    btns.btns[idx].last_state = true;
                    log::info!("button {:} pressed", idx); 
                    
                    button_action(idx, &rs);
                }
                //std::thread::sleep(std::time::Duration::from_millis(100));
            } else {
                btns.btns[idx].last_state = false; //wtf logic goes there
            }
        }

    }
}
