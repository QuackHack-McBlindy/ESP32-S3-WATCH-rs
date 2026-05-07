// BASE/WIFI
// BASIC WIFI CONFIGURATION
// ++ EMBASSY-NET RUNNER


// WIFI CONNECTION TASK
#[embassy_executor::task]
pub async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    let credentials = crate::state::WIFI_CREDENTIALS;
    let mut idx = 0;
    let mut fail_count = 0u8;

    loop {
        let (ssid, password) = credentials[idx];

        // CONFIG THE CONTROLLER  FOR THIS WIFI SSID/PASSWORD
        let station_config = esp_radio::wifi::sta::StationConfig::default()
            .with_ssid(ssid)
            .with_password(alloc::string::ToString::to_string(password));
        let wifi_config = esp_radio::wifi::Config::Station(station_config);
        controller.set_config(&wifi_config).unwrap();

        if let Err(e) = controller.set_power_saving(esp_radio::wifi::PowerSaveMode::Maximum) {
            defmt::info!("FAILED TO SET POWER SAVING: {:?}", e);
        }

        // TRY AGAIN!
        match controller.connect_async().await {
            Ok(conn_info) => {
                defmt::info!(
                    "WiFi - ✅ CONNECTED ({}), CHANNEL: {}",
                    ssid,
                    conn_info.channel
                );
                fail_count = 0; // RESET FAILURE COUNTER ON SUCESS
                crate::store!(crate::state::WIFI_CONNECTED, true);

                // SUCCESS! MONITOR RSSI & WAIT FOR DISCONNECT
                loop {
                    if let Ok(rssi) = controller.rssi() {
                        crate::store!(crate::state::RSSI, rssi);
                    }

                    match embassy_futures::select::select(
                        controller.wait_for_disconnect_async(),
                        embassy_time::Timer::after(embassy_time::Duration::from_millis(6000)),
                    )
                    .await
                    {
                        embassy_futures::select::Either::First(result) => {
                            match result {
                                Ok(info) => defmt::info!(
                                    "WiFi - ❌ DISCONNECTED! REASON: {:?}",
                                    info.reason
                                ),
                                Err(e) => defmt::info!("WiFi - ❌ DISCONNECT ERROR: {:?}", e),
                            }
                            break; // LEAVE INNER LOOP LOOP > RECONNECT AGAIN ON SAME
                        }
                        embassy_futures::select::Either::Second(()) => {
                            // TIMEOUT – KEEP LOOPIN' 
                        }
                    }
                }
                // WHEN DISCONNECTED WE BREAK TO THE OUTER LOOP & TRY SAME CREDS AGAIN
            }
            Err(e) => {
                defmt::info!("WiFi - ❌ CONNECTION FAILED for {}: {:?}", ssid, e);
                fail_count += 1;

                if fail_count >= 2 {
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
        }
    }
}

// EMBASSY-NET RUNNER
#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await;
}

// ONE‑SHOT NETWORK INIT
/// FULLY INITIALISE WI‑FI & EMBASSY‑NET, OBTAIN IP, RESOLVE BACKEND
/// ADDRESS, & RETURN:
/// + STATIC STACK
/// + BACKEND SOCKET ADDRESS.
pub async fn init(
    spawner: &embassy_executor::Spawner,
    wifi_peripheral: esp_hal::peripherals::WIFI<'static>,
    backend_port: u16,
) -> (&'static embassy_net::Stack<'static>, core::net::SocketAddr) {
    // 1: CREATE WI‑FI CONTROLLER AND STATION INTERFACE
    let (wifi_controller, interfaces) = esp_radio::wifi::new(
        wifi_peripheral,
        esp_radio::wifi::ControllerConfig::default(),
    )
    .expect("Wi‑Fi - ❌ INIT FAILED");

    let station = interfaces.station;

    // 2: SPAWN THE CONNECTION‑MAINTAINING TASK (USES SPAWN! MACRO)
    crate::spawn!(spawner, connection(wifi_controller));

    // 3: BUILD EMBASSY‑NET STACK
    let net_config = embassy_net::Config::dhcpv4(embassy_net::DhcpConfig::default());

    // RANDOM SEED (USES HARDWARE RNG INTERNALLY)
    let rng = esp_hal::rng::Rng::new();
    let seed: u64 = (u64::from(rng.random())) << 32 | u64::from(rng.random());

    let stack_resources = crate::mk_static!(embassy_net::StackResources<16>, embassy_net::StackResources::<16>::new());
    let (stack, runner) = embassy_net::new(station, net_config, stack_resources, seed);
    let stack = crate::mk_static!(embassy_net::Stack<'static>, stack);

    crate::spawn!(spawner, net_task(runner));

    // 4: WAIT FOR LINK + DHCP
    stack.wait_link_up().await;
    stack.wait_config_up().await;

    // 5: GRAB IPV4 AND STORE IT
    let ip = loop {
        if let Some(config) = stack.config_v4() {
            break config.address;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    };
    let ip_raw = u32::from(ip.address());
    crate::store!(crate::state::CURRENT_IP, ip_raw);
    defmt::info!("IP: {}", ip.address());

    // 6: RESOLVE BACKEND ADDRESS (COMPILE‑TIME CONSTANTS, PORT IS ALREADY u16)
    let remote_addr = loop {
        match stack
            .dns_query(crate::state::BACKEND_TCP_HOST, embassy_net::dns::DnsQueryType::A)
            .await
        {
            Ok(addrs) => {
                let addr = (addrs[0], backend_port).into();
                break addr;
            }
            Err(e) => {
                defmt::info!(
                    "DNS LOOKUP ERROR FOR {}: {}",
                    crate::state::BACKEND_TCP_HOST,
                    e
                );
                embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
            }
        }
    };

    (stack, remote_addr)
}

// HELPER SLEEP
pub async fn sleep(millis: u64) {
    embassy_time::Timer::after(embassy_time::Duration::from_millis(millis)).await;
}
