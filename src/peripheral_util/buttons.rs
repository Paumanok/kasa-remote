use esp_idf_svc::hal::gpio;
use std::sync::mpsc::Sender;

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

fn button_action_generic(btn_idx: usize, btn_state: &Buttons) {
    if let Some(tx) = &btn_state.action_tx {
        log::info!("sending from buttons");
        tx.send(btn_idx).unwrap();
    }
}

pub fn button_service(btn_gpio: Vec<impl gpio::IOPin + 'static>, but_tx: Sender<usize>) {
    let mut btns = Buttons::new(Some(but_tx));

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
        for (idx, button) in buttons.iter().enumerate() {
            if button.is_low() {
                if !btns.btns[idx].last_state {
                    btns.btns[idx].last_state = true;
                    log::info!("button {:} pressed", idx);

                    button_action_generic(idx, &btns);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            } else {
                btns.btns[idx].last_state = false;
            }
        }
    }
}
