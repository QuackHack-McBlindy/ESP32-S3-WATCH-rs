// GUI/PAGES
// THIS IS WHERE THE DEVICE DISPLAY PAGES ARE DEFINED
// AND HOME OF THE TOUCH CONTROLLER TASK THAT HANDLES THEM.
// TASK IS DESIGNED TO SLEEP WHEN NOT USED FOR A MASSIVE BATTERY REDUCTION COST 

use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};
use crate::components::ft3168::{Ft3168Touch, Gesture, SwipeDirection, TouchPoint};
// THIS SETS THE HOMESCREEN TO CLOCK PAGE
crate::init_u8!(CURRENT_PAGE, 1);

// HOW OFTEN I2C IS POLLED (WHEN FINGER IS ON SCREEN ONLY)
const TRACKING_POLL_INTERVAL: Duration = Duration::from_millis(10);

const DOUBLE_TAP_MAX_DISTANCE: u16 = 50;
const DOUBLE_TAP_TIMEOUT: Duration = Duration::from_millis(400);

// PIXELS FROM THE EDGE OF THE DISPLAY
const BOTTOM_SWIPE_THRESHOLD: u16 = 40;
const TOP_SWIPE_THRESHOLD: u16 = 40;

// ───────────────────────────────────────────────────────────────────────
// TOUCH TYPES FOR THE TOUCH TASK
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
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Page {
    // NORMAL PAGES
    Apps        = 0,
    Clock       = 1,
    Battery     = 2,
    Weather     = 3,
    // APPLICATIONS
    MediaPlayer = 10,
    DuckTv      = 11,
    House       = 12,
    DuckCloud   = 13,
    Settings    = 14,
    // SETTINGS APP
    SettingsWifi  = 140,
    SettingsRssi  = 141,
    SettingsBle   = 142,
    SettingsApi   = 143,
    SettingsWake  = 144,
    SettingsStream  = 145,
    SettingsSpeaker = 146,
    SettingsMic     = 147,
    SettingsDisplay = 148,
    SettingsTimeout = 149,
    SettingsAmp     = 150,
    SettingsCpu     = 151,    
    SettingsInfo    = 152,
    // SPECIAL PAGES
    Call        = 100,
    Text        = 101,
}

impl Page {
    pub fn next_main(&self) -> Self {
        match self {
            Page::Apps    => Page::Clock,
            Page::Clock   => Page::Battery,
            Page::Battery => Page::Weather,
            Page::Weather => Page::Weather, // STOP – NO FURTHER
            _             => Page::Clock,
        }
    }

    pub fn prev_main(&self) -> Self {
        match self {
            Page::Clock   => Page::Apps,
            Page::Battery => Page::Clock,
            Page::Weather => Page::Battery,
            Page::Apps    => Page::Apps, // STOP – NO FURTHER
            _             => Page::Clock,
        }
    }
    
