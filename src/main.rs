//! ESP32-S3-WATCH-rs ⮞ https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs
//!  BARE METAL RUST  - HARDWARE ABSTRACTION LAYER `esp-hal`
//!   SMARTWATCH OS   - BY QuackHack-McBlindy 🦆🧑‍🦯
// ───────────────────────────────────────────────────────────────────────
//! “A powerful voice assistant can make a huge difference for blind people.”
//! “Imagine yourself stumbling blindly across the room looking for the remote — meanwhile, I call it using only my voice.”
// ───────────────────────────────────────────────────────────────────────

#![no_std]
#![no_main]
// NOBODY TELLS ME WHAT TO DO!
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused)]
#![deny(clippy::mem_forget)]
#![deny(clippy::large_stack_frames)]

// IMPORTS
use esp_println as _;

// PANIC HANDLER (defmt)
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("⚠️ Panic: {}", defmt::Debug2Format(info));
    loop {}
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

// SHARED RESOURCES
pub static TOUCH_CHANNEL: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, crate::components::ft3168::SwipeDirection, 5> = embassy_sync::channel::Channel::new();
pub static ES7210: critical_section::Mutex<core::cell::RefCell<Option<es7210::Es7210>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static ES8311: critical_section::Mutex<core::cell::RefCell<Option<es8311::Es8311>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static I2C_BUS: critical_section::Mutex<core::cell::RefCell<Option<I2cBus>>> = 
    critical_section::Mutex::new(core::cell::RefCell::new(None));


// ───────────────────────────────────────────────────────────────────────
// CONSTRUCT THE VOICE HANDLER
struct VoiceHandler;

