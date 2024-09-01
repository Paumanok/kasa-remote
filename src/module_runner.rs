use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use std::mem::replace;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::peripheral_util::display::DisplayMessage;

fn dummy_module() -> Box<dyn RemoteModule + Send> {
    struct Dummy;
    impl RemoteModule for Dummy {
        fn set_channel(
            &mut self,
            _receiver: mpsc::Receiver<RemoteMessage>,
            _sender: mpsc::Sender<DisplayMessage>,
        ) {
        }
        fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>> {
            None
        }
        fn get_display_name(&self) -> String {
            "dummy".to_string()
        }
        fn run(&mut self) {}
    }
    Box::new(Dummy)
}

#[derive(Clone)]
pub struct RemoteMessage {
    pub status: u32,
}
pub trait RemoteModule {
    fn set_channel(
        &mut self,
        chnl: mpsc::Receiver<RemoteMessage>,
        sender: mpsc::Sender<DisplayMessage>,
    );
    fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>>;
    fn get_display_name(&self) -> String;
    fn run(&mut self);
}

#[derive(PartialEq)]
enum Focus {
    Inner,
    Outer,
    Special,
}

pub enum SwitchDirection {
    Previous,
    Next,
    None,
}

pub struct ModuleRunner {
    focus: Focus, //inner vs outer
    btn_action: mpsc::Receiver<usize>,
    module_tx: mpsc::Sender<RemoteMessage>,
    module_rx: Option<mpsc::Receiver<RemoteMessage>>,
    state_tx: mpsc::Sender<DisplayMessage>,
    modules: Vec<Box<dyn RemoteModule + Send>>,
    module_started: bool,
    module_idx: usize,
    last_module_idx: usize,
    module_handle: Option<thread::JoinHandle<Box<dyn RemoteModule + Send>>>,
}

impl ModuleRunner {
    pub fn new(
        btn_channel: mpsc::Receiver<usize>,
        disp_tx: mpsc::Sender<DisplayMessage>,
        modules: Vec<Box<dyn RemoteModule + Send>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<RemoteMessage>();
        Self {
            focus: Focus::Outer,
            btn_action: btn_channel,
            modules,
            module_tx: tx,
            module_rx: Some(rx),
            state_tx: disp_tx,
            module_started: false,
            module_idx: 0,
            last_module_idx: 0,
            module_handle: None,
        }
    }

    //toggle ui state
    pub fn move_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Inner => Focus::Outer,
            Focus::Outer => Focus::Inner,
            _ => Focus::Special,
        }
    }

    pub fn switch_module(&mut self, dir: SwitchDirection) {
        let n_mod = self.modules.len();
        match dir {
            SwitchDirection::Previous => {
                if self.module_idx > 0 {
                    self.module_idx -= 1;
                }
            }
            SwitchDirection::Next => {
                if self.module_idx < n_mod - 1 {
                    self.module_idx += 1;
                }
            }
            SwitchDirection::None => (),
        };
    }

    fn check_buttons(&mut self) {
        //98.999% of the time the buttons wont be pressed, let it time out quick
        if let Ok(event) = self.btn_action.recv_timeout(Duration::from_millis(10)) {
            log::info!("Press Registered: {:}", event);
            if event == 1 {
                self.move_focus();
                if self.focus == Focus::Inner {
                    log::info!("inner")
                }
            }
            if event == 0 || event == 2 {
                if self.focus == Focus::Outer {
                    let dir = match event {
                        0 => SwitchDirection::Previous,
                        2 => SwitchDirection::Next,
                        _ => SwitchDirection::None,
                    };
                    //change modules
                    self.switch_module(dir);
                    log::info!("{:} {:}", self.module_idx, self.last_module_idx);
                } else {
                    //pass to module
                    let _ = self.module_tx.send(RemoteMessage {
                        status: event as u32,
                    });
                }
            } else {
                //pass to module
                let _ = self.module_tx.send(RemoteMessage {
                    status: event as u32,
                });
            }
        }
    }

    fn create_module_thread(&mut self) {
        ThreadSpawnConfiguration {
            name: Some("cur_module\0".as_bytes()),
            stack_size: 10000,
            priority: 16,
            ..Default::default()
        }
        .set()
        .unwrap();
        //will need to remove from vec, lets replace it with a dummy for now
        //let replaced_name = self.modules[self.module_idx].get_display_name();
        let mut module = replace(&mut self.modules[self.module_idx], dummy_module());
        log::info!("creating thread");
        self.module_handle = Some(
            thread::Builder::new()
                .stack_size(10000)
                .spawn(move || {
                    module.run();
                    module
                })
                .unwrap(),
        );
    }
}

pub fn runner_service(mr: &mut ModuleRunner) {
    loop {
        //check for any button events and respond
        mr.check_buttons();

        if !mr.module_started {
            log::info!("module not started, lets try to start it");
            //do we currently own the reciever in order to give it away
            if mr.module_rx.is_some() {
                //take runner's receiver
                let rx = replace(&mut mr.module_rx, None).unwrap();
                //give the receiver and sender to module that is being started
                mr.modules[mr.module_idx].set_channel(rx, mr.state_tx.clone());
                mr.module_rx = None; //is this redundant?
            }
            mr.create_module_thread();
            mr.module_started = true;
            mr.last_module_idx = mr.module_idx;
        }
        //running module but there's been a change
        else if mr.module_started && mr.module_idx != mr.last_module_idx {
            //send exit command
            let _ = mr.module_tx.send(RemoteMessage { status: 10 });
            //join thread that is returning, take will automatically
            //replace mr.module_handle with None
            mr.modules[mr.last_module_idx] = mr
                .module_handle
                .take()
                .map(|x| x.join())
                .unwrap()
                .expect("Returning Module failed to join.");
            mr.module_started = false;
            log::info!("module stopped");
            //release the channel's clone
            mr.module_rx = mr.modules[mr.last_module_idx].release_channel();
        } else {
            //log::info!("everything being skipped");
        }
    }
}