    pub fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            // NORMAL PAGES
            0 => Some(Page::Apps),
            1 => Some(Page::Clock),
            2 => Some(Page::Battery),
            3 => Some(Page::Weather),
            // APPLICATIONS
            10 => Some(Page::MediaPlayer),
            11 => Some(Page::DuckTv),
            12 => Some(Page::House),
            13 => Some(Page::DuckCloud),
            14 => Some(Page::Settings),
            //  SUBPAGES
            140 => Some(Page::SettingsWifi),
            141 => Some(Page::SettingsRssi),
            142 => Some(Page::SettingsBle),
            143 => Some(Page::SettingsApi),
            144 => Some(Page::SettingsWake),
            145 => Some(Page::SettingsStream),
            146 => Some(Page::SettingsSpeaker),
            147 => Some(Page::SettingsMic),
            148 => Some(Page::SettingsDisplay),
            149 => Some(Page::SettingsTimeout),
            150 => Some(Page::SettingsAmp),
            151 => Some(Page::SettingsCpu),            
            152 => Some(Page::SettingsInfo),
            _   => None,
        }
    }
    
    pub fn as_raw(self) -> u8 {
        self as u8
    }

    pub fn is_app_page(&self) -> bool {
        core::matches!(
            self,
            Page::MediaPlayer
                | Page::DuckTv
                | Page::House
                | Page::DuckCloud
                | Page::Settings
                | Page::SettingsWifi
                | Page::SettingsRssi
                | Page::SettingsBle
                | Page::SettingsApi
                | Page::SettingsWake
                | Page::SettingsStream
                | Page::SettingsSpeaker
                | Page::SettingsMic
                | Page::SettingsDisplay
                | Page::SettingsTimeout
                | Page::SettingsAmp
                | Page::SettingsCpu                
                | Page::SettingsInfo
        )
    }

    pub fn is_settings_page(&self) -> bool {
        core::matches!(
            self,
            Page::Settings
                | Page::SettingsWifi
                | Page::SettingsRssi
                | Page::SettingsBle
                | Page::SettingsApi
                | Page::SettingsWake
                | Page::SettingsStream
                | Page::SettingsSpeaker
                | Page::SettingsMic
                | Page::SettingsDisplay
                | Page::SettingsTimeout
                | Page::SettingsAmp
                | Page::SettingsCpu
                | Page::SettingsInfo
        )
    }

    pub fn next_setting(&self) -> Self {
        match self {
            Page::Settings          => Page::SettingsWifi,
            Page::SettingsWifi      => Page::SettingsRssi,
            Page::SettingsRssi      => Page::SettingsBle,
            Page::SettingsBle       => Page::SettingsApi,
            Page::SettingsApi       => Page::SettingsWake,
            Page::SettingsWake      => Page::SettingsStream,
            Page::SettingsStream    => Page::SettingsSpeaker,
            Page::SettingsSpeaker   => Page::SettingsMic,
            Page::SettingsMic       => Page::SettingsDisplay,
            Page::SettingsDisplay   => Page::SettingsTimeout,
            Page::SettingsTimeout   => Page::SettingsAmp,
            Page::SettingsAmp       => Page::SettingsCpu,
            Page::SettingsCpu       => Page::SettingsInfo,
            Page::SettingsInfo      => Page::SettingsWifi,
            _                       => Page::Clock,
        }
    }

    pub fn prev_setting(&self) -> Self {
        match self {
            Page::SettingsWifi      => Page::SettingsInfo,
            Page::SettingsRssi      => Page::SettingsWifi,
            Page::SettingsBle       => Page::SettingsRssi,
            Page::SettingsApi       => Page::SettingsBle,
            Page::SettingsWake      => Page::SettingsApi,
            Page::SettingsStream    => Page::SettingsWake,
            Page::SettingsSpeaker   => Page::SettingsStream,
            Page::SettingsMic       => Page::SettingsSpeaker,
            Page::SettingsDisplay   => Page::SettingsMic,
            Page::SettingsTimeout   => Page::SettingsDisplay,
            Page::SettingsAmp       => Page::SettingsTimeout,
            Page::SettingsCpu       => Page::SettingsAmp,
            Page::SettingsInfo      => Page::SettingsCpu,
            _                       => Page::Clock,
        }
    }
}    

// ───────────────────────────────────────────────────────────────────────
// INTERNAL HELPERS FOR THIS MODULE
fn set_page(page: Page) {
    crate::store!(CURRENT_PAGE, page.as_raw());
}

fn current_page() -> Page {
    let raw = crate::load!(CURRENT_PAGE);
    Page::from_raw(raw).unwrap_or(Page::Clock)
}

