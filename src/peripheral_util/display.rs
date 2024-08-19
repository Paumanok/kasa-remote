use anyhow::Result;
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text},
};
use sh1106::{displayrotation::DisplayRotation, prelude::*, Builder};
use std::sync::mpsc;

pub enum TextSize {
    Small,
    Normal,
}
/// Display a line of text.
pub struct DisplayLine {
    /// The String to be displayed.
    pub line: String,
    /// TextSize enum, Small/Normal
    pub size: TextSize,
    /// X offset of text start
    pub x_offset: i32,
    /// Y offset of text start
    pub y_offset: i32,
}

// two byte generic coordinate
pub struct XY {
    pub x: u8,
    pub y: u8,
}

/// Display a rectangular chunk of a buffer
pub struct DisplayBuffer {
    /// 2d array buf binaryColors that should wrap in the rectangle
    pub buf: Vec<BinaryColor>,
    /// Rectangle size
    pub size: Size,
    /// Upper left start of Rectangle
    pub offset: Point,
}

/// MessageType Enum
/// There's two types of display messages
/// First being Lines, a vector of DisplayLines that render in
///     a single screen refresh. This will overwrite previous
///     lines of text if offset isn't adjusted.
/// Second, Buffer, a vector of DisplayBuffer rectangles that
///     render in a single screen refresh. This will overwrite.
pub enum MessageType {
    Lines(Vec<DisplayLine>),
    Buffer(Vec<DisplayBuffer>),
}

/// DisplayMessage
/// The standard message format for writing to the display via
/// the display service
pub struct DisplayMessage {
    //pub lines: Vec<DisplayLine>,
    /// Message content
    pub content: MessageType,
    /// Draw to upper displayline for long term text that
    /// wont need to be re-written every frame.
    pub status_line: bool,
    /// Area of message to clear before writing
    pub clear_rect: Rectangle,
}

pub fn display_error(sender: mpsc::Sender<DisplayMessage>, error_msg: String) {
    let _ = sender.send(DisplayMessage {
        content: MessageType::Lines(vec![DisplayLine {
            line: error_msg,
            size: TextSize::Small,
            x_offset: 20,
            y_offset: 20,
        }]),
        status_line: false,
        clear_rect: Rectangle::new(Point::new(0, 15), Size::new(128, 44)),
    });
}

pub struct Display<'a> {
    text_normal: MonoTextStyle<'a, BinaryColor>,
    text_small: MonoTextStyle<'a, BinaryColor>,
}

impl<'a> Display<'a> {
    pub fn new() -> Self {
        let text_normal = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        let text_small = MonoTextStyleBuilder::new()
            .font(&FONT_5X8)
            .text_color(BinaryColor::On)
            .build();

        Self {
            text_normal,
            text_small,
        }
    }

    fn lazier_font_selector(&mut self, small: bool) -> MonoTextStyle<'a, BinaryColor> {
        if small {
            self.text_small
        } else {
            self.text_normal
        }
    }

    pub fn display_service<I2C>(
        &mut self,
        i2c: I2C,
        recv: mpsc::Receiver<DisplayMessage>,
    ) -> Result<()>
    where
        I2C: embedded_hal::i2c::I2c,
    {
        println!("display_service hit");
        //this Builder is the specific SH1106 builder
        let mut display: GraphicsMode<I2cInterface<_>> = Builder::new()
            .with_rotation(DisplayRotation::Rotate180)
            .connect_i2c(i2c)
            .into();

        display.init().unwrap();
        display.flush().unwrap();

        Text::with_baseline(
            "Hello world!",
            Point::zero(),
            self.text_normal,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();

        display.flush().unwrap();
        display.clear();
        loop {
            if let Ok(msg) = recv.try_recv() {
                //clear part of display writer is tells us its using
                let _ = display.fill_solid(&msg.clear_rect, BinaryColor::Off);
                //render what was received
                match msg.content {
                    MessageType::Lines(lines) => {
                        for line in lines {
                            Text::with_baseline(
                                line.line.as_str(),
                                Point::new(line.x_offset, line.y_offset),
                                self.lazier_font_selector(msg.status_line),
                                Baseline::Top,
                            )
                            .draw(&mut display)
                            .unwrap();
                        }
                    }
                    MessageType::Buffer(bufs) => {
                        for buf in bufs {
                            display
                                .fill_contiguous(&Rectangle::new(buf.offset, buf.size), buf.buf)
                                .unwrap();
                        }
                    }
                };

                display.flush().unwrap();
            }

            //time for ~24fps
            std::thread::sleep(std::time::Duration::from_millis((1000 / 12) as u64));
        }
    }
}
