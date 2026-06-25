// ★ ─────────────────────────────────────────────────────────────────────── ★
//! ESP32-S3-WATCH-rs ⮞ https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs
//!  BARE METAL RUST  - HARDWARE ABSTRACTION LAYER: `esp-hal`
//!   SMARTWATCH OS   - BY QuackHack-McBLindy 🦆🧑‍🦯
// ★ ─────────────────────────────────────────────────────────────────────── ★
//! “A powerful voice assistant can make a huge difference for blind people.
//!   Imagine yourself stumbling blindly across the room looking for the TV remote,
//!   meanwhile, I call the remote using only my voice.
//!   Just to find it and throw it out the window -- because I won't ever need it.“
// ★ ─────────────────────────────────────────────────────────────────────── ★

#![no_std]
#![no_main]

#![allow(
    non_snake_case,
    dead_code,
    unused,
    private_interfaces,
    clippy::large_stack_frames,
    reason = "NOBODY TELLS ME WHAT TO DO!"
)]

use esp_println as _;


// PANIC HANDLER
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("⚠️ PANIC: {}", defmt::Debug2Format(info));
    defmt::error!("⚠️ REBOOT DEVICE!");
    loop {} // REBOOT BY HOLDING POWER BUTTON!
}


// MEMORY
extern crate alloc;

// BOOTLOADER (REQUIRED TO BOOT WITHOUT ESP-IDF)
esp_bootloader_esp_idf::esp_app_desc!();

// LOAD MODULES
mod state;
mod components;
mod base;
mod gui;
mod applications;


// TYPE ALIASES
pub type I2cBus = esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>;

#[derive(defmt::Format)]
pub enum DisplayCommand {
    Start,
    Stop,
}


// SHARED RESOURCES
static WG_CONFIG: static_cell::StaticCell<crate::base::wireguard::WgConfig> = static_cell::StaticCell::new();
pub static TOUCH_CHANNEL: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::components::ft3168::SwipeDirection, 5> = embassy_sync::channel::Channel::new();
pub static ES7210: critical_section::Mutex<core::cell::RefCell<Option<es7210::Es7210>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static ES8311: critical_section::Mutex<core::cell::RefCell<Option<es8311::Es8311>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static I2C_BUS: critical_section::Mutex<core::cell::RefCell<Option<I2cBus>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static PMU: critical_section::Mutex<core::cell::RefCell<Option<crate::components::axp2101::Axp2101>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static AMP_PIN: critical_section::Mutex<core::cell::RefCell<core::option::Option<esp_hal::gpio::Output<'static>>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(core::option::Option::None));
pub static DISPLAY_CMD: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, DisplayCommand, 1> = embassy_sync::channel::Channel::new();
static mut VPN_STACK: Option<&'static embassy_net::Stack<'static>> = None;
pub static VPN_STACK_READY: embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    (),
> = embassy_sync::signal::Signal::new();

// ───────────────────────────────────────────────────────────────────────
// CONSTRUCT THE VOICE HANDLER
struct VoiceHandler;

// CONFIGURE VOICE EVENTS (WAKE-WORD DETECTION)
// BACKEND RESPONDS WITH SINGLE-BYTE EVENT CODES
// FOR A LOW-OVERHEAD EMBEDDED NETWORK PROTOCOL
impl yo_esp::CommandHandler for VoiceHandler {
    // 0x01 === WAKE WORD DETECTED
    fn on_detected(&mut self) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async {
            // TURN ON DISPLAY & PLAY SOUND
            crate::components::co5300::wake_up();
            yo_esp::play_ding().await;            
        })
    }

    // 0x02 === SERVER STARTED TRANSCRIPTION
    fn on_thinking(&mut self) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async {
            // FLASH DISPLAY WHILE “THINKING“
            crate::components::co5300::start_flash();
        })
    }

    // 0x03 === COMMAND EXECUTED SUCCESSFULLY
    fn on_executed(&mut self, _ms: Option<u64>) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async move {
            // PLAY SUCCESS SOUND & TURN OFF DISPLAY
            crate::components::co5300::stop_flash();
            yo_esp::play_done().await;
            crate::components::co5300::sleep_now();
        })
    }

    // 0x04 === FAILED COMMAND EXECUTION
    fn on_failed(&mut self, _ms: Option<u64>) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async move {
            // LET THE 🦆 SAY “FUCK!“ & TURN OFF THE DISPLAY
            crate::components::co5300::stop_flash();
            yo_esp::play_fail().await;
            crate::components::co5300::sleep_now();
        })
    }
}



