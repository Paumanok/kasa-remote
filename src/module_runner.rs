use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use std::mem::replace;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use embedded_graphics::{
    geometry::{Point, Size},
    primitives:: Rectangle};
use crate::kasa_control;
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, TextSize};

/*
* What do we want this to do?
* outer UI
* keep track of inner/outer mode
* button action handling
* polling modules
* display updates
*
*/
#[derive(Clone)]
pub struct RemoteMessage {
    pub status: u32,
}

struct TestModule {
    member: u32,
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
}

impl RemoteModule for TestModule {
    fn set_channel(
        &mut self,
        receiver: mpsc::Receiver<RemoteMessage>,
        sender: mpsc::Sender<DisplayMessage>,
    ) {
        log::info!("setting channel");
        self.receiver = Some(receiver);
        self.sender = Some(sender);
    }

    fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>> {
        let rec = replace(&mut self.receiver, None);
        let _send = replace(&mut self.sender, None);
        rec

    }
    
    fn get_display_name(self) -> String {
        "Test".to_string()
    }

    fn run(&mut self) {
        self.member = 0;
        loop {
            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            log::info!("returning via command");
                            log::info!("member count up to: {:}", self.member);
                            return;
                        } else {
                            log::info!("got button event: {:}", msg.status);
                        }
                    }
                    _ => (),
                };
            } else {
                log::info!("no channel receiver configured");
                return;
            }
            self.member += 1;
            std::thread::sleep(std::time::Duration::from_millis(20));
            if let Some(tx) = &self.sender {
                let _ = tx.send(DisplayMessage {
                    lines: vec![DisplayLine {
                        line: format!("counter: {:}", self.member),
                        size: TextSize::Normal,
                        x_offset: 20,
                        y_offset: 20,
                    }],
                    status_line: false,
                    //clear_rect: Rectangle::new(Point::new(0, 15),Size::new(128,44)),
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        } //service loop
    }
}

fn dummy_module() -> Box<dyn RemoteModule + Send> {
    struct Dummy;
    impl RemoteModule for Dummy {
        fn set_channel(
            &mut self,
            _receiver: mpsc::Receiver<RemoteMessage>,
            _sender: mpsc::Sender<DisplayMessage>,
        ) {
        } //this wont be called
        fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>> {
            None
        }
        fn get_display_name(self) -> String { "dummy".to_string()}
        fn run(&mut self) {}
    }
    Box::new(Dummy)
}

pub trait RemoteModule {
    fn set_channel(
        &mut self,
        chnl: mpsc::Receiver<RemoteMessage>,
        sender: mpsc::Sender<DisplayMessage>,
    );
    fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>>;
    fn get_display_name(self) -> String;
    //TODO: Change these to set/release the shared resource
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
    pub fn new(btn_channel: mpsc::Receiver<usize>, disp_tx: mpsc::Sender<DisplayMessage>) -> Self {
        let (tx, rx) = mpsc::channel::<RemoteMessage>();
        Self {
            focus: Focus::Outer,
            btn_action: btn_channel,
            //modules: modules,
            modules: vec![
                Box::new(kasa_control::KasaControl::new()),
                Box::new(TestModule {
                    member: 0,
                    receiver: None,
                    sender: None,
                }),
            ],
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
                log::info!("previous");
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
        if let Ok(event) = self.btn_action.recv_timeout(Duration::from_millis(10)) {
                log::info!("btn press registered: {:}", event);
                if event == 1 {
                    self.move_focus();
                    if self.focus == Focus::Inner {
                        log::info!("inner")
                    }
                }
                if event == 0 || event == 2 {
                    log::info!("trying to switch module");
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
                    let _ = module.run();
                    return module;
                })
                .unwrap(),
        );
    }
}

pub fn runner_service(mr: &mut ModuleRunner) {
    //poll for button action channel
    log::info!("does this even start?");
    loop {
        //99.999% of the time the buttons wont be pressed, let it time out quick
        mr.check_buttons();

        if !mr.module_started {
            log::info!("module not started, lets try to start it");
            //module not started,
            //time to start the next one
            if mr.module_rx.is_some() {
                log::info!("is some");
                let rx = replace(&mut mr.module_rx, None).unwrap();
                mr.modules[mr.module_idx].set_channel(rx, mr.state_tx.clone());
                mr.module_rx = None;
            }
            mr.create_module_thread();
            mr.module_started = true;
            mr.last_module_idx = mr.module_idx;
        }
        //running module but there's been a change
        else if mr.module_started && mr.module_idx != mr.last_module_idx {
            //send exit command
            //join thread that is returning, take will automatically
            //replace mr.module_handle with None
            //std::thread::sleep(std::time::Duration::from_millis(5000));
            let _ = mr.module_tx.send(RemoteMessage { status: 10 });
            mr.modules[mr.last_module_idx] = mr
                .module_handle
                .take()
                .map(|x| x.join())
                .unwrap()
                .expect("failed to join mod thread");
            mr.module_started = false;
            //release the channel's clone
            log::info!("module stopped");
            mr.module_rx = mr.modules[mr.last_module_idx].release_channel();

        } else {
            //log::info!("everything being skipped");
        }
        //how do we determine start and stop of module
    }
}
