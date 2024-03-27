use std::sync::{mpsc};
use std::time::Duration;
use std::mem::replace;
use std::thread;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
//use crate::peripheral_util::{buttons};

/*
* What do we want this to do?
* outer UI
* keep track of inner/outer mode
* button action handling
* polling modules
* display updates
*
*/
pub struct RemoteMessage {
    status: u32,
}

struct TestModule {
    member: u32,
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
}

impl RemoteModule for TestModule {
    fn set_channel(&mut self, chnl: mpsc::Receiver<RemoteMessage>) {
        log::info!("setting channel");
        self.receiver = Some(chnl);
    }

    fn release_channel(&mut self) {
        self.receiver = None;
    }

    fn run(&mut self) {
        loop {
            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            log::info!("returning via command");
                            log::info!("member count up to: {:}", self.member);
                            return
                        }
                    }
                    _ => (),
                };
            } else {
                log::info!("no channel receiver configured");
                return
            }
            self.member += 1;
            std::thread::sleep(std::time::Duration::from_millis(20));
        } //service loop
    }
}

fn dummy_module() -> Box<dyn RemoteModule + Send> {
    struct Dummy;
    impl RemoteModule for Dummy {
        fn set_channel(&mut self, chnl: mpsc::Receiver<RemoteMessage>) {} //this wont be called
        fn release_channel(&mut self) {}
        fn run(&mut self) { }
    }
    Box::new(Dummy)
}

pub trait RemoteModule {
    fn set_channel(&mut self, chnl: mpsc::Receiver<RemoteMessage>);
    fn release_channel(&mut self);
    fn run(&mut self);
}

#[derive(PartialEq)]
enum Focus {
    Inner,
    Outer,
    Special,
}

pub struct ModuleRunner {
    focus: Focus, //inner vs outer
    btn_action: mpsc::Receiver<usize>,
    module_tx: mpsc::Sender<RemoteMessage>,
    module_rx: mpsc::Receiver<RemoteMessage>,
    modules: Vec<Box<dyn RemoteModule + Send>>,
    module_started: bool,
    module_idx: usize,
    last_module_idx: usize,
    module_handle: Option<thread::JoinHandle<Box<dyn RemoteModule + Send>>>,

}
//common traits needed
//update? or should there be a service
//all modules need to take a reciever
//receiver should pass a varied amount of info
//need
impl ModuleRunner {
    pub fn new(btn_channel: mpsc::Receiver<usize>) -> Self {
        let (tx, rx) = mpsc::channel::<RemoteMessage>();
        Self {
            focus: Focus::Outer,
            btn_action: btn_channel,
            modules: vec![ ],
            module_tx: tx,
            module_rx: rx,
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

    fn check_buttons(&mut self) {
        match self.btn_action.recv_timeout(Duration::from_millis(10)) {
            Ok(event) => {
                if event == 1 {
                    self.move_focus();
                }
                if event == 0 | 2 {
                    if self.focus == Focus::Outer {
                        //change modules
                    } else {
                        //pass to module
                    }
                } else {
                    //pass to module
                }
            }
            _ => (), //dont care about timeouts
        };

    }

    fn create_module_thread(&mut self) {
        
        ThreadSpawnConfiguration {
            name: Some("cur_module\0".as_bytes()),
            stack_size: 10000,
            priority: 15,
            ..Default::default()
        }
        .set()
        .unwrap();
        //will need to remove from vec, lets replace it with a dummy for now
        let mut module = replace(&mut self.modules[self.module_idx], dummy_module());

        self.module_handle = Some(thread::Builder::new().stack_size(10000).spawn(move || {
            let _ = module.run();
            return module
        }).unwrap());
    }
}

pub fn runner_service(mr: &mut ModuleRunner) {
    //poll for button action channel

    loop {
        //99.999% of the time the buttons wont be pressed, let it time out quick
        mr.check_buttons();

        if !mr.module_started {
            //module not started,
            //time to start the next one
            mr.create_module_thread();

        }
        //running module but there's been a change
        else if mr.module_started && mr.module_idx != mr.last_module_idx {
            //send exit command
            let _ = mr.module_tx.send(RemoteMessage{status: 10});
            //join thread that is returning, take will automatically
            //replace mr.module_handle with None
            mr.modules[mr.last_module_idx] = mr.module_handle.take().map(|x| x.join()).unwrap().expect("failed to join mod thread");

            //release the channel's clone
            mr.modules[mr.last_module_idx].release_channel();

            //start new module's thread
            mr.create_module_thread();
        }
        //how do we determine start and stop of module 
    }
}
