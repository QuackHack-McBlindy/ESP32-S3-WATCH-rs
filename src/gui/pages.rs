// GUI/PAGES
// LISTEN FOR TOUCH EVENTS AND ACT UPON THEM


// ───────────────────────────────────────────────────────────────────────
// TOUCH TYPES FOR THE TOUCH TASK
const DOUBLE_TAP_MAX_DISTANCE: u16 = 50;
const DOUBLE_TAP_TIMEOUT: embassy_time::Duration = embassy_time::Duration::from_millis(80);

pub static TOUCH_EVENTS: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    TouchEvent,
> = embassy_sync::signal::Signal::new();

#[derive(Clone, Copy, defmt::Format)]
pub enum TouchEvent {
    Tap { x: u16, y: u16 },
    Swipe(crate::components::ft3168::SwipeDirection, u16, u16),
}


// ───────────────────────────────────────────────────────────────────────
// PAGE DEFINITIONS

crate::init_u8!(CURRENT_PAGE, 0);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Page {
    // NORMAL PAGES
    Clock       = 0,
    Battery     = 1,
    Apps        = 2,
    // APPLICATIONS
    MediaPlayer = 10,
    Settings    = 11,
    App3        = 12,
    House       = 13,
    // SPECIAL PAGES
    Call        = 100,
    Text        = 101,
}

impl Page {
    pub fn next_main(&self) -> Self {
        match self {
            Page::Clock   => Page::Battery,
            Page::Battery => Page::Apps,
            Page::Apps    => Page::Clock,
            _             => Page::Clock,
        }
    }

    pub fn prev_main(&self) -> Self {
        match self {
            Page::Clock   => Page::Apps,
            Page::Battery => Page::Clock,
            Page::Apps    => Page::Battery,
            _             => Page::Clock,
        }
    }

    pub fn from_raw(raw: u8) -> core::option::Option<Self> {
        match raw {
            0  => core::option::Option::Some(Page::Clock),
            1  => core::option::Option::Some(Page::Battery),
            2  => core::option::Option::Some(Page::Apps),
            10 => core::option::Option::Some(Page::MediaPlayer),
            11 => core::option::Option::Some(Page::Settings),
            12 => core::option::Option::Some(Page::App3),
            13 => core::option::Option::Some(Page::House),
            _  => core::option::Option::None,
        }
    }

    pub fn as_raw(self) -> u8 {
        self as u8
    }

    pub fn is_app_page(&self) -> bool {
        core::matches!(self, Page::MediaPlayer | Page::Settings | Page::App3 | Page::House)
    }
}


// ───────────────────────────────────────────────────────────────────────
// HELPERS FOR THIS MODULE
fn set_page(page: Page) {
    crate::store!(CURRENT_PAGE, page.as_raw());
}

fn current_page() -> Page {
    let raw = crate::load!(CURRENT_PAGE);
    Page::from_raw(raw).unwrap_or(Page::Clock)
}


