#![no_std]

//! # ES8311 – Low‑Power Audio Codec Driver
//!
//! A platform‑independent, `no_std` driver for the Everest Semi **ES8311**
//! audio ADC/DAC codec.  
//! It communicates over I²C using the [`embedded_hal`] abstractions and
//! supports sample rates from 8 kHz to 96 kHz, configurable audio formats,
//! microphone gain, volume control, and smooth fade effects.
//!
//! ## Features
//!
//! *   Generic over any I²C implementation (`embedded_hal::i2c::I2c`)
//! *   Generic over any delay provider (`embedded_hal::delay::DelayNs`)
//! *   Full set of `no_std`‑compatible API functions
//! *   No additional dependencies – only `embedded-hal`
//! *   Exhaustive clock coefficient table for 20+ MCLK / sample‑rate combos
//! *   Simple, ergonomic configuration structs
//!
//! ## Example
//!
//! ```rust
//! use es8311::{Es8311, ClockConfig, Resolution};
//! // Assume i2c and delay instances from your HAL.
//! # fn example<I: embedded_hal::i2c::I2c, D: embedded_hal::delay::DelayNs>(
//! #     i2c: &mut I,
//! #     delay: &mut D,
//! # ) -> Result<(), es8311::Error<I::Error>> {
//! let codec = Es8311::new(0x18);
//!
//! let clock_cfg = ClockConfig {
//!     mclk_inverted: false,
//!     sclk_inverted: false,
//!     mclk_from_mclk_pin: true,
//!     mclk_frequency: 12_288_000,
//!     sample_frequency: 48_000,
//! };
//!
//! codec.init(i2c, &clock_cfg, Resolution::Bits16, Resolution::Bits16, delay)?;
//! codec.volume_set(i2c, 80, None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Licence
//! MIT
//!
//! ## Author
//! QuackHack-McBlidy
#![allow(unused)]
#![allow(dead_code)]

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;

// ---------------------------------------------------------------------------
// Register addresses
// ---------------------------------------------------------------------------
const RESET_REG00: u8 = 0x00;
const CLK_MANAGER_REG01: u8 = 0x01;
const CLK_MANAGER_REG02: u8 = 0x02;
const CLK_MANAGER_REG03: u8 = 0x03;
const CLK_MANAGER_REG04: u8 = 0x04;
const CLK_MANAGER_REG05: u8 = 0x05;
const CLK_MANAGER_REG06: u8 = 0x06;
const CLK_MANAGER_REG07: u8 = 0x07;
const CLK_MANAGER_REG08: u8 = 0x08;
const SDPIN_REG09: u8 = 0x09;
const SDPOUT_REG0A: u8 = 0x0A;
const SYSTEM_REG0B: u8 = 0x0B;
const SYSTEM_REG0C: u8 = 0x0C;
const SYSTEM_REG0D: u8 = 0x0D;
const SYSTEM_REG0E: u8 = 0x0E;
const SYSTEM_REG0F: u8 = 0x0F;
const SYSTEM_REG10: u8 = 0x10;
const SYSTEM_REG11: u8 = 0x11;
const SYSTEM_REG12: u8 = 0x12;
const SYSTEM_REG13: u8 = 0x13;
const SYSTEM_REG14: u8 = 0x14;
const ADC_REG15: u8 = 0x15;
const ADC_REG16: u8 = 0x16;
const ADC_REG17: u8 = 0x17;
const ADC_REG18: u8 = 0x18;
const ADC_REG19: u8 = 0x19;
const ADC_REG1A: u8 = 0x1A;
const ADC_REG1B: u8 = 0x1B;
const ADC_REG1C: u8 = 0x1C;
const DAC_REG31: u8 = 0x31;
const DAC_REG32: u8 = 0x32;
const DAC_REG33: u8 = 0x33;
const DAC_REG34: u8 = 0x34;
const DAC_REG35: u8 = 0x35;
const DAC_REG37: u8 = 0x37;
const GPIO_REG44: u8 = 0x44;
const GP_REG45: u8 = 0x45;
const CHD1_REGFD: u8 = 0xFD;
const CHD2_REGFE: u8 = 0xFE;
const CHVER_REGFF: u8 = 0xFF;

// ---------------------------------------------------------------------------
// Public types & enums
// ---------------------------------------------------------------------------

