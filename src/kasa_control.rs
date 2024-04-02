use std::net::TcpStream;
use crate::module_runner::{ RemoteModule, RemoteMessage };
use std::mem::replace;
use crate::peripheral_util::display::{ DisplayMessage, DisplayLine, TextSize };
use std::sync::mpsc;
use rust_kasa::kasa_protocol;



#[toml_cfg::toml_config]
pub struct Config {
    #[default("blah")]
    wifi_ssid: &'static str,
    #[default("blah")]
    wifi_psk: &'static str,
    #[default("127.0.0.1")]
    target_ip: &'static str,
}


struct KasaControl {
    receiver: Option<mpsc::Receiver<RemoteMessage>>,
    stats: Vec<kasa_protocol::Realtime>,
    monitor_idx: usize,
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
                },
                DisplayLine {
                    line: "line 2".to_string(),
                    size: TextSize::Small,
                },
            ]
        }
    }

}

impl RemoteModule for KasaControl {

    fn set_channel(&mut self, chnl: mpsc::Receiver<RemoteMessage>) {
        log::info!("setting channel");
        self.receiver = Some(chnl);
    }

    fn release_channel(&mut self) -> Option<mpsc::Receiver<RemoteMessage>> {
        let channel = replace(&mut self.receiver, None);
        return channel;
    }

    fn run(&mut self) {
        
    }

}
