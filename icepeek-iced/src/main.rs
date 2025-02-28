mod ui;

use crate::ui::{AppSettings, IcedApp};
use iced::{Application, Settings};
use std::env;

#[tokio::main]
async fn main() {
    println!("IcePeek UI Iced");
    println!("parameters: [--testnet|--signet]");

    let mut app_settings = AppSettings::default();
    let args: Vec<String> = env::args().collect();
    for a in &args {
        if a == "--testnet" {
            app_settings.network = "Testnet";
        } else if a == "--signet" {
            app_settings.network = "Signet";
        }
    }

    let settings = Settings::with_flags(app_settings);
    let _res = IcedApp::run(settings);
}