// ───────────────────────────────────────────────────────────────────────
// DISPLAY CONTROLLER TASK – BOOTS DISABLED
#[embassy_executor::task]
async fn display_task(
    mut fb: crate::components::framebuffer::Framebuffer,
    mut display: crate::components::co5300::Co5300Display<'static>,
    te: esp_hal::gpio::Input<'static>,
) {
    display.display_off();

    // PREVIOUS OVERLAY OFFSET 
    let mut prev_overlay_offset: i32 = i32::MIN;
    let mut was_media_playing = false;

    loop {
        // IDLE – SCREEN OFF, WAITING FOR START COMMAND
        display.display_off();
        crate::store!(crate::state::DISPLAY_STATE, false);

        let cmd = crate::DISPLAY_CMD.receive().await;
        match cmd {
            DisplayCommand::Start => { /* PROCEED! */ },
            DisplayCommand::Stop => { continue; },
        }

        // SCREEN ON – START RENDERING LOOP
        display.display_on();
        crate::store!(crate::state::DISPLAY_STATE, true);
        crate::gui::control_center::close();
        prev_overlay_offset = i32::MIN; // RESET AFTER SCREEN OFF/ON

        // STATE VARIABLES
        // LOW DEFAULT BRIGHTNESS (35%) DON'T HURT 🦆 BLIND EYES & SAVES BATTERY 
        let mut current_brightness = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
        let mut flash_toggle = false;
        let mut last_page: Option<crate::gui::pages::Page> = None;

        // READ TIMEOUT DURATION (CAN BE CHANGED AT RUNTIME)
        let timeout_secs = crate::load!(crate::state::DISPLAY_TIMEOUT_SECS) as u64;
        let render_duration = embassy_time::Duration::from_secs(timeout_secs);
        let mut render_start = embassy_time::Instant::now();

        loop {
            // RESET THE IDLE TIMER IF THE SCREEN WAS TOUCHED
            if crate::load!(crate::state::DISPLAY_TOUCH_ACTIVITY) {
                crate::dirty!();
                crate::base::timer::reset();
                crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, false);
                render_start = embassy_time::Instant::now();
            }

            // CHECK FOR STOP COMMAND
            if let Ok(DisplayCommand::Stop) = crate::DISPLAY_CMD.try_receive() {
                break;
            }

            // DISPLAY IS REDRAWN WHEN TOUCHED
            // DELAYED DIRTY SIGNAL ENSURE REDRAW AFTER FINGER IS RELEASED FROM DISPLAY AS WELL 
            let delayed_dirty = critical_section::with(|cs| {
                let cell = crate::state::DELAYED_DIRTY_TIME.borrow(cs);
                let t = cell.get();
                if let Some(due) = t {
                    if embassy_time::Instant::now() >= due {
                        cell.set(None);
                        true
                    } else { false }
                } else { false }
            });
            if delayed_dirty {
                crate::state::DISPLAY_DIRTY.store(true, core::sync::atomic::Ordering::Release);
            }
            
            // TRACK EXTERNAL MEDIA PLAYBACK
            let is_media_playing = yo_esp::MEDIA_IS_PLAYING.load(core::sync::atomic::Ordering::Acquire);
            // IF MEDIA WAS STARTED EXTERNALLY - SET INTERNAL BOOL & GO TO MEDIA PLAYER PAGE
            if is_media_playing && !was_media_playing {
                crate::store!(crate::state::MEDIA_IS_PLAYING, true);
                // TODO:
                // 1. STORE CURRENT crate::state::POWERDOWN_TIMEOUT_SECS (u32) SOMEWHERE ELSE
                // 2. SET crate::state::POWERDOWN_TIMEOUT_SECS TO 0 WHILE PLAYING
                crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::MediaPlayer as u8);
                crate::state::DISPLAY_DIRTY.store(true, core::sync::atomic::Ordering::Release);
                last_page = None;
            } // FINISHED PLAYBACK
            if !is_media_playing && was_media_playing {
                crate::store!(crate::state::MEDIA_IS_PLAYING, false);
                // TODO: RESTORE PREVIOUS crate::state::POWERDOWN_TIMEOUT_SECS
                crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::Clock as u8);
            }
            was_media_playing = is_media_playing;

            // IF DISPLAY STATE TOGGLED OFF EXTERNALLY, TURN OFF AND GO IDLE
            if !crate::load!(crate::state::DISPLAY_STATE) || crate::components::co5300::consume_sleep() {
                display.display_off();
                crate::store!(crate::state::DISPLAY_STATE, false);
                last_page = None;
                break;
            }

            // PROCESS ONE-SHOT COMMANDS FROM YO VOICE HANDLER
            if crate::components::co5300::consume_wake() {
                last_page = None;
            }

            // CLICKING POWER BUTTON ALSO TURNS ON THE DISPLAY (BUT WE ARE ALREADY ON)
            if crate::load!(crate::state::DISPLAY_STATE) {
                display.display_on();
                crate::store!(crate::state::DISPLAY_STATE, true);
            } else {
                display.display_off();
                crate::store!(crate::state::DISPLAY_STATE, false);
            }

            // DISPLAY BRIGHTNESS CONTROL
            let desired = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
            if desired != current_brightness {
                let byte = (desired as u16 * 255 / 100) as u8;
                display.set_brightness(byte);
                current_brightness = desired;
            }

            // FLASH DISPLAY OR RENDER PAGE NORMALLY?
            let flashing = crate::components::co5300::is_flashing();
            if flashing {
                flash_toggle = !flash_toggle;
                if flash_toggle {
                    display.fill_screen(crate::gui::colors::YELLOW);
                } else { display.fill_screen(crate::gui::colors::BLACK); }
            } else {
                if crate::load!(crate::state::DISPLAY_STATE) {
                    let page = crate::gui::pages::current_page();
                    let is_on_launcher = page == crate::gui::pages::Page::Apps;
                    let dirty = crate::state::DISPLAY_DIRTY.swap(false, core::sync::atomic::Ordering::Acquire);

                    if is_on_launcher || Some(page) != last_page || dirty {
                        let mut need_flush = true;

                        // APP LAUNCHER
                        if is_on_launcher {
                            let offset = critical_section::with(|cs| {
                                let mut launcher = crate::gui::apps::LAUNCHER.borrow_ref_mut(cs);
                                let diff = launcher.target_scroll - launcher.scroll_offset;
                                if diff.abs() > 2 {
                                    launcher.scroll_offset += diff / 2;
                                } else {
                                    launcher.scroll_offset = launcher.target_scroll;
                                }
                                launcher.scroll_offset
                            });
                            crate::gui::apps::compose(fb.buffer_mut(), offset);
                            let start = embassy_time::Instant::now();
                            crate::gui::flush_vsync_async(&mut fb, &mut display, &te).await;
                            let elapsed = start.elapsed().as_millis();
                            defmt::debug!("vsync async flush took {} ms", elapsed);
                            need_flush = false;
                        } else {
                            // OTHER PAGES
                            let current_offset = critical_section::with(|cs| {
                                crate::gui::control_center::OVERLAY.borrow_ref(cs).current_offset
                            });
                            let panel_h = crate::gui::control_center::panel_height();
                            let panel_any_visible = crate::gui::control_center::is_visible();

                            if panel_any_visible {
                                // DRAW THE CONTROL CENTER (PARTIAL DRAWING & SNAPSHOT SAVE/RESTORE OF THE BACKGROUND)
                                crate::gui::control_center::draw_overlay(&mut fb, current_offset);

                                if need_flush {
                                    let screen_h = crate::state::LCD_HEIGHT as i32;
                                    // DIRTY RECTANGLE - UNION OF OLD/NEW CONTROL CENTER POSITIONS
                                    let old_top = prev_overlay_offset;
                                    let new_top = current_offset;
                                    let dirty_top = old_top.min(new_top).max(0);
                                    let dirty_bottom = (old_top.max(new_top) + panel_h).min(screen_h);
                                    let dirty_y = dirty_top as u16;
                                    let dirty_h = (dirty_bottom - dirty_top).max(0) as u16;

                                    if dirty_h > 0 {
                                        fb.flush_region(&mut display, 0, dirty_y, crate::state::LCD_WIDTH, dirty_h);
                                        defmt::debug!("partial flush union: y={} h={}", dirty_y, dirty_h);
                                    }
                                }
                                // REMEMBER OFFSET FOR NEXT FRAME
                                prev_overlay_offset = current_offset;
                                need_flush = false;
                                crate::state::DISPLAY_DIRTY.store(false, core::sync::atomic::Ordering::Release);
                            } else { // CONTROL CENTER HIDDEN - FULL PAGE REDRAW
                                fb.clear_color(crate::gui::colors::BLACK);
                                match page {
                                    // HOME SCREEN PAGES
                                    crate::gui::pages::Page::Apps    => (), // LAUNCHER HANDLED ABOVE
                                    crate::gui::pages::Page::Clock   => crate::gui::time::draw(&mut fb),
                                    crate::gui::pages::Page::Battery => crate::gui::battery::draw(&mut fb),
                                    crate::gui::pages::Page::Weather => crate::gui::weather::draw(&mut fb),
                                    // APPLICATIONS
                                    crate::gui::pages::Page::MediaPlayer  => crate::gui::media_player::draw(&mut fb),
                                    crate::gui::pages::Page::DuckTv       => crate::gui::duck_tv::draw(&mut fb),
                                    crate::gui::pages::Page::DuckCloud    => crate::gui::duckcloud::draw(&mut fb),
                                    crate::gui::pages::Page::Settings     => crate::gui::settings::draw(&mut fb),
                                    // SETTINGS
                                    crate::gui::pages::Page::SettingsWifi => crate::gui::options::wifi::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsRssi => crate::gui::options::rssi::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsBle  => crate::gui::options::bluetooth::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsApi  => crate::gui::options::api::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsWake => crate::gui::options::wakeword::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsStream  => crate::gui::options::streaming::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsSpeaker => crate::gui::options::speaker::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsMic     => crate::gui::options::mic::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsDisplay => crate::gui::options::display::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsTimeout => crate::gui::options::timeout::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsAmp   => crate::gui::options::amplifier::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsCpu   => crate::gui::options::cpu::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsSleep => crate::gui::options::sleep::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsSsh   => crate::gui::options::ssh::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsVpn   => crate::gui::options::vpn::draw(&mut fb),
                                    crate::gui::pages::Page::SettingsInfo  => crate::gui::options::info::draw(&mut fb),
                                    // SPECIAL PAGES
                                    crate::gui::pages::Page::Call      => crate::gui::call::draw(&mut fb),
                                    crate::gui::pages::Page::Text      => crate::gui::text::draw(&mut fb),
                                    crate::gui::pages::Page::TextInput => crate::gui::input::draw(&mut fb),
                                    crate::gui::pages::Page::Gallery   => crate::gui::gallery::draw(&mut fb),
                                    _ => {}
                                }

                                prev_overlay_offset = i32::MIN; // RESET
                            }
                        }

                        if need_flush {
                            let start = embassy_time::Instant::now();
                            fb.flush(&mut display);
                            let elapsed = start.elapsed().as_millis();
                            defmt::debug!("flush took {} ms", elapsed);
                        }
                        last_page = Some(page);
                    }
                }
            }

            // CONTROL CENTER ANIMATION
            let overlay_animating = critical_section::with(|cs| {
                let ol = crate::gui::control_center::OVERLAY.borrow_ref(cs);
                ol.current_offset != ol.target_offset
            });
            if overlay_animating {
                crate::dirty!();
                crate::gui::control_center::animate(22); // PIXELS PER-FRAME
                last_page = None;
            }

            // MEDIA PLAYER ANIMATION
            let is_media_page = crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer;
            let split_animating = critical_section::with(|cs| {
                let split = crate::gui::media_player::MEDIA_SPLIT.borrow_ref(cs);
                split.current_offset != split.target_offset
            });
            if is_media_page {
                // SPLIT MEDIA PLAYER IN HALF & SHOW PLAYLIST
                crate::gui::media_player::animate_split(22); // PIXELS PER-FRAME
                if split_animating {
                    crate::dirty!();
                    last_page = None;
                }
            }
            // PLAYLIST SCROLLING
            if is_media_page && crate::gui::media_player::is_split_open() {
                crate::gui::media_player::animate_playlist_scroll(20); // PIXELS PER-FRAME
                let scroll_active = critical_section::with(|cs| {
                    let s = crate::gui::media_player::PLAYLIST_SCROLL.borrow_ref(cs);
                    s.offset != s.target
                });
                if scroll_active {
                    crate::dirty!();
                    last_page = None; // FORCE FULL PAGE REDRAW
                }
            }

            // INFO PAGE (SETTINGS) SCROLLING
            if crate::gui::pages::current_page() == crate::gui::pages::Page::SettingsInfo {
                crate::gui::options::info::animate_info(30); // SCROLL SPEED
                if critical_section::with(|cs| {
                    let scroll = crate::gui::options::info::INFO_SCROLL.borrow_ref(cs);
                    scroll.current_offset != scroll.target_offset
                }) { last_page = None; }
            }

            let current_page = crate::load!(crate::gui::pages::CURRENT_PAGE);
            // SET APPROPRIATE REFRESH RATE DEPENDING ON WHAT PAGE IS BEING DISPLAYED 
            let playlist_scrolling = is_media_page
                && crate::gui::media_player::is_split_open()
                && critical_section::with(|cs| {
                    let s = crate::gui::media_player::PLAYLIST_SCROLL.borrow_ref(cs);
                    s.offset != s.target
                });

            let delay_ms = if overlay_animating || crate::gui::control_center::is_visible()
                || split_animating
                || playlist_scrolling
            {
                1    // SMOOTH ANIMATIONS
            } else if crate::gui::pages::current_page() == crate::gui::pages::Page::Apps {
                16   // SMOOTH SCROLLING ON LAUNCHER
            } else if crate::gui::pages::current_page() == crate::gui::pages::Page::MediaPlayer {
                2000 // EVERY OTHER SECOND FOR NEDIA PROGRESS UPDATE
            } else if !crate::load!(crate::state::DISPLAY_STATE) {
                1000 // EVERY SECOND TO CHECK IF WE SHOULD TURN DISPLAY ON
            } else {
                400 // DEFAULT
            };

            let stop_fut = crate::DISPLAY_CMD.receive();
            let delay_fut = embassy_time::Timer::after(embassy_time::Duration::from_millis(delay_ms));
            match embassy_futures::select::select(delay_fut, stop_fut).await {
                embassy_futures::select::Either::Second(DisplayCommand::Stop) => break,
                embassy_futures::select::Either::Second(DisplayCommand::Start) => { /* ALREADY ON! */ }
                _ => {}
            }

            if render_start.elapsed() >= render_duration {
                defmt::debug!("DISPLAY TIMEOUT – GOING IDLE");
                break;
            }
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// FUNCTION TO CONTROL SPEAKER VOLUME (0-100%)
pub fn set_speaker_volume(volume: u8) {
    let volume = volume.min(100);
    crate::store!(crate::state::SPEAKER_VOLUME, volume);
    let was_muted = crate::load!(crate::state::SPEAKER_MUTED);

    if volume == 0 {
        // MIGHT AS WELL MUTE THE ES8311 CODEC HERE - POWER SAVER!
        critical_section::with(|cs| {
            let mut bus = crate::I2C_BUS.borrow_ref_mut(cs);
            let mut codec = crate::ES8311.borrow_ref_mut(cs);
            if let (Some(i2c), Some(es8311)) = (bus.as_mut(), codec.as_mut()) {
                let _ = es8311.mute(i2c, true);
                // AND SET CODEC TO FULL STANDBY (~0 µA)
                let _ = es8311.set_power_mode(i2c, es8311::PowerMode::Standby);
            }
        });
        // WE DON'T NEED AMPLIFIER ON IF WE ARE NOT OUPUTTING SOUND
        amp_off();
        defmt::info!("🔇 Speaker MUTED!");
        crate::store!(crate::state::SPEAKER_MUTED, true);
    } else { // ABOVE ZERO
        if was_muted { // CHECK IF LAST STATE WAS MUTE 
            // WAKE UP FROM STANDBY IF PREVIOUSLY MUTED 
            critical_section::with(|cs| {
                let mut bus = crate::I2C_BUS.borrow_ref_mut(cs);
                let mut codec = crate::ES8311.borrow_ref_mut(cs);
                if let (Some(i2c), Some(es8311)) = (bus.as_mut(), codec.as_mut()) {
                    // REBUILD CODEC CONFIG
                    let mclk_freq = crate::state::I2S_SAMPLE_RATE * 256;  // mclk_ratio = 256, as in main
                    let clock_cfg = es8311::ClockConfig {
                        mclk_inverted: false,
                        sclk_inverted: false,
                        mclk_from_mclk_pin: true,
                        mclk_frequency: mclk_freq,
                        sample_frequency: crate::state::I2S_SAMPLE_RATE,
                    };
                    let resolution = match crate::state::I2S_BIT_WIDTH {
                        16 => es8311::Resolution::Bits16,
                        24 => es8311::Resolution::Bits24,
                        32 => es8311::Resolution::Bits32,
                        _ => es8311::Resolution::Bits16,
                    };
                    let mut delay = esp_hal::delay::Delay::new();
                    // RE-INIT CODEC
                    if let Err(e) = es8311.init(i2c, &clock_cfg, resolution, resolution, &mut delay) {
                        defmt::error!("ES8311 wake‑up failed: {:?}", defmt::Debug2Format(&e));
                        return;
                    }
                    // UNMUTE
                    let _ = es8311.mute(i2c, false);
                }
            });
            // TURN ON AMPLIFIER AFTER CODEC IS READY
            amp_on();
            crate::store!(crate::state::SPEAKER_MUTED, false);
        }

        // SET NEW VOLUME
        critical_section::with(|cs| {
            let mut bus = crate::I2C_BUS.borrow_ref_mut(cs);
            let mut codec = crate::ES8311.borrow_ref_mut(cs);
            if let (Some(i2c), Some(es8311)) = (bus.as_mut(), codec.as_mut()) {
                let _ = es8311.volume_set(i2c, volume, None);
            }
        });
        defmt::info!("🔊 Volume {}%", volume);
    }
}


