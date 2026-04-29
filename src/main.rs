// ESP32-S3-WATCH-rs https://github.com/QuackHack-McBlindy/ESP32-S3-BOX-3-rs
// BARE METAL NO_STD
// VOICE ASSISTANT FIRMWARE FOR: 
// `Waveshare ESP32-S3-Touch-AMOLED-2.06` 

#![no_std]
#![no_main]
// NOBODY TELLS ME WHAT TO DO!
#![allow(non_snake_case)]
#![deny(clippy::mem_forget)]
#![deny(clippy::large_stack_frames)]

// IMPORTS
use esp_println as _;
use defmt::{info, Debug2Format, error};
use core::cell::RefCell;
use critical_section::Mutex as CsMutex;
use embedded_hal_bus::i2c::CriticalSectionDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_hal::i2c::I2c as HalI2c;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};

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

// COMPILE-TIME ENVIORMENT VARIABLES
const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");
const BACKEND_TCP_HOST: &str = env!("BACKEND_TCP_HOST");
const BACKEND_TCP_PORT_STR: &str = env!("BACKEND_TCP_PORT");
const FW_VERSION: &str = env!("CARGO_PKG_VERSION");
const SAMPLE_RATE: u32 = 16000;
const SAMPLE_COUNT: usize = 256;
const BUFFER_SIZE: usize = 4 * 4092;

// LOAD MODULES
mod base;

// INIT ATOMIC DEFAULTS 
init_bool!(MIC_MUTED, false);
init_bool!(SPEAKER_MUTED, false);
init_bool!(DISPLAY_STATE, false);
init_u8!(MIC_VOLUME, 72);
init_u8!(SPEAKER_VOLUME, 58);
init_u8!(BATTERY_PERCENT, 100);
init_u32!(BATTERY_VOLTAGE, 0);
init_u32!(CURRENT_IP, 0);
init_i32!(RSSI, 0);

pub static ES7210: CsMutex<RefCell<Option<es7210::Es7210>>> = CsMutex::new(RefCell::new(None));
pub static ES8311: CsMutex<RefCell<Option<es8311::Es8311>>> = CsMutex::new(RefCell::new(None));
pub static I2C_BUS: CsMutex<RefCell<Option<I2cBus>>> = CsMutex::new(RefCell::new(None));
pub type I2cBus = I2c<'static, esp_hal::Blocking>;

// CONSTRUCT THE VOICE HANDLER
struct VoiceHandler;

// CONFIGURE VOICE EVENTS
impl yo_esp::CommandHandler for VoiceHandler {    
    // 0x01 === WAKE WORD DETECTED
    fn on_detected(&mut self) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        alloc::boxed::Box::pin(async {
            // PLAY DING SOUND
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
        Box::pin(async move {       
            // PLAY DONE SOUND
            yo_esp::play_done().await;
        })
    }

    // 0x04 === FAILED COMMAND EXECUTION
    fn on_failed(&mut self, _ms: Option<u64>) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + '_>> {
        Box::pin(async move {         
            // PLAY DUCK SAY `OH FUCK` SOUND
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
    defmt::info!("Started ESP32-S3-WATCH (version {})", FW_VERSION);

    // POWER BUTTON (RIGHT SIDE - DOWN BUTTON)
    //let _button_power = esp_hal::gpio::Input::new(
    //    peripherals.GPIO10,
    //    esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up)
    //);

    // ENABLE POWER AMPLIFIER
    let pa_enable = esp_hal::gpio::Output::new(
        peripherals.GPIO46,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default()
    );


    // I2C BUS A
    let i2c_a = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(esp_hal::time::Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO15)
    .with_scl(peripherals.GPIO14);    

    // LOCK & SHARE BUS
    let i2c_a_mutex = Box::leak(Box::new(CsMutex::new(RefCell::new(i2c_a))));

    // AUDIO CODEC CONFIGURATION
    let es7210 = es7210::Es7210::new(0x40);
    let es8311 = es8311::Es8311::new(0x18);

    { // CONFIGURE AUDIO CODECS
        let mut i2c = CriticalSectionDevice::new(&i2c_a_mutex);

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
        match es7210.config_codec(&mut i2c, &codec_cfg) {
            Ok(()) => info!("ES7210 initialized successfully"),
            Err(e) => info!("ES7210 init failed: {:?}", Debug2Format(&e)),
        }
        if let Err(e) = es7210.gain_set(&mut i2c, 20) {
            info!("ES7210 volume set failed: {:?}", Debug2Format(&e));
        }
        if let Err(e) = es7210.set_mute(&mut i2c, false) {
            info!("Failed to configure ES7210 mute status {:?}", Debug2Format(&e));
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
            &mut i2c,
            &clock_cfg,
            es8311::Resolution::Bits16,
            es8311::Resolution::Bits16,
            &mut delay,
        ) {
            Ok(()) => info!("ES8311 initialised successfully"),
            Err(e) => info!("ES8311 init failed: {:?}", Debug2Format(&e)),
        }
        let _ = es8311.volume_set(&mut i2c, 80, None);
        let _ = es8311.mute(&mut i2c, false);
    } // RELEASE I2C

 
    // WIFI SETUP
    let backend_port: u16 = BACKEND_TCP_PORT_STR.parse().expect("Invalid BACKEND_TCP_PORT");
    let (stack, remote_addr) = base::wifi::init(&spawner, peripherals.WIFI, backend_port).await;

    // I2S AUDIO SETUP 
    let (_rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(BUFFER_SIZE);

    let i2s = esp_hal::i2s::master::I2s::new(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        esp_hal::i2s::master::Config::new_tdm_philips()
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
    // BUILD I2S TX (MASTER) WITH BCLK, LRCLK AND DIGITAL OUT PINS 
    let i2s_tx = i2s.i2s_tx
        .with_bclk(peripherals.GPIO41)
        .with_ws(peripherals.GPIO45)
        .with_dout(peripherals.GPIO42)
        .build(tx_descriptors);

    // AUDIO INPUT
    // BUILD I2S RX (SLAVE) WITH DIGITAL-IN PIN 
    let i2s_rx = i2s
        .i2s_rx
        .with_din(peripherals.GPIO40)
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
    //crate::base::api::init_routes().await;


    // TASKS

    // WEB SERVER TASK PORT 80
    //spawn!(spawner, tinyapi::web_server_task(stack));    
    // START THE SPEAKER DMA PUMP
    spawn!(spawner, yo_esp::speaker_task(tx_transfer));
    // ROUTE TCP 12345 TO THE SPEAKER
    spawn!(spawner, yo_esp::stream_speaker(stack, backend_port));
    // ROUTE TCP 12345 FROM MIC TO SERVER
    // A BIDIRECTIONAL CONNECTION IS ESTABLISHED. 
    spawn!(spawner, yo_esp::audio_capture_task(i2s_rx, stack, remote_addr, "esp", handler));

    
    // MAIN LOOP
    loop {        
        delay_s!(60);
    }    
}
