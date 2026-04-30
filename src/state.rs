// STATE MACHINE INITIATION
// CHIP GPIO, CONFIGURATION DEFINITIONS
// ++ ME STATE TRACKER
// ++ INIT ATOMIC VARIABLES

use crate::{init_bool, init_u8, init_u32, init_i32};

// ME (QUACKHACK-MCBLINDY)
// TRACK ME - I MIGHT BE LOST
init_bool!(ME_HOME, true);
init_bool!(ME_SLEEPING, false);
// init_str!(ME_ROOM, "livingroom");
// init_...!(ME_LONGITUDE, ...);
// init_...!(ME_LATITUDE, ...);

// THIS FIRMWARE
pub const FW_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

// NETWORK
init_u32!(CURRENT_IP, 0);
init_i32!(RSSI, 0);

// WIFI - COMPILE-TIME ENVIRONMENT VARIABLES
pub const SSID: &str = env!("WIFI_SSID");
pub const PASSWORD: &str = env!("WIFI_PASSWORD");


// WIFI - STATE
init_bool!(WIFI_CONNECTED, false);

// BACKEND
pub const BACKEND_TCP_HOST: &str = env!("BACKEND_TCP_HOST");
pub const BACKEND_TCP_PORT_STR: &str = env!("BACKEND_TCP_PORT");

// QSPI DISPLAY (CO5300)
init_u8!(LCD_SDIO0, 4);
init_u8!(LCD_SDIO1, 5);
init_u8!(LCD_SDIO2, 6);
init_u8!(LCD_SDIO3, 7);
init_u8!(LCD_SCLK, 11);
init_u8!(LCD_CS, 12);
init_u8!(LCD_RESET, 8);
init_u32!(LCD_WIDTH, 410);
init_u32!(LCD_HEIGHT, 502);
init_u32!(LCD_COL_OFFSET, 22);
init_u32!(LCD_ROW_OFFSET, 0);

init_bool!(DISPLAY_STATE, false);
init_u8!(DISPLAY_BRIGHTNESS, 70);

// I2C Bus
init_u8!(I2C_SDA, 15);
init_u8!(I2C_SCL, 14);
init_u32!(I2C_FREQ_HZ, 400_000);

// TOUCH (FT3168)
init_u8!(TP_INT, 38);
init_u8!(TP_RESET, 9);
init_u8!(TP_I2C_ADDR, 0x38);

// PMU (AXP2101)
init_u8!(PMIC_I2C_ADDR, 0x34);
init_bool!(POWER_STATE, true);

// BATTERY
init_u8!(BATTERY_PERCENT, 100);
init_u32!(BATTERY_VOLTAGE, 0);
init_bool!(BATTERY_CHARGING, false);
init_bool!(BATTERY_NEED_CHARGING, false);
init_bool!(BATTERY_FULL, false);
// init_..!(BATTERY_VOLTAGE_MV, 0);

// IMU (QMI8658)
init_u8!(IMU_I2C_ADDR, 0x6B);
// IMU INTERRUPT
init_u8!(IMU_INT, 21);

// RTC (PCF85063A)
init_u8!(RTC_I2C_ADDR, 0x51);
// RTC INTERRUPT
init_u8!(RTC_INT, 39);

// SD CARD
init_u8!(SD_CLK, 2);
init_u8!(SD_CMD, 1);
init_u8!(SD_DATA, 3);
init_u8!(SD_CS, 17);

// DISPLAY TE (TEARING EFFECT SYNC)
init_u8!(LCD_TE, 13);

// I2S AUDIO GPIO
init_u8!(I2S_MCLK, 16);
init_u8!(I2S_SCLK, 41);  // BCLK
init_u8!(I2S_LRCK, 45);  // WS
init_u8!(I2S_DSDIN, 40); // DAC data in (speaker)
init_u8!(I2S_ASDOUT, 42);// ADC data out (microphone)
init_u8!(PA_CTRL, 46);   // Power amplifier enable

// I2S AUDIO CONFIG
init_u32!(I2S_SAMPLE_RATE, 16000);
pub const I2S_SAMPLE_COUNT: usize = 256;
pub const I2S_BUFFER_SIZE: usize = 4 * 4092;
// BACKWARD COMPABILITY
pub const SAMPLE_RATE: u32 = 16000;
pub const SAMPLE_COUNT: usize = 256;
pub const BUFFER_SIZE: usize = 4 * 4092;


// SPEAKER / MIC VOLUME CTRL
init_u8!(MIC_VOLUME, 72);
init_u8!(SPEAKER_VOLUME, 58);
init_bool!(MIC_MUTED, false);
init_bool!(SPEAKER_MUTED, false);
init_bool!(MIC_ACTIVE, true);

// BUTTONS
init_u8!(BOOT_BUTTON, 0);
init_bool!(BOOT_BUTTON_PRESSED, false);
init_u8!(PWR_BUTTON, 10);
init_bool!(PWR_BUTTON_PRESSED, false);
