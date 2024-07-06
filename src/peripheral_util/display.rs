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

pub struct DisplayLine {
    pub line: String,
    pub size: TextSize,
    pub x_offset: i32,
    pub y_offset: i32,
}

pub struct DisplayMessage {
    pub lines: Vec<DisplayLine>,
    pub status_line: bool,
    pub clear_rect: Rectangle,
}

pub fn display_error(sender: mpsc::Sender<DisplayMessage>, error_msg: String) {
    
    let _ = sender.send(
        DisplayMessage {
            lines: vec![
                DisplayLine {
                    line: error_msg,
                    size: TextSize::Small,
                    x_offset: 20,
                    y_offset: 20,
                },
            ],
            status_line: false,
            clear_rect: Rectangle::new(Point::new(0,15), Size::new(128,44)),
        }
    );
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
                for line in msg.lines {
                    Text::with_baseline(
                        line.line.as_str(),
                        Point::new(line.x_offset, line.y_offset),
                        self.lazier_font_selector(msg.status_line),
                        Baseline::Top,
                    )
                    .draw(&mut display)
                    .unwrap();
                }

                display.flush().unwrap();
            }

            //time for ~24fps
            std::thread::sleep(std::time::Duration::from_millis((1000 / 12) as u64));
        }
    }
}
