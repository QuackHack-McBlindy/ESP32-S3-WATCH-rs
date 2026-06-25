// BASE/WIFI
// BASIC WIFI CONFIGURATION
// ++ EMBASSY-NET RUNNER
// ───────────────────────────────────────────────────────────────────────
// USAGE: ON
// WIFI_CMD.send(WifiCommand::Enable).await;
// USAGE: OFF
// WIFI_CMD.send(WifiCommand::Disable).await;
// ───────────────────────────────────────────────────────────────────────

use embassy_net_driver_channel as ch;
use crate::base::wireguard::WgConfig;
use embassy_net_driver_channel::{self, Device};

pub enum WifiCommand {
    Enable,
    Disable,
    Scan,
    Connect { ssid: heapless::String<32>, password: heapless::String<64> },
}

pub static WIFI_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    WifiCommand,
    3,
> = embassy_sync::channel::Channel::new();

pub static SCAN_RESULTS: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    heapless::Vec<esp_radio::wifi::ap::AccessPointInfo, 16>,
> = embassy_sync::mutex::Mutex::new(heapless::Vec::new());


// ───────────────────────────────────────────────────────────────────────
// WIFI CONNECTION TASK
#[embassy_executor::task]
pub async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    let credentials = crate::state::WIFI_CREDENTIALS;
    let mut idx = 0;
    let mut fail_count = 0u8;

    // CHECK DEFAULT WIFI BOOT STATE
    let mut enabled = crate::load!(crate::state::WIFI_STATE);
    defmt::info!("🛜 💤");

    'outer: loop {
        // WAIT UNTIL ENABLED
        while !enabled {
            let cmd = WIFI_CMD.receive().await;
            if matches!(cmd, WifiCommand::Enable) {
                enabled = true;
                crate::store!(crate::state::WIFI_STATE, true);
                break;
            }
        }

        // CONFIG THE CONTROLLER  FOR THIS WIFI SSID/PASSWORD
        let (ssid, password) = credentials[idx];
        let station_config = esp_radio::wifi::sta::StationConfig::default()
            .with_ssid(ssid)
            .with_password(alloc::string::ToString::to_string(password));
        let wifi_config = esp_radio::wifi::Config::Station(station_config);
        controller.set_config(&wifi_config).unwrap();

        if let Err(e) = controller.set_power_saving(esp_radio::wifi::PowerSaveMode::Maximum) {
            defmt::info!("FAILED TO SET POWER SAVING: {:?}", e);
        }

        // TRY TO CONNECT – & LISTEN FOR DISABLE COMMAND
        let connect_fut = controller.connect_async();
        let disable_cmd_fut = WIFI_CMD.receive();

        match embassy_futures::select::select(connect_fut, disable_cmd_fut).await {
            // CONNECTION SUCCEEDED
            embassy_futures::select::Either::First(Ok(conn_info)) => {
                defmt::info!(
                    "🛜 ☑️ - ({}), CHANNEL: {}",
                    ssid,
                    conn_info.channel
                );
                fail_count = 0; // RESET FAILURE COUNTER ON SUCESS
                
                // STORE WIFI STATE
                crate::store!(crate::state::WIFI_CONNECTED, true);
                *crate::state::CONNECTED_SSID.lock().await = Some(heapless::String::try_from(ssid).unwrap());

                // SUCCESS! MONITOR RSSI & WAIT FOR DISCONNECT OR Disable
                loop {
                    if let Ok(rssi) = controller.rssi() {
                        crate::store!(crate::state::RSSI, rssi);
                    }

                    let disconnect_fut = controller.wait_for_disconnect_async();
                    let timer = embassy_time::Timer::after(
                        embassy_time::Duration::from_millis(6000));
                    let disable_cmd_fut = WIFI_CMD.receive();

                    // WAIT FOR ANY OF: DISCONNECT, TIMEOUT, OR A COMMAND
                    match embassy_futures::select::select(
                        disconnect_fut,
                        embassy_futures::select::select(timer, disable_cmd_fut),
                    )
                    .await
                    {
                        // DISCONNECTED
                        embassy_futures::select::Either::First(result) => {
                            match result {
                                Ok(info) => defmt::info!(
                                    "🛜 ❌ - DISCONNECTED! REASON: {:?}",
                                    info.reason
                                ),
                                Err(e) => defmt::info!("WiFi - ❌ DISCONNECT ERROR: {:?}", e),
                            }
                            crate::store!(crate::state::WIFI_CONNECTED, false);
                            *crate::state::CONNECTED_SSID.lock().await = None;
                            break; // LEAVE INNER LOOP → RECONNECT AGAIN ON SAME CREDS
                        }
                        // TIMEOUT OR COMMAND
                        embassy_futures::select::Either::Second(inner) => match inner {
                            // TIMEOUT – KEEP LOOPIN'
                            embassy_futures::select::Either::First(()) => {}
                            // RECEIVED A COMMAND
                            embassy_futures::select::Either::Second(cmd) => {
                                if matches!(cmd, WifiCommand::Disable) {
                                    // TURNED OFF WIFI – DISCONNECT & GO BACK TO DISABLED WAIT
                                    let _ = controller.disconnect_async().await;
                                    crate::store!(crate::state::WIFI_CONNECTED, false);
                                    *crate::state::CONNECTED_SSID.lock().await = None;
                                    enabled = false;
                                    crate::store!(crate::state::WIFI_STATE, false);
                                    continue 'outer;
                                }

                                if matches!(cmd, WifiCommand::Scan) {
                                    enabled = false;
                                    crate::store!(crate::state::WIFI_CONNECTED, false);
                                    crate::store!(crate::state::WIFI_STATE, false);
                                    *crate::state::CONNECTED_SSID.lock().await = None;
                                    defmt::info!("🛜 ❌ - DISCONNECTED! REASON: SCANNING FOR WIRELESS NETWORKS");
                                    let scan_config = esp_radio::wifi::scan::ScanConfig::default()
                                        .with_max(16);

                                    if let Err(e) = controller.set_config(
                                        &esp_radio::wifi::Config::Station(
                                            esp_radio::wifi::sta::StationConfig::default(),
                                        ),
                                    ) { 
                                        defmt::warn!("scan prep failed: {:?}", e);
                                    } else {
                                        match controller.scan_async(&scan_config).await {
                                            Ok(results) => {
                                                defmt::info!("found {} networks:", results.len());
                                                {
                                                    let mut guard = SCAN_RESULTS.lock().await;
                                                    guard.clear();
                                                    let _ = guard.extend_from_slice(&results);
                                                }
                                                for (i, ap) in results.iter().enumerate() {
                                                    let ssid_str = ap.ssid.as_str();
                                                    defmt::info!(
                                                        "  {}: SSID: {}, Signal: {}, Auth: {:?}, Channel: {}",
                                                        i + 1,
                                                        ssid_str,
                                                        ap.signal_strength,
                                                        ap.auth_method,
                                                        ap.channel,
                                                    );
                                                }
                                            }
                                            Err(e) => defmt::warn!("WiFi scan failed: {:?}", e),
                                        }
                                    }
                                    // RETURN TO IDLE
                                    continue 'outer;
                                }
                            }
                        },
                    }
                }
                // WHEN DISCONNECTED WE BREAK TO THE OUTER LOOP & TRY SAME CREDS AGAIN
            }

            // CONNECTION FAILED (CONTROLLER ERROR)
            embassy_futures::select::Either::First(Err(e)) => {
                defmt::info!("🛜 ❌ - CONNECTION FAILED for {}: {:?}", ssid, e);
                fail_count += 1;

                if fail_count >= 3 {
                    defmt::info!(
                        "🛜 ⏭️ ... ({} failures)",
                        fail_count
                    );
                    fail_count = 0;
                    idx = (idx + 1) % credentials.len();
                }

                // WAIT! & RETURN
                embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await;
            }

            // RECEIVED DISABLE WHILE CONNECTING – GO BACK TO DISABLED STATE
            embassy_futures::select::Either::Second(_) => {
                *crate::state::CONNECTED_SSID.lock().await = None;
                crate::store!(crate::state::WIFI_STATE, false);
                enabled = false;
            }
        }
    }
}