// CONFIGURE VOICE EVENTS
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
// DISPLAY CONTROLLER TASK
#[embassy_executor::task]
async fn display_task(
    mut fb: crate::components::framebuffer::Framebuffer,
    mut display: crate::components::co5300::Co5300Display<'static>,
) {
    // INIT - SET DEFAULT DISPLAY BRIGHTNESS
    display.set_brightness(0xB3); // 70%
    // START WITH THE SCREEN OFF
    // WAKE IT UP BY SAYING: (OR TOUCHING SCREEN)
    // `YO BITCH!` (YOUR WAKE WORD)
    display.display_off();

    // STATE VARIABLES 
    let mut screen_on = false;
    // DEFAULT BRIGHTNESS 35%
    let mut current_brightness = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
    let mut flash_toggle = false;
    let mut last_page = crate::load!(crate::gui::pages::CURRENT_PAGE);

    loop {
        // PROCESS ONE-SHOT COMMANDS FROM THE VOICE HANDLER
        if crate::components::co5300::consume_wake() {
            if !screen_on {
                display.display_on();
                screen_on = true;
            }
            // FORCE REDRAW OF CURRENT PAGE IMMEDIATELY.
            last_page = u8::MAX;
        }

        // CLICKING POWER BUTTON ALSO TURNS ON THE DISPLAY
        if crate::load!(crate::state::DISPLAY_STATE) {
            display.display_on();
            screen_on = true;
        } // CLICKING IT AGAIN TURNS IT BACK OFF
        if !crate::load!(crate::state::DISPLAY_STATE) {
            display.display_off();
            screen_on = false;
        }

        // DISPLAY BRIGHTNESS CONTROL
        let desired = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
        if desired != current_brightness {
            let byte = (desired as u16 * 255 / 100) as u8;
            display.set_brightness(byte);
            current_brightness = desired;
        }

        if crate::components::co5300::consume_sleep() {
            if screen_on {
                display.display_off();
                screen_on = false;
                // AFTER SLEEP, WE WANT A FULL REDRAW ON NEXT WAKE
                last_page = u8::MAX;
            }
        }

        // FLASH DISPLAY OR RENDER PAGE NORMALLY?
        let flashing = crate::components::co5300::is_flashing();
        if flashing {
            flash_toggle = !flash_toggle;
            if flash_toggle {
                display.fill_screen(crate::gui::colors::YELLOW);
            } else { display.fill_screen(crate::gui::colors::BLACK); }
        } else { // NORMAL PAGE RENDERING ( ONLY WHEN SCREEN IS ON )
            if screen_on {
                let page = crate::load!(crate::gui::pages::CURRENT_PAGE);
                let is_apps_page = page == 2; // APPS LAUNCHER

                if is_apps_page || page != last_page || crate::is_dirty!() {
                    fb.clear_color(crate::gui::colors::BLACK);

                    match page {
                        0 => crate::gui::time::draw(&mut fb),
                        1 => crate::gui::battery::draw(&mut fb),
                        2 => crate::gui::apps::draw(&mut fb),
                        10 => crate::gui::media_player::draw(&mut fb),
                        11 => crate::gui::settings::draw(&mut fb),
                        12 => crate::gui::media_player::draw(&mut fb),
                        13 => crate::gui::house::draw(&mut fb),
                        100 => crate::gui::call::draw(&mut fb),
                        101 => crate::gui::text::draw(&mut fb),
                        _ => {}
                    } // DON'T FORGET TO FLUSH FRAMEBUFFER
                    fb.flush(&mut display);
                    last_page = page;
                }
            }
        }

        // SET APPROPRIATE DELAY (REFRESH RATE) FOR THE DISPLAY
        // ( ANIMATIONS ARE NOT VERY PRETTY ON 2 FRAMES PER SECOND )
        let delay_ms = if !screen_on {
            400 // SCREEN OFF – LOW POWER (STILL NEED TO REFRESH TO KNOW WHEN TO TURN BACK ON AGAIN)
        } else if crate::load!(crate::gui::pages::CURRENT_PAGE) == 2 {
            33  // APP LAUNCHER – SMOOTH SCROLLING BETWEEN APPLICATIONS (~30 FPS)        
        } else if crate::load!(crate::gui::pages::CURRENT_PAGE) == 10 {
            50 // MEDIA PLAYER APP – SMOOTHER ANIMATION FOR THE PROGRESS BAR
        } else {
            200// OTHER PAGES (DEFAULT DELAY)
        };
        embassy_time::Timer::after(embassy_time::Duration::from_millis(delay_ms)).await;
    }
}