/// Audio sample resolution (bit width).
///
/// Used for both ADC (SDP In) and DAC (SDP Out) serial ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Resolution {
    /// 16‑bit PCM
    Bits16 = 16,
    /// 18‑bit PCM
    Bits18 = 18,
    /// 20‑bit PCM
    Bits20 = 20,
    /// 24‑bit PCM
    Bits24 = 24,
    /// 32‑bit PCM
    Bits32 = 32,
}

/// Analogue microphone gain.
///
/// The gain values represent the PGA (Programmable Gain Amplifier)
/// setting for the built‑in microphone preamplifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MicGain {
    /// Minimum gain (0x00)
    Min = 0,
    /// 0 dB
    Gain0dB = 1,
    /// +6 dB
    Gain6dB = 2,
    /// +12 dB
    Gain12dB = 3,
    /// +18 dB
    Gain18dB = 4,
    /// +24 dB
    Gain24dB = 5,
    /// +30 dB
    Gain30dB = 6,
    /// +36 dB
    Gain36dB = 7,
    /// +42 dB
    Gain42dB = 8,
    /// Maximum gain (same as +42 dB, 0x3F)
    Max = 9,
}

/// Volume fade speed.
///
/// Controls how many LRCK (left‑right clock) cycles are required to
/// change the volume by 0.25 dB. A value of [`Fade::Off`] disables
/// the fade entirely (instant volume changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Fade {
    /// No fading – immediate volume change.
    Off = 0,
    /// 4 LRCK cycles per 0.25 dB step
    LRCK4 = 1,
    /// 8 LRCK cycles
    LRCK8 = 2,
    /// 16 LRCK cycles
    LRCK16 = 3,
    /// 32 LRCK cycles
    LRCK32 = 4,
    /// 64 LRCK cycles
    LRCK64 = 5,
    /// 128 LRCK cycles
    LRCK128 = 6,
    /// 256 LRCK cycles
    LRCK256 = 7,
    /// 512 LRCK cycles
    LRCK512 = 8,
    /// 1024 LRCK cycles
    LRCK1024 = 9,
    /// 2048 LRCK cycles
    LRCK2048 = 10,
    /// 4096 LRCK cycles
    LRCK4096 = 11,
    /// 8192 LRCK cycles
    LRCK8192 = 12,
    /// 16384 LRCK cycles
    LRCK16384 = 13,
    /// 32768 LRCK cycles
    LRCK32768 = 14,
    /// 65536 LRCK cycles
    LRCK65536 = 15,
}

/// Master clock and sample‑rate configuration.
///
/// The ES8311 can derive its internal clocks from either a dedicated
/// MCLK signal or from the SCLK (bit clock) pin. Choose the
/// appropriate source and provide the associated frequency.
///
/// # Example
///
/// ```rust
/// let clock = ClockConfig {
///     mclk_inverted: false,
///     sclk_inverted: false,
///     mclk_from_mclk_pin: true,
///     mclk_frequency: 12_288_000,
///     sample_frequency: 48_000,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ClockConfig {
    /// Invert the MCLK signal polarity.
    pub mclk_inverted: bool,
    /// Invert the SCLK (BCLK) signal polarity.
    pub sclk_inverted: bool,
    /// `true` → MCLK comes from the MCLK pin;  
    /// `false` → MCLK is derived from SCLK (useful in slave mode).
    pub mclk_from_mclk_pin: bool,
    /// MCLK frequency in Hz.  
    /// Ignored when `mclk_from_mclk_pin` is `false`.
    pub mclk_frequency: u32,
    /// Desired audio sample rate in Hz (8 000 … 96 000).
    pub sample_frequency: u32,
}

/// Errors returned by the driver.
#[derive(Debug)]
pub enum Error<E> {
    /// An error occurred on the I²C bus.
    I2c(E),
    /// A configuration parameter was out of range or inconsistent.
    InvalidConfig,
    /// The requested sample rate / MCLK combination is not
    /// covered by the coefficient table.
    NotSupported,
}

// ---------------------------------------------------------------------------
// Driver struct
// ---------------------------------------------------------------------------

/// ES8311 codec driver.
///
/// Holds only the I²C slave address. All operations require
/// a shared I²C bus instance.
pub struct Es8311 {
    addr: u8,
}

