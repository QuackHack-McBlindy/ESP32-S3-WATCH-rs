// GUI/PAGES
// LISTEN FOR TOUCH EVENTS AND ACT UPON THEM

pub static TOUCH_EVENTS: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    TouchEvent,
> = embassy_sync::signal::Signal::new();

pub static CURRENT_PAGE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);

#[derive(Clone, Copy, defmt::Format)]
pub enum TouchEvent {
    Tap { x: u16, y: u16 },
    Swipe(crate::components::ft3168::SwipeDirection, u16, u16),   // direction, end_x, end_y
}

#[embassy_executor::task]
pub async fn page_switcher_task() {
    loop {
        let event = TOUCH_EVENTS.wait().await;
        match event {
            TouchEvent::Swipe(dir, _, _) => match dir {
                crate::components::ft3168::SwipeDirection::Left => {
                    // NEXT PAGE
                    let mut page = CURRENT_PAGE.load(core::sync::atomic::Ordering::Relaxed);
                    page = (page + 1) % 3;
                    CURRENT_PAGE.store(page, core::sync::atomic::Ordering::Relaxed);
                    defmt::debug!("Page changed to {}", page);
                }
                crate::components::ft3168::SwipeDirection::Right => {
                    // PREVIOUS PAGE
                    let mut page = CURRENT_PAGE.load(core::sync::atomic::Ordering::Relaxed);
                    page = (page + 2) % 3; // +2 === EQUIVALENT TO -1 IN MOD 3
                    CURRENT_PAGE.store(page, core::sync::atomic::Ordering::Relaxed);
                    defmt::debug!("Now on page: {}", page);
                }
                _ => {
                    // UP/DOWN/TAP – OTHER ACTIONS HERE
                }
            },
            TouchEvent::Tap { x, y } => {
                // OPTIONAL - HANDLE TAP (TOGGLE SOMETHING)
                defmt::debug!("Tap at ({},{}) – page unchanged", x, y);
            }
        }
    }
}
