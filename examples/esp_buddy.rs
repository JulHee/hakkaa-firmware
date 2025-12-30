//! ESP-NOW Example taken from [esp-hal](https://github.com/esp-rs/esp-hal) esp_now example

#![no_std]
#![no_main]

use embassy_executor::Spawner;
// use embassy_time::{Duration, Ticker};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::time::{self, Duration};
use esp_println::println;
use esp_radio::{
    esp_now::{PeerInfo, BROADCAST_ADDRESS},
    wifi::{self, Config},
};
use hakkaa::{board::Board, led::Storeys};

esp_bootloader_esp_idf::esp_app_desc!();

fn rssi_to_leds(rssi: i32) -> u8 {
    match rssi {
        x if x >= 170 => 8,
        x if x >= 130 => 7,
        x if x >= 100 => 6,
        x if x >= 70 => 5,
        x if x >= 50 => 4,
        x if x >= 20 => 3,
        x if x >= 0 => 2,
        x if x >= -20 => 1,
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

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let board = Board::init();
    let mut storeys = Storeys::new(board.storey_leds);

    let esp_radio_controller = esp_radio::init().unwrap();
    // We must initialize some kind of interface and start it.
    let (mut controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_controller, board.wifi, Config::default()).unwrap();

    controller.set_mode(wifi::WifiMode::Sta).unwrap();
    controller.start().unwrap();
    let own_id = wifi::sta_mac();
    log::info!("Own wifi mac: {own_id:?}");

    let mut esp_now = interfaces.esp_now;

    println!("esp-now version {}", esp_now.version().unwrap());

    esp_now.set_channel(11).unwrap();

    let partner_id;
    if own_id == [32, 110, 241, 106, 208, 220] {
        // Simon
        partner_id = [136, 86, 166, 98, 41, 32];
    } else {
        // Me
        partner_id = [32, 110, 241, 106, 208, 220];
    }

    let mut next_send_time = time::Instant::now() + Duration::from_secs(1);
    let mut next_announce_time = time::Instant::now() + Duration::from_secs(1);
    loop {
        let r = esp_now.receive();
        if let Some(r) = r {
            let rssi = r.info.rx_control.rssi;

            // Register esps
            if r.info.dst_address == BROADCAST_ADDRESS {
                log::info!("Received broadcast from {:?}", r.info.src_address);
                if !esp_now.peer_exists(&r.info.src_address) {
                    esp_now
                        .add_peer(PeerInfo {
                            interface: esp_radio::esp_now::EspNowWifiInterface::Sta,
                            peer_address: r.info.src_address,
                            lmk: None,
                            channel: None,
                            encrypt: false,
                        })
                        .unwrap();
                }
            }

            if r.info.src_address == partner_id {
                log::info!("Received partner message with strength: {rssi}");
                storeys.set_pattern(gen_pattern(rssi));
                //     match esp_now.send(&r.info.src_address, b"Pong") {
                //         Ok(w) => {
                //             let status = w.wait();
                //             log::info!("Send added to peer status: {:?}", status);
                //         }
                //         Err(e) => log::error!("Error sending pong: {e:?}"),
                //     }
            }
        }
        // Send regular ping
        if time::Instant::now() >= next_send_time {
            next_send_time = time::Instant::now() + Duration::from_secs(2);
            if esp_now.peer_exists(&partner_id) {
                log::info!("Send to partner {:?}", partner_id);
                match esp_now.send(&partner_id, b"Ping") {
                    Ok(w) => {
                        let status = w.wait();
                        log::info!("Send ping to partner");
                        if status.is_err() {
                            storeys.set_pattern(gen_pattern(-30));
                        }
                    }
                    Err(e) => {
                        storeys.set_pattern(gen_pattern(-30));
                        log::error!("Error sending ping: {e:?}")
                    }
                }
            } else {
                storeys.set_pattern(gen_pattern(-30));
            }
        }

        // Anounce ID
        if time::Instant::now() >= next_announce_time {
            next_announce_time = time::Instant::now() + Duration::from_secs(5);
            match esp_now.send(&BROADCAST_ADDRESS, b"Elo") {
                Ok(w) => {
                    let status = w.wait();
                    if let Err(e) = status {
                        log::error!("Error sending broadcast: {:?}", e);
                    }
                }
                Err(e) => log::error!("Error sending broadcast: {e:?}"),
            }
        }
    }
}