// EMBASSY-NET RUNNER
#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await;
}

// ONE‑SHOT NETWORK INIT
// RETURNS THE EMBASSY‑NET STACK IMMEDIATELY.  
pub async fn init(
    spawner: &embassy_executor::Spawner,
    wifi_peripheral: esp_hal::peripherals::WIFI<'static>,
    backend_port: u16,
    wg_device: Device<'static, 1420>,
    wg_conf: &'static WgConfig,
) -> &'static embassy_net::Stack<'static> {

    // 1: CREATE WI‑FI CONTROLLER AND STATION INTERFACE
    let (wifi_controller, interfaces) = esp_radio::wifi::new(
        wifi_peripheral,
        esp_radio::wifi::ControllerConfig::default(),
    )
    .expect("🛜 ❌ - INIT FAILED");

    let station = interfaces.station;

    // ───────────────────────────────────────────────────────────────────────
    // 2: SPAWN THE CONNECTION‑MAINTAINING TASK (STARTS DISABLED)
    crate::spawn!(spawner, connection(wifi_controller));

    // ───────────────────────────────────────────────────────────────────────
    // 3: BUILD EMBASSY‑NET STACK
    let net_config = embassy_net::Config::dhcpv4(embassy_net::DhcpConfig::default());

    let rng = esp_hal::rng::Rng::new();
    let seed: u64 = (u64::from(rng.random())) << 32 | u64::from(rng.random());

    let stack_resources = crate::mk_static!(embassy_net::StackResources<16>, embassy_net::StackResources::<16>::new());
    let (stack, runner) = embassy_net::new(station, net_config, stack_resources, seed);
    let stack = crate::mk_static!(embassy_net::Stack<'static>, stack);

    crate::spawn!(spawner, net_task(runner));

    // ───────────────────────────────────────────────────────────────────────
    // 4: SPAWN A BACKGROUND TASK THAT COMPLETES NETWORK SETUP
    // WHEN WIFI IS ACTUALLY ENABLED AND CONNECTED.
    crate::spawn!(spawner, network_ready_task(*spawner, stack, backend_port, wg_device, wg_conf));
    stack
}


// BACKGROUND TASK – WAITS FOR WIFI TO BECOME READY, THEN
// COMPLETES DNS, NTP, AND SPAWNS NETWORK‑DEPENDENT TASKS.
#[embassy_executor::task]
pub async fn network_ready_task(
    spawner: embassy_executor::Spawner,
    stack: &'static embassy_net::Stack<'static>,
    backend_port: u16,
    wg_device: Device<'static, 1420>,
    wg_conf: &'static WgConfig, 
) {
    loop {
        let cmd = WIFI_CMD.receive().await;
        if matches!(cmd, WifiCommand::Enable) {
            break;
        }
    }

    // WAIT FOR THE NETWORK LINK AND DHCP
    stack.wait_link_up().await;
    stack.wait_config_up().await;
    crate::store!(crate::state::WIFI_CONNECTED, true);


    // BUILD 2ND embassy-net STACK THAT SITS ON TOP OF THE WG DEVICE
    let vpn_config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: wg_conf.address,
        gateway: Some(wg_conf.address.address()),
        dns_servers: Default::default(),
    });
    
    let vpn_resources = crate::mk_static!(
        embassy_net::StackResources<12>,
        embassy_net::StackResources::<12>::new()
    );

    let mut rng = esp_hal::rng::Rng::new();
    let vpn_seed: u64 = (u64::from(rng.random())) << 32 | u64::from(rng.random());
    let (vpn_stack, vpn_runner) =
        embassy_net::new(wg_device, vpn_config, vpn_resources, vpn_seed);

    let vpn_stack = crate::mk_static!(
        embassy_net::Stack<'static>,
        vpn_stack
    );
    unsafe { crate::VPN_STACK = Some(vpn_stack); }
    crate::VPN_STACK_READY.signal(());

    crate::spawn!(spawner, crate::base::wireguard::vpn_runner_task(vpn_runner));

    // ENABLE VPN
    crate::base::wireguard::WG_CMD.send(crate::base::wireguard::WgCommand::Enable).await;
    crate::store!(crate::state::WG_STATE, true);
    

    // GRAB IPV4 AND STORE IT
    embassy_time::Timer::after_millis(100).await;
    for _ in 0..10 {
        if let Some(config) = stack.config_v4() {
            let ip_raw = u32::from(config.address.address());
            crate::store!(crate::state::CURRENT_IP, ip_raw);
            defmt::info!("IP: {}", config.address);
            break;
        }
        embassy_time::Timer::after_millis(100).await;
    }



    // RESOLVE BACKEND ADDRESS
    let remote_addr: core::net::SocketAddr = loop {
        match stack
            .dns_query(crate::state::BACKEND_TCP_HOST, embassy_net::dns::DnsQueryType::A)
            .await
        {
            Ok(addrs) => break core::net::SocketAddr::from((addrs[0], backend_port)),
            Err(e) => {
                defmt::error!("DNS LOOKUP ERROR: {}", e);
                embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
            }
        }
    };

    // SYNC RTC VIA NTP
    match crate::components::pcf85063a::ntp_sync(stack).await {
        Ok(()) => defmt::debug!("PCF85063A Synchronized"),
        Err(e) => defmt::warn!("NTP sync failed: {}", e),
    }

    // START THE WEBSERVER/API
    tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Start).await;
    crate::store!(crate::state::API_STATE, true);
    // START SSH SERVER
    crate::base::ssh::SSH_CMD.send(crate::base::ssh::SshCommand::Enable).await;
    crate::store!(crate::state::SSH_STATE, true);
    // ALLOW STREAMING AUDIO TO THE SPEAKER
    yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
    crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true);
    
    yo_esp::play_ding().await;

}


