// ★ ─────────────────────────────────────────────────────────────────────── ★
//! ESP32-S3-WATCH-rs ⮞ https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs
//!  BARE METAL RUST  - HARDWARE ABSTRACTION LAYER: `esp-hal`
//!   SMARTWATCH OS   - BY QuackHack-McBLindy 🦆🧑‍🦯
// ★ ─────────────────────────────────────────────────────────────────────── ★
//! “A powerful voice assistant can make a huge difference for blind people.”
//! “Imagine yourself stumbling blindly across the room looking for the TV remote — meanwhile, I call the remote using only my voice.”
//! “Just to find it and throw it out the window -- because I won't ever need it.“
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


// IMPORT
use esp_println as _;

// PANIC HANDLER
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("⚠️ Panic: {}", defmt::Debug2Format(info));
    loop {} // REBOOT THE DEVICE! (HOLD POWER BUTTON)
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
pub static TOUCH_CHANNEL: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::components::ft3168::SwipeDirection, 5> = embassy_sync::channel::Channel::new();
pub static ES7210: critical_section::Mutex<core::cell::RefCell<Option<es7210::Es7210>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static ES8311: critical_section::Mutex<core::cell::RefCell<Option<es8311::Es8311>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static I2C_BUS: critical_section::Mutex<core::cell::RefCell<Option<I2cBus>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static AMP_PIN: critical_section::Mutex<core::cell::RefCell<core::option::Option<esp_hal::gpio::Output<'static>>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(core::option::Option::None));
pub static DISPLAY_CMD: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, DisplayCommand, 1> = embassy_sync::channel::Channel::new();


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
// ───────────────────────────────────────────────────────────────────────
// DISPLAY CONTROLLER TASK – BOOTS DISABLED
#[embassy_executor::task]
async fn display_task(
    mut fb: crate::components::framebuffer::Framebuffer,
    mut display: crate::components::co5300::Co5300Display<'static>,
    te: esp_hal::gpio::Input<'static>,
) { // START WITH THE SCREEN OFF
    // WAKE IT UP BY SAYING: (OR TOUCHING SCREEN)
    // `YO BITCH!` (IF WAKE WORD ENABLED)
    display.display_off();

    loop {
        // IDLE – SCREEN OFF, WAITING FOR START COMMAND
        display.display_off();
        crate::store!(crate::state::DISPLAY_STATE, false);

        let cmd = crate::DISPLAY_CMD.receive().await;
        match cmd {
            DisplayCommand::Start => { /* PROCEED */ },
            DisplayCommand::Stop => { continue; },
        }

        // SCREEN ON – START RENDERING LOOP
        display.display_on();
        crate::store!(crate::state::DISPLAY_STATE, true);

        // STATE VARIABLES 
        // LOW DEFAULT BRIGHTNESS (35%) DON'T HURT 🦆 BLIND EYES & SAVES BATTERY 
        let mut current_brightness = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
        let mut flash_toggle = false;
        let mut last_page = crate::load!(crate::gui::pages::CURRENT_PAGE);

        // READ TIMEOUT DURATION (CAN BE CHANGED AT RUNTIME)
        let timeout_secs = crate::load!(crate::state::DISPLAY_TIMEOUT_SECS) as u64;
        let render_duration = embassy_time::Duration::from_secs(timeout_secs);
        let mut render_start = embassy_time::Instant::now();

        loop {
            // RESET THE IDLE TIMER IF THE SCREEN WAS TOUCHED
            if crate::load!(crate::state::DISPLAY_TOUCH_ACTIVITY) {
                crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, false);
                render_start = embassy_time::Instant::now();
            }

            // CHECK FOR STOP COMMAND
            if let Ok(DisplayCommand::Stop) = crate::DISPLAY_CMD.try_receive() {
                break;
            }

            // IF DISPLAY STATE TOGGLED OFF EXTERNALLY, TURN OFF AND GO IDLE
            if !crate::load!(crate::state::DISPLAY_STATE) || crate::components::co5300::consume_sleep() {
                display.display_off();
                crate::store!(crate::state::DISPLAY_STATE, false);
                last_page = u8::MAX;
                break;
            }

            // PROCESS ONE-SHOT COMMANDS FROM YO VOICE HANDLER
            if crate::components::co5300::consume_wake() {
                // FORCE REDRAW OF CURRENT PAGE IMMEDIATELY
                last_page = u8::MAX;
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
                // WE ONLY RENDER PAGES WHEN THE SCREEN IS ON
                if crate::load!(crate::state::DISPLAY_STATE) {
                    let page = crate::load!(crate::gui::pages::CURRENT_PAGE);
                    let is_on_launcher = page == 0;

                    // DIRTY CHECK: READ THEN CLEAR ONLY IF WE DRAW
                    let dirty = crate::state::DISPLAY_DIRTY.swap(false, core::sync::atomic::Ordering::Acquire);

                    if is_on_launcher || page != last_page || dirty {
                        let mut need_flush = true;

                        // APP LAUNCHER
                        if is_on_launcher {
                            // CALCULATE SCROLLING DISTANCE
                            let offset = critical_section::with(|cs| {
                                let mut launcher = crate::gui::apps::LAUNCHER.borrow_ref_mut(cs);
                                let diff = launcher.target_scroll - launcher.scroll_offset;
                                if diff.abs() > 2 {
                                    launcher.scroll_offset += diff / 3;
                                } else { launcher.scroll_offset = launcher.target_scroll; }
                                launcher.scroll_offset
                            });
                            crate::gui::apps::compose(fb.buffer_mut(), offset);
                            // VSYNC FLUSH FOR SMOOTH ANIMATION
                            crate::gui::flush_vsync_async(&mut fb, &mut display, &te).await;
                            need_flush = false;
                        } else {
                            // OTHER PAGES
                            fb.clear_color(crate::gui::colors::BLACK);
                            match page {
                                0 => (),
                                1 => crate::gui::time::draw(&mut fb),
                                2 => crate::gui::battery::draw(&mut fb),
                                3 => crate::gui::weather::draw(&mut fb),
                                // APPLICATIONS
                                10 => crate::gui::media_player::draw(&mut fb),
                                11 => crate::gui::duck_tv::draw(&mut fb),
                                12 => crate::gui::house::draw(&mut fb),
                                13 => crate::gui::duckcloud::draw(&mut fb),
                                14 => crate::gui::settings::draw(&mut fb),
                                // SETTINGS APP
                                140 => crate::gui::options::wifi::draw(&mut fb),
                                141 => crate::gui::options::rssi::draw(&mut fb),
                                142 => crate::gui::options::bluetooth::draw(&mut fb),
                                143 => crate::gui::options::api::draw(&mut fb),
                                144 => crate::gui::options::wakeword::draw(&mut fb),
                                145 => crate::gui::options::streaming::draw(&mut fb),
                                146 => crate::gui::options::speaker::draw(&mut fb),
                                147 => crate::gui::options::mic::draw(&mut fb),
                                148 => crate::gui::options::display::draw(&mut fb),
                                149 => crate::gui::options::timeout::draw(&mut fb),
                                150 => crate::gui::options::amplifier::draw(&mut fb),
                                151 => crate::gui::options::cpu::draw(&mut fb),
                                152 => crate::gui::options::info::draw(&mut fb),
                                // MISC PAGES (CALLED BY API WHEN THEY'RE WANTED)
                                100 => crate::gui::call::draw(&mut fb),
                                101 => crate::gui::text::draw(&mut fb),
                                _ => {}
                            }
                            if crate::gui::control_center::is_visible() {
                                let current_offset = critical_section::with(|cs| {
                                    crate::gui::control_center::OVERLAY.borrow_ref(cs).current_offset
                                });
                                crate::gui::control_center::draw_overlay(&mut fb, current_offset);
                            } // DON'T FORGET TO FLUSH FRAMEBUFFER
                        } // FLUSH ONLY IF UNFLUSHED
                        if need_flush { fb.flush(&mut display); }
                        last_page = page;
                    }
                }
            }

            // CONTROL CENTER SLIDE IN/OUT ANIMATION
            let overlay_animating = critical_section::with(|cs| {
                let ol = crate::gui::control_center::OVERLAY.borrow_ref(cs);
                ol.current_offset != ol.target_offset
            });
            if overlay_animating {
                crate::gui::control_center::animate(60);
                last_page = u8::MAX;
            }
            
            // MEDIA PLAYER SPLIT ANIMATION
            let split_animating = critical_section::with(|cs| {
                let split = crate::gui::media_player::MEDIA_SPLIT.borrow_ref(cs);
                split.current_offset != split.target_offset
            });
            let is_media_page = crate::load!(crate::gui::pages::CURRENT_PAGE) == 10;

            if split_animating || is_media_page {
                crate::gui::media_player::animate_split(60);
                if split_animating {
                    last_page = u8::MAX;
                }
            }

            // INFO PAGE (SETTINGS) SCROLL ANIMATION
            if crate::load!(crate::gui::pages::CURRENT_PAGE) == 151 {
                crate::gui::options::info::animate_info(60);
                if critical_section::with(|cs| {
                    let scroll = crate::gui::options::info::INFO_SCROLL.borrow_ref(cs);
                    scroll.current_offset != scroll.target_offset
                }) {
                    last_page = u8::MAX;
                }
            }

            let current_page = crate::load!(crate::gui::pages::CURRENT_PAGE);
            // SET AN APPROPRIATE DELAY (REFRESH RATE) FOR THE DISPLAY
            // DEPENDING ON WHAT PAGE WE'RE ON
            let delay_ms = if overlay_animating || crate::gui::control_center::is_visible() || split_animating {
                16   // SMOOTH OVERLAYS & SPLIT
            } else if crate::load!(crate::gui::pages::CURRENT_PAGE) == 2 {
                16   // APP LAUNCHER – FOR A SMOOTH SCROLLING BETWEEN APPS 16 SHOULD BE ENOUGH
            } else if crate::load!(crate::gui::pages::CURRENT_PAGE) == 10 {
                2000 // MEDIA PLAYER – SONG DURATION ONLY NEEDS TO BE UPDATED EVERY OTHER SECOND
            } else if !crate::load!(crate::state::DISPLAY_STATE) {
                1000 // WE CHECK EVERY SECOND IF DISPLAY SHOULD BE TURNED ON
            } else { 400 }; // DEFAULT

            // WAIT WITH INTERRUPTIBLE STOP CHECK (INSTEAD OF BLOCKING)
            let stop_fut = crate::DISPLAY_CMD.receive();
            let delay_fut = embassy_time::Timer::after(embassy_time::Duration::from_millis(delay_ms));
            match embassy_futures::select::select(delay_fut, stop_fut).await {
                embassy_futures::select::Either::Second(DisplayCommand::Stop) => break,
                embassy_futures::select::Either::Second(DisplayCommand::Start) => { /* IGNORE */ }
                _ => {}
            }

            // AUTO‑IDLE AFTER TIMEOUT
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
// MAIN
#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    // WE CAN CONTROL CLOCKS AT RUNTIME LATER
    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(config);

    // ALLOCATE PSRAM
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    // HEAP ALLOC
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    // TTF PARSING IS HEAVY? - LET'S CACHE THE FONT WE'LL USE. 
    critical_section::with(|_| unsafe {
        crate::gui::ROBOTO_BOLD_FONT =
            Some(rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap());
    });

    // SOFTWARE INTERRUPT SETUP
    let _sw_ints = esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let sw_int0 = unsafe { esp_hal::interrupt::software::SoftwareInterrupt::steal() };
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int0);

    // TRACK TIME SINCE BOOT FOR DEVICE UPTIME CALCULATION
    let boot_time = embassy_time::Instant::now();


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
    // SD CARD NOW CONFIGURED - BUT NOT STARTED (POWER SAVER!)
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
    let (rx_buf, rx_desc, tx_buf, tx_desc) = esp_hal::dma_buffers!(8000);
    let dma_rx = esp_hal::dma::DmaRxBuf::new(rx_desc, rx_buf).unwrap();
    let dma_tx = esp_hal::dma::DmaTxBuf::new(tx_desc, tx_buf).unwrap();
    let spi = esp_hal::spi::master::Spi::new(peripherals.SPI2, spi_config)
        .expect("SPI failed")
        .with_sck(peripherals.GPIO11)
        .with_sio0(peripherals.GPIO4)
        .with_sio1(peripherals.GPIO5)
        .with_sio2(peripherals.GPIO6)
        .with_sio3(peripherals.GPIO7)
        .with_dma(peripherals.DMA_CH1)
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
    // SETUP WIFI (ON LOW-POWER MODE)
    let backend_port: u16 = crate::state::BACKEND_TCP_PORT_STR.parse().expect("Invalid BACKEND_TCP_PORT");
    let stack = base::wifi::init(&spawner, peripherals.WIFI, backend_port).await;
    
    // WIFI CONFIGURED TO SIT IDLE AND AWAIT START/STOP COMMANDS
    // VOICE COMMUNICATION REQUIRES LOCAL NETWORK - START IT UP! (CAN BE TOGGLED AT RUNTIME)
    crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Enable).await;
    crate::store!(crate::state::WIFI_STATE, true);
    // LET IT CONNECT PROPERLY BEFORE CONTINUING
    // WILL TRY TO CONNECT 3 TIMES BEFORE MOVING ON TO THE NEXT CONFIGURED SSID (ENV VARS)
    crate::delay_s!(5);
    defmt::info!("IP: {:08x}", crate::load!(crate::state::CURRENT_IP));

    // SYNC REAL TIME CLOCK (RTC) TO NETWORK TIME PROTOCOL POOL (NTP)
    // DONE ONCE - WE THEN +1 MINUTE EVERY 60 SECONDS
    // THIS AVOIDS POLLING FROM I2C BUS TO TRACK TIME - SEEMS WAY MORE BATTERY EFFICIENT
    match crate::components::pcf85063a::ntp_sync(&stack).await {
        Ok(()) => defmt::info!("PCF85063A Successful synchronization"),
        Err(e) => defmt::warn!("NTP sync failed: {}", e),
    }


    // ───────────────────────────────────────────────────────────────────────
    // I2S AUDIO SETUP 
    // CREATE RX & TX DMA BUFFERS
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(crate::state::I2S_BUFFER_SIZE);

    // CONFIGURE THE I2S PERIPHERAL
    // BY VARIABLES FOR A CODEC SYNCRONIZED AUDIO SETUP
    let i2s = esp_hal::i2s::master::I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
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
    // PRINT OS INFORMATION
    defmt::info!("╬═══════════════════════════════╬");
    defmt::info!("╬ STARTED {} v{} ╬",
        crate::state::PROJECT_NAME,
        crate::state::FW_VERSION
    ); defmt::info!("╬═══════════════════════════════╬");

    // ───────────────────────────────────────────────────────────────────────
    // TASKS

    // SPEAKER TASK (WRITES AUDIO DATA INTO PIPE + KEEP CLOCKS FOR MIC)
    // TASK STARTS IDLE AND WAITS FOR A COMMAND 
    crate::spawn!(spawner, yo_esp::speaker_task(tx_transfer));
    // WE START IT - TO AVOID LATE DMA
    yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Start).await;
    crate::store!(crate::state::SPEAKER_TASK_STATE, true);
    
    // NETWORK DEPENDENT TASKS
    if crate::load!(crate::state::WIFI_CONNECTED) {
        // SPEAKER TASK (STREAM AUDIO TO THE SPEAKER OVER TCP PORT 12345)
        // (SLEEPS UNLESS AUDIO IS RECIEVED)
        crate::spawn!(spawner, yo_esp::stream_speaker(stack, backend_port));
        // MICROPHONE TASK (STREAMS AUDIO TO BACKEND OVER TCP PORT 12345)
        // (SLEEPS UNLESS WAKE-WORD ENABLED/BUTTON IS PRESSED)
        crate::spawn!(spawner, yo_esp::audio_capture_task(i2s_rx, stack, crate::state::BACKEND_TCP_HOST, backend_port, "esp", handler));
        // HTTP API & WEB SERVER TASK (PORT 80)
        // (IDLE - SEND START/STOP COMMAND)
        crate::spawn!(spawner, tinyapi::web_server_task(stack));
        // TO ENABLE VOICE DRIVEN OPERATIONS - WE START IT NOW!
        tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Start).await;
        crate::store!(crate::state::API_STATE, true);
    }
    // LISTEN FOR GUI MEDIA TOUCH EVENTS (PLAY/PAUSE/NEXT ETC)
    // (NEVER WAKES CPU)
    crate::spawn!(spawner, crate::applications::media_player::media_command_task(spawner));
    // BUTTON MONITORING TASK
    crate::spawn!(spawner, crate::components::buttons::buttons_task(button_boot, button_power));
    // TOUCH TASK (SLEEPS WHEN NO TOUCH)
    crate::spawn!(spawner, crate::gui::pages::touch_task(touch_int));
    // DISPLAY TASK
    crate::spawn!(spawner, display_task(fb, display, te_pin));


    // ───────────────────────────────────────────────────────────────────────
    // IT'S NOW SAFE TO CRANK UP THE AMP
    // WITH NO LOAD POPPIN' NOISE
    crate::amp_on();
    crate::delay_s!(1);

    // PLAY BOOT SOUND
    yo_esp::play_ding().await;
    crate::delay_s!(2);

    // HAVING AMPPLIFIER ON WHEN NOT USED IS NOT BEST PRACTICE & BURNS BATTERY!
    // WE TURN IT BACK ON AGAIN ONLY WHEN AUDIO WILL BE PLAYED
    crate::amp_off();
  
      
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
        // BUT ONLY EVERY 5TH MINUTE SINCE THIS DRAWS BATTERY
        if minutes % 10 == 5 {
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
        let rssi = crate::load!(crate::state::RSSI);
        defmt::info!("🛜 {} dBm", rssi);        
        defmt::info!("{} {}% ({} mv)", emoji, percent, mv);
        
        // DISPLAY IS NOW DIRTY
        crate::dirty!();
        
        // SLEEP 60 SECONDS AND RERUN LOOP
        crate::delay_s!(60);      
        // THE END!
    } // 🦆🧑‍🦯 thank you for quackin' along!
    // if you found this helpful - please concider buying me a coffee 
} // ☕ ⮞ https://buymeacoffee.com/quackhackmcblindy
