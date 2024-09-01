use crate::module_runner::{RemoteMessage, RemoteModule};
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, MessageType, TextSize};

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};
use std::mem::replace;
use std::sync::mpsc;

pub struct TestModule {
    member: u32,
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
}

impl TestModule {
    pub fn new() -> Self {
        Self {
            member: 0,
            receiver: None,
            sender: None,
        }
    }
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

    fn get_display_name(&self) -> String {
        "Test".to_string()
    }

    fn run(&mut self) {
        self.member = 0;
        loop {
            if let Some(rx) = &self.receiver {
                if let Ok(msg) = rx.try_recv() {
                    if msg.status == 10 {
                        log::info!("returning via command");
                        log::info!("member count up to: {:}", self.member);
                        return;
                    } else {
                        log::info!("got button event: {:}", msg.status);
                    }
                }
            } else {
                log::info!("no channel receiver configured");
                return;
            }
            self.member += 1;
            std::thread::sleep(std::time::Duration::from_millis(20));
            if let Some(tx) = &self.sender {
                let _ = tx.send(DisplayMessage {
                    module_name: self.get_display_name(),
                    content: MessageType::Lines(vec![DisplayLine {
                        line: format!("counter: {:}", self.member),
                        size: TextSize::Normal,
                        x_offset: 20,
                        y_offset: 20,
                    }]),
                    status_line: false,
                    clear_rect: Rectangle::new(Point::new(0, 15), Size::new(128, 44)),
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        } //service loop
    }
}
