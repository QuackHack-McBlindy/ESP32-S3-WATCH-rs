// COMPONENTS/BUTTONS
// MONITOR & ACT UPON BUTTON PRESSES

use crate::{init_bool, store, toggle};

init_bool!(BOOT_BUTTON_PRESSED, false);
init_bool!(PWR_BUTTON_PRESSED, false);

async fn wait_for_release(button: &mut esp_hal::gpio::Input<'_>) {
    while button.is_low() {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
    }
}

#[embassy_executor::task]
pub async fn buttons_task(mut boot_button: esp_hal::gpio::Input<'static>, mut pwr_button: esp_hal::gpio::Input<'static>) {
    loop { // BOOT BUTTON
        if boot_button.is_low() {
            store!(BOOT_BUTTON_PRESSED, true);
            toggle!(crate::state::DISPLAY_STATE);
            yo_esp::play_ding().await;
            wait_for_release(&mut boot_button).await;
            store!(BOOT_BUTTON_PRESSED, false);
        } // POWER BUTTON
        if pwr_button.is_low() {
            store!(PWR_BUTTON_PRESSED, true);
            toggle!(crate::state::DISPLAY_STATE);
            yo_esp::play_ding().await;
            wait_for_release(&mut pwr_button).await;
            store!(PWR_BUTTON_PRESSED, false);
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