// ───────────────────────────────────────────────────────────────────────
// WIFI SCANNER
//async fn do_scan(controller: &mut esp_radio::wifi::WifiController<'_>) {
//    if let Err(e) = controller.set_config(
//        &esp_radio::wifi::Config::Station(esp_radio::wifi::sta::StationConfig::default()),
//    ) {
//        defmt::warn!("Failed to set config for scan: {:?}", e);
//        return;
//    }

//    let scan_config = esp_radio::wifi::scan::ScanConfig::default()
//        .with_max(16);
//    match controller.scan_async(&scan_config).await {
//        Ok(results) => {
//            defmt::info!("Found {} networks:", results.len());
            // Store for public access
//            {
//                let mut guard = SCAN_RESULTS.lock().await;
//                guard.clear();
                // extend_from_slice is a method of heapless::Vec
//                guard.extend_from_slice(&results).ok();
//            }
//            for (i, ap) in results.iter().enumerate() {
                // Use the public as_str() method on Ssid
//                let ssid_str = ap.ssid.as_str();
//                defmt::info!(
//                    "  {}: SSID: {}, Signal: {}, Auth: {:?}, Channel: {}",
//                    i + 1,
//                    ssid_str,
//                    ap.signal_strength,
//                    ap.auth_method,
//                    ap.channel,
//                );
//            }
//        }
//        Err(e) => {
//            defmt::warn!("WiFi scan failed: {:?}", e);
//        }
//    }
//}