// ───────────────────────────────────────────────────────────────────────
// TOUCH TASK
#[embassy_executor::task]
pub async fn touch_task() {
    // ONE-TIME INIT
    critical_section::with(|cs| {
        let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
        if let core::option::Option::Some(i2c_bus) = bus_ref.as_mut() {
            let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
            if let Err(e) = touch.init() {
                defmt::error!("Touch init failed: {:?}", e);
            }
        }
    });

    let mut tracking = false;
    let mut start_x = 0u16;
    let mut start_y = 0u16;
    let mut last_x = 0u16;
    let mut last_y = 0u16;

    // DOUBLE-TAP STATE
    let mut pending_tap: core::option::Option<(u16, u16, embassy_time::Instant, Page)> = core::option::Option::None;

    loop {
        let point = critical_section::with(|cs| {
            let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
            let i2c_bus = bus_ref.as_mut()?;
            let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
            touch.read().ok().flatten()
        });

        match point {
            core::option::Option::Some(tp) => {
                // NEW FINGER DOWN - CANCEL ANY PENDING TAP 
                pending_tap = core::option::Option::None;

                // WAKE DISPLAY IF NEEDED
                if !crate::load!(crate::state::DISPLAY_STATE) {
                    crate::components::co5300::wake_up();
                    crate::store!(crate::state::DISPLAY_STATE, true);
                }

                if !tracking {
                    tracking = true;
                    start_x = tp.x;
                    start_y = tp.y;
                }
                last_x = tp.x;
                last_y = tp.y;

                defmt::debug!("👆 X={} Y={}", tp.x, tp.y);
            }
            core::option::Option::None => {
                if tracking {
                    tracking = false;

                    // GESTURE DETECTION
                    let dx = last_x as i32 - start_x as i32;
                    let dy = last_y as i32 - start_y as i32;
                    let abs_dx = dx.unsigned_abs();
                    let abs_dy = dy.unsigned_abs();

                    let direction = if abs_dx < 30 && abs_dy < 30 {
                        crate::components::ft3168::SwipeDirection::Tap
                    } else if abs_dx > abs_dy * 3 / 2 {
                        if dx > 0 { crate::components::ft3168::SwipeDirection::Right } else { crate::components::ft3168::SwipeDirection::Left }
                    } else if abs_dy > abs_dx * 3 / 2 {
                        if dy > 0 { crate::components::ft3168::SwipeDirection::Down } else { crate::components::ft3168::SwipeDirection::Up }
                    } else {
                        crate::components::ft3168::SwipeDirection::Tap
                    };

                    let page = current_page();

                    match direction {
                        crate::components::ft3168::SwipeDirection::Tap => {
                        // ───────────────────────────────────────────────────────────────────────
                        // APP LAUNCHER PAGE
                        // ───────────────────────────────────────────────────────────────────────    
                            // TAP!
                            if page == Page::Apps {
                                // DOUBLE-TAP LOGIC FOR THE LAUNCHER
                                let now = embassy_time::Instant::now();
                                if let core::option::Option::Some((px, py, ptime, ppage)) = pending_tap {
                                    // CHECK DISTANCE & TIME
                                    let dist_x = (start_x as i32 - px as i32).unsigned_abs();
                                    let dist_y = (start_y as i32 - py as i32).unsigned_abs();
                                    if ppage == Page::Apps
                                        && dist_x <= DOUBLE_TAP_MAX_DISTANCE as u32
                                        && dist_y <= DOUBLE_TAP_MAX_DISTANCE as u32
                                        && now.duration_since(ptime) <= DOUBLE_TAP_TIMEOUT
                                    {
                                        // DOUBLE TAP DETECTED - LAUNCH APP
                                        crate::gui::apps::handle_double_tap(start_x, start_y);
                                        pending_tap = core::option::Option::None;
                                    } else {
                                        // NOT A DOUBLE TAP! DISPATCH THE OLD PENDING TAP (SINGLE TAP)
                                        if ppage == Page::Apps {
                                            crate::gui::apps::handle_tap();
                                        }
                                        // REPLACE WITH THE NEW PENDING TAP
                                        pending_tap = core::option::Option::Some((start_x, start_y, now, page));
                                    }
                                } else {
                                    // NO PENDING TAP YET – STORE THIS ONE
                                    pending_tap = core::option::Option::Some((start_x, start_y, now, page));
                                }
                            } else {     
                                // OTHER PAGES - SINGLE TAP IMMEDIATELY
                                match page {
                            // ───────────────────────────────────────────────────────────────────────
                            // MEDIA PLAYER PAGE
                            // ───────────────────────────────────────────────────────────────────────   
                                    Page::MediaPlayer => {
                                        if let core::option::Option::Some(action) = crate::gui::media_player::handle_touch(
                                            start_x as i32, start_y as i32
                                        ) {
                                            match action {
                                                // PREVIOUS TRACK ACTION
                                                crate::gui::TouchAction::MediaPrev => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        crate::state::MediaCommand::Prev as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                } // PLAY / PAUSE ACTION
                                                crate::gui::TouchAction::MediaPlayPause => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        crate::state::MediaCommand::PlayPause as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                } // NEXT TRACK ACTION
                                                crate::gui::TouchAction::MediaNext => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        crate::state::MediaCommand::Next as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                crate::gui::TouchAction::ZigbeeToggleLights => {
                                                    defmt::info!("TOGGLED LIGHTS HTTP POST");
                                                }
                                                crate::gui::TouchAction::OpenQwackify => {
                                                    defmt::info!("Open Qwackify app");
                                                    crate::applications::media_player::open_app();
                                                }
                                                crate::gui::TouchAction::OpenSettings => {
                                                    defmt::info!("Open settings app");
                                                    crate::applications::settings::open_app();
                                                }
                                                crate::gui::TouchAction::OpenApp3 => {
                                                    defmt::info!("Open app3 app");
                                                    crate::applications::app3::open_app();
                                                }
                                                crate::gui::TouchAction::OpenHouse => {
                                                    defmt::info!("Open House app");
                                                    crate::applications::house::open_app();
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {} // TODO: OTHER PAGES HERE
                                }
                            }
                        }
                        _ => {
                            // SWIPE HANDLING
                            // CANCEL ANY PENDING TAP WHEN A SWIPE OCCURS
                            pending_tap = core::option::Option::None;

                            defmt::debug!("Swipe {:?} – Start: ({},{}) -> End: ({},{})",
                                direction, start_x, start_y, last_x, last_y);

                            // SWIPING UP WHILE IN AN APPLICATION CLOSES THE APP
                            if page == Page::Apps {
                                crate::gui::apps::handle_swipe(direction);
                            } else if page.is_app_page() && direction == crate::components::ft3168::SwipeDirection::Up {
                                set_page(Page::Clock);
                                defmt::debug!("Closed app, back to clock page");
                            } else {
                                match direction {
                                    // SWIPING LEFT CHANGES PAGE
                                    crate::components::ft3168::SwipeDirection::Left => {
                                        let next = page.next_main();
                                        set_page(next);
                                        defmt::debug!("Page changed to {:?}", next);
                                    }
                                    // SWIPING RIGHT CHANGES PAGES
                                    crate::components::ft3168::SwipeDirection::Right => {
                                        let prev = page.prev_main();
                                        set_page(prev);
                                        defmt::debug!("Page changed to {:?}", prev);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // HANDLE PENDING TAP TIMEOUT 
        if let core::option::Option::Some((px, py, ptime, ppage)) = pending_tap {
            if embassy_time::Instant::now().duration_since(ptime) > DOUBLE_TAP_TIMEOUT {
                // TIME'S UP YO! FIRE A SINGLE TAP ON THE APP PAGE
                if ppage == Page::Apps && current_page() == Page::Apps {
                    crate::gui::apps::handle_tap();
                }
                pending_tap = core::option::Option::None;
            }
        }

        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
