use crate::module_runner::{RemoteMessage, RemoteModule};
use crate::peripheral_util::display::{
    DisplayBuffer, DisplayLine, DisplayMessage, MessageType, TextSize,
};
use embedded_graphics::pixelcolor::BinaryColor;
//use crate::CONFIG;
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};
use std::mem::replace;
use std::sync::mpsc;

pub struct Snake {
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
    update: bool,
}

impl Snake {
    pub fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            update: true,
        }
    }

    fn display_buffer_builder(&mut self) -> DisplayMessage {
        let start = Point::new(20, 15);
        let size = Size::new(20, 20);
        DisplayMessage {
            content: MessageType::Buffer(vec![DisplayBuffer {
                buf: [BinaryColor::On; 400].to_vec(),
                offset: start.clone(),
                size: size.clone(),
            }]),
            status_line: false,
            clear_rect: Rectangle::new(Point::new(0, 15), Size::new(128, 44)),
        }
    }
}

impl RemoteModule for Snake {
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
        return rec;
    }

    fn get_display_name(self) -> String {
        return "snake".to_string();
    }

    fn run(&mut self) {
        let mut poll_counter: usize = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            if poll_counter == 100 {
                //every 5 seconds with 100mili loop delay unless toggle takes time
                poll_counter = 0;
                self.update = true;
            }
            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            self.update = true;
                            log::info!("kc exiting");
                            return;
                        }
                    }
                    _ => (),
                }
                if self.update {
                    let msg = self.display_buffer_builder();
                    if let Some(tx) = &self.sender {
                        log::info!("is this sending");
                        let _ = tx.send(msg);
                    }
                    self.update = false;
                }
            }
        }
    }
}