// ───────────────────────────────────────────────────────────────────────
// PUBLIC HELPERS

// TOGGLE WI‑FI ON/OFF
pub async fn toggle_wifi() {
    let current = crate::load!(crate::state::WIFI_STATE);
    if current { // TURN IT OFF
        crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Disable).await;
        crate::store!(crate::state::WIFI_STATE, false);
        defmt::info!("Wi‑Fi disabled");
    } else { // TURN IT ON
        crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Enable).await;
        crate::store!(crate::state::WIFI_STATE, true);
        defmt::info!("Wi‑Fi enabled");
    }
}

pub fn toggle_wifi_now() {
    let current = crate::load!(crate::state::WIFI_STATE);
    if current {
        let _ = WIFI_CMD.try_send(WifiCommand::Disable);
        crate::store!(crate::state::WIFI_STATE, false);
        defmt::info!("Wi‑Fi disabled (non‑async)");
    } else {
        let _ = WIFI_CMD.try_send(WifiCommand::Enable);
        crate::store!(crate::state::WIFI_STATE, true);
        defmt::info!("Wi‑Fi enabled (non‑async)");
    }
}

// HELPERS
pub async fn sleep(millis: u64) { embassy_time::Timer::after(embassy_time::Duration::from_millis(millis)).await; }

pub async fn scan() {
    WIFI_CMD.send(WifiCommand::Scan).await;
}


