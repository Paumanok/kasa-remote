use anyhow::Result;
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, TextSize};
use embedded_graphics::{
    geometry::{Point, Size},
    primitives:: Rectangle};
use std::sync::mpsc;

use max170xx::Max17048;

pub struct BatteryMonitor{
    last_soc: i32,
    clear_rect: Rectangle,
}

impl BatteryMonitor {
    
    pub fn new() -> Self
    {
        BatteryMonitor {
            last_soc: 0,
            clear_rect: Rectangle::new(Point::new(100, 0),Size::new(30,10)), 
        }
    }

    pub fn battery_service<I2C>(
        &mut self,
        i2c:  I2C,
        disp_tx: mpsc::Sender<DisplayMessage>,
    ) -> Result<()> 
    where 
        I2C: embedded_hal::i2c::I2c,
    {

        let mut sensor = Max17048::new(i2c);

        loop {

            let soc = sensor.soc().unwrap() as i32;
            //log::info!("state of charge: {:}", soc);
            //log::info!("last: {:}", last);
            if self.last_soc != soc {
                let msg = DisplayMessage {
                    lines: vec![
                        DisplayLine {
                            line: { 
                                format!("{:}%", soc)
                            },
                            size: TextSize::Normal,
                            x_offset: 100,
                            y_offset: 0,
                        },
                    ],
                    status_line: true,
                    clear_rect: self.clear_rect.clone(),
                };

                let _ = disp_tx.send(msg);
                self.last_soc = soc;
            }
            std::thread::sleep(std::time::Duration::from_millis(30000));

        }
    }

}

