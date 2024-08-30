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

pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// LocalButtons
/// enum to map the numerical button events
/// to logical controls for the snake game
enum LocalButton {
    Up = 4,
    Left = 6,
    Down = 7,
    Right = 8,
    Pause = 3,
    Undef,
}

pub struct Player {
    start: Point,
    size: Size,
    direction: Direction,
}

pub struct Snake {
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
    update: bool,

    player: Player,
}

impl Snake {
    pub fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            update: true,
            player: Player {
                start: Point::new(20, 15),
                size: Size::new(20, 20),
                direction: Direction::Left,
            },
        }
    }

    fn step(&mut self) {
        match self.player.direction {
            Direction::Left => {
                match self.player.start.x {
                    x if x > 0 => {
                        self.player.start.x -= 1;
                    }
                    x if x <= 0 => {
                        self.player.start.x += 1;
                        self.player.direction = Direction::Right;
                    }
                    _ => (),
                };
            }
            Direction::Right => {
                match self.player.start.x {
                    x if x < 108 => {
                        self.player.start.x += 1;
                    }
                    x if x >= 108 => {
                        self.player.start.x -= 1;
                        self.player.direction = Direction::Left;
                    }
                    _ => (),
                };
            }
            _ => (),
        };
    }

    fn handle_control_event(&mut self, msg: u32) {
        let button = match msg {
            3 => LocalButton::Pause,
            4 => LocalButton::Up,
            6 => LocalButton::Left,
            7 => LocalButton::Down,
            8 => LocalButton::Right,
            _ => LocalButton::Undef,
        };
    }

    fn display_buffer_builder(&mut self) -> DisplayMessage {
        let start = self.player.start; //Point::new(20, 15);
        let size = self.player.size; //Size::new(20, 20);
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
            // 24fps
            std::thread::sleep(std::time::Duration::from_millis((1000 / 48) as u64));
            poll_counter += 1;
            if poll_counter == 4 {
                //every 5 seconds with 100mili loop delay unless toggle takes time
                poll_counter = 0;
                self.update = true;
            }

            self.step();

            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            self.update = true;
                            log::info!("kc exiting");
                            return;
                        } else {
                            self.handle_control_event(msg.status)
                        }
                    }
                    _ => (),
                }

                if self.update {
                    let msg = self.display_buffer_builder();
                    if let Some(tx) = &self.sender {
                        //log::info!("is this sending");
                        let _ = tx.send(msg);
                    }
                    self.update = false;
                }
            }
        }
    }
}
