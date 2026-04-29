// BASE/WIFI
// BASIC WIFI CONFIGURATION
// ++ EMBASSY-NET RUNNER

use core::net::SocketAddr;
use core::sync::atomic::{AtomicI32, Ordering};
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{
    Config as NetConfig,
    DhcpConfig,
    Runner,
    Stack,
    StackResources,
    dns::DnsQueryType,
};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::{
    Config,
    ControllerConfig,
    Interface,
    PowerSaveMode,
    WifiController,
    sta::StationConfig,
};

use crate::alloc::string::ToString;
use crate::{
    store,
    CURRENT_IP,
    PASSWORD,
    SSID,
    BACKEND_TCP_HOST,
    mk_static,
    spawn,                // <-- THE SPAWN! MACRO
};

pub static CURRENT_RSSI: AtomicI32 = AtomicI32::new(0);

// WIFI CONNECTION TASK
#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    let station_config = StationConfig::default()
        .with_ssid(SSID)
        .with_password(PASSWORD.to_string());

    let wifi_config = Config::Station(station_config);
    controller.set_config(&wifi_config).unwrap();

    // ENABLE POWER SAVING
    if let Err(e) = controller.set_power_saving(PowerSaveMode::Maximum) {
        info!("FAILED TO SET POWER SAVING: {:?}", e);
    }

    loop {
        match controller.connect_async().await {
            Ok(conn_info) => {
                info!("WiFi - ✅ CONNECTED, CHANNEL: {}", conn_info.channel);

                // RSSI UPDATE LOOP
                loop {
                    if let Ok(rssi) = controller.rssi() {
                        CURRENT_RSSI.store(rssi, Ordering::Relaxed);
                    }

                    match select(
                        controller.wait_for_disconnect_async(),
                        Timer::after(Duration::from_millis(6000)),
                    )
                    .await
                    {
                        Either::First(result) => {
                            match result {
                                Ok(info) => info!(
                                    "WiFi - ❌ DISCONNECTED! REASON: {:?}",
                                    info.reason
                                ),
                                Err(e) => info!("WiFi - ❌ DISCONNECT ERROR: {:?}", e),
                            }
                            break; // GO BACK TO RECONNECT
                        }
                        Either::Second(()) => {
                            // TIMEOUT – JUST LOOP AGAIN
                        }
                    }
                }
            }
            Err(e) => {
                info!("WiFi - ❌ CONNECTION FAILED: {:?}", e);
                Timer::after(Duration::from_millis(5000)).await;
            }
        }
    }
}

// EMBASSY-NET RUNNER
#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}

// ONE‑SHOT NETWORK INIT
/// FULLY INITIALISE WI‑FI & EMBASSY‑NET, OBTAIN IP, RESOLVE BACKEND
/// ADDRESS, & RETURN:
/// + STATIC STACK
/// + BACKEND SOCKET ADDRESS.
pub async fn init(
    spawner: &Spawner,
    wifi_peripheral: WIFI<'static>,
    backend_port: u16,
) -> (&'static Stack<'static>, SocketAddr) {
    // 1: CREATE WI‑FI CONTROLLER AND STATION INTERFACE
    let (wifi_controller, interfaces) = esp_radio::wifi::new(
        wifi_peripheral,
        ControllerConfig::default(),
    )
    .expect("Wi‑Fi - ❌ INIT FAILED");

    let station = interfaces.station;

    // 2: SPAWN THE CONNECTION‑MAINTAINING TASK (USES SPAWN! MACRO)
    spawn!(spawner, connection(wifi_controller));

    // 3: BUILD EMBASSY‑NET STACK
    let net_config = NetConfig::dhcpv4(DhcpConfig::default());

    // RANDOM SEED (USES HARDWARE RNG INTERNALLY)
    let rng = Rng::new();
    let seed: u64 = (u64::from(rng.random())) << 32 | u64::from(rng.random());

    let stack_resources = mk_static!(StackResources<16>, StackResources::<16>::new());
    let (stack, runner) = embassy_net::new(station, net_config, stack_resources, seed);
    let stack = mk_static!(Stack<'static>, stack);

    spawn!(spawner, net_task(runner));

    // 4: WAIT FOR LINK + DHCP
    stack.wait_link_up().await;
    stack.wait_config_up().await;

    // 5: GRAB IPV4 AND STORE IT
    let ip = loop {
        if let Some(config) = stack.config_v4() {
            break config.address;
        }
        Timer::after(Duration::from_millis(500)).await;
    };
    let ip_raw = u32::from(ip.address());
    store!(CURRENT_IP, ip_raw);
    info!("IP: {}", ip.address());

    // 6: RESOLVE BACKEND ADDRESS (COMPILE‑TIME CONSTANTS, PORT IS ALREADY u16)
    let remote_addr = loop {
        match stack.dns_query(BACKEND_TCP_HOST, DnsQueryType::A).await {
            Ok(addrs) => {
                let addr = (addrs[0], backend_port).into();
                break addr;
            }
            Err(e) => {
                info!("DNS LOOKUP ERROR FOR {}: {}", BACKEND_TCP_HOST, e);
                Timer::after(Duration::from_secs(5)).await;
            }
        }
    };

    (stack, remote_addr)
}

// HELPER SLEEP
pub async fn sleep(millis: u64) {
    Timer::after(Duration::from_millis(millis)).await;
}
