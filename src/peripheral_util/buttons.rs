use crate::kasa_protocol;
use crate::peripheral_util::Direction;
use crate::peripheral_util::RemoteState;
use crate::CONFIG;
use esp_idf_svc::hal::gpio;
use std::net::TcpStream;
use std::sync::{mpsc::Sender, Arc, Mutex};

#[derive(Copy, Clone)]
struct Button {
    last_state: bool,
}

struct Buttons {
    btns: [Button; 9],
    action_tx: Option<Sender<usize>>,
}

impl Buttons {
    pub fn new(btn_tx: Option<Sender<usize>>) -> Self {
        Self {
            btns: [Button { last_state: false }; 9],
            action_tx: btn_tx,
        }
    }
}

fn button_action(btn_idx: usize, rs: &Arc<Mutex<RemoteState>>) {
    let app_config = CONFIG;
    if btn_idx <= 2 {
        match rs.lock() {
            Ok(mut state) => state.update_from_encoder(match btn_idx {
                0 => Direction::CounterClockwise,
                1 => Direction::Press,
                2 => Direction::Clockwise,
                _ => Direction::None,
            }),
            _ => (),
        };
        // do control button stuff
    } else {
        if let Ok(mut stream) = TcpStream::connect(format!("{:}:9999", app_config.target_ip)) {
            let _res = kasa_protocol::toggle_relay_by_idx(&mut stream, btn_idx - 3);
        }
    }
}

fn button_action_generic(btn_idx: usize, btn_state: &Buttons) {
    if let Some(tx) = &btn_state.action_tx{
        
        tx.send(btn_idx).unwrap();
    }
}

pub fn button_service(btn_gpio: Vec<impl gpio::IOPin + 'static>, rs: Arc<Mutex<RemoteState>>) {
    let mut btns = Buttons::new(None);

    if btn_gpio.len() != 9 {
        return;
    }

    //take the vector of pins and make them into something useful
    let mut buttons: Vec<_> = btn_gpio
        .into_iter()
        .map(|x| gpio::PinDriver::input(x.downgrade()).unwrap())
        .collect::<Vec<_>>();

    //set them all to internal pullup
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
                btns.btns[idx].last_state = false;
            }
        }
    }
}