// ───────────────────────────────────────────────────────────────────────
// PROCESS A FINISHED GESTURE (SWIPE OR TAP). ON LAUNCHER A TAP IS STORED AS PENDING,
// OTHER PAGES IT'S INSTANT!
async fn process_gesture(
    start_x: u16,
    start_y: u16,
    last_x: u16,
    last_y: u16,
    pending_tap: &mut Option<(u16, u16, Instant, Page)>,
) {
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
            if page == Page::Apps {
                // STORE AS PENDING INSTEAD OF EXECUTING INSTANTLY.
                *pending_tap = Some((start_x, start_y, Instant::now(), page));
                defmt::debug!("Tap pending…");
            } else {
                handle_page_tap(page, start_x, start_y).await;
            }
        }
        _ => {
            // ANY SWIPE CANCELS A PENDING TAP
            *pending_tap = None;
            defmt::debug!(
                "Swipe {:?} – Start: ({},{}) -> End: ({},{})",
                direction, start_x, start_y, last_x, last_y
            );
            handle_swipe(direction, page, start_x, start_y, last_x, last_y);
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// READ TOUCH
// RETURNS `Some(TouchPoint)` IF A FINGER IS ON THE PANEL OTHERWISE `None`
async fn read_touch() -> Option<TouchPoint> {
    critical_section::with(|cs| {
        let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
        let i2c_bus = bus_ref.as_mut()?;
        let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
        touch.read().ok().flatten()
    })
}


// ───────────────────────────────────────────────────────────────────────
// HANDLE PAGE TAPS
async fn handle_page_tap(page: Page, x: u16, y: u16) {
    // IF CONTROL CENTER IS VISIBLE - ALL TAPS GO THERE
    if crate::gui::control_center::is_visible() {
        if let Some(action) = crate::gui::control_center::handle_touch(x as i32, y as i32) {
            match action {
                // WIFI TOGGLE
                crate::gui::TouchAction::ControlCenterBox1 => {
                    crate::base::wifi::toggle_wifi().await;
                    defmt::info!("CONTROL CENTER BOX1 PRESSED (WIFI)");
                }
                //  TOGGLES EVERYTHING REQUIRED FOR VOICE ASSISTANT (AMP, SPEAKER TASKS, MIC, ...)
                crate::gui::TouchAction::ControlCenterBox2 => {
                    crate::base::routes::api::settings::voice::state::toggle_voice().await;
                    defmt::info!("CONTROL CENTER BOX2 PRESSED (VOICE ASSISTANT TOGGLE)");
                }
                // API TOGGLE
                crate::gui::TouchAction::ControlCenterBox3 => {
                    crate::base::routes::api::settings::api::off::toggle_api().await;
                    defmt::info!("CONTROL CENTER BOX3 PRESSED");
                }
                // OPEN SETTINGS APP
                crate::gui::TouchAction::ControlCenterBox4 => {                    
                    defmt::info!("CONTROL CENTER BOX4 PRESSED (SETTINGS)");
                    crate::store!(CURRENT_PAGE, 140);
                    crate::gui::control_center::close();
                }                    
                _ => {}
            }
        }
        return;
    }
    match page {
        // ───────────────────────────────────────────────────────────────────────
        // WEATHER PAGE
        Page::Weather => {
            crate::gui::weather::handle_touch(x as i32, y as i32);
        }
        // ───────────────────────────────────────────────────────────────────────
        // SETTOMGS PAGE
        Page::Settings => {
            let _ = crate::gui::settings::handle_touch(x as i32, y as i32);
        }
        // WIFI
        Page::SettingsWifi => {
            if let core::option::Option::Some(action) = crate::gui::options::wifi::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggle => {
                        crate::base::wifi::toggle_wifi().await;
                        defmt::info!("Wifi TOGGLED");
                    }
                    _ => {}
                }
            }
        } 
        // BLUETOOTH
        Page::SettingsBle => {
            if let core::option::Option::Some(action) = crate::gui::options::bluetooth::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleBle => {
                        defmt::info!("BLE TOGGLED");
                        crate::swap!(crate::state::BLUETOOTH_STATE);
                    }
                    _ => {}
                }
            }
        }         
        // API
        Page::SettingsApi => {
            if let core::option::Option::Some(action) = crate::gui::options::api::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleApi => {
                        crate::base::routes::api::settings::api::off::toggle_api().await;
                        defmt::info!("API TOGGLED");
                    }
                    _ => {}
                }
            }
        }         
        // MIC
        Page::SettingsMic => {
            if let core::option::Option::Some(action) = crate::gui::options::mic::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleMic => {
                        defmt::info!("MIC TOGGLED");
                    }
                    _ => {}
                }
            }
        } 
        // SPEAKER
        Page::SettingsSpeaker => {
            if let core::option::Option::Some(action) = crate::gui::options::speaker::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleSpeaker => {
                        defmt::info!("SPEAKER TOGGLED");
                    }
                    _ => {}
                }
            }
        }         
        // WAKE WORD
        Page::SettingsWake => {
            if let core::option::Option::Some(action) = crate::gui::options::wakeword::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleWakeWord => {
                        defmt::info!("WAKE WORD TOGGLED");
                        crate::base::routes::api::settings::voice::wakeword::toggle_wake_word().await;
                    }
                    _ => {}
                }
            }
        }  
        // DISPLAY
        Page::SettingsDisplay => {
            if let core::option::Option::Some(action) = crate::gui::options::display::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleDisplay => {
                        defmt::info!("DISPLAY TOGGLED");
                        crate::swap!(crate::state::DISPLAY_STATE);
                    }
                    _ => {}
                }
            }
        } 
        // AUDIO STREAMING
        Page::SettingsStream => {
            if let core::option::Option::Some(action) = crate::gui::options::streaming::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleStreaming => {
                        crate::base::routes::api::settings::speaker::stream::toggle_stream().await;
                        defmt::info!("STREAMING TOGGLED");
                    }
                    _ => {}
                }
            }
        } 
        // AMP
        Page::SettingsAmp => {
            if let core::option::Option::Some(action) = crate::gui::options::amplifier::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    crate::gui::TouchAction::SettingsToggleAmp => {
                        crate::base::routes::api::settings::speaker::amp::toggle_amp();
                        defmt::info!("AMP TOGGLED");
                    }
                    _ => {}
                }
            }
        }
       
        // ───────────────────────────────────────────────────────────────────────
        // MEDIA PLAYER PAGE        
        Page::MediaPlayer => {
            if let core::option::Option::Some(action) = crate::gui::media_player::handle_touch(
                x as i32, y as i32
            ) {
                match action {
                    // PREVIOUS TRACK
                    crate::gui::TouchAction::MediaPrev => {
                        crate::state::MEDIA_COMMAND.store(
                            crate::state::MediaCommand::Prev as u8,
                            core::sync::atomic::Ordering::Relaxed,
                        );
                    } // PLAY/PAUSE
                    crate::gui::TouchAction::MediaPlayPause => {
                        crate::state::MEDIA_COMMAND.store(
                            crate::state::MediaCommand::PlayPause as u8,
                            core::sync::atomic::Ordering::Relaxed,
                        );
                    } // NEXT TRACK
                    crate::gui::TouchAction::MediaNext => {
                        crate::state::MEDIA_COMMAND.store(
                            crate::state::MediaCommand::Next as u8,
                            core::sync::atomic::Ordering::Relaxed,
                        );
                    } // HEART CURRENTLY PLAYING SONG
                    crate::gui::TouchAction::MediaHeart => {
                        let is_liked = crate::load!(crate::state::MEDIA_IS_LIKED);
                        crate::store!(crate::state::MEDIA_IS_LIKED, !is_liked);
                        defmt::info!("MEDIA HEART toggled to {}", !is_liked);
                    } // CLEAR PLAYLIST
                    crate::gui::TouchAction::MediaClear => {
                        crate::components::storage::clear_playlist();
                        defmt::info!("MEDIA CLEAR");
                    } // SPLIT MEDIA PLAYER VIEW
                    crate::gui::TouchAction::MediaSplitView => {
                        if crate::gui::media_player::is_split_open() {
                            crate::gui::media_player::close_split();
                        } else {
                            crate::gui::media_player::open_split();
                        }
                        defmt::info!("SPLITTING MEDIA PLAYER VIEW!");
                    }                    
                    crate::gui::TouchAction::ZigbeeToggleLights => {
                        defmt::info!("TOGGLED LIGHTS HTTP POST");
                    }
                    crate::gui::TouchAction::OpenQwackify => {
                        defmt::info!("Opening app: Qwackify");
                        crate::applications::media_player::open_app();
                    }
                    crate::gui::TouchAction::OpenSettings => {
                        defmt::info!("Opening app: Settings");
                        crate::applications::settings::open_app();
                    }
                    crate::gui::TouchAction::OpenDuckTv => {
                        defmt::info!("Opening app: duck-TV");
                        crate::applications::duck_tv::open_app();
                    }
                    crate::gui::TouchAction::OpenHouse => {
                        defmt::info!("Opening app: House");
                        crate::applications::house::open_app();
                    }
                    
                    _ => {}
                }
            }
        }
        // ...
        _ => {}
    }
}


