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
    mut pwr_button: esp_hal::gpio::Input<'static>,
) {
    let mut boot_was_pressed = false;
    let mut boot_press_start: Option<embassy_time::Instant> = None;
    let mut pwr_press_start: Option<embassy_time::Instant> = None;

    loop {
        match embassy_futures::select::select(boot_button.wait_for_low(), pwr_button.wait_for_low()).await
        {
            // ───────────────────────────────────────────────────────────────────────
            // BOOT BUTTON
            embassy_futures::select::Either::First(_) => {
                // DEBOUNCE
                embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
                if !boot_button.is_low() {
                    continue;
                }

                // PLAYING MEDIA OR IN INTERCOM SESSION?
                // INCREASE THE VOLUME
                let is_media_page = crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer;
                if crate::load!(crate::state::MEDIA_IS_PLAYING)
                    || is_media_page
                    || crate::load!(crate::state::INTERCOM_STATE)
                {
                    // VOLUME UP
                    crate::applications::media_player::volume_up();
                } else { // HOLD-TO-TALK TO VOICE ASSISTANT
                    if !crate::load!(crate::state::AMPLIFIER_STATE) { crate::amp_on(); }
                    if !crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
                        yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
                        crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true);
                    }
                    if crate::load!(crate::state::SPEAKER_MUTED) || crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
                        crate::set_speaker_volume(65);
                    }
                    // START RECORDING MIC AUDIO WHILE HOLDING
                    let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Pushed).await;

                    // WAIT FOR RELEASE & SEND TO BACKEND
                    boot_button.wait_for_high().await;

                    let _ = yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Released).await;
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
                }

                // WAIT FOR RELEASE
                boot_button.wait_for_high().await;
            }

            // ───────────────────────────────────────────────────────────────────────
            // POWER BUTTON
            embassy_futures::select::Either::Second(_) => {
                // PRESSED: TOGGLE DISPLAY ON/OFF
                // PLAYING MEDIA/INTERCOM? DECREASE VOLUME

                embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
                if !pwr_button.is_high() {
                    continue;
                }

                let press_start = embassy_time::Instant::now();

                // PLAYING MEDIA OR IN INTERCOM SESSION?
                // DECREASE VOLUME
                let is_media_page = crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer;
                if crate::load!(crate::state::MEDIA_IS_PLAYING)
                    || is_media_page
                    || crate::load!(crate::state::INTERCOM_STATE)
                {
                    crate::applications::media_player::volume_down();
                } else { // POWER ON/OFF DISPLAY
                    crate::toggle!(crate::state::DISPLAY_STATE);
                    defmt::debug!("POWER BUTTON PRESSED");
                }

                // HOLD ACTIONS:
                loop {
                    match embassy_futures::select::select(pwr_button.wait_for_high(), embassy_time::Timer::after(embassy_time::Duration::from_secs(2))).await
                    {
                        embassy_futures::select::Either::First(_) => break,
                        embassy_futures::select::Either::Second(_) => {
                            // HOLD 6 SEC: POWER DOWN!
                            defmt::info!("POWER HELD 2 SECONDS");

                            // PREVENTS REPEATED SPAM
                            pwr_button.wait_for_high().await;
                            break;
                        }
                    }
                }
            }
        }
    }
}
