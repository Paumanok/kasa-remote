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

#[derive(Copy, Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub struct Player {
    start: Point,
    size: Size,
    direction: Direction,
    segments: Vec<Point>,
    score: u32,
}

pub struct Board {
    food: Option<Point>,
    board_rect: Rectangle,
}

pub struct Snake {
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
    update: bool,

    player: Player,
    board: Board,
    paused: bool,
    score_rect: Rectangle,
}

impl Snake {
    pub fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            update: true,
            player: Player {
                start: Point::new(20, 15),
                size: Size::new(5, 5),
                direction: Direction::Left,
                segments: vec![Point::new(20, 15)],
                score: 0,
            },
            board: Board {
                food: None,
                board_rect: Rectangle::new(Point::new(0, 10), Size::new(128, 54)),
            },
            paused: false,
            score_rect: Rectangle::new(Point::new(10, 0), Size::new(90, 10)),
        }
    }

    fn step(&mut self) {
        if self.board.food == None {}
        println!("{:},{:}", self.player.start.x, self.player.start.y);
        if !self.paused {
            match self.player.direction {
                Direction::Left => {
                    match self.player.start.x {
                        x if x > 0 => {
                            self.player.start.x -= 1;
                        }
                        //x if x <= 0 => {
                        //    self.player.start.x += 1;
                        //    self.player.direction = Direction::Right;
                        //}
                        _ => (),
                    };
                }
                Direction::Right => {
                    match self.player.start.x {
                        x if x < 108 => {
                            self.player.start.x += 1;
                        }
                        //x if x >= 108 => {
                        //    self.player.start.x -= 1;
                        //    self.player.direction = Direction::Left;
                        //}
                        _ => (),
                    };
                }
                Direction::Up => match self.player.start.y {
                    y if y > 10 => self.player.start.y -= 1,
                    _ => println!("stuck up"),
                },
                Direction::Down => match self.player.start.y {
                    y if y < 57 => self.player.start.y += 1,
                    _ => println!("stuck down: {:}", self.player.start.y),
                },
            };
        }
    }

    fn handle_control_event(&mut self, msg: u32) {
        //    3 => LocalButton::Pause,
        //    4 => LocalButton::Up,
        //    6 => LocalButton::Left,
        //    7 => LocalButton::Down,
        //    8 => LocalButton::Right,
        //    _ => LocalButton::Undef,

        if msg == 3 {
            self.paused = !self.paused;
        }
        let last_dir = self.player.direction;
        self.player.direction = match msg {
            4 => Direction::Up,
            6 => Direction::Left,
            7 => Direction::Down,
            8 => Direction::Right,
            _ => last_dir,
        };
    }

    fn display_buffer_builder(&mut self) -> DisplayMessage {
        let start = self.player.start; //Point::new(20, 15);
        let size = self.player.size; //Size::new(20, 20);
        DisplayMessage {
            module_name: self.get_display_name(),
            content: MessageType::Buffer(vec![DisplayBuffer {
                buf: [BinaryColor::On; 400].to_vec(),
                offset: start.clone(),
                size: size.clone(),
            }]),
            status_line: false,
            clear_rect: self.board.board_rect,
        }
    }

    fn display_score(&mut self) -> DisplayMessage {
        DisplayMessage {
            module_name: self.get_display_name(),
            content: MessageType::Lines(vec![DisplayLine {
                line: { format!("Score: {:}", self.player.score) },
                size: TextSize::Normal,
                x_offset: 10,
                y_offset: 0,
            }]),
            status_line: true,
            clear_rect: self.score_rect,
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

    fn get_display_name(&self) -> String {
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
                    let msgs = vec![self.display_buffer_builder(), self.display_score()];
                    if let Some(tx) = &self.sender {
                        //log::info!("is this sending");
                        for msg in msgs {
                            let _ = tx.send(msg);
                        }
                    }
                    self.update = false;
                }
            }
        }
    }
}
