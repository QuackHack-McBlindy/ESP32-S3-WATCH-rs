// COMPONENTS/BUTTONS
// MONITOR & PERFORM ACTIONS
// DEFINES PRESS/HOLD ACTIONS (HOLD POWER 5 SEC FOR DEEP SLEEP/WAKEUP)

use crate::applications::APPS;

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
        let boot_now = boot_button.is_low();

        if boot_now {
            let is_media_page = crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer;

            if crate::load!(crate::state::MEDIA_IS_PLAYING) || is_media_page {
                crate::applications::media_player::volume_up();
            } else {
                if !crate::load!(crate::state::AMPLIFIER_STATE) { crate::amp_on(); }
                if !crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
                    yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
                    crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true);
                }
                if crate::load!(crate::state::SPEAKER_MUTED) || crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
                    crate::set_speaker_volume(65);
                }

            
                let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Pushed).await;

                wait_for_release(&mut boot_button).await;

               let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Released).await;
                embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;

            }
            embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
        }



        // ───────────────────────────────────────────────────────────────────────
        // POWER BUTTON
        if pwr_button.is_high() {
            // PRESSED: TOGGLE DISPLAY ON/OFF (IF TOUCH DISPLAY FAILS TO WAKE)
            // PLAYING MEDIA? DECREASE VOLUME
            if pwr_press_start.is_none() {
                pwr_press_start = Some(embassy_time::Instant::now());
                
                // PRESSSED WHILE PLAYING MEDIA: DECREASE SPEAKER VOLUME
                let is_media_page = crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer;

                if crate::load!(crate::state::MEDIA_IS_PLAYING) || is_media_page {
                    crate::applications::media_player::volume_down();
                } else {
                    crate::toggle!(crate::state::DISPLAY_STATE);
                    defmt::debug!("POWER BUTTON PRESSED");
                }    
            }

            // HOLD ACTIONS
            if let Some(start) = pwr_press_start {
                if embassy_time::Instant::now() - start
                    >= embassy_time::Duration::from_secs(2)
                { // HOLD 6 SEC: ENTER DEEP SLEEP
                    defmt::info!("POWER HELD 2 SECONDS");
                    
                    // PREVENTS REPEATED SPAM
                    pwr_press_start = None;
                }
            }
        } else { pwr_press_start = None; }

        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
