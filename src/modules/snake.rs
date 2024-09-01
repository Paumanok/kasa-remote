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

use esp_idf_svc::hal::sys::rand;

//each segment of the snake will be n x n
const SEGMENT_SIZE: u32 = 5;
const STEP_SIZE: i32 = SEGMENT_SIZE as i32;
const X_MAX: i32 = 128 - STEP_SIZE;
const X_MIN: i32 = 0;
const Y_MAX: i32 = 64 - STEP_SIZE;
const Y_MIN: i32 = 10; // status bar is 10 px tall

#[derive(Copy, Clone, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub struct Player {
    head: Point,
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

fn check_segment_intersection(a: &Point, b: &Point) -> bool {
    //axis aligned bounding box
    if a.x < b.x + STEP_SIZE
        && a.x + STEP_SIZE > b.x
        && a.y < b.y + STEP_SIZE
        && a.y + STEP_SIZE > b.y
    {
        return true;
    }
    return false;
}

fn get_random_point() -> Point {
    //display area is 129x64
    //this could use some work properly defining where a new point can be
    //this works but only barely
    let randint: i32;
    let modulo = 118 * (64 - (STEP_SIZE * 2));
    //yucky yucky but I don't need to bring in the PAC crate
    unsafe {
        randint = rand();
    }
    let wrapped_coor = randint % modulo;
    //add 10 to y to account for status
    let y = (wrapped_coor / 118) + 10;
    let x = wrapped_coor % 118;
    Point::new(x, y)
}

impl Snake {
    pub fn new() -> Self {
        let head_start = get_random_point();
        Self {
            receiver: None,
            sender: None,
            update: true,
            player: Player {
                head: head_start,
                size: Size::new(SEGMENT_SIZE, SEGMENT_SIZE),
                direction: Direction::Left,
                segments: vec![head_start],
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

    fn restart_game(&mut self) {
        let head_start = get_random_point();
        self.player.head = head_start;
        self.player.segments = vec![head_start];
        self.board.food = None;
        self.player.score = 0;
    }

    fn step_segments(&mut self, grow: bool) {
        self.player.segments.insert(0, self.player.head.clone());

        if self.player.segments.len() > 1 && !grow {
            self.player.segments.pop();
        }
    }

    fn check_self_intersection(&mut self) -> bool {
        if self.player.segments.len() >= 3 {
            for segment in &self.player.segments[2..] {
                if check_segment_intersection(&self.player.head, segment) {
                    println!("head: {:?} segment: {:?}", self.player.head, segment);
                    return true;
                }
            }
        }
        return false;
    }

    fn spawn_food(&mut self) {
        let new_food = get_random_point();
        println!("new food: {:}, {:}", new_food.x, new_food.y);
        self.board.food = Some(get_random_point());
    }

    fn check_food_capture(&mut self) -> bool {
        if let Some(food) = self.board.food {
            return check_segment_intersection(&food, &self.player.head);
        }
        return false;
    }

    fn step(&mut self) {
        if !self.paused {
            let mut grow = false;
            //println!("{:?}", self.player.head);
            if self.board.food == None {
                println!("making food");
                self.spawn_food();
            }

            if self.check_food_capture() {
                println!("food gained!");
                self.player.score += 1;
                grow = true;
                self.board.food = None;
                self.spawn_food();
            }

            if self.check_self_intersection() {
                self.restart_game();
                println!("intersect!");
            }

            //adjust the head's location

            match self.player.direction {
                Direction::Left => {
                    match self.player.head.x {
                        x if x > X_MIN => self.player.head.x -= STEP_SIZE,
                        x if x <= X_MIN => self.player.head.x = X_MAX,
                        _ => (),
                    };
                }
                Direction::Right => {
                    match self.player.head.x {
                        x if x < X_MAX => self.player.head.x += STEP_SIZE,
                        x if x >= X_MAX => self.player.head.x = X_MIN,
                        _ => (),
                    };
                }
                Direction::Up => match self.player.head.y {
                    y if y > Y_MIN => self.player.head.y -= STEP_SIZE,
                    y if y <= Y_MIN => self.player.head.y = Y_MAX,
                    _ => println!("stuck up"),
                },
                Direction::Down => match self.player.head.y {
                    y if y < Y_MAX => self.player.head.y += STEP_SIZE,
                    y if y >= Y_MAX => self.player.head.y = Y_MIN,
                    _ => println!("stuck down: {:}", self.player.head.y),
                },
            };
            //move the segments along with the new head
            self.step_segments(grow);
        }
    }

    fn handle_control_event(&mut self, msg: u32) {
        if msg == 3 {
            self.paused = !self.paused;
        }
        let last_dir = self.player.direction;
        let new_dir = match msg {
            4 => Direction::Up,
            6 => Direction::Left,
            7 => Direction::Down,
            8 => Direction::Right,
            _ => last_dir,
        };
        //self.player.direction = new_dir;
        //dont let the player reverse
        self.player.direction = match new_dir {
            Direction::Up if last_dir != Direction::Down => Direction::Up,
            Direction::Down if last_dir != Direction::Up => Direction::Down,
            Direction::Left if last_dir != Direction::Right => Direction::Left,
            Direction::Right if last_dir != Direction::Left => Direction::Right,
            _ => last_dir,
        }
    }

    fn display_board(&mut self) -> DisplayMessage {
        let size = self.player.size.clone();
        let mut board_buffer: Vec<DisplayBuffer> = vec![];

        // push the player segments in first, this gets squirrely with the iterator and appending
        board_buffer.extend(self.player.segments.iter().map(|s| DisplayBuffer {
            buf: [BinaryColor::On; 400].to_vec(),
            offset: *s,
            size,
        }));

        //push the food in if it exists
        if let Some(food) = self.board.food {
            let food_buffer = DisplayBuffer {
                buf: [BinaryColor::On; 400].to_vec(),
                offset: food,
                size,
            };
            board_buffer.push(food_buffer);
        }

        DisplayMessage {
            module_name: self.get_display_name(),
            content: MessageType::Buffer(board_buffer),
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

    fn clear_score(&mut self) {
        if let Some(tx) = &self.sender {
            //log::info!("is this sending");
            let _ = tx.send(DisplayMessage {
                module_name: self.get_display_name(),
                content: MessageType::Lines(vec![DisplayLine {
                    line: "".to_string(),
                    size: TextSize::Normal,
                    x_offset: 10,
                    y_offset: 0,
                }]),
                status_line: true,
                clear_rect: self.score_rect,
            });
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
            std::thread::sleep(std::time::Duration::from_millis((1000 / 48) as u64));
            poll_counter += 1;
            if poll_counter == 8 {
                //about every 170ms, timing is a little loose
                poll_counter = 0;
                self.update = true;
                self.step();
            }

            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            //self.update = true;
                            log::info!("snake exiting");
                            self.paused = true;
                            self.clear_score();
                            return;
                        } else {
                            self.handle_control_event(msg.status)
                        }
                    }
                    _ => (),
                }

                if self.update {
                    let msgs = vec![self.display_board(), self.display_score()];
                    if let Some(tx) = &self.sender {
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
