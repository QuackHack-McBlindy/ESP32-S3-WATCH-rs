//! Runtime CPU frequency switching (DVFS) for ESP32-S3.
//!
//! esp-hal marks `CpuClock::configure()` as `pub(crate)`, but the hardware
//! doesn't care about Rust visibility. We replicate the exact same register
//! writes that esp-hal's `set_cpu_clock()` does internally:
//!
//!   1. Set `SYSTEM.cpu_per_conf.cpuperiod_sel` (0 = 80 MHz, 1 = 160 MHz, 2 = 240 MHz)
//!   2. Ensure `SYSTEM.cpu_per_conf.pll_freq_sel` = 1 (480 MHz PLL, which is the default)
//!   3. Ensure `SYSTEM.sysclk_conf.soc_clk_sel` = 1 (PLL source)
//!   4. Call the ROM function `ets_update_cpu_frequency(mhz)` so delay functions stay accurate.
//!
//! This is safe to call at runtime as long as no peripheral is mid-transaction
//! on a clock-derived bus. In practice the watchface main loop is single-threaded
//! and we only switch between render frames, so it's fine.

// ESP32-S3 SYSTEM PERIPHERAL BASE ADDRESS (FROM TRM §4.12).
const SYSTEM_BASE: u32 = 0x600C_0000;
// SYSTEM.sysclk_conf REGISTER OFFSET
const SYSCLK_CONF_OFFSET: u32 = 0x058;
// SYSTEM.cpu_per_conf REGISTER OFFSET
const CPU_PER_CONF_OFFSET: u32 = 0x068;

// SWITCH THE CPU CLOCK TO THE GIVEN FREQUENCY (80, 160, OR 240 MHz)
// Returns the actual frequency set.
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
