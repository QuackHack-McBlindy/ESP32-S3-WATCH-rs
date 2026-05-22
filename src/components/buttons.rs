// COMPONENTS/BUTTONS
// MONITOR & PERFORM ACTIONS
// DEFINES PRESS/HOLD ACTIONS (HOLD POWER 5 SEC FOR DEEP SLEEP/WAKEUP)


async fn wait_for_release(button: &mut esp_hal::gpio::Input<'_>) {
    while button.is_low() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
    }
}


// ───────────────────────────────────────────────────────────────────────
// BUTTON TASK
#[embassy_executor::task]
pub async fn buttons_task(
    mut boot_button: esp_hal::gpio::Input<'static>,
    pwr_button: esp_hal::gpio::Input<'static>,
) {
    let mut boot_was_pressed = false;
    let mut boot_press_start: Option<embassy_time::Instant> = None;
    let mut pwr_press_start: Option<embassy_time::Instant> = None;

    loop {
        // ───────────────────────────────────────────────────────────────────────
        // BOOT BUTTON
        let boot_now = boot_button.is_low();

        if boot_now {
            if crate::load!(crate::state::MEDIA_IS_PLAYING) {
                // MEDIA IS PLAYING > VOLUME UP
                crate::applications::media_player::volume_up();
            } else {
                // NO MEDIA > START PUSH-TO-TALK RECORDING
                let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Pushed).await;

                // HOLD THE TASK HERE UNTIL THE BUTTON IS RELEASED
                wait_for_release(&mut boot_button).await;

                // RELEASED - END PTT RECORDING
                let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Released).await;
            }

            // DEBOUNCE
            embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
        }


        // ───────────────────────────────────────────────────────────────────────
        // POWER BUTTON
        if pwr_button.is_high() {
            // PRESSED: TOGGLE DISPLAY ON/OFF (IF TOUCH DISPLAY FAILS TO WAKE)
            // PLAYING MEDIA? DECREASE VOLUME
            if pwr_press_start.is_none() {
                crate::toggle!(crate::state::DISPLAY_STATE);
                defmt::debug!("POWER BUTTON PRESSED");
                pwr_press_start = Some(embassy_time::Instant::now());
                
                // PRESSSED WHILE PLAYING MEDIA: DECREASE SPEAKER VOLUME
                if crate::load!(crate::state::MEDIA_IS_PLAYING) {
                    crate::applications::media_player::volume_down();
                }
            }

            // HOLD ACTIONS
            if let Some(start) = pwr_press_start {
                if embassy_time::Instant::now() - start
                    >= embassy_time::Duration::from_secs(2)
                { // HOLD 2 SEC: TODO
                  // HOLD 5 SEC: ENTER DEEP SLEEP
                    defmt::info!("POWER HELD 2 SECONDS");
                    
                    // PREVENTS REPEATED SPAM
                    pwr_press_start = None;
                }
            }
        } else { pwr_press_start = None; }

        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
