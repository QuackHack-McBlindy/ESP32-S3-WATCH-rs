// ESP32-S3-WATCH-rs https://github.com/QuackHack-McBlindy/ESP32-S3-BOX-3-rs
// BARE METAL NO_STD
// VOICE ASSISTANT FIRMWARE FOR: 
// `WaveShare ESP32-S3-Touch-AMOLED-2.06` 

#![no_std]
#![no_main]
// NOBODY TELLS ME WHAT TO DO!
#![allow(non_snake_case)]
#![deny(clippy::mem_forget)]
#![deny(clippy::large_stack_frames)]

// IMPORTS
use esp_println as _;
use defmt::{info, Debug2Format, error};

// PANIC HANDLER
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// MEMORY
extern crate alloc;
use alloc::boxed::Box; 

// BOOTLOADER (REQUIRED TO BOOT WITHOUT ESP-IDF)
esp_bootloader_esp_idf::esp_app_desc!();


// LOAD MODULES
mod state;
mod components;
mod base;
//mod gui;
//mod applications;


// TYPE ALIAS FOR I2C BUS
pub type I2cBus = esp_hal::i2c::master::I2c<'static, esp_hal::Blocking>;

// SHARED RESOURCES WITH FULLY QUALIFIED PATHS
pub static ES7210: critical_section::Mutex<core::cell::RefCell<Option<es7210::Es7210>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static ES8311: critical_section::Mutex<core::cell::RefCell<Option<es8311::Es8311>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
pub static I2C_BUS: critical_section::Mutex<core::cell::RefCell<Option<I2cBus>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

// CONSTRUCT THE VOICE HANDLER
struct VoiceHandler;

// CONFIGURE VOICE EVENTS
impl yo_esp::CommandHandler for VoiceHandler {
    // 0x01 === WAKE WORD DETECTED
    fn on_detected(&mut self) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async {
            yo_esp::play_ding().await;
        })
    }

    // 0x02 === SERVER STARTED TRANSCRIPTION
    fn on_thinking(&mut self) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        })
    }

    // 0x03 === COMMAND EXECUTED
    fn on_executed(&mut self, _ms: Option<u64>) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async move {
            yo_esp::play_done().await;
        })
    }

    // 0x04 === FAILED COMMAND EXECUTION
    fn on_failed(&mut self, _ms: Option<u64>) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async move {
            yo_esp::play_fail().await;
        })
    }
}

