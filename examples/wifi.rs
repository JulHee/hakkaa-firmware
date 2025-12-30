//! WiFi sniffer example taken from [esp-hal](https://github.com/esp-rs/esp-hal) sniffer example

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use embassy_executor::Spawner;
use esp_backtrace as _;

use hakkaa::{board::Board, led::Storeys};

use alloc::string::ToString;
use core::cell::RefCell;

use critical_section::Mutex;
use embassy_time::{Duration, Timer};

use esp_radio::wifi::{self, Config};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

esp_bootloader_esp_idf::esp_app_desc!();

async fn delay(duration: Duration) {
    Timer::after(duration).await;
}

fn rssi_to_leds(rssi: i32) -> u8 {
    // TODO: Find better defaults
    match rssi {
        x if x >= -45 => 8,
        x if x >= -50 => 7,
        x if x >= -55 => 6,
        x if x >= -60 => 5,
        x if x >= -65 => 4,
        x if x >= -70 => 3,
        x if x >= -75 => 2,
        x if x >= -80 => 1,
        _ => 0,
    }
}

fn leds_to_mask(leds: u8) -> u8 {
    if leds == 8 {
        0b11111111
    } else {
        (1u8 << leds) - 1
    }
}

fn gen_pattern(signal: i32) -> u8 {
    leds_to_mask(rssi_to_leds(signal))
}

static SIGNAL_STRENGTH: Mutex<RefCell<i32>> = Mutex::new(RefCell::new(-100));

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let board = Board::init();
    let mut storeys = Storeys::new(board.storey_leds);

    let t1 = Duration::from_millis(500);

    let esp_radio_controller = esp_radio::init().unwrap();

    let (mut controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_controller, board.wifi, Config::default()).unwrap();

    controller.set_mode(wifi::WifiMode::ApSta).unwrap();
    controller.start().unwrap();

    let _relevant_ssid = "39C3".to_string();

    let mut sniffer = interfaces.sniffer;
    sniffer.set_promiscuous_mode(true).unwrap();
    sniffer.set_receive_cb(|packet| {
        let rssi = packet.rx_cntl.rssi;
        let _ = match_frames! {
            packet.data,
            beacon = BeaconFrame => {
                let Some(ssid) = beacon.ssid() else {
                    return;
                };
                log::info!("Found AP with SSID: {ssid}");
                if ssid == "39C3" {
                    critical_section::with(|cs| {
                        *SIGNAL_STRENGTH.borrow_ref_mut(cs) = rssi;
                 })
                }
            }
        };
    });

    loop {
        critical_section::with(|cs| {
            let rssi = SIGNAL_STRENGTH.borrow_ref(cs);
            log::info!("New signal strength: {rssi}");
            storeys.set_pattern(gen_pattern(*rssi))
        });
        delay(t1).await;
    }

    // loop {
    //     log::info!("ハッカー the planet!");
    //     log::info!("Starting scan...");
    //     let mut scan_config = ScanConfig::default();
    //     // scan_config = scan_config.with_ssid(relevant_ssid);
    //     scan_config.show_hidden();
    //     match controller.scan_with_config_async(scan_config).await {
    //         Ok(ssids) => {
    //             log::info!("Starting done found {}...", &ssids.len() );
    //             for ssid in &ssids {
    //                 log::info!("SSID: {}", ssid.ssid)
    //             }
    //             if ssids.len() > 0 {
    //                 // Set leds
    //                 let ssid = &ssids[0];
    //                 storeys.set_pattern(gen_pattern(ssid.signal_strength));
    //             }
    //         }
    //         Err(e) => log::error!("Error: {e}")
    //     }
    //     log::info!("Short wait...");
    //     delay(t1).await;
    // }
}