impl Es8311 {
    /// Creates a new ES8311 driver.
    ///
    /// `addr` is the 7‑bit I²C base address of the codec (typically `0x18`).
    pub fn new(addr: u8) -> Self {
        Self { addr }
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn write_reg<I2C, E>(&self, i2c: &mut I2C, reg: u8, val: u8) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let buf = [reg, val];
        i2c.write(self.addr, &buf).map_err(Error::I2c)
    }

    fn read_reg<I2C, E>(&self, i2c: &mut I2C, reg: u8) -> Result<u8, Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut buf = [0];
        i2c.write_read(self.addr, &[reg], &mut buf)
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Initialise the codec with the given clock configuration and
    /// audio resolutions.
    ///
    /// Performs a hardware reset, configures the clock tree, sets up the
    /// audio serial data port, and powers up the analogue blocks.
    ///
    /// # Parameters
    /// * `i2c` – I²C bus
    /// * `clk_cfg` – Clock source and sample‑rate settings
    /// * `res_in` – Resolution of the ADC (input) path
    /// * `res_out` – Resolution of the DAC (output) path
    /// * `delay` – Delay provider used during the reset sequence
    ///
    /// # Errors
    /// * [`Error::InvalidConfig`] if `sample_frequency` is out of range
    ///   or if MCLK is derived from SCLK but `res_in != res_out`.
    /// * [`Error::NotSupported`] if the MCLK / sample rate combination
    ///   is not in the coefficient table.
    pub fn init<I2C, E, D>(
        &self,
        i2c: &mut I2C,
        clk_cfg: &ClockConfig,
        res_in: Resolution,
        res_out: Resolution,
        delay: &mut D,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
        D: DelayNs,
    {
        if !(8000..=96000).contains(&clk_cfg.sample_frequency) {
            return Err(Error::InvalidConfig);
        }
        if !clk_cfg.mclk_from_mclk_pin && res_in != res_out {
            return Err(Error::InvalidConfig);
        }

        // Reset sequence – requires a 20 ms delay
        self.write_reg(i2c, RESET_REG00, 0x1F)?;
        delay.delay_ms(20);
        self.write_reg(i2c, RESET_REG00, 0x00)?;
        self.write_reg(i2c, RESET_REG00, 0x80)?; // Power‑on command

        // Configure clocks, format, and analogue blocks
        self.clock_config(i2c, clk_cfg, res_out)?;
        self.fmt_config(i2c, res_in, res_out)?;

        self.write_reg(i2c, SYSTEM_REG0D, 0x01)?;
        self.write_reg(i2c, SYSTEM_REG0E, 0x02)?;
        self.write_reg(i2c, SYSTEM_REG12, 0x00)?;
        self.write_reg(i2c, SYSTEM_REG13, 0x10)?;
        self.write_reg(i2c, ADC_REG1C, 0x6A)?;
        self.write_reg(i2c, DAC_REG37, 0x08)?;

        Ok(())
    }

    /// Set up the clock dividers and sources.
    fn clock_config<I2C, E>(
        &self,
        i2c: &mut I2C,
        clk_cfg: &ClockConfig,
        res_out: Resolution,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut reg01 = 0x3F; // Enable all internal clocks

        // Determine MCLK frequency
        let mclk_hz = if clk_cfg.mclk_from_mclk_pin {
            reg01 &= !0x80;
            clk_cfg.mclk_frequency
        } else {
            reg01 |= 0x80;
            clk_cfg.sample_frequency * (res_out as u32) * 2
        };

        if clk_cfg.mclk_inverted {
            reg01 |= 0x40;
        }
        self.write_reg(i2c, CLK_MANAGER_REG01, reg01)?;

        // SCLK polarity
        let mut reg06 = self.read_reg(i2c, CLK_MANAGER_REG06)?;
        if clk_cfg.sclk_inverted {
            reg06 |= 0x20;
        } else {
            reg06 &= !0x20;
        }
        self.write_reg(i2c, CLK_MANAGER_REG06, reg06)?;

        // Set sample rate dividers
        self.sample_frequency_config(i2c, mclk_hz, clk_cfg.sample_frequency)
    }

    /// Configure the sample rate dividers for a given MCLK and sample rate.
    ///
    /// This is a low‑level routine – most users should call [`init`] instead.
    /// It can be useful if you need to change the sample rate without a
    /// full re‑initialisation.
    pub fn sample_frequency_config<I2C, E>(
        &self,
        i2c: &mut I2C,
        mclk_hz: u32,
        sample_hz: u32,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let coeff = get_coeff(mclk_hz, sample_hz).ok_or(Error::NotSupported)?;

        // Register 0x02: pre_div and pre_multi
        let mut reg02 = self.read_reg(i2c, CLK_MANAGER_REG02)?;
        reg02 &= 0x07;
        reg02 |= ((coeff.pre_div - 1) as u8) << 5;
        reg02 |= (coeff.pre_multi as u8) << 3;
        self.write_reg(i2c, CLK_MANAGER_REG02, reg02)?;

        // Register 0x03: fs_mode and adc_osr
        let reg03 = ((coeff.fs_mode as u8) << 6) | coeff.adc_osr;
        self.write_reg(i2c, CLK_MANAGER_REG03, reg03)?;

        // Register 0x04: dac_osr
        self.write_reg(i2c, CLK_MANAGER_REG04, coeff.dac_osr)?;

        // Register 0x05: adc_div and dac_div
        let reg05 = ((coeff.adc_div - 1) as u8) << 4 | ((coeff.dac_div - 1) as u8);
        self.write_reg(i2c, CLK_MANAGER_REG05, reg05)?;

        // Register 0x06: bclk_div
        let mut reg06 = self.read_reg(i2c, CLK_MANAGER_REG06)?;
        reg06 &= 0xE0;
        if coeff.bclk_div < 19 {
            reg06 |= (coeff.bclk_div - 1) as u8;
        } else {
            reg06 |= coeff.bclk_div as u8;
        }
        self.write_reg(i2c, CLK_MANAGER_REG06, reg06)?;

        // Register 0x07: lrck_h
        let mut reg07 = self.read_reg(i2c, CLK_MANAGER_REG07)?;
        reg07 &= 0xC0;
        reg07 |= coeff.lrck_h as u8;
        self.write_reg(i2c, CLK_MANAGER_REG07, reg07)?;

        // Register 0x08: lrck_l
        self.write_reg(i2c, CLK_MANAGER_REG08, coeff.lrck_l)?;

        Ok(())
    }

    /// Configure the audio serial data port format (I²S, resolution, …).
    fn fmt_config<I2C, E>(
        &self,
        i2c: &mut I2C,
        res_in: Resolution,
        res_out: Resolution,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        // Ensure slave mode (the default)
        let reg00 = self.read_reg(i2c, RESET_REG00)? & 0xBF;
        self.write_reg(i2c, RESET_REG00, reg00)?;

        let mut reg09 = 0; // SDP In (ADC)
        let mut reg0a = 0; // SDP Out (DAC)

        // Set SDP In resolution
        reg09 |= match res_in {
            Resolution::Bits16 => 3 << 2,
            Resolution::Bits18 => 2 << 2,
            Resolution::Bits20 => 1 << 2,
            Resolution::Bits24 => 0 << 2,
            Resolution::Bits32 => 4 << 2,
        };

        // Set SDP Out resolution
        reg0a |= match res_out {
            Resolution::Bits16 => 3 << 2,
            Resolution::Bits18 => 2 << 2,
            Resolution::Bits20 => 1 << 2,
            Resolution::Bits24 => 0 << 2,
            Resolution::Bits32 => 4 << 2,
        };

        self.write_reg(i2c, SDPIN_REG09, reg09)?;
        self.write_reg(i2c, SDPOUT_REG0A, reg0a)?;

        Ok(())
    }

    /// Configure the microphone input path.
    ///
    /// When `digital_mic` is `true` the digital microphone interface is
    /// enabled; otherwise the analogue microphone amplifier is used.
    pub fn microphone_config<I2C, E>(
        &self,
        i2c: &mut I2C,
        digital_mic: bool,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut reg14 = 0x1A; // Enable analogue MIC, max PGA gain
        if digital_mic {
            reg14 |= 0x40;
        }
        self.write_reg(i2c, ADC_REG17, 0xC8)?; // ADC gain
        self.write_reg(i2c, SYSTEM_REG14, reg14)
    }

    /// Set the microphone preamplifier gain.
    ///
    /// The gain is applied uniformly to all microphone channels.
    pub fn microphone_gain_set<I2C, E>(
        &self,
        i2c: &mut I2C,
        gain: MicGain,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let val = match gain {
            MicGain::Min => 0x00,
            MicGain::Gain0dB => 0x08,
            MicGain::Gain6dB => 0x10,
            MicGain::Gain12dB => 0x18,
            MicGain::Gain18dB => 0x20,
            MicGain::Gain24dB => 0x28,
            MicGain::Gain30dB => 0x30,
            MicGain::Gain36dB => 0x38,
            MicGain::Gain42dB => 0x3F,
            MicGain::Max => 0x3F,
        };
        self.write_reg(i2c, ADC_REG16, val)
    }

    /// Set the output volume (0 – 100).
    ///
    /// A value of 0 means mute, 100 is maximum volume. The underlying
    /// hardware register is updated immediately unless a fade is active.
    ///
    /// If a `volume_set` reference is provided, it will be written with
    /// the (clamped) volume that was actually set.
    pub fn volume_set<I2C, E>(
        &self,
        i2c: &mut I2C,
        volume: u8,
        volume_set: Option<&mut u8>,
    ) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let volume = volume.min(100);
        let reg32 = if volume == 0 {
            0
        } else {
            ((volume as u32 * 256 / 100) - 1) as u8
        };
        if let Some(v) = volume_set {
            *v = volume;
        }
        self.write_reg(i2c, DAC_REG32, reg32)
    }