// ───────────────────────────────────────────────────────────────────────
// FUNCTION TO CONTROL SPEAKER VOLUME (0-100%)
pub fn set_speaker_volume(volume: u8) {
    let volume = volume.min(100);
    crate::store!(crate::state::SPEAKER_VOLUME, volume);
    critical_section::with(|cs| {
        let mut bus = crate::I2C_BUS.borrow_ref_mut(cs);
        let mut codec = crate::ES8311.borrow_ref_mut(cs);

        if let (Some(i2c), Some(es8311)) = (bus.as_mut(), codec.as_mut()) {
            if volume == 0 { // MIGHT AS WELL MUTE THE ES8311 CODEC HERE - SAVES US A FEW mV
                defmt::info!("🔇 Speaker MUTED!");
            } else { defmt::info!("🔊 Volume {}%", volume); }
            let _ = es8311.volume_set(i2c, volume, None);
        }
    });
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
// MAIN
#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    // WE CAN CONTROL CLOCKS AT RUNTIME LATER
    // ALTHOUGH THE VOICE ASSISTANT NEED ALL 240MHz TO FUNCTION PROPERLY.
    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(config);

    // ALLOCATE PSRAM
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    // HEAP ALLOC
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    

    // SOFTWARE INTERRUPT SETUP
    let _sw_ints = esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let sw_int0 = unsafe { esp_hal::interrupt::software::SoftwareInterrupt::steal() };
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int0);

    // TRACK TIME SINCE BOOT FOR DEVICE UPTIME CALCULATION
    let boot_time = embassy_time::Instant::now();


    // BUTTONS (WE MONITOR THEM IN DEDICATED TASK DOWN BELOW)
    // BOOT BUTTON (UPPER RIGHT SIDE BUTTON)
    // WHEN PLAYING MEDIA - THIS INCREASES VOLUME
    let button_boot = esp_hal::gpio::Input::new(
        peripherals.GPIO0,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    // POWER BUTTON (LOWER RIGHT SIDE BUTTON)
    // WHEN PLAYING MEDIA - THIS LOWERS VOLUME
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


    // ───────────────────────────────────────────────────────────────────────
    // MICRO SECURE DIGITAL CARD (STORAGE OVER SPI)
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

    // MOVES OWNERSHIP INTO THE MODULE 
    crate::components::storage::init(sd_card);    
    defmt::info!("STORAGE Successful initialization");
    

    // ───────────────────────────────────────────────────────────────────────
    // I2C
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
    
    // INITIALISE ALL I²C DEVICES (PMU, codecs, touch, battery readings etc)
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
        let mut imu = crate::components::qmi8658::Qmi8658Imu::new(&mut *i2c_bus);
        let _ = imu.init();
    
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
    
        // BATTERY READINGS
        let _mv = pmu.get_battery_voltage(i2c_bus).unwrap_or(0);
        let _is_charging = pmu.is_charging(i2c_bus).unwrap_or(false);    
    
        //  FT3168 - (TOUCH CONTROLLER)
        let mut touch_rst = esp_hal::gpio::Output::new(
            peripherals.GPIO9,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default()
        );
        // GPIO38 IS THE FT3168 INTERUPT LINE - HELD HIGH ON BY PULL-UP, PULLED LOW BY THE CONTROLLER
        // WHEN A FINGER IS ON THE SCREEN WE USE IT BOTH FOR LEVEL CHECKS & AS AN ASYNC WAKE SOURCE
        let mut _touch_int = esp_hal::gpio::Input::new(
            peripherals.GPIO38,
            esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
        ); 
        // TOUCH INIT SEQUENCE
        touch_rst.set_low();
        delay.delay_millis(10);
        touch_rst.set_high();
        delay.delay_millis(50);
        let mut touch = crate::components::ft3168::Ft3168Touch::new(i2c_bus);
        let _ = touch.init();
        defmt::info!("FT3168  Successful initialization");
   
        // STORE THE DRIVER OBJECTS GLOBALLY FOR LATER USE
        ES7210.borrow_ref_mut(cs).replace(es7210);
        ES8311.borrow_ref_mut(cs).replace(es8311);

    });


    // ───────────────────────────────────────────────────────────────────────    
    // DISPLAY - (80MHz OVER SPI)
    let spi_config = esp_hal::spi::master::Config::default()
        .with_frequency(esp_hal::time::Rate::from_mhz(80))
        .with_mode(esp_hal::spi::Mode::_0);

    // CREATE DISPLAY DMA BUFFER
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
    // COMMAND 0x35 === TEARON, PARAMETER 0x00 === VBlank ONLY
    let _te_pin = esp_hal::gpio::Input::new(peripherals.GPIO13, esp_hal::gpio::InputConfig::default());
    defmt::info!("CO5300  Successful initialization");

    // FRAMEBUFFER
    let mut fb = crate::components::framebuffer::Framebuffer::new();
    fb.clear_color(crate::gui::colors::BLACK);
    fb.flush(&mut display);
    


    // ───────────────────────────────────────────────────────────────────────
    // WIFI SETUP
    let backend_port: u16 = crate::state::BACKEND_TCP_PORT_STR.parse().expect("Invalid BACKEND_TCP_PORT");
    let stack = base::wifi::init(&spawner, peripherals.WIFI, backend_port).await;
    
    // LET IT CONNECT PROPERLY BEFORE CONTINUING
    // WILL TRY TO CONNECT 3 TIMES BEFORE MOVING ON TO THE NEXT CONFIGURED SSID
    crate::delay_s!(5);

    // SYNC REAL TIME CLOCK (RTC) TO NETWORK TIME PROTOCOL POOL (NTP)
    match crate::components::pcf85063a::ntp_sync(&stack).await {
        Ok(()) => defmt::info!("PCF85063AA Successful synchronization"),
        Err(e) => defmt::warn!("NTP sync failed: {}", e),
    }


    // ───────────────────────────────────────────────────────────────────────
    // I2S AUDIO SETUP 
    // CREATE RX & TX DMA BUFFERS
    let (_rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(crate::state::I2S_BUFFER_SIZE);

    // CONFIGURE THE I2S PERIPHERAL
    // (I LIKE TO BE EXPLICIT)
    let i2s = esp_hal::i2s::master::I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        // I2SCONFIG MATCHING AUDIO CODEC CONFIGURATION
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

    // AUDIO OUTPUT
    // BUILD I2S TX (MASTER) WITH:
    // ++ BCLK ++ LRCLK ++ DIGITAL-OUTPUT PINS  
    let i2s_tx = i2s.i2s_tx
        .with_bclk(peripherals.GPIO41)
        .with_ws(peripherals.GPIO45)
        .with_dout(peripherals.GPIO40)
        .build(tx_descriptors);

    // AUDIO INPUT
    // BUILD I2S RX (SLAVE) WITH DIGITAL-INPUT PIN ONLY
    let i2s_rx = i2s.i2s_rx
        .with_din(peripherals.GPIO42)
        .build(rx_descriptors);

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
    // FINISHED BOOTING PROCESS
    // PRINT FIRMWARE INFORMATION
    defmt::info!("╬═══════════════════════════════╬");
    defmt::info!("╬ STARTED {} v{} ╬",
        crate::state::PROJECT_NAME,
        crate::state::FW_VERSION
    ); defmt::info!("╬═══════════════════════════════╬");


    // ───────────────────────────────────────────────────────────────────────
    // TASKS

    // SPEAKER TASK (WRITES AUDIO DATA INTO PIPE)
    crate::spawn!(spawner, yo_esp::speaker_task(tx_transfer));
    // NETWORK DEPENDENT TASKS
    if crate::load!(crate::state::WIFI_CONNECTED) {
        // SPEAKER TASK (STREAM AUDIO TO SPEAKER OVER TCP PORT 12345)
        crate::spawn!(spawner, yo_esp::stream_speaker(stack, backend_port));
        // MICROPHONE TASK (STREAM AUDIO TO SERVER OVER TCP PORT 12345)
        crate::spawn!(spawner, yo_esp::audio_capture_task(i2s_rx, stack, crate::state::BACKEND_TCP_HOST, backend_port, "esp", handler));
        // HTTP WEB SERVER TASK (PORT 80)
        crate::spawn!(spawner, tinyapi::web_server_task(stack));          
    }
    // LISTEN FOR GUI MEDIA TOUCH EVENTS (PLAY/PAUSE/NEXT ETC)
    crate::spawn!(spawner, crate::applications::media_player::media_command_task(spawner));
    // BUTTON MONITOR TASK
    crate::spawn!(spawner, crate::components::buttons::buttons_task(button_boot, button_power));
    // TOUCH TASK
    crate::spawn!(spawner, crate::gui::pages::touch_task());
    // DISPLAY TASK
    crate::spawn!(spawner, display_task(fb, display));
    // RTC TASK 
    crate::spawn!(spawner, crate::components::pcf85063a::rtc_update_task());


    // ───────────────────────────────────────────────────────────────────────
    // IT'S NOW SAFE TO CRANK UP THE AMP
    // WITHOUT LOAD POPPING NOISE
    amp.set_high();
    crate::delay_s!(1);


    // PLAY BOOT SOUND
    yo_esp::play_ding().await;
 

    // MAIN LOOP
    loop { // GET BATTERY STATUS
        let (percent, voltage_mv, charging, usb_connected) = critical_section::with(|cs| {
            let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
            let i2c_bus = bus_ref.as_mut().unwrap();
            (
                pmu.get_battery_percent(i2c_bus).unwrap_or(0),
                pmu.get_battery_voltage(i2c_bus).unwrap_or(0),
                pmu.is_charging(i2c_bus).unwrap_or(false),
                pmu.is_vbus_in(i2c_bus).unwrap_or(false),
            )
        });

        // CALCULATE DEVICE UPTIME
        let elapsed = embassy_time::Instant::now() - boot_time;
        let uptime_secs = elapsed.as_secs() as u32;
        let days = elapsed.as_secs() / 86400;
        let hours = (elapsed.as_secs() % 86400) / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;

        // STORE ATOMIC VARIABLES
        if percent == 100 {
            crate::store!(crate::state::BATTERY_FULL, true);
        } else { crate::store!(crate::state::BATTERY_FULL, false); }
        if percent < 25 {
            crate::store!(crate::state::BATTERY_NEED_CHARGING, true);
        } else { crate::store!(crate::state::BATTERY_NEED_CHARGING, false); }
        crate::store!(crate::state::UPTIME_SECS, uptime_secs);
        crate::store!(crate::state::BATTERY_VOLTAGE, voltage_mv as u32);
        crate::store!(crate::state::BATTERY_PERCENT, percent);
        crate::store!(crate::state::BATTERY_CHARGING, charging);
        crate::store!(crate::state::BATTERY_USB_CONNECTED, usb_connected);

        // TIME: (HH:MM:SS)
        let maybe_time = critical_section::with(|cs| crate::state::CURRENT_TIME.borrow(cs).get());
        if let Some(dt) = maybe_time {
            defmt::info!("⏰ {:02}:{:02}:{:02}", dt.hours, dt.minutes, dt.seconds);   // logging stays
            let secs = dt.hours as u32 * 3600 + dt.minutes as u32 * 60 + dt.seconds as u32;
            crate::store!(crate::state::CURRENT_TIME_SECS, secs);
        }

        // FORMAT
        let rssi = crate::load!(crate::state::RSSI);
        let emoji = match (percent, charging) {
            (0..=10, false) => "🪫⚠️",
            (0..=10, true)  => "🪫⚡",
            (11..=29, false) => "🪫",
            (11..=29, true)  => "🪫⚡",
            (30..=70, false) => "🔋",
            (30..=70, true)  => "🔋⚡",
            (_, false)       => "🔋",
            (_, true)        => "🔋⚡",
        };
        
        // PRINT BATTERY INFORMATION
        defmt::info!("{} {}% ({} mV)", emoji, percent, voltage_mv);
        // PRINT WIFI SIGNAL STRENGTH
        defmt::info!("🛜 {} dBm", rssi);
        // PRINT UPTIME (TIME SINCE BOOT)
        if days > 0 {
            if hours > 0 {
                defmt::info!("⏱️  {}D {:02}H {:02}M uptime", days, hours, minutes);
            } else { defmt::info!("⏱️  {}D {:02}M uptime", days, minutes); }
        } else if hours > 0 {
            defmt::info!("⏱️  {:02}H {:02}M uptime", hours, minutes);
        } else { defmt::info!("⏱️  {:02}M uptime", minutes); }

        // REDRAW THE DISPLAY (IT'S DIRTY!)
        crate::dirty!();
        // EVERY MINUTE
        crate::delay_s!(60);        
        
    } // 🦆🧑‍🦯 thank you for quackin' along!
    // if you found this helpful - please concider buying me a coffee 
} // ☕ ⮞ https://buymeacoffee.com/quackhackmcblindy
