use esp_idf_svc::hal::gpio;
use std::sync::{Arc, Mutex};
use crate::peripheral_util::RemoteState;


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