// ───────────────────────────────────────────────────────────────────────
// SCHEDULE A MUTE AFTER A GIVEN NUMBER OF SECONDS
pub async fn mute_in(seconds: u64) {
    embassy_time::Timer::after(embassy_time::Duration::from_secs(seconds)).await;
    set_speaker_volume(0);
}


// ───────────────────────────────────────────────────────────────────────
// FUNCTION TO CONTROL MICROPHONE GAIN (0-100%)
pub fn set_mic_gain(percent: u8) {
    let percent = percent.min(100);
    crate::store!(crate::state::MIC_VOLUME, percent);
    // 0  % === -95 dB
    // 100% === +32 dB
    let db = -95.0 + (127.0 * percent as f32 / 100.0);
    let db_i8 = db as i8;

    critical_section::with(|cs| {
        let mut bus = crate::I2C_BUS.borrow_ref_mut(cs);
        let mut codec = crate::ES7210.borrow_ref_mut(cs);

        if let (Some(i2c), Some(es7210)) = (bus.as_mut(), codec.as_mut()) {
            if percent == 0 { // MIGHT AS WELL MUTE THE ES7210 CODEC HERE - SAVES US A FEW mV
                defmt::info!("🎙️⛔ Mic MUTED!");
            } else { defmt::info!("🎙️ Gain {}%", percent); }
            let _ = es7210.gain_set(i2c, db_i8);
        }
    });
}

