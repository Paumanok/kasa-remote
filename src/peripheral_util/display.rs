use anyhow::Result;
use embedded_graphics::{
    mono_font::{ascii::FONT_5X8, ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_idf_svc::hal::i2c;
use sh1106::{displayrotation::DisplayRotation, prelude::*, Builder};
use std::sync::{Arc, Mutex};

use crate::peripheral_util::{Mode, RemoteState};


pub enum TextSize {
    Small,
    Normal,
}

pub struct DisplayLine {
    pub line: String,
    pub size: TextSize
}

pub struct DisplayMessage {
    pub lines: Vec<DisplayLine>,
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
            text_normal: text_normal,
            text_small: text_small,
        }
    }

    fn lazy_menu_setup(
        &mut self,
        cur_mode: Mode,
        drawing_mode: Mode,
    ) -> (i32, MonoTextStyle<'a, BinaryColor>) {
        if cur_mode == drawing_mode {
            return (4, self.text_normal);
        } else {
            return (2, self.text_small);
        }
    }

    pub fn display_service(
        &mut self,
        i2c: i2c::I2cDriver,
        rs: Arc<Mutex<RemoteState>>,
    ) -> Result<()> {
        println!("display_service hit");
        //this Builder is the specific SH1106 builder
        let mut display: GraphicsMode<I2cInterface<i2c::I2cDriver>> = Builder::new()
            .with_rotation(DisplayRotation::Rotate180)
            .connect_i2c(i2c)
            .into();

        display.init().unwrap();
        display.flush().unwrap();

        //let text_style = MonoTextStyleBuilder::new()
        //    .font(&FONT_6X10)
        //    .text_color(BinaryColor::On)
        //    .build();
        //let text_small = MonoTextStyleBuilder::new()
        //    .font(&FONT_5X8)
        //    .text_color(BinaryColor::On)
        //    .build();
        Text::with_baseline(
            "Hello world!",
            Point::zero(),
            self.text_normal,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();

        //let info = format!("ip: {:?}",(*_wifi).sta_netif().get_ip_info()?.ip);
        //
        //Text::with_baseline(&info, Point::new(0, 16), text_style, Baseline::Top)
        //    .draw(&mut display)
        //    .unwrap();

        display.flush().unwrap();

        loop {
            match rs.lock() {
                Ok(msg) => {
                    display.clear();
                    let modes = ["Monitor", "Totals", "Settings"];
                    let mode1 = "Monitor";
                    let mode2 = "Totals";
                    let mode3 = "Info";

                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Monitor);
                    Text::with_baseline(mode1, Point::new(0, y), text, Baseline::Top)
                        .draw(&mut display)
                        .unwrap();

                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Totals);
                    Text::with_baseline(
                        mode2,
                        Point::new((6 * mode1.len() as i32) + 6, y),
                        text,
                        Baseline::Top,
                    )
                    .draw(&mut display)
                    .unwrap();

                    let (y, text) = self.lazy_menu_setup(msg.mode, Mode::Info);
                    Text::with_baseline(
                        mode3,
                        Point::new((6 * mode1.len() + 6 * mode2.len()) as i32 + 12, y),
                        text,
                        Baseline::Top,
                    )
                    .draw(&mut display)
                    .unwrap();

                    //msg.monitor.update();
                    match msg.mode {
                        Mode::Monitor => {
                            if let Some(stats) = &msg.monitor.stats {
                                let ma = format!(
                                    "I:{:>4}mA   P: {:>4}mW\r\n\r\nPt: {:>3}Wh",
                                    stats.current_ma, stats.power_mw, stats.total_wh,
                                );
                                Text::with_baseline(
                                    &ma,
                                    Point::new(0, 18),
                                    self.text_normal,
                                    Baseline::Top,
                                )
                                .draw(&mut display)
                                .unwrap();

                                let mut outlet = String::from("");
                                for i in 1..7 {
                                    if msg.monitor.idx + 1 == i {
                                        outlet.push_str(format!(" {:?}", i).as_str());
                                    } else {
                                        outlet.push_str(" *");
                                    }
                                }
                                let outlet = outlet.as_str();

                                Text::with_baseline(
                                    outlet,
                                    Point::new(28, 57),
                                    self.text_small,
                                    Baseline::Top,
                                )
                                .draw(&mut display)
                                .unwrap();
                            };
                        }
                        Mode::Totals => {
                            if let Some(stats) = &msg.totals.stats {
                                let ma = format!(
                                    "I:{:>4}mA   P: {:>4}mW\r\n\r\nPt: {:>3}Wh",
                                    stats.current_ma, stats.power_mw, stats.total_wh,
                                );
                                Text::with_baseline(
                                    &ma,
                                    Point::new(0, 18),
                                    self.text_normal,
                                    Baseline::Top,
                                )
                                .draw(&mut display)
                                .unwrap();
                            } else {
                                println!("stats none in totals");
                            }
                        }
                        _ => (),
                    }

                    display.flush().unwrap();
                }
                _ => (),
            };
            //time for ~24fps
            std::thread::sleep(std::time::Duration::from_millis((1000 / 12) as u64));
        }
    }
}
