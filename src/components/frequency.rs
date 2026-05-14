// COMPONENTS/FREQUENCY
// RUNTIME CPU FREQUENCY SWITCHING (DVFS) FOR ESP32-S3

// ESP-HAL MARKS `CpuClock::configure()` AS `pub(crate)`, BUT THE HARDWARE
// DOESN'T CARE ABOUT RUST VISIBILITY. WE REPLICATE THE EXACT SAME REGISTER
// WRITES THAT ESP-HAL'S `set_cpu_clock()` DOES INTERNALLY:
//
//   1. SET `SYSTEM.cpu_per_conf.cpuperiod_sel` (0 = 80 MHZ, 1 = 160 MHZ, 2 = 240 MHZ)
//   2. ENSURE `SYSTEM.cpu_per_conf.pll_freq_sel` = 1 (480 MHZ PLL, WHICH IS THE DEFAULT)
//   3. ENSURE `SYSTEM.sysclk_conf.soc_clk_sel` = 1 (PLL SOURCE)
//   4. CALL THE ROM FUNCTION `ets_update_cpu_frequency(mhz)` SO DELAY FUNCTIONS STAY ACCURATE.
//
// THIS IS SAFE TO CALL AT RUNTIME AS LONG AS NO PERIPHERAL IS MID-TRANSACTION
// ON A CLOCK-DERIVED BUS.

// ESP32-S3 SYSTEM PERIPHERAL BASE ADDRESS (FROM TRM §4.12).
const SYSTEM_BASE: u32 = 0x600C_0000;
// SYSTEM.sysclk_conf REGISTER OFFSET
const SYSCLK_CONF_OFFSET: u32 = 0x058;
// SYSTEM.cpu_per_conf REGISTER OFFSET
const CPU_PER_CONF_OFFSET: u32 = 0x068;

// SWITCH THE CPU CLOCK TO THE GIVEN FREQUENCY (80, 160, OR 240 MHz)
// RETURNS THE ACTUAL FREQUENCY SET.
pub fn set_cpu_mhz(mhz: u16) -> u16 {
    let (period_sel, actual_mhz): (u32, u16) = match mhz {
        0..=80 => (0, 80),
        81..=160 => (1, 160),
        _ => (2, 240),
    };

    unsafe {
        let sysclk_conf = (SYSTEM_BASE + SYSCLK_CONF_OFFSET) as *mut u32;
        let cpu_per_conf = (SYSTEM_BASE + CPU_PER_CONF_OFFSET) as *mut u32;

        // ENSURE PLL SOURCE SELECTED (SOC_CLK_SET === BITS [15:14] === 0b01)
        let mut sc = core::ptr::read_volatile(sysclk_conf);
        // SOC_CLK === 1 (PLL)
        sc = (sc & !(0b11 << 14)) | (1 << 14);
        core::ptr::write_volatile(sysclk_conf, sc);

        // SET `cpuperiod_sel` (BITS [1:0]) & `pll_freq_sel` (BIT 2 === 1 FOR 480MHz PLL)
        let mut cp = core::ptr::read_volatile(cpu_per_conf);
        cp = (cp & !0b111) | (1 << 2) | period_sel; // pll_freq_sel=1, cpuperiod_sel=N
        core::ptr::write_volatile(cpu_per_conf, cp);
    }

    // TELL ROM THE NEW FREQUENCY SO `ets_delay_us()` STAY ACCURATE
    esp_hal::rom::ets_update_cpu_frequency_rom(actual_mhz as u32);

    actual_mhz
}