// ───────────────────────────────────────────────────────────────────────
// FUNCTIONS TO TURN ON/OFF THE AMPLIFIER
pub fn amp_on() {
    critical_section::with(|cs| {
        if let core::option::Option::Some(pin) = AMP_PIN.borrow_ref_mut(cs).as_mut() {
            pin.set_high();
        }
    }); defmt::info!("📢 ☑️");
    crate::store!(crate::state::AMPLIFIER_STATE, true);
}

pub fn amp_off() {
    critical_section::with(|cs| {
        if let core::option::Option::Some(pin) = AMP_PIN.borrow_ref_mut(cs).as_mut() {
            pin.set_low();
        }
    }); defmt::info!("📢 ❌");
    crate::store!(crate::state::AMPLIFIER_STATE, false);
}

// ───────────────────────────────────────────────────────────────────────
// FUNCTION TO SHUTDOWN POWER ENTIRELY
// DISABLES EVERYTHING POSSIBLE FOR A LOW POWER STATE
// BEFORE CUTTING THE POWER! (RTC STILL ALIVE)
pub fn deep_sleep_now() {
    defmt::info!("⚠️ Shutting down....");
    critical_section::with(|cs| {
        let mut pmu_opt = PMU.borrow_ref_mut(cs);
        let mut bus_opt = I2C_BUS.borrow_ref_mut(cs);

        crate::base::routes::api::settings::power::low::low_power_on();    
        if let (Some(pmu), Some(i2c)) = (pmu_opt.as_ref(), bus_opt.as_mut()) {
            if let Err(e) = pmu.prepare_deep_sleep(i2c) {
                defmt::error!("PMU Deep Sleep Prep failed: {:?}", defmt::Debug2Format(&e));
            }
            if let Err(e) = pmu.shutdown(i2c) {
                defmt::error!("PMU shutdown failed: {:?}", defmt::Debug2Format(&e));
            }
        } else { defmt::error!("PMU or I2C bus not available – cannot shutdown"); }
    });
}


