// GUI/PAGES
// LISTEN FOR TOUCH EVENTS AND ACT UPON THEM

use embassy_time::Instant;
use crate::applications::APPS;
use crate::components::ft3168::SwipeDirection;
use crate::gui::TouchAction;
use crate::state::MediaCommand;
use crate::I2C_BUS;


crate::init_u8!(CURRENT_PAGE, 0);

const DOUBLE_TAP_MAX_DISTANCE: u16 = 50;
const DOUBLE_TAP_TIMEOUT: embassy_time::Duration = embassy_time::Duration::from_millis(80);

pub static TOUCH_EVENTS: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    TouchEvent,
> = embassy_sync::signal::Signal::new();

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Page {
    Clock       = 0,
    Battery     = 1,
    Apps        = 2,
    MediaPlayer = 10,
    App2        = 11,
    App3        = 12,
    House       = 13,
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

    pub fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0  => Some(Page::Clock),
            1  => Some(Page::Battery),
            2  => Some(Page::Apps),
            10 => Some(Page::MediaPlayer),
            11 => Some(Page::App2),
            12 => Some(Page::App3),
            13 => Some(Page::House),
            _  => None,
        }
    }

    pub fn as_raw(self) -> u8 {
        self as u8
    }

    pub fn is_app_page(&self) -> bool {
        matches!(self, Page::MediaPlayer | Page::App2 | Page::App3 | Page::House)
    }
}

#[derive(Clone, Copy, defmt::Format)]
pub enum TouchEvent {
    Tap { x: u16, y: u16 },
    Swipe(SwipeDirection, u16, u16),
}

fn set_page(page: Page) {
    crate::store!(CURRENT_PAGE, page.as_raw());
}

fn current_page() -> Page {
    let raw = crate::load!(CURRENT_PAGE);
    Page::from_raw(raw).unwrap_or(Page::Clock)
}


#[embassy_executor::task]
pub async fn touch_task() {
    // ONE-TIME INIT
    critical_section::with(|cs| {
        let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
        if let Some(i2c_bus) = bus_ref.as_mut() {
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
    let mut pending_tap: Option<(u16, u16, Instant, Page)> = None;

    loop {
        let point = critical_section::with(|cs| {
            let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
            let i2c_bus = bus_ref.as_mut()?;
            let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
            touch.read().ok().flatten()
        });

        match point {
            Some(tp) => {
                // NEW FINGER DOWN - CANCEL ANY PENDING TAP 
                pending_tap = None;

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
            None => {
                if tracking {
                    tracking = false;

                    //  GESTURE DETECTION
                    let dx = last_x as i32 - start_x as i32;
                    let dy = last_y as i32 - start_y as i32;
                    let abs_dx = dx.unsigned_abs();
                    let abs_dy = dy.unsigned_abs();

                    let direction = if abs_dx < 30 && abs_dy < 30 {
                        SwipeDirection::Tap
                    } else if abs_dx > abs_dy * 3 / 2 {
                        if dx > 0 { SwipeDirection::Right } else { SwipeDirection::Left }
                    } else if abs_dy > abs_dx * 3 / 2 {
                        if dy > 0 { SwipeDirection::Down } else { SwipeDirection::Up }
                    } else {
                        SwipeDirection::Tap
                    };

                    let page = current_page();

                    match direction {
                        SwipeDirection::Tap => {
                            // TAP!
                            if page == Page::Apps {
                                // DOUBLE-TAP LOGIC FOR  THE LAUNCHER
                                let now = Instant::now();
                                if let Some((px, py, ptime, ppage)) = pending_tap {
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
                                        pending_tap = None;
                                    } else {
                                        // NOT A DOUBLE TAP! DISPATCH THE OLD PENDING TAP (SINGLE TAP)
                                        if ppage == Page::Apps {
                                            crate::gui::apps::handle_tap();
                                        }
                                        // REPLACE WITH THE NEW PENDING TAP
                                        pending_tap = Some((start_x, start_y, now, page));
                                    }
                                } else {
                                    // NO PENDING TAP YET – STORE THIS ONE
                                    pending_tap = Some((start_x, start_y, now, page));
                                }
                            } else {
                                // OTHER PAGES - SINGLE TAP IMMEDIATELY
                                match page {
                                    Page::MediaPlayer => {
                                        if let Some(action) = crate::gui::media_player::handle_touch(
                                            start_x as i32, start_y as i32
                                        ) {
                                            match action {
                                                TouchAction::MediaPrev => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        MediaCommand::Prev as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                TouchAction::MediaPlayPause => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        MediaCommand::PlayPause as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                TouchAction::MediaNext => {
                                                    crate::state::MEDIA_COMMAND.store(
                                                        MediaCommand::Next as u8,
                                                        core::sync::atomic::Ordering::Relaxed,
                                                    );
                                                }
                                                TouchAction::ZigbeeToggleLights => {
                                                    defmt::info!("TOGGLED LIGHTS HTTP POST");
                                                }
                                                TouchAction::OpenQwackify => {
                                                    defmt::info!("Open Qwackify app");
                                                    crate::applications::media_player::open_app();
                                                }
                                                TouchAction::OpenApp2 => {
                                                    defmt::info!("Open app2 app");
                                                    crate::applications::app2::open_app();
                                                }
                                                TouchAction::OpenApp3 => {
                                                    defmt::info!("Open app3 app");
                                                    crate::applications::app3::open_app();
                                                }
                                                TouchAction::OpenHouse => {
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
                            pending_tap = None;

                            defmt::debug!("Swipe {:?} – Start: ({},{}) -> End: ({},{})",
                                direction, start_x, start_y, last_x, last_y);

                            if page == Page::Apps {
                                crate::gui::apps::handle_swipe(direction);
                            } else if page.is_app_page() && direction == SwipeDirection::Up {
                                set_page(Page::Clock);
                                defmt::debug!("Closed app, back to clock page");
                            } else {
                                match direction {
                                    SwipeDirection::Left => {
                                        let next = page.next_main();
                                        set_page(next);
                                        defmt::debug!("Page changed to {:?}", next);
                                    }
                                    SwipeDirection::Right => {
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
        if let Some((px, py, ptime, ppage)) = pending_tap {
            if Instant::now().duration_since(ptime) > DOUBLE_TAP_TIMEOUT {
                // TIME'S UP YO! FIRE A SINGLE TAP ON THE APP PAGE
                if ppage == Page::Apps && current_page() == Page::Apps {
                    crate::gui::apps::handle_tap();
                }
                pending_tap = None;
            }
        }

        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