// MAIN
#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(esp_hal::clock::CpuClock::max());
    let peripherals = esp_hal::init(config);

    // ALLOCATE MEMORY
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    // SOFTWARE INTERRUPT SETUP
    let _sw_ints = esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let sw_int0 = unsafe { esp_hal::interrupt::software::SoftwareInterrupt::steal() };
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int0);
    defmt::info!(
        "Started {} v{}",
        crate::state::PROJECT_NAME,
        crate::state::FW_VERSION
    );

    let boot_time = embassy_time::Instant::now();

    // POWER BUTTON (RIGHT SIDE - DOWN BUTTON)
    let button_power = esp_hal::gpio::Input::new(
        peripherals.GPIO10,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    let button_boot = esp_hal::gpio::Input::new(
        peripherals.GPIO0,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    );

    // ENABLE POWER AMPLIFIER
    let _pa_enable = esp_hal::gpio::Output::new(
        peripherals.GPIO46,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
    );

    // I2C BUS A
    let i2c_a = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(esp_hal::time::Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO15)
    .with_scl(peripherals.GPIO14);

    // LOCK & SHARE BUS
    let i2c_a_mutex = alloc::boxed::Box::leak(alloc::boxed::Box::new(critical_section::Mutex::new(core::cell::RefCell::new(i2c_a))));
    let mut i2c_device = embedded_hal_bus::i2c::CriticalSectionDevice::new(i2c_a_mutex);

    // CREATE PMU
    let pmu = crate::components::axp2101::Axp2101::new();

    { // <-- JUMP INSIDE
        // CONFIGURE AUDIO CODECS
        let es7210 = es7210::Es7210::new(0x40);
        let es8311 = es8311::Es8311::new(0x18);

        // INIT PMU
        if let Err(e) = pmu.init(&mut i2c_device, &crate::components::axp2101::Axp2101Config::default()) {
            defmt::error!("PMU init failed: {:?}", defmt::Debug2Format(&e));
        } else { defmt::info!("AXP2101 ready"); }

        // ES7210 (ADC)
        let codec_cfg = es7210::CodecConfig {
            sample_rate_hz: 16000,
            mclk_ratio: 256,
            i2s_format: es7210::I2sFormat::I2S,
            bit_width: es7210::I2sBits::Bits16,
            mic_bias: es7210::MicBias::V2_87,
            mic_gain: es7210::MicGain::Gain30dB,
            tdm_enable: false,
        };
        match es7210.config_codec(&mut i2c_device, &codec_cfg) {
            Ok(()) => defmt::info!("ES7210 initialized successfully"),
            Err(e) => defmt::info!("ES7210 init failed: {:?}", defmt::Debug2Format(&e)),
        }
        if let Err(e) = es7210.gain_set(&mut i2c_device, 20) {
            defmt::info!("ES7210 volume set failed: {:?}", defmt::Debug2Format(&e));
        }
        if let Err(e) = es7210.set_mute(&mut i2c_device, false) {
            defmt::info!("Failed to configure ES7210 mute status {:?}", defmt::Debug2Format(&e));
        }

        // ES8311 (DAC) 
        let clock_cfg = es8311::ClockConfig {
            mclk_inverted: false,
            sclk_inverted: false,
            mclk_from_mclk_pin: true,
            mclk_frequency: 4096000,
            sample_frequency: 16000,
        };
        let mut delay = esp_hal::delay::Delay::new();
        match es8311.init(
            &mut i2c_device,
            &clock_cfg,
            es8311::Resolution::Bits16,
            es8311::Resolution::Bits16,
            &mut delay,
        ) {
            Ok(()) => defmt::info!("ES8311 initialised successfully"),
            Err(e) => defmt::info!("ES8311 init failed: {:?}", defmt::Debug2Format(&e)),
        }
        let _ = es8311.volume_set(&mut i2c_device, 80, None);
        let _ = es8311.mute(&mut i2c_device, false);

        // INIT FIRST BATTERY READING (mV)
        let mv = pmu.get_battery_voltage(&mut i2c_device).unwrap_or(0);
        defmt::info!("Initial battery voltage: {} mV", mv);
        // READ CHARGING STATE
        let charging = pmu.is_charging(&mut i2c_device).unwrap_or(false);
        defmt::info!("Charging: {}", charging);
    
    } // <-- RELEASE! I2C


    // WIFI SETUP
    let backend_port: u16 = crate::state::BACKEND_TCP_PORT_STR.parse().expect("Invalid BACKEND_TCP_PORT");
    let (stack, remote_addr) = base::wifi::init(&spawner, peripherals.WIFI, backend_port).await;


    // I2S AUDIO SETUP 
    // CREATE RX & TX DMA BUFFERS
    let (_rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(crate::state::I2S_BUFFER_SIZE);

    // CONFIGURE THE I2S INSTANCE
    // (I LIKE TO BE EXPLICIT)
    let i2s = esp_hal::i2s::master::I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        // I2SCONFIG MUST MATCH AUDIO CODEC CONFIG
        esp_hal::i2s::master::Config::new_tdm_philips()
            // SIGNAL LOOPBACK === SET I2S RX AS SSLAVE
            // ++ ENABLES BIDRECTIONAL FULL-DUPLEX I2S ON SINGLE PERIPHERAL
            .with_signal_loopback(true)
            .with_sample_rate(esp_hal::time::Rate::from_hz(16000))
            .with_data_format(esp_hal::i2s::master::DataFormat::Data16Channel16)
            .with_endianness(esp_hal::i2s::master::Endianness::LittleEndian)
            .with_channels(esp_hal::i2s::master::Channels::STEREO),
    )
    .unwrap()
    .into_async()
    .with_mclk(peripherals.GPIO16);

    // AUDIO OUTPUT
    // BUILD I2S TX (MASTER) WITH:
    // ++ BCLK ++ LRCLK ++ DIGITALOUT ++  
    let i2s_tx = i2s.i2s_tx
        .with_bclk(peripherals.GPIO41)
        .with_ws(peripherals.GPIO45)
        .with_dout(peripherals.GPIO40)
        .build(tx_descriptors);

    // AUDIO INPUT
    // BUILD I2S RX (SLAVE) WITH DIGITAL-IN PIN 
    let i2s_rx = i2s
        .i2s_rx
        .with_din(peripherals.GPIO42)
        .build(rx_descriptors);

    // I2S TX CIRCULAR WRITE
    // CONTINUOSLY WRITE TO I2S TX TO KEEP CLOCKS UP FOR RX (SLAVE)
    let tx_transfer = match i2s_tx.write_dma_circular_async(tx_buffer) {
        Ok(t) => t,
        Err(e) => {
            defmt::error!("I2S circular TX failed: {:?}", defmt::Debug2Format(&e));
            panic!("I2S setup error");
        }
    };

    // YO-HANDLER 
    let handler: alloc::boxed::Box<dyn yo_esp::CommandHandler> = alloc::boxed::Box::new(VoiceHandler);

    // INIT API ROUTES
    crate::base::api::init_routes().await;

    // TASKS
    
    // WEB SERVER TASK PORT 80
    spawn!(spawner, tinyapi::web_server_task(stack));  
    
    spawn!(spawner, yo_esp::speaker_task(tx_transfer));
    spawn!(spawner, yo_esp::stream_speaker(stack, backend_port));
    // MICROPHONE TASK (STREAM AUDIO TO SERVER OVER TCP PORT 12345)
    spawn!(spawner, yo_esp::audio_capture_task(i2s_rx, stack, remote_addr, "esp", handler));
    // BUTTON MONITOR TASK
    spawn!(spawner, crate::components::buttons::buttons_task(button_boot, button_power));

    yo_esp::play_ding().await;

    // MAIN LOOP
    loop { // GET BATTERY STATUS
        let percent = pmu.get_battery_percent(&mut i2c_device).unwrap_or(0);
        let voltage_mv = pmu.get_battery_voltage(&mut i2c_device).unwrap_or(0);
        let charging = pmu.is_charging(&mut i2c_device).unwrap_or(false);

        let elapsed = embassy_time::Instant::now() - boot_time;
        let days = elapsed.as_secs() / 86400;
        let hours = (elapsed.as_secs() % 86400) / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;

        store!(crate::state::BATTERY_VOLTAGE, voltage_mv as u32);
        store!(crate::state::BATTERY_PERCENT, percent);
        store!(crate::state::BATTERY_CHARGING, charging);

        // FORMAT
        let rssi = load!(base::wifi::CURRENT_RSSI);
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
        // PRINT
        defmt::info!("{} {}% ({} mV)", emoji, percent, voltage_mv);
        defmt::info!("⏱️: {}D {:02}H {:02}M", days, hours, minutes);
        defmt::info!("🛜 {} dBm", rssi);
        // EVERY 60 SECONDS
        // TODO DYNAMIC TIMER
        delay_s!(60);
    }
}
