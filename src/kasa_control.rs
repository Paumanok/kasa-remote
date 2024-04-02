use crate::module_runner::{RemoteMessage, RemoteModule};
use crate::peripheral_util::display::{DisplayLine, DisplayMessage, TextSize};
use crate::CONFIG;
use rust_kasa::kasa_protocol;
use std::mem::replace;
use std::net::TcpStream;
use std::sync::mpsc;

pub struct KasaControl {
    pub receiver: Option<mpsc::Receiver<RemoteMessage>>,
    pub sender: Option<mpsc::Sender<DisplayMessage>>,
    pub stats: Vec<kasa_protocol::Realtime>,
    pub monitor_idx: usize,
    //we need some actual data state keeping similar to the monitor and totals
    //perhaps a mutable vec where we can update one at a time or
    //update the entire vec
}

impl KasaControl {
    pub fn get_target_stat(idx: u8) -> Option<kasa_protocol::Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        kasa_protocol::get_realtime_by_idx(&mut stream, idx.into())
    }

    pub fn get_all_stats() -> Option<kasa_protocol::Realtime> {
        let app_config = CONFIG;
        let mut stream = TcpStream::connect(format!("{:}:9999", app_config.target_ip)).ok()?;
        let stats_vec = kasa_protocol::get_all_realtime(&mut stream)?;
        Some(kasa_protocol::Realtime {
            current_ma: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.current_ma),
            err_code: 0,
            power_mw: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.power_mw),
            slot_id: 0,
            total_wh: stats_vec.iter().fold(0u32, |sum, rt| sum + rt.total_wh),
            voltage_mv: (stats_vec.iter().fold(0u32, |sum, rt| sum + rt.voltage_mv))
                / stats_vec.len() as u32,
        })
    }

    fn display_line_builder() -> DisplayMessage {
        DisplayMessage {
            lines: vec![
                DisplayLine {
                    line: "line 1".to_string(),
                    size: TextSize::Normal,
                    x_offset: 0,
                    y_offset: 18,
                },
                DisplayLine {
                    line: "line 2".to_string(),
                    size: TextSize::Small,
                    x_offset: 28,
                    y_offset: 40,
                },
            ],
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

    fn run(&mut self) {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));

            if let Some(rx) = &self.receiver {
                match rx.try_recv() {
                    Ok(msg) => {
                        if msg.status == 10 {
                            return;
                        } else {
                            KasaControl::toggle_by_idx(msg.status);
                        }
                    }
                    _ => (),
                }
                if let Some(tx) = &self.sender {
                    let _ = tx.send(KasaControl::display_line_builder());
                }
            }
        }
    }
}
