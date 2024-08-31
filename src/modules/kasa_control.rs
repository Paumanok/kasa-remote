use crate::module_runner::{RemoteMessage, RemoteModule};
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, MessageType, TextSize};
use crate::CONFIG;
use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};
use rust_kasa::{kasa_protocol, models::Realtime};
use std::mem::replace;
use std::net::TcpStream;
use std::sync::mpsc;

enum BoolDir {
    Next,
    Prev,
}

pub struct KasaControl {
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    sender: Option<mpsc::Sender<DisplayMessage>>,
    stats: Vec<Realtime>,
    monitor_idx: usize,
    update: bool,
}

impl KasaControl {
    pub fn new() -> Self {
        Self {
            receiver: None,
            sender: None,
            stats: vec![
                Realtime {
                    current_ma: 0,
                    err_code: 0,
                    power_mw: 0,
                    slot_id: 0,
                    total_wh: 0,
                    voltage_mv: 0,
                };
                7
            ],
            monitor_idx: 0,
            update: true,
        }
    }
    pub fn get_target_stat(idx: u8) -> Option<Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        Some(kasa_protocol::get_realtime_by_idx(&mut stream, idx.into()).ok()?)
    }

    pub fn get_all_stats() -> Option<Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        let stats_vec = kasa_protocol::get_all_realtime(&mut stream).ok()?;
        Some(Realtime {
            current_ma: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.current_ma),
            err_code: 0,
            power_mw: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.power_mw),
            slot_id: 0,
            total_wh: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.total_wh),
            voltage_mv: (stats_vec.iter().fold(0u32, |sum, rt| sum + rt.voltage_mv))
                / stats_vec.len() as u32,
        })
    }

    fn display_line_builder(&mut self) -> DisplayMessage {
        DisplayMessage {
            module_name: self.get_display_name(),
            content: MessageType::Lines(vec![
                DisplayLine {
                    //line: "line 1".to_string(),
                    line: {
                        let cur_stats = &self.stats[self.monitor_idx];
                        format!(
                            "I:{:>4}mA   P: {:>4}mW\r\n\r\nPt: {:>3}Wh",
                            cur_stats.current_ma, cur_stats.power_mw, cur_stats.total_wh,
                        )
                    },
                    size: TextSize::Normal,
                    x_offset: 0,
                    y_offset: 18,
                },
                DisplayLine {
                    //line: "line 2".to_string(),
                    line: {
                        let mut outlet = String::from("");
                        for i in 1..7 {
                            if self.monitor_idx + 1 == i {
                                outlet.push_str(format!(" {:?}", i).as_str());
                            } else {
                                outlet.push_str(" *");
                            }
                        }
                        outlet
                    },
                    size: TextSize::Small,
                    x_offset: 28,
                    y_offset: 50,
                },
            ]),
            status_line: false,
            clear_rect: Rectangle::new(Point::new(0, 15), Size::new(128, 44)),
        }
    }

    fn toggle_by_idx(btn_idx: u32) {
        let app_config = CONFIG;
        if btn_idx > 2 && btn_idx < 9 {
            if let Ok(mut stream) = TcpStream::connect(format!("{:}:9999", app_config.target_ip)) {
                let _res = kasa_protocol::toggle_relay_by_idx(&mut stream, (btn_idx - 3) as usize);
            }
        }
    }

    fn update_idx(&mut self, d: BoolDir) {
        match d {
            BoolDir::Next => self.monitor_idx += 1,
            BoolDir::Prev => self.monitor_idx -= 1,
        };
        self.update = true;
    }
}

impl RemoteModule for KasaControl {
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
        return "Kasa".to_string();
    }

    fn run(&mut self) {
        let mut poll_counter: usize = 0;
        //redraw the display at load
        self.update = true;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            poll_counter += 1;
            if poll_counter == 100 {
                //every 5 seconds with 100mili loop delay unless toggle takes time
                poll_counter = 0;
                if let Some(stat) = KasaControl::get_target_stat(self.monitor_idx as u8) {
                    self.stats[self.monitor_idx] = stat;
                    self.update = true;
                }
            }
            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            self.update = true;
                            log::info!("kc exiting");
                            return;
                        } else if msg.status == 0 && self.monitor_idx > 0 {
                            self.update_idx(BoolDir::Prev);
                            log::info!("{:}", self.monitor_idx);
                        } else if msg.status == 2 && self.monitor_idx < 7 {
                            self.update_idx(BoolDir::Next);
                            log::info!("{:}", self.monitor_idx);
                        } else {
                            KasaControl::toggle_by_idx(msg.status);
                        }
                    }
                    _ => (),
                }
                if self.update {
                    let msg = self.display_line_builder();
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