// ───────────────────────────────────────────────────────────────────────
// HANDLE SETTINGS SWIPES (VOLUME CONTROL, ETC.)
fn handle_settings_swipe(
    page: Page,
    direction: SwipeDirection,
    start_x: u16,
    start_y: u16,
    last_x: u16,
    last_y: u16,
) {
    match page {
        Page::SettingsSpeaker => crate::gui::options::speaker::handle_swipe(direction, start_x, start_y, last_x, last_y),
        Page::SettingsMic     => crate::gui::options::mic::handle_swipe(direction, start_x, start_y, last_x, last_y),
        Page::SettingsDisplay => crate::gui::options::display::handle_swipe(direction, start_x, start_y, last_x, last_y),
        Page::SettingsCpu     => crate::gui::options::cpu::handle_swipe(direction, start_x, start_y, last_x, last_y),        
        _ => {}
    }
}

// ───────────────────────────────────────────────────────────────────────
// TRY TO RETURN TO HOMESCREEN
// SWIPING UP FROM THE VERY BOTTOM OF THE SCREEN CLOSES APP AND RETURNS TO CLOCK SCREEN
fn try_return_home(start_y: u16) -> bool {
    if start_y >= crate::state::LCD_HEIGHT - BOTTOM_SWIPE_THRESHOLD {
        defmt::info!("Closed app, back to clock page");
        set_page(Page::Clock);
        true
    } else { false }
}