    /// Read the current output volume (0 – 100).
    pub fn volume_get<I2C, E>(&self, i2c: &mut I2C) -> Result<u8, Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let reg32 = self.read_reg(i2c, DAC_REG32)?;
        Ok(if reg32 == 0 {
            0
        } else {
            ((reg32 as u32 * 100 / 256) + 1) as u8
        })
    }

    /// Mute or unmute the DAC output.
    ///
    /// This is a soft mute – the signal is ramped down slowly
    /// if a fade is configured.
    pub fn mute<I2C, E>(&self, i2c: &mut I2C, mute: bool) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut reg31 = self.read_reg(i2c, DAC_REG31)?;
        if mute {
            reg31 |= 0x60;
        } else {
            reg31 &= !0x60;
        }
        self.write_reg(i2c, DAC_REG31, reg31)
    }

    /// Set the fade rate for DAC volume changes.
    ///
    /// When fading is active, volume changes are applied gradually
    /// to avoid audible pops and clicks.
    pub fn fade<I2C, E>(&self, i2c: &mut I2C, fade: Fade) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut reg37 = self.read_reg(i2c, DAC_REG37)?;
        reg37 &= 0x0F;
        reg37 |= (fade as u8) << 4;
        self.write_reg(i2c, DAC_REG37, reg37)
    }

    /// Set the fade rate for microphone (ADC) volume changes.
    ///
    /// When enabled, the microphone gain changes will be smoothed
    /// over time.
    pub fn microphone_fade<I2C, E>(&self, i2c: &mut I2C, fade: Fade) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E>,
    {
        let mut reg15 = self.read_reg(i2c, ADC_REG15)?;
        reg15 &= 0x0F;
        reg15 |= (fade as u8) << 4;
        self.write_reg(i2c, ADC_REG15, reg15)
    }
}

