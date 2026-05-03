// STATE MACHINE INITIATION
// CHIP GPIO, CONFIGURATION DEFINITIONS
// ++ ME STATE TRACKER
// ++ INIT ATOMIC VARIABLES


// ME (QUACKHACK-MCBLINDY)
// TRACK ME - I MIGHT BE LOST
crate::init_bool!(ME_HOME, true);
crate::init_bool!(ME_SLEEPING, false);
crate::init_bool!(ME_DND, false);
// init_str!(ME_ROOM, "livingroom");
// init_...!(ME_LONGITUDE, ...);
// init_...!(ME_LATITUDE, ...);

// THIS FIRMWARE
pub const FW_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

// NETWORK
crate::init_u32!(CURRENT_IP, 0);
crate::init_i32!(RSSI, 0);

// WIFI - COMPILE-TIME ENVIRONMENT VARIABLES
pub const SSID: &str = env!("WIFI_SSID");
pub const PASSWORD: &str = env!("WIFI_PASSWORD");
// OPTIONAL MORE WIFI
// ADD AS MANY AS NEEDED
pub const WIFI_CREDENTIALS: &[(&str, &str)] = &[
    (SSID, PASSWORD),
    (env!("WIFI_SSID2"), env!("WIFI_PASSWORD2")),
    (env!("WIFI_SSID3"), env!("WIFI_PASSWORD3")),
];

// WIFI - STATE
crate::init_bool!(WIFI_CONNECTED, false);

// BACKEND
pub const BACKEND_TCP_HOST: &str = env!("BACKEND_TCP_HOST");
pub const BACKEND_TCP_PORT_STR: &str = env!("BACKEND_TCP_PORT");

// QSPI DISPLAY (CO5300)
pub const LCD_SDIO0: u8 = 4;
pub const LCD_SDIO1: u8 = 5;
pub const LCD_SDIO2: u8 = 6;
pub const LCD_SDIO3: u8 = 7;
pub const LCD_SCLK: u8 = 11;
pub const LCD_CS: u8 = 12;
pub const LCD_RESET: u8 = 8;
pub const LCD_WIDTH: u16 = 410;
pub const LCD_HEIGHT: u16 = 502;
pub const LCD_COL_OFFSET: u16 = 22;
pub const LCD_ROW_OFFSET: u16 = 0;

crate::init_bool!(DISPLAY_STATE, false);
crate::init_u8!(DISPLAY_BRIGHTNESS, 70);

// I2C Bus
crate::init_u8!(I2C_SDA, 15);
crate::init_u8!(I2C_SCL, 14);
pub const I2C_FREQ_HZ: u32 = 400_000;

// TOUCH (FT3168)
crate::init_u8!(TP_INT, 38);
crate::init_u8!(TP_RESET, 9);
crate::init_u8!(TP_I2C_ADDR, 0x38);

// PMU (AXP2101)
crate::init_u8!(PMIC_I2C_ADDR, 0x34);
crate::init_bool!(POWER_STATE, true);

// BATTERY
crate::init_u8!(BATTERY_PERCENT, 100);
crate::init_u32!(BATTERY_VOLTAGE, 0);
crate::init_bool!(BATTERY_CHARGING, false);
crate::init_bool!(BATTERY_NEED_CHARGING, false);
crate::init_bool!(BATTERY_FULL, false);
// init_..!(BATTERY_VOLTAGE_MV, 0);

// IMU (QMI8658)
crate::init_u8!(IMU_I2C_ADDR, 0x6B);
// IMU INTERRUPT
crate::init_u8!(IMU_INT, 21);

// RTC (PCF85063A)
crate::init_u8!(RTC_I2C_ADDR, 0x51);

pub static CURRENT_TIME: critical_section::Mutex<core::cell::Cell<Option<crate::components::pcf85063a::DateTime>>> =
    critical_section::Mutex::new(core::cell::Cell::new(None));

// RTC INTERRUPT
crate::init_u8!(RTC_INT, 39);

// SD CARD
crate::init_u8!(SD_CLK, 2);
crate::init_u8!(SD_CMD, 1);
crate::init_u8!(SD_DATA, 3);
crate::init_u8!(SD_CS, 17);

// DISPLAY TE (TEARING EFFECT SYNC)
crate::init_u8!(LCD_TE, 13);

// I2S AUDIO GPIO
crate::init_u8!(I2S_MCLK, 16);
crate::init_u8!(I2S_SCLK, 41);  // BCLK
crate::init_u8!(I2S_LRCK, 45);  // WS
crate::init_u8!(I2S_DSDIN, 40); // DAC data in (speaker)
crate::init_u8!(I2S_ASDOUT, 42);// ADC data out (microphone)
crate::init_u8!(PA_CTRL, 46);   // Power amplifier enable

// I2S AUDIO CONFIG
pub const I2S_SAMPLE_RATE: u32 = 16000;
pub const I2S_SAMPLE_COUNT: usize = 256;
pub const I2S_BIT_WIDTH: u8 = 16;
pub const I2S_BUFFER_SIZE: usize = 4 * 4092;
pub const I2S_DATA_FORMAT: esp_hal::i2s::master::DataFormat = esp_hal::i2s::master::DataFormat::Data16Channel16;
pub const I2S_ENDIANNESS: esp_hal::i2s::master::Endianness = esp_hal::i2s::master::Endianness::LittleEndian;
pub const I2S_CHANNELS: esp_hal::i2s::master::Channels = esp_hal::i2s::master::Channels::STEREO;
pub const I2S_SIGNAL_LOOPBACK: bool = true;

// BACKWARD COMPABILITY
pub const SAMPLE_RATE: u32 = 16000;
pub const SAMPLE_COUNT: usize = 256;
pub const BUFFER_SIZE: usize = 4 * 4092;


// SPEAKER / MIC VOLUME CTRL
crate::init_u8!(MIC_VOLUME, 72);
crate::init_u8!(SPEAKER_VOLUME, 58);
crate::init_bool!(MIC_MUTED, false);
crate::init_bool!(SPEAKER_MUTED, false);
crate::init_bool!(MIC_ACTIVE, true);

// BUTTONS
crate::init_u8!(BOOT_BUTTON, 0);
crate::init_bool!(BOOT_BUTTON_PRESSED, false);
crate::init_u8!(PWR_BUTTON, 10);
crate::init_bool!(PWR_BUTTON_PRESSED, false);
