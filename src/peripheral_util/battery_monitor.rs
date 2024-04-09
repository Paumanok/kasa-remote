use crate::module_runner::{RemoteMessage, RemoteModule};
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, TextSize};
use esp_idf_svc::hal::i2c::{self, I2cDriver};
use std::mem::replace;
use std::sync::mpsc;

use max170xx::Max17048;

pub struct BatteryMonitor{
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
    //i2c: embedded_hal::i2c::I2c,
    soc: f32,
}

impl BatteryMonitor {
    
    pub fn new() -> BatteryMonitor 
    {
        BatteryMonitor {
            receiver: None,
            sender: None,
            soc: 0.0,
        }
    }

}

impl RemoteModule for BatteryMonitor {
     
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
        return "BatteryMonitor".to_string()
    }

    fn run(&mut self) {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            //let mut sensor = max170xx::Max17048::new(self.i2c);
            //let soc = sensor.soc().unwrap();
            //
            //let msg = DisplayMessage {
            //    lines: vec![
            //        DisplayLine {
            //            line: { 
            //                format!("Battery SOC {:}", soc)
            //            },
            //            size: TextSize::Normal,
            //            x_offset: 20,
            //            y_offset: 20,
            //        },
            //    ],
            //};

            //if let Some(tx) = &self.sender {
            //    let _ = tx.send(msg);
            //}

        };

    }
}
