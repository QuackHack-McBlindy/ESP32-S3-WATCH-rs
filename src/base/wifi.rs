// BASE/WIFI
// BASIC WIFI CONFIGURATION
// ++ EMBASSY-NET RUNNER
// ───────────────────────────────────────────────────────────────────────
// USAGE: ON
// WIFI_CMD.send(WifiCommand::Enable).await;
// USAGE: OFF
// WIFI_CMD.send(WifiCommand::Disable).await;
// ───────────────────────────────────────────────────────────────────────

pub enum WifiCommand {
    Enable,
    Disable,
}

pub static WIFI_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    WifiCommand,
    1,
> = embassy_sync::channel::Channel::new();
// ───────────────────────────────────────────────────────────────────────

/// TOGGLE WI‑FI ON/OFF
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

// WIFI CONNECTION TASK
#[embassy_executor::task]
pub async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    let credentials = crate::state::WIFI_CREDENTIALS;
    let mut idx = 0;
    let mut fail_count = 0u8;

    // CHECK DEFAULT WIFI BOOT STATE
    let mut enabled = crate::load!(crate::state::WIFI_STATE);
    if !enabled { defmt::info!("🛜 💤"); }

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
                *crate::state::CONNECTED_SSID.lock().await = Some(ssid);

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
                        "WiFi - Switching to next credentials ({} failures)",
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
    crate::spawn!(spawner, network_ready_task(*spawner, stack, backend_port));
    stack
}


// BACKGROUND TASK – WAITS FOR WIFI TO BECOME READY, THEN
// COMPLETES DNS, NTP, AND SPAWNS NETWORK‑DEPENDENT TASKS.
#[embassy_executor::task]
pub async fn network_ready_task(
    spawner: embassy_executor::Spawner,
    stack: &'static embassy_net::Stack<'static>,
    backend_port: u16,
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
                defmt::info!("DNS LOOKUP ERROR: {}", e);
                embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
            }
        }
    };

    // SYNC RTC VIA NTP
    match crate::components::pcf85063a::ntp_sync(stack).await {
        Ok(()) => defmt::info!("PCF85063A Synchronized"),
        Err(e) => defmt::warn!("NTP sync failed: {}", e),
    }

}


// HELPER SLEEP
pub async fn sleep(millis: u64) { embassy_time::Timer::after(embassy_time::Duration::from_millis(millis)).await; }
