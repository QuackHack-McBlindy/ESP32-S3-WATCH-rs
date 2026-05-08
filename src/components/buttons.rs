// COMPONENTS/BUTTONS
// MONITOR & ACT UPON BUTTON PRESSES


crate::init_bool!(BOOT_BUTTON_PRESSED, false);
crate::init_bool!(PWR_BUTTON_PRESSED, false);

async fn wait_for_release(button: &mut esp_hal::gpio::Input<'_>) {
    while button.is_low() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
    }
}

#[embassy_executor::task]
pub async fn buttons_task(
    mut boot_button: esp_hal::gpio::Input<'static>,
    mut pwr_button: esp_hal::gpio::Input<'static>,
) {
    let mut boot_press_start: Option<embassy_time::Instant> = None;
    let mut pwr_press_start: Option<embassy_time::Instant> = None;

    loop {
        // =========================
        // BOOT BUTTON
        // =========================

        if boot_button.is_low() {
            if boot_press_start.is_none() {
                defmt::info!("BOOT PRESSED");
                boot_press_start = Some(embassy_time::Instant::now());
            }

            if let Some(start) = boot_press_start {
                if embassy_time::Instant::now() - start
                    >= embassy_time::Duration::from_secs(2)
                {
                    defmt::info!("BOOT HELD 2 SECONDS");

                    // prevent repeated spam
                    boot_press_start = None;
                }
            }
        } else {
            boot_press_start = None;
        }

        // =========================
        // POWER BUTTON
        // =========================

        if pwr_button.is_high() {
            if pwr_press_start.is_none() {
                defmt::info!("POWER PRESSED");
                pwr_press_start = Some(embassy_time::Instant::now());
            }

            if let Some(start) = pwr_press_start {
                if embassy_time::Instant::now() - start
                    >= embassy_time::Duration::from_secs(2)
                {
                    defmt::info!("POWER HELD 2 SECONDS");

                    // prevent repeated spam
                    pwr_press_start = None;
                }
            }
        } else {
            pwr_press_start = None;
        }

        embassy_time::Timer::after(
            embassy_time::Duration::from_millis(50)
        ).await;
    }
}