// ───────────────────────────────────────────────────────────────────────
// MAIN
#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    // WE WILL CONTROL CPU CLOCK LATER (ALSO AVAILABLE VIA GUI)
    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(config);

    // ALLOCATE EXTERNAL PSEUDO STATIC RANDOM ACCESS MEMORY
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    // INTERNAL DRAM HEAP
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let mut storage = esp_storage::FlashStorage::new();


    // TTF PARSING IS HEAVY? - NOT SURE BUT LET'S CACHE THE BOLD FONT USED EVERYWHERE ANYWAY. 
    critical_section::with(|_| unsafe {
        crate::gui::ROBOTO_BOLD_FONT =
            Some(rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap());
    });

    // MEDIA PLAYER HAS HEAVY SPLIT ANIMATION & MULTIPLE IMAGES
    // CACHE & INIT THE MEDIA PLAYER EARLY
    crate::gui::media_player::init(); 

    // SOFTWARE INTERRUPT SETUP
    let _sw_ints = esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let sw_int0 = unsafe { esp_hal::interrupt::software::SoftwareInterrupt::steal() };
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int0);

    // TRACK TIME SINCE BOOT FOR DEVICE UPTIME CALCULATION
    let boot_time = embassy_time::Instant::now();

    // RANDOM
    let rng = esp_hal::rng::Rng::new();
    let seed: u64 = (u64::from(rng.random())) << 32 | u64::from(rng.random());
    crate::base::rng::init(rng);

    // ───────────────────────────────────────────────────────────────────────
    // BUTTONS (WE MONITOR THEM IN A DEDICATED TASK DOWN BELOW)

    // BOOT BUTTON (UPPER RIGHT SIDE BUTTON)
    // HOLD DOWN TO SEND VOICE COMMANDS (WHEN WAKE-WORD DISABLED)
    // WHEN PLAYING MEDIA - PUSH INCREASES VOLUME
    let button_boot = esp_hal::gpio::Input::new(
        peripherals.GPIO0,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    // POWER BUTTON (LOWER RIGHT SIDE BUTTON)
    // HELD DOWN FOR DEEP SLEEP/WAKE UP
    // WHEN PLAYING MEDIA - PUSH LOWERS VOLUME
    // TOGGLES DISPLAY ON/OFF OTHERWISE
    let button_power = esp_hal::gpio::Input::new(
        peripherals.GPIO10,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    // DISABLE (LOW) POWER AMPLIFIER 
    // WE ENABLE (HIGH) LATER
    // TO AVOID THE SCARY POP SOUND
    let mut amp = esp_hal::gpio::Output::new(
        peripherals.GPIO46,
        esp_hal::gpio::Level::Low,
        esp_hal::gpio::OutputConfig::default()
    );
    
    // LET'S MAKE THE AMP A SHARED RESOURCE
    // SO THE PUBLIC FUNCTIONS CAN SET HIGH/LOW
    critical_section::with(|cs| {
        *AMP_PIN.borrow_ref_mut(cs) = core::option::Option::Some(amp);
    });


    // GPIO38 IS A TOUCH INTERUPT PIN - HIGH (PULL-UP) BY DEFAULT
    // PULLED LOW BY THE TOUCH CONTROLLER UPON TOUCH
    // WHEN A FINGER IS ON THE SCREEN WE USE IT AS AN WAKE-UP CALL
    let mut touch_int = esp_hal::gpio::Input::new(
        peripherals.GPIO38,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    ); 


    // ───────────────────────────────────────────────────────────────────────
    // MICRO SECURE DIGITAL CARD (STORAGE OVER SPI)
    // CONFIGURATION
    let sd_spi_config = esp_hal::spi::master::Config::default()
        .with_frequency(esp_hal::time::Rate::from_mhz(4))
        .with_mode(esp_hal::spi::Mode::_0);
    let sd_spi = esp_hal::spi::master::Spi::new(peripherals.SPI3, sd_spi_config)
        .expect("SPI FAILURE! (SD-CARD)")
        .with_sck(peripherals.GPIO2)
        .with_mosi(peripherals.GPIO1)
        .with_miso(peripherals.GPIO3);
    let sd_cs = esp_hal::gpio::Output::new(
        peripherals.GPIO17,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
    );

    let sd_spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(sd_spi, sd_cs).unwrap();
    let sd_card = embedded_sdmmc::SdCard::new(sd_spi_dev, esp_hal::delay::Delay::new());

    // MOVE OWNERSHIP INTO THE STORAGE MODULE
    crate::components::storage::init(sd_card, &spawner);
    // SD CARD NOW CONFIGURED - BUT NOT STARTED (BATTERY EXPENSIVE!)
    // IT'S AUTO-INITIATED WHEN WE NEED IT!
    defmt::info!("STORAGE Successfully setup, awaiting initialization");
    

    // ───────────────────────────────────────────────────────────────────────
    // I2C BUS
    let i2c_a = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(esp_hal::time::Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO15)
    .with_scl(peripherals.GPIO14);

    
    // STORE BUS GLOBALLY
    critical_section::with(|cs| {
        I2C_BUS.borrow_ref_mut(cs).replace(i2c_a);
    });
    
    // CREATE THE POWER MANAGEMENT UNIT
    // (DRIVER STRUCT – DOES NOT HOLD BUS REF)
    let pmu = crate::components::axp2101::Axp2101::new();
    
    // INITIALISE ALL I²C DEVICES (PMU, AUDIO CODECS, TOUCH, IMU, RTC)
    critical_section::with(|cs| {
        let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
        let i2c_bus = bus_ref.as_mut().expect("I2C bus missing");
        PMU.borrow_ref_mut(cs).replace(pmu);

        // ES7210 / ES8311 STRUCTS
        let es7210 = es7210::Es7210::new(0x40);
        let es8311 = es8311::Es8311::new(0x18);
    
        // PMU INIT
        if let Err(e) = pmu.init(i2c_bus, &crate::components::axp2101::Axp2101Config::default()) {
            defmt::error!("PMU init failed: {:?}", defmt::Debug2Format(&e));
        } else { defmt::info!("AXP2101 Successful initialization"); }

        // QMI8658 - IMU (ACCELEROMETER ++ GYROSCOPE) 
        // CURRENTLY NOT USED - WHY BOTHER INIT?
        //let mut imu = crate::components::qmi8658::Qmi8658Imu::new(&mut *i2c_bus);
        //let _ = imu.init();
    
        // ES7210 (ADC/MICROPHONES)
        let codec_cfg = es7210::CodecConfig {
            sample_rate_hz: crate::state::I2S_SAMPLE_RATE,
            mclk_ratio: 256,
            i2s_format: es7210::I2sFormat::I2S,
            bit_width: match crate::state::I2S_BIT_WIDTH {
                16 => es7210::I2sBits::Bits16,
                24 => es7210::I2sBits::Bits24,
                32 => es7210::I2sBits::Bits32,
                _ => es7210::I2sBits::Bits16,
            },
            mic_bias: es7210::MicBias::V2_87,
            mic_gain: es7210::MicGain::Gain30dB,
            tdm_enable: false,
        };
        match es7210.config_codec(i2c_bus, &codec_cfg) {
            Ok(()) => defmt::info!("ES7210  Successful initialization"),
            Err(e) => defmt::info!("ES7210 INIT FAILED: {:?}", defmt::Debug2Format(&e)),
        }
        if let Err(e) = es7210.gain_set(i2c_bus, 20) {
            defmt::info!("ES7210 volume set failed: {:?}", defmt::Debug2Format(&e));
        }
        if let Err(e) = es7210.set_mute(i2c_bus, false) {
            defmt::info!("Failed to configure ES7210 mute status {:?}", defmt::Debug2Format(&e));
        }

        // ES8311 (DAC/SPEAKER) 
        let resolution = match crate::state::I2S_BIT_WIDTH {
            16 => es8311::Resolution::Bits16,
            24 => es8311::Resolution::Bits24,
            32 => es8311::Resolution::Bits32,
            _ => es8311::Resolution::Bits16,
        };
        let mclk_freq = crate::state::I2S_SAMPLE_RATE * 256; 
        let clock_cfg = es8311::ClockConfig {
            mclk_inverted: false,
            sclk_inverted: false,
            mclk_from_mclk_pin: true,
            mclk_frequency: mclk_freq,
            sample_frequency: crate::state::I2S_SAMPLE_RATE,
        };
        let mut delay = esp_hal::delay::Delay::new();
        match es8311.init(
            i2c_bus,
            &clock_cfg,
            resolution,
            resolution,
            &mut delay,
        ) {
            Ok(()) => defmt::info!("ES8311  Successful initialization"),
            Err(e) => defmt::info!("ES8311 INIT FAILED: {:?}", defmt::Debug2Format(&e)),
        }
        let _ = es8311.volume_set(i2c_bus, 55, None);
        let _ = es8311.mute(i2c_bus, false);

        // PCF85063A - (Real Time Clock)
        let mut rtc = crate::components::pcf85063a::Pcf85063aRtc::new(i2c_bus);
        let _ = rtc.init();
    
        // PERFORM INITIAL BATTERY READINGS 
        let mv = pmu.get_battery_voltage(i2c_bus).unwrap_or(0);
        let percent = pmu.get_battery_percent(i2c_bus).unwrap_or(0);
        let is_usb_connected = pmu.is_vbus_in(i2c_bus).unwrap_or(false); 
        
        // STPRE THE RESULTS
        if percent == 100 {
            crate::store!(crate::state::BATTERY_FULL, true);
        } else { crate::store!(crate::state::BATTERY_FULL, false); }
        if percent < 25 {
            crate::store!(crate::state::BATTERY_NEED_CHARGING, true);
        } else { crate::store!(crate::state::BATTERY_NEED_CHARGING, false); }
        crate::store!(crate::state::BATTERY_VOLTAGE, mv as u32);
        crate::store!(crate::state::BATTERY_PERCENT, percent);
        crate::store!(crate::state::BATTERY_USB_CONNECTED, is_usb_connected);

   
        //  FT3168 - (TOUCH CONTROLLER)
        let mut touch_rst = esp_hal::gpio::Output::new(
            peripherals.GPIO9,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default()
        );

        // TOUCH RESET SEQUENCE
        touch_rst.set_low();
        delay.delay_millis(10);
        touch_rst.set_high();
        delay.delay_millis(50);
        let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
        if let Err(e) = touch.init() {
            defmt::error!("FT3168 init failed: {:?}", defmt::Debug2Format(&e));
        } else { defmt::info!("FT3168  Successful initialization"); }


        // STORE THE DRIVER OBJECTS GLOBALLY FOR LATER VOLUME CONTROL
        ES7210.borrow_ref_mut(cs).replace(es7210);
        ES8311.borrow_ref_mut(cs).replace(es8311);

    });


    // ───────────────────────────────────────────────────────────────────────    
    // DISPLAY - (80MHz OVER SPI)
    let spi_config = esp_hal::spi::master::Config::default()
        .with_frequency(esp_hal::time::Rate::from_mhz(80))
        .with_mode(esp_hal::spi::Mode::_0);

    // CREATE DISPLAY DMA BUFFERS
    //let (rx_buf, rx_desc, tx_buf, tx_desc) = esp_hal::dma_buffers!(32768);
    let (rx_buf, rx_desc, tx_buf, tx_desc) = esp_hal::dma_buffers!(8008);
    let dma_rx = esp_hal::dma::DmaRxBuf::new(rx_desc, rx_buf).unwrap();
    let dma_tx = esp_hal::dma::DmaTxBuf::new(tx_desc, tx_buf).unwrap();
    let spi = esp_hal::spi::master::Spi::new(peripherals.SPI2, spi_config)
        .expect("SPI failed")
        .with_sck(peripherals.GPIO11)
        .with_sio0(peripherals.GPIO4)
        .with_sio1(peripherals.GPIO5)
        .with_sio2(peripherals.GPIO6)
        .with_sio3(peripherals.GPIO7)
        .with_dma(peripherals.DMA_CH0)
        .with_buffers(dma_rx, dma_tx);

    let cs = esp_hal::gpio::Output::new(
        peripherals.GPIO12,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
    );

    let reset = esp_hal::gpio::Output::new(
        peripherals.GPIO8,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
    );

    let mut display = crate::components::co5300::Co5300Display::new(crate::components::qspi_bus::QspiBus::new(spi, cs), reset);
    display.init();
    
    // TEARING EFFECT OUTPUT ON CO5300
    let te_pin = esp_hal::gpio::Input::new(peripherals.GPIO13, esp_hal::gpio::InputConfig::default());
    defmt::info!("CO5300  Successful initialization");

    // FRAMEBUFFER
    let mut fb = crate::components::framebuffer::Framebuffer::new();
    fb.clear_color(crate::gui::colors::BLACK);
    fb.flush(&mut display);


    // ───────────────────────────────────────────────────────────────────────
    // FETCH WIREGUARD CONFIGURATION
    let wg_conf = crate::base::wireguard::parse_wg_conf()
        .expect("Invalid wg-client.conf");
    let wg_conf = WG_CONFIG.init(wg_conf);

    // CREATE WIREGUARD CHANNEL & SPLIT
    let (wg_device, wg_runner) = crate::base::wireguard::init_wg_channel();


    // ───────────────────────────────────────────────────────────────────────
    // SETUP WIFI (ON LOW-POWER MODE)
    let backend_port: u16 = crate::state::BACKEND_TCP_PORT_STR.parse().expect("Invalid BACKEND_TCP_PORT");    
    let wifi_stack = base::wifi::init(&spawner, peripherals.WIFI, backend_port, wg_device, wg_conf).await;
        
    // WIFI CONFIGURED TO SIT IDLE AND AWAIT START/STOP COMMANDS
    // VOICE COMMUNICATION REQUIRES LOCAL NETWORK - START IT UP! (CAN BE TOGGLED AT RUNTIME)
    crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Enable).await;
    crate::store!(crate::state::WIFI_STATE, true);

    let wg_rng = esp_hal::rng::Rng::new();
    // START THE VPN TASK
    // (IDLE, AUTO ENABLED WHEN WIFI CONNECTION ESTABLISHED)
    crate::spawn!(spawner, crate::base::wireguard::wireguard_task(
        wifi_stack,
        wg_conf,
        wg_rng,
        wg_runner
    ));   
    
    VPN_STACK_READY.wait().await;
    let vpn_stack: &'static embassy_net::Stack<'static> = unsafe {
        VPN_STACK.expect("VPN stack not initialized")
    };

    // ───────────────────────────────────────────────────────────────────────
    // I2S AUDIO SETUP 
    // CREATE RX & TX DMA BUFFERS
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_circular_buffers!(crate::state::I2S_BUFFER_SIZE);

    // CONFIGURE THE I2S PERIPHERAL
    // BY VARIABLES FOR A CODEC SYNCRONIZED AUDIO SETUP
    let mut i2s = esp_hal::i2s::master::I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH1,
        esp_hal::i2s::master::Config::new_tdm_philips()
            // SIGNAL LOOPBACK === SET I2S RX AS SLAVE
            // ++ ENABLES BIDRECTIONAL FULL-DUPLEX I2S ON SINGLE PERIPHERAL
            .with_signal_loopback(crate::state::I2S_SIGNAL_LOOPBACK)
            .with_sample_rate(esp_hal::time::Rate::from_hz(crate::state::I2S_SAMPLE_RATE))
            .with_data_format(crate::state::I2S_DATA_FORMAT)
            .with_endianness(crate::state::I2S_ENDIANNESS)
            .with_channels(crate::state::I2S_CHANNELS),
    )
    .unwrap()
    .into_async()
    .with_mclk(peripherals.GPIO16);

    i2s.i2s_rx.rx_channel.set_priority(esp_hal::dma::DmaPriority::Priority5);
    i2s.i2s_tx.tx_channel.set_priority(esp_hal::dma::DmaPriority::Priority8);

    // AUDIO INPUT
    // BUILD I2S RX (SLAVE) WITH DIGITAL-INPUT PIN ONLY
    let i2s_rx = i2s.i2s_rx
        .with_din(peripherals.GPIO42)
        .build(rx_descriptors);

    // AUDIO OUTPUT
    // BUILD I2S TX (MASTER) WITH:
    // ++ BCLK ++ LRCLK ++ DIGITAL-OUTPUT PINS  
    let i2s_tx = i2s.i2s_tx
        .with_bclk(peripherals.GPIO41)
        .with_ws(peripherals.GPIO45)
        .with_dout(peripherals.GPIO40)
        .build(tx_descriptors);

    // I2S TX CIRCULAR WRITE
    // CONTINUOUSLY WRITE TO I2S TX TO KEEP CLOCKS UP FOR RX (SLAVE)
    // WHEN WE DON'T HAVE AUDIO TO WRITE - WE WRITE ZEROS INTO THE BUFFER 
    let tx_transfer = match i2s_tx.write_dma_circular_async(tx_buffer) {
        Ok(t) => t,
        Err(e) => {
            defmt::error!("I2S circular TX failed: {:?}", defmt::Debug2Format(&e));
            panic!("I2S setup error");
        }
    };

    // INIT YO HANDLER (OUR VOICE COMMAND HANDLER)
    let handler: alloc::boxed::Box<dyn yo_esp::CommandHandler> = alloc::boxed::Box::new(VoiceHandler);

    // ───────────────────────────────────────────────────────────────────────
    // INIT ENDPOINT ROUTES FOR THE INTERNAL API
    crate::base::api::init_routes().await;


    // ───────────────────────────────────────────────────────────────────────
    // BOOT PROCESS COMPLETE
    // PRINT OS INFORMATION & INIT TASKS
    defmt::info!("╬═══════════════════════════════╬");
    defmt::info!("╬ STARTED {} v{} ╬",
        crate::state::PROJECT_NAME,
        crate::state::FW_VERSION
    ); defmt::info!("╬═══════════════════════════════╬");

    // ───────────────────────────────────────────────────────────────────────
    // TASKS

  
    // SPEAKER TASK (WRITES AUDIO DATA INTO PIPE + KEEP CLOCKS UP FOR MIC)
    // TASK STARTS IDLE AND WAITS FOR A COMMAND 
    crate::spawn!(spawner, yo_esp::speaker_task(tx_transfer));
    // WE START IT HERE - TO AVOID LATE DMA!
    yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Start).await;
    crate::store!(crate::state::SPEAKER_TASK_STATE, true);
    
    // STREAMING SPEAKER TASK (STREAM INCOMING AUDIO TO THE SPEAKER OVER TCP PORT 12345)
    // (IDLE - SEND START/STOP COMMAND)
    // AUTO STARTED ON VPN CONNECTION
    crate::spawn!(spawner, yo_esp::stream_speaker(vpn_stack, backend_port));
        
    // MICROPHONE TASK (STREAMS AUDIO TO BACKEND OVER TCP PORT 12345)
    // (SLEEPS UNLESS WAKE-WORD ENABLED/BUTTON IS PRESSED)
    crate::spawn!(spawner, yo_esp::audio_capture_task(i2s_rx, vpn_stack, crate::state::BACKEND_TCP_HOST, backend_port, "esp", handler));

    // START SSHD ON PORT 2222
    // (IDLE - SEND START/STOP COMMAND)
    // AUTO STARTED ON VPN CONNECTION
    // USES ED25519 PUBLIC KEY AUTHENTICATION
    crate::spawn!(spawner, crate::base::ssh::sshd_task(vpn_stack));
        
    // HTTP API & WEB SERVER TASK (PORT 80)
    // (IDLE - SEND START/STOP COMMAND)
    // AUTO STARTED ON VPN CONNECTION
    crate::spawn!(spawner, tinyapi::web_server_task(vpn_stack));
     
    // START TINYWEATHER TASK IN THE BACKGROUND
    crate::spawn!(spawner, crate::applications::tinyweather::weather_task(vpn_stack));
    
    // START THE MEDIA PLAYER TASK
    crate::spawn!(spawner, crate::applications::media_player::playback_task(spawner));
  
    // BUTTON MONITORING TASK
    crate::spawn!(spawner, crate::components::buttons::buttons_task(button_boot, button_power));
    
    // TOUCH TASK (SLEEPS WHEN NO TOUCH)
    crate::spawn!(spawner, crate::gui::pages::touch_task(touch_int));

    // INACTIVITY TIMER TASK 
    //  CUT POWER TO PMU IF IDLE! (CONFIGURABLE VIA GUI)
    //   BRUTAL BUT WORTH IT
    crate::spawn!(spawner, crate::base::timer::timer_task());
    
    // DISPLAY TASK - HYBRID APPROACH
    // (THE ONLY TASK THAT WAKES CPU ON TIMER) 
    crate::spawn!(spawner, display_task(fb, display, te_pin));

    // FEATURE FLAGGED STORAGE TESTING 
    #[cfg(feature = "sd-test")]
    crate::spawn!(spawner, crate::components::storage::test_task());


    // IT'S NOW SAFE TO CRANK UP THE AMP
    // WITH NO LOAD POPPIN' NOISE
    crate::amp_on();


    // ───────────────────────────────────────────────────────────────────────
    // SLOW IT DOWN!!
    //crate::components::frequency::set_cpu_mhz(160);
    //crate::store!(crate::state::CPU_FREQ, 160);
    crate::delay_s!(2);   
 

    // MAIN LOOP
    loop { // CALCULATE TIME SINCE BOOT
        let elapsed = embassy_time::Instant::now() - boot_time;
        let uptime_secs = elapsed.as_secs() as u32;
        let days = elapsed.as_secs() / 86400;
        let hours = (elapsed.as_secs() % 86400) / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;
        // & STORE IT
        crate::store!(crate::state::UPTIME_SECS, uptime_secs);

        // +1 MINUTE TO CURRENT TIME
        critical_section::with(|cs| {
            let time_cell = crate::state::CURRENT_TIME.borrow(cs);
            if let Some(mut dt) = time_cell.get() {
                crate::components::pcf85063a::up_one_min(&mut dt);
                time_cell.set(Some(dt));
            }
        });

        // PRINT TIME + UPTIME
        if days > 0 {
            if hours > 0 {
                defmt::info!("⏱️  {}D {:02}H {:02}M uptime", days, hours, minutes);
            } else { defmt::info!("⏱️  {}D {:02}M uptime", days, minutes); }
        } else if hours > 0 {
            defmt::info!("⏱️  {:02}H {:02}M uptime", hours, minutes);
        } else { defmt::info!("⏱️  {:02}M uptime", minutes); }
        let maybe_time = critical_section::with(|cs| crate::state::CURRENT_TIME.borrow(cs).get());
        if let Some(dt) = maybe_time { defmt::info!("⏰ {:02}:{:02}", dt.hours, dt.minutes); }
        
        
        // POLL BATTERY STATUS FROM I2C BUS
        // BUT ONLY EVERY 5TH MINUTE -- CHECKING BATTERY, TAKES BATTERY! WORTH IT?
        let should_poll = if crate::load!(crate::state::BATTERY_USB_CONNECTED) { true } else { minutes % 10 == 5 };
        if should_poll {
            let (percent, voltage_mv, usb_connected) = critical_section::with(|cs| {
                let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
                let i2c_bus = bus_ref.as_mut().unwrap(); 
                (
                    pmu.get_battery_percent(i2c_bus).unwrap_or(0),
                    pmu.get_battery_voltage(i2c_bus).unwrap_or(0),
                    pmu.is_vbus_in(i2c_bus).unwrap_or(false),
                )
            });
 
            // STORE ATOMIC VARIABLES
            if percent == 100 {
                crate::store!(crate::state::BATTERY_FULL, true);
            } else { crate::store!(crate::state::BATTERY_FULL, false); }
            if percent < 25 {
                crate::store!(crate::state::BATTERY_NEED_CHARGING, true);
            } else { crate::store!(crate::state::BATTERY_NEED_CHARGING, false); }
            crate::store!(crate::state::BATTERY_VOLTAGE, voltage_mv as u32);
            crate::store!(crate::state::BATTERY_PERCENT, percent);
            crate::store!(crate::state::BATTERY_USB_CONNECTED, usb_connected);
        }
        let percent = crate::load!(crate::state::BATTERY_PERCENT);
        let mv = crate::load!(crate::state::BATTERY_VOLTAGE);
        let emoji = if crate::load!(crate::state::BATTERY_USB_CONNECTED) {"🔋⚡"} else { "🔋" };
        
        // PRINT WIFI SIGNAL & BATTERY INFO
        if crate::load!(crate::state::WIFI_CONNECTED) {
            let rssi = crate::load!(crate::state::RSSI);
            let rssi_percent = (rssi + 100) * 100 / 70;
            let rssi_percent = rssi_percent.clamp(0, 100);
            defmt::info!("🛜 {} dBm ({}%)", rssi, rssi_percent);
        }; defmt::info!("{} {}% ({} mv)", emoji, percent, mv);
        
        // DISPLAY IS NOW DIRTY
        crate::dirty!();

        // START THE POWER DOWN TIMER 
        // UNLESS USB IS CONNECTED OR POWERDOWN TIMER DISABLED (SET TO ZERO)
        if !crate::load!(crate::state::BATTERY_USB_CONNECTED) {
            if crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS) != 0 {
                crate::base::timer::start();
            } else { crate::base::timer::stop(); }
        }

        // SLEEP 60 SECONDS AND RERUN LOOP
        crate::delay_s!(60);
        // THE END!
    } // 🦆🧑‍🦯 thank you for quackin' along!
    // if you found this helpful - please concider buying me a coffee 
} // ☕ ⮞ https://buymeacoffee.com/quackhackmcblindy