// WE REVERSE THAT AND CREATE A CONTROL CENTER
// SLIDE DOWN FROM TOP - ONLY ON CLOCK SCREEN
// TRY TO CONTROL CENTER ON HOME
fn try_to_control_center_on_home(start_y: u16) -> bool {
    if start_y <= TOP_SWIPE_THRESHOLD {
        crate::gui::control_center::open();
        defmt::info!("Opening control center");
        true
    } else { false }
}


// ───────────────────────────────────────────────────────────────────────
// HANDLE SWIPE
fn handle_swipe(direction: SwipeDirection, page: Page, start_x: u16, start_y: u16, last_x: u16, last_y: u16) {
    // OPEN CONTROL CENTER FROM ANY PAGE (SWIPE DOWN FROM TOP)
    if direction == SwipeDirection::Down
        && !crate::gui::control_center::is_visible()
        && start_y <= TOP_SWIPE_THRESHOLD
    {
        crate::gui::control_center::open();
        defmt::info!("Opening control center");
        return;
    }

    // LAUNCHER - FORWARD UP/DOWN/TAP TO IT'S OWN HANDLER
    if page == Page::Apps {
        if direction != SwipeDirection::Left && direction != SwipeDirection::Right {
            crate::gui::apps::handle_swipe(direction);
            return;
        }
    }

    // IF CONTROL CENTER IS VISIBLE, SWIPE UP CLOSES IT (ANYWHERE)
    if crate::gui::control_center::is_visible() && direction == SwipeDirection::Up {
        crate::gui::control_center::close();
        defmt::debug!("Closing control center");
        return;
    }

    // CLOSE MEDIA PLAYER SPLIT-VIEW (PLAYLIST) FROM BOTTOM SWIPE
    if page == Page::MediaPlayer
        && direction == SwipeDirection::Up
        && crate::gui::media_player::is_split_open()
        && start_y >= crate::state::LCD_HEIGHT - BOTTOM_SWIPE_THRESHOLD
    {
        crate::gui::media_player::close_split();
        defmt::info!("Closed split view, staying on Media Player");
        return;
    }

    // APP PAGES – SWIPE UP FROM BOTTOM CLOSE APP & GO HOME
    if page.is_app_page() && direction == SwipeDirection::Up {
        if try_return_home(start_y) {
            return;
        }
        if !page.is_settings_page() {
            return;
        }
    }

    // SETTINGS PAGES LEFT/RIGHT NAVIGATION
    // + UP/DOWN DELEGATION
    if page.is_settings_page() {
        // INFO PAGE – ALL SWIPES GO THROUGH IT'S OWN HANDLER FIRST
        if page == Page::SettingsInfo {
            let consumed = crate::gui::options::info::handle_swipe(direction, start_x, start_y, last_x, last_y);
            if consumed {
                return;
            }
        }

        match direction {
            SwipeDirection::Left => {
                let next = page.next_setting();
                set_page(next);
                defmt::debug!("Settings page changed to {:?}", next);
            }
            SwipeDirection::Right => {
                let prev = page.prev_setting();
                set_page(prev);
                defmt::info!("Settings page changed to {:?}", prev);
            }
            SwipeDirection::Up | SwipeDirection::Down => {
                handle_settings_swipe(page, direction, start_x, start_y, last_x, last_y);
            }
            _ => {}
        }
        return;
    }

    // MAIN NAVIGATION
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


// ───────────────────────────────────────────────────────────────────────
// TOUCH TASK
// TASK SLEEPS UNTIL DISPLAY IS TURNED ON & FINGER IS ON SCREEN
#[embassy_executor::task]
pub async fn touch_task(mut touch_int: esp_hal::gpio::Input<'static>) {
    let mut tracking = false;
    let mut start_x = 0u16;
    let mut start_y = 0u16;
    let mut last_x = 0u16;
    let mut last_y = 0u16;

    // STORE STATE OF LAST TAP
    let mut pending_tap: Option<(u16, u16, Instant, Page)> = None;

    loop {
        // 1: WAIT UNTIL DISPLAY IS ON AND TOUCH IS DETECTED.
        if !crate::load!(crate::state::DISPLAY_STATE) {
            touch_int.wait_for_low().await;
            crate::components::co5300::wake_up();
            crate::store!(crate::state::DISPLAY_STATE, true);
            crate::DISPLAY_CMD.send(crate::DisplayCommand::Start).await;
            crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
            crate::dirty!();
            Timer::after(Duration::from_millis(50)).await;
            continue;
        }

        // 2: IF WE HAVE A PENDING TAP, WE MUST RACE THE TIMEOUT AGAINST NEXT TOUCH
        if let Some((px, py, ptime, ppage)) = pending_tap {
            let elapsed = Instant::now().duration_since(ptime);
            // TIMEOUT PASSED
            let remaining = if elapsed < DOUBLE_TAP_TIMEOUT {
                DOUBLE_TAP_TIMEOUT - elapsed
            } else {
                Duration::from_millis(0)
            };

            match select(
                touch_int.wait_for_low(),
                Timer::after(remaining),
            ).await {
                Either::First(_) => {
                    // SECOND FINGER LANDED WITHIN THE TIMEOUT
                    // TRACK IT
                    tracking = true;
                    let fp = read_touch().await;
                    if let Some(tp) = fp {
                        start_x = tp.x;
                        start_y = tp.y;
                        last_x = tp.x;
                        last_y = tp.y;
                    }

                    // POLL I2C UNTIL FINGER LIFTED
                    loop {
                        if touch_int.is_low() {
                            if let Some(tp) = read_touch().await {
                                last_x = tp.x;
                                last_y = tp.y;
                            }
                            Timer::after(TRACKING_POLL_INTERVAL).await;
                            crate::dirty!();
                            continue;
                        }
                        if tracking {
                            let final_point = read_touch().await;
                            if final_point.is_none() {
                                // LIFTOFF CONFIRMED! CHECK IF DOUBLE-TAP?
                                tracking = false;
                                let dx = (last_x as i32 - px as i32).unsigned_abs();
                                let dy = (last_y as i32 - py as i32).unsigned_abs();
                                let time_ok = Instant::now().duration_since(ptime) <= DOUBLE_TAP_TIMEOUT;
                                let dist_ok = dx <= DOUBLE_TAP_MAX_DISTANCE as u32 && dy <= DOUBLE_TAP_MAX_DISTANCE as u32;
                                
                                // IT'S A DOUBLE-TAP (ON LAUNCHER)
                                if time_ok && dist_ok && ppage == Page::Apps && current_page() == Page::Apps {
                                    defmt::info!("👆👆");
                                    crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                                    
                                    crate::gui::apps::handle_double_tap(start_x, start_y);
                                } else {
                                    // IT'S NOT A DOUBLE-TAP - FIRE PENDING SINGLE TAP!
                                    // & MAKE THIS SECOND TAP A NEW PENDING TAP.
                                    if ppage == Page::Apps && current_page() == Page::Apps {
                                        defmt::debug!("👆");
                                        crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                                        crate::gui::apps::handle_tap();
                                    }
                                    // STORE AS PENDING
                                    let now = Instant::now();
                                    let page = current_page();
                                    pending_tap = Some((start_x, start_y, now, page));
                                    continue; // NOW BACK TO THE RACE!
                                }
                                pending_tap = None;
                            } else {
                                last_x = final_point.unwrap().x;
                                last_y = final_point.unwrap().y;
                                Timer::after(TRACKING_POLL_INTERVAL).await;
                                continue;
                            }
                        }
                        break;
                    }
                    // AFTER SECOND TAP HANDLED CONTINUE MAIN LOOP AGAIN
                    continue;
                }
                Either::Second(_) => {
                    // TIMEOUT WON THE RACE – NO SECOND TAP
                    if ppage == Page::Apps && current_page() == Page::Apps {
                        defmt::info!("👆");
                        crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                        crate::gui::apps::handle_tap();
                    }
                    pending_tap = None;
                    // FRESH RESTART (await new touch) BACK TO MAIN LOOP
                    continue;
                }
            }
        }

        // 3: NO PENDING TAP – WAIT FOR A FRESH START
        touch_int.wait_for_low().await;

        tracking = true;        
        let first_point = read_touch().await;
        crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
        if let Some(tp) = first_point {
            start_x = tp.x;
            start_y = tp.y;
            last_x = tp.x;
            last_y = tp.y;
            defmt::debug!("👆 START X={} Y={}", tp.x, tp.y);
        }

        // 4: POLL UNTIL FINGER LIFTOFF
        loop {
            if touch_int.is_low() {
                crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                if let Some(tp) = read_touch().await {
                    last_x = tp.x;
                    last_y = tp.y;
                    defmt::debug!("👆 MOVE X={} Y={}", tp.x, tp.y);
                }
                crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);

                Timer::after(TRACKING_POLL_INTERVAL).await;
                continue;
            }
            if tracking {
                let final_point = read_touch().await;
                if final_point.is_none() {
                    tracking = false;
                    // GESTURE COMPLETED - SEND FOR PROCESSING
                    process_gesture(
                        start_x, start_y, last_x, last_y,
                        &mut pending_tap,
                    ).await;
                } else {
                    last_x = final_point.unwrap().x;
                    last_y = final_point.unwrap().y;
                    Timer::after(TRACKING_POLL_INTERVAL).await;
                    continue;
                }
            }
            break;
        }
    }
}