// ---------------------------------------------------------------------------
// Clock coefficient table (lookup for MCLK / sample rate pairs)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct CoeffDiv {
    mclk: u32,
    rate: u32,
    pre_div: u8,
    pre_multi: u8,
    adc_div: u8,
    dac_div: u8,
    fs_mode: u8,
    lrck_h: u8,
    lrck_l: u8,
    bclk_div: u8,
    adc_osr: u8,
    dac_osr: u8,
}

const COEFF_TABLE: &[CoeffDiv] = &[
    // 8k
    CoeffDiv { mclk: 12288000, rate: 8000, pre_div: 0x06, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 8000, pre_div: 0x03, pre_multi: 0x01, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x00, lrck_h: 0x05, lrck_l: 0xff, bclk_div: 0x18, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 16384000, rate: 8000, pre_div: 0x08, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 8192000, rate: 8000, pre_div: 0x04, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 8000, pre_div: 0x03, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 4096000, rate: 8000, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 8000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2048000, rate: 8000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 8000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1024000, rate: 8000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 11.025k
    CoeffDiv { mclk: 11289600, rate: 11025, pre_div: 0x04, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 5644800, rate: 11025, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2822400, rate: 11025, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1411200, rate: 11025, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 12k
    CoeffDiv { mclk: 12288000, rate: 12000, pre_div: 0x04, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 12000, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 12000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 12000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 16k
    CoeffDiv { mclk: 12288000, rate: 16000, pre_div: 0x03, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 16000, pre_div: 0x03, pre_multi: 0x01, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x00, lrck_h: 0x02, lrck_l: 0xff, bclk_div: 0x0c, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 16384000, rate: 16000, pre_div: 0x04, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 8192000, rate: 16000, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 16000, pre_div: 0x03, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 4096000, rate: 16000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 16000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2048000, rate: 16000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 16000, pre_div: 0x03, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1024000, rate: 16000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 22.05k
    CoeffDiv { mclk: 11289600, rate: 22050, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 5644800, rate: 22050, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2822400, rate: 22050, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1411200, rate: 22050, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 705600, rate: 22050, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 24k
    CoeffDiv { mclk: 12288000, rate: 24000, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 24000, pre_div: 0x03, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 24000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 24000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 24000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 32k
    CoeffDiv { mclk: 12288000, rate: 32000, pre_div: 0x03, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 32000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x00, lrck_h: 0x02, lrck_l: 0xff, bclk_div: 0x0c, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 16384000, rate: 32000, pre_div: 0x02, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 8192000, rate: 32000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 32000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 4096000, rate: 32000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 32000, pre_div: 0x03, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2048000, rate: 32000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 32000, pre_div: 0x03, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x01, lrck_h: 0x00, lrck_l: 0x7f, bclk_div: 0x02, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1024000, rate: 32000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 44.1k
    CoeffDiv { mclk: 11289600, rate: 44100, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 5644800, rate: 44100, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2822400, rate: 44100, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1411200, rate: 44100, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 48k
    CoeffDiv { mclk: 12288000, rate: 48000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 48000, pre_div: 0x03, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 48000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 48000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 48000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    // 64k
    CoeffDiv { mclk: 12288000, rate: 64000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 64000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x01, lrck_h: 0x01, lrck_l: 0x7f, bclk_div: 0x06, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 16384000, rate: 64000, pre_div: 0x01, pre_multi: 0x00, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 8192000, rate: 64000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 64000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x01, lrck_h: 0x01, lrck_l: 0x7f, bclk_div: 0x06, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 4096000, rate: 64000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 64000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x03, dac_div: 0x03, fs_mode: 0x01, lrck_h: 0x01, lrck_l: 0x7f, bclk_div: 0x06, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2048000, rate: 64000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 64000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x01, lrck_h: 0x00, lrck_l: 0xbf, bclk_div: 0x03, adc_osr: 0x18, dac_osr: 0x18 },
    CoeffDiv { mclk: 1024000, rate: 64000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x01, lrck_h: 0x00, lrck_l: 0x7f, bclk_div: 0x02, adc_osr: 0x10, dac_osr: 0x10 },
    // 88.2k
    CoeffDiv { mclk: 11289600, rate: 88200, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 5644800, rate: 88200, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 2822400, rate: 88200, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1411200, rate: 88200, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x01, lrck_h: 0x00, lrck_l: 0x7f, bclk_div: 0x02, adc_osr: 0x10, dac_osr: 0x10 },
    // 96k
    CoeffDiv { mclk: 12288000, rate: 96000, pre_div: 0x01, pre_multi: 0x01, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 18432000, rate: 96000, pre_div: 0x03, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 6144000, rate: 96000, pre_div: 0x01, pre_multi: 0x02, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 3072000, rate: 96000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x00, lrck_h: 0x00, lrck_l: 0xff, bclk_div: 0x04, adc_osr: 0x10, dac_osr: 0x10 },
    CoeffDiv { mclk: 1536000, rate: 96000, pre_div: 0x01, pre_multi: 0x03, adc_div: 0x01, dac_div: 0x01, fs_mode: 0x01, lrck_h: 0x00, lrck_l: 0x7f, bclk_div: 0x02, adc_osr: 0x10, dac_osr: 0x10 },
];

/// Look up the coefficient entry for a given MCLK and sample rate.
fn get_coeff(mclk: u32, rate: u32) -> Option<&'static CoeffDiv> {
    COEFF_TABLE.iter().find(|c| c.mclk == mclk && c.rate == rate)
}
