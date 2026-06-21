// COMPONENTS/AXP2101
// AXP2101 - POWER MANAGEMENT UNIT DRIVER


use embedded_hal::i2c::I2c;

// ───────────────────────────────────────────────────────────────────────
// REGISTER ADDRESSES

pub const AXP2101_ADDR: u8 = 0x34;   // I²C slave address

// Status / ID / data buffer
pub const REG_STATUS1:          u8 = 0x00;   // also REG_PMU_STATUS0
pub const REG_PMU_STATUS0:      u8 = 0x00;   // alias
pub const REG_STATUS2:          u8 = 0x01;   // also REG_PMU_STATUS1, REG_CHG_STATUS
pub const REG_PMU_STATUS1:      u8 = 0x01;   // alias
pub const REG_CHG_STATUS:       u8 = 0x01;   // alias (charger status bits)
pub const REG_IC_TYPE:          u8 = 0x03;   // also REG_CHIP_ID
pub const REG_CHIP_ID:          u8 = 0x03;   // alias
pub const REG_DATABUF_START:    u8 = 0x04;

// System configuration
pub const REG_PMU_CONFIG:       u8 = 0x10;
pub const REG_BATFET_CONTROL:   u8 = 0x12;
pub const REG_TDIE_CONTROL:     u8 = 0x13;
pub const REG_VSYS_LOW_THRESH:  u8 = 0x14;
pub const REG_VIN_LOW_THRESH:   u8 = 0x15;
pub const REG_IIN_HIGH_THRESH:  u8 = 0x16;
pub const REG_GAUGE_RST:        u8 = 0x17;
pub const REG_CHARGER_GAUGE_WATCHDOG_SW: u8 = 0x18;
pub const REG_WATCHDOG_CONTROL: u8 = 0x19;
pub const REG_BAT_LOW_WARN_THRESH: u8 = 0x1A;
pub const REG_GPIO1_CONFIG:     u8 = 0x1B;

// Power on/off / reset behaviour
pub const REG_POWERON_REASON:   u8 = 0x20;
pub const REG_POWEROFF_REASON:  u8 = 0x21;
pub const REG_POWEROFF_EN_BEHAVIOR: u8 = 0x22;   // was previously guessed 0x32
pub const REG_DCDC_PROTECT:     u8 = 0x23;
pub const REG_POWEROFF_VBAT_LOW_THRESH: u8 = 0x24;
pub const REG_POWER_TIMING:     u8 = 0x25;
pub const REG_SLEEP_WAKE_CONFIG: u8 = 0x26;
pub const REG_KEY_EVENT_TIME:   u8 = 0x27;       // was previously guessed 0x23

// Fast power‑on config
pub const REG_FAST_PWRON_CONFIG0: u8 = 0x28;
pub const REG_FAST_PWRON_CONFIG1: u8 = 0x29;
pub const REG_FAST_PWRON_CONFIG2: u8 = 0x2A;
pub const REG_FAST_PWRON_CONFIG3: u8 = 0x2B;

// ADC control & results
pub const REG_ADC_ENABLE:       u8 = 0x30;   // also REG_ADC_CONTROL
pub const REG_ADC_CONTROL:      u8 = 0x30;   // alias
pub const REG_VBAT_H:           u8 = 0x34;   // also REG_ADC_VBAT_H
pub const REG_ADC_VBAT_H:       u8 = 0x34;   // alias
pub const REG_VBAT_L:           u8 = 0x35;   // also REG_ADC_VBAT_L
pub const REG_ADC_VBAT_L:       u8 = 0x35;   // alias
pub const REG_TS_H:             u8 = 0x36;   // also REG_ADC_TS_H
pub const REG_ADC_TS_H:         u8 = 0x36;   // alias
pub const REG_TS_L:             u8 = 0x37;   // also REG_ADC_TS_L
pub const REG_ADC_TS_L:         u8 = 0x37;   // alias
pub const REG_VBUS_H:           u8 = 0x38;   // also REG_ADC_VBUS_H
pub const REG_ADC_VBUS_H:       u8 = 0x38;   // alias
pub const REG_VBUS_L:           u8 = 0x39;   // also REG_ADC_VBUS_L
pub const REG_ADC_VBUS_L:       u8 = 0x39;   // alias
pub const REG_VSYS_H:           u8 = 0x3A;   // also REG_ADC_VSYS_H
pub const REG_ADC_VSYS_H:       u8 = 0x3A;   // alias
pub const REG_VSYS_L:           u8 = 0x3B;   // also REG_ADC_VSYS_L
pub const REG_ADC_VSYS_L:       u8 = 0x3B;   // alias
pub const REG_ADC_TDIE_H:       u8 = 0x3C;
pub const REG_ADC_TDIE_L:       u8 = 0x3D;
pub const REG_ADC_GPADC_H:      u8 = 0x3E;
pub const REG_ADC_GPADC_L:      u8 = 0x3F;

// IRQ
pub const REG_IRQ_ENABLE0:      u8 = 0x40;
pub const REG_IRQ_ENABLE1:      u8 = 0x41;
pub const REG_IRQ_ENABLE2:      u8 = 0x42;
pub const REG_IRQ_STATUS0:      u8 = 0x48;
pub const REG_IRQ_STATUS1:      u8 = 0x49;
pub const REG_IRQ_STATUS2:      u8 = 0x4A;

// TS / JEITA
pub const REG_TS_CONFIG:        u8 = 0x50;
pub const REG_TS_HYSTER_L2H:    u8 = 0x52;
pub const REG_TS_HYSTER_H2L:    u8 = 0x53;
pub const REG_TSV_CHARGER_LOW:  u8 = 0x54;
pub const REG_TSV_CHARGER_HIGH: u8 = 0x55;
pub const REG_TSV_WORK_LOW:     u8 = 0x56;
pub const REG_TSV_WORK_HIGH:    u8 = 0x57;
pub const REG_JEITA_EN:         u8 = 0x58;
pub const REG_JEITA_IV_CONFIG:  u8 = 0x59;
pub const REG_JEITA_COOL:       u8 = 0x5A;
pub const REG_JEITA_WARM:       u8 = 0x5B;
pub const REG_TS_VOLT_H:        u8 = 0x5C;
pub const REG_TS_VOLT_L:        u8 = 0x5D;

// Charger / battery
pub const REG_RECHARGE_CONFIG:  u8 = 0x60;
pub const REG_CHARGER_IPRE:     u8 = 0x61;
pub const REG_CHARGER_ICC:      u8 = 0x62;
pub const REG_CHARGER_ITERM:    u8 = 0x63;
pub const REG_CHARGER_CV:       u8 = 0x64;
pub const REG_THERMAL_THRESH:   u8 = 0x65;
pub const REG_CHARGER_TIMER:    u8 = 0x67;
pub const REG_BAT_DETECT_EN:    u8 = 0x68;
pub const REG_CHGLED_CONTROL:   u8 = 0x69;
pub const REG_COIN_BAT_VTERM:   u8 = 0x6A;

// Power rails on/off & voltage (existing)
pub const REG_DC_ONOFF:         u8 = 0x80;
pub const REG_DC_VOL0:          u8 = 0x82;
pub const REG_LDO_ONOFF0:       u8 = 0x90;
pub const REG_LDO_VOL0:         u8 = 0x92;

// Fuel gauge / battery percent
pub const REG_BAT_PERCENT:      u8 = 0xA4;
pub const REG_BATTERY_PERCENT:  u8 = 0xA4;   // alias


// ───────────────────────────────────────────────────────────────────────
// PUBLIC TYPES & ENUMS
/// CHARGER STATUS EXTRACTED FROM STATUS2 BITS 5‑7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChargeStatus {
    NotCharging = 0,
    Charging = 1,
    ChargingDone = 2,
    Unknown = 3,
}


/// PWRON key active duration to trigger an IRQ event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDurationIrq {
    T1000MS = 0,
    T1500MS = 1,
    T2000MS = 2,
    T2500MS = 3,
}

impl KeyDurationIrq {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::T1000MS,
            1 => Self::T1500MS,
            2 => Self::T2000MS,
            _ => Self::T2500MS,
        }
    }
}

/// PWRON key active duration to trigger a power off event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDurationPowerOff {
    T4S  = 0,
    T6S  = 1,
    T8S  = 2,
    T10S = 3,
}

impl KeyDurationPowerOff {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::T4S,
            1 => Self::T6S,
            2 => Self::T8S,
            _ => Self::T10S,
        }
    }
}

/// PWRON key active duration to trigger a power on event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDurationPowerOn {
    T128MS  = 0,
    T512MS  = 1,
    T1000MS = 2,
    T2000MS = 3,
}

impl KeyDurationPowerOn {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::T128MS,
            1 => Self::T512MS,
            2 => Self::T1000MS,
            _ => Self::T2000MS,
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
/// CONFIGURATION FOR INIT
#[derive(Debug, Clone, Copy)]
pub struct Axp2101Config {
    /// ENABLE ADC FOR VBAT, VBUS, VSYS, & DIE TEMPERATURE (DEFAULT: TRUE)
    pub enable_adc: bool,
    /// TRIM ADC CHANNELS TO SAVE POWER (DISABLE TEMPERATURE TS)
    pub trim_adc: bool,
    pub key_irq_duration: KeyDurationIrq,
    pub key_poweroff_duration: KeyDurationPowerOff,
    pub key_poweron_duration: KeyDurationPowerOn,
}

impl Default for Axp2101Config {
    fn default() -> Self {
        Self {
            enable_adc: true,
            trim_adc: true,
            key_irq_duration: KeyDurationIrq::T1000MS,
            key_poweroff_duration: KeyDurationPowerOff::T6S,
            key_poweron_duration: KeyDurationPowerOn::T512MS,
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
/// DRIVER ERRORS
#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
    InvalidVoltage,
    InvalidParameter,
}

// ───────────────────────────────────────────────────────────────────────
// DRIVER STRUCT
/// AXP2101 PMU DRIVER (ESP32‑S3 REGISTER LAYOUT)
#[derive(Clone, Copy)]
pub struct Axp2101;
impl Axp2101 {
    /// CREATE A NEW DRIVER INSTANCE.
    pub fn new() -> Self {
        Self
    }

    /// Power off the PMU immediately (cuts all outputs except RTCLDO).
    pub fn shutdown<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        // Read current value of register 0x32
        let current = self.read_reg(i2c, 0x32)?;
        defmt::info!("Register 0x32 value before shutdown: {:#04x}", current);
        // Set bit 7 (shutdown)
        self.write_reg(i2c, 0x32, current | 0x80)
    }

    pub fn prepare_deep_sleep<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        // 1. System power‑off voltage threshold (register 0x31)
        //    Set bit 3 to select 3.0 V (based on original code)
        let reg31 = self.read_reg(i2c, 0x31)?;
        self.write_reg(i2c, 0x31, reg31 | (1 << 3))?;

        // 2. Float GPIO1 (register 0x90, bits 0‑2 = 0b111)
        let reg90 = self.read_reg(i2c, 0x90)?;
        self.write_reg(i2c, 0x90, reg90 | 0x07)?;

        // 3. Disable all ADCs (register 0x82)
        self.write_reg(i2c, 0x82, 0x00)?;

        // 4. Disable all outputs except DCDC1 (register 0x12)
        //    Original code: Write1Byte(0x12, Read8bit(0x12) & 0xA1);
        //    0xA1 = 0b10100001 → keeps DCDC1 (bit0) and bits 5,7
        let reg12 = self.read_reg(i2c, 0x12)?;
        self.write_reg(i2c, 0x12, reg12 & 0xA1)?;

        Ok(())
    }


    pub fn restart<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        // Register 0x32, bit 6 = 1 → reset (power‑on reset sequence)
        self.set_bit(i2c, 0x32, 6)
    }

    // PRIVATE HELPERS
    fn write_reg<I: I2c>(&self, i2c: &mut I, reg: u8, val: u8) -> Result<(), Error<I::Error>> {
        i2c.write(AXP2101_ADDR, &[reg, val]).map_err(Error::I2c)
    }

    fn read_reg<I: I2c>(&self, i2c: &mut I, reg: u8) -> Result<u8, Error<I::Error>> {
        let mut buf = [0u8];
        i2c.write_read(AXP2101_ADDR, &[reg], &mut buf)
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    fn set_bit<I: I2c>(&self, i2c: &mut I, reg: u8, bit: u8) -> Result<(), Error<I::Error>> {
        let val = self.read_reg(i2c, reg)?;
        self.write_reg(i2c, reg, val | (1 << bit))
    }

    fn clear_bit<I: I2c>(&self, i2c: &mut I, reg: u8, bit: u8) -> Result<(), Error<I::Error>> {
        let val = self.read_reg(i2c, reg)?;
        self.write_reg(i2c, reg, val & !(1 << bit))
    }

    fn get_bit<I: I2c>(&self, i2c: &mut I, reg: u8, bit: u8) -> Result<bool, Error<I::Error>> {
        let val = self.read_reg(i2c, reg)?;
        Ok((val & (1 << bit)) != 0)
    }


    // PUBLIC API
    /// INIT THE PMU - 
    // ENABLE REQUIRED POWER RAILS, DISABLE  IRQs, CONFIGURE ADC
    pub fn init<I: I2c>(&self, i2c: &mut I, cfg: &Axp2101Config) -> Result<(), Error<I::Error>> {
        // POWER RAILS (as required for ESP32‑S3 DISPLAY) 
        // DCDC1 === 3300 mV
        self.set_dc1_voltage(i2c, 3300)?;
        self.enable_dc1(i2c)?;

        // ALDO1 === 3300 mV
        self.set_aldo1_voltage(i2c, 3300)?;
        self.enable_aldo1(i2c)?;

        // IRQs
        // DISABLE ALL & CLEAR STATUS
        self.write_reg(i2c, REG_IRQ_ENABLE0, 0x00)?;
        self.write_reg(i2c, REG_IRQ_ENABLE1, 0x00)?;
        self.write_reg(i2c, REG_IRQ_ENABLE2, 0x00)?;
        self.write_reg(i2c, REG_IRQ_STATUS0, 0xFF)?;
        self.write_reg(i2c, REG_IRQ_STATUS1, 0xFF)?;
        self.write_reg(i2c, REG_IRQ_STATUS2, 0xFF)?;

        // ADC
        if cfg.enable_adc {
            if cfg.trim_adc {
                // VBAT + VBUS + VSYS ONLY (NO TEMP, NO TS)
                self.write_reg(i2c, REG_ADC_ENABLE, 0b00001101)?;
            } else {
                // VBAT + VBUS + VSYS + DIE TEMPERATURE
                self.write_reg(i2c, REG_ADC_ENABLE, 0b00011101)?;
            }
        } else {
            self.write_reg(i2c, REG_ADC_ENABLE, 0x00)?;
        }

        let mut reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        reg = (reg & !(0b11 << 4)) | (cfg.key_irq_duration as u8) << 4;
        reg = (reg & !(0b11 << 2)) | (cfg.key_poweroff_duration as u8) << 2;
        reg = (reg & !0b11) | (cfg.key_poweron_duration as u8);
        self.write_reg(i2c, REG_KEY_EVENT_TIME, reg)?;

        Ok(())
    }

    // DCDC1 (MAIN 3.3V RAIL)
    pub fn enable_dc1<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        self.set_bit(i2c, REG_DC_ONOFF, 0)
    }

    pub fn disable_dc1<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        self.clear_bit(i2c, REG_DC_ONOFF, 0)
    }

    pub fn is_dc1_enabled<I: I2c>(&self, i2c: &mut I) -> Result<bool, Error<I::Error>> {
        self.get_bit(i2c, REG_DC_ONOFF, 0)
    }

    /// SET DCDC1 VOLTAGE (1500‑3400 mV, 100 mV STEPS)
    pub fn set_dc1_voltage<I: I2c>(&self, i2c: &mut I, millivolt: u16) -> Result<(), Error<I::Error>> {
        const MIN: u16 = 1500;
        const MAX: u16 = 3400;
        const STEP: u16 = 100;
        if millivolt < MIN || millivolt > MAX || millivolt % STEP != 0 {
            return Err(Error::InvalidVoltage);
        }
        let val = ((millivolt - MIN) / STEP) as u8;
        self.write_reg(i2c, REG_DC_VOL0, val)
    }

    pub fn get_dc1_voltage<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let val = self.read_reg(i2c, REG_DC_VOL0)?;
        Ok((val as u16) * 100 + 1500)
    }


    // ALDO1 (PERIPHERAL / DISPLAY POWER)
    pub fn enable_aldo1<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        self.set_bit(i2c, REG_LDO_ONOFF0, 0)
    }

    pub fn disable_aldo1<I: I2c>(&self, i2c: &mut I) -> Result<(), Error<I::Error>> {
        self.clear_bit(i2c, REG_LDO_ONOFF0, 0)
    }

    pub fn is_aldo1_enabled<I: I2c>(&self, i2c: &mut I) -> Result<bool, Error<I::Error>> {
        self.get_bit(i2c, REG_LDO_ONOFF0, 0)
    }

    /// SET ALDO1 VOLTAGE (500‑3500 mV, 100 mV STEPS)
    pub fn set_aldo1_voltage<I: I2c>(&self, i2c: &mut I, millivolt: u16) -> Result<(), Error<I::Error>> {
        const MIN: u16 = 500;
        const MAX: u16 = 3500;
        const STEP: u16 = 100;
        if millivolt < MIN || millivolt > MAX || millivolt % STEP != 0 {
            return Err(Error::InvalidVoltage);
        }
        let val = ((millivolt - MIN) / STEP) as u8;
        // REG_LDO_VOL0 ONLY CONTROLS ALDO1; ALDO2-4 HAVE SEPARATE REGISTERS
        self.write_reg(i2c, REG_LDO_VOL0, val)
    }

    pub fn get_aldo1_voltage<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let val = self.read_reg(i2c, REG_LDO_VOL0)?;
        Ok((val as u16) * 100 + 500)
    }


    // BATTERY MONITORING
    /// BATTERY VOLTAGE IN mV (14‑BIT ADC, 1.1 mV PER LSB – RAW)
    /// (ACTUAL VOLTAGE === RAW * 1.1, BUT MOST USERS EXPECT mV)
    pub fn get_battery_voltage_raw<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let high = self.read_reg(i2c, REG_VBAT_H)? as u16;
        let low = self.read_reg(i2c, REG_VBAT_L)? as u16;
        Ok(((high << 8) | low) & 0x3FFF)
    }

    /// BATTERY VOLTAGE IN MILLIVOLTS (CONVERTED USING 1.1 mV/BIT)
    pub fn get_battery_voltage<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let raw = self.get_battery_voltage_raw(i2c)?;
        // SCALE TO mV: RAW * 1.1 === RAW * 11 / 10
        Ok((raw as u32 * 11 / 10) as u16)
    }

    pub fn get_battery_percent<I: I2c>(&self, i2c: &mut I) -> Result<u8, Error<I::Error>> {
        self.read_reg(i2c, REG_BAT_PERCENT)
    }


    // VBUS & SYSTEM VOLTAGE
    pub fn get_vbus_voltage_raw<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let high = self.read_reg(i2c, REG_VBUS_H)? as u16;
        let low = self.read_reg(i2c, REG_VBUS_L)? as u16;
        Ok(((high << 8) | low) & 0x3FFF)
    }

    pub fn get_vbus_voltage<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let raw = self.get_vbus_voltage_raw(i2c)?;
        Ok((raw as u32 * 11 / 10) as u16)
    }

    pub fn get_system_voltage_raw<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let high = self.read_reg(i2c, REG_VSYS_H)? as u16;
        let low = self.read_reg(i2c, REG_VSYS_L)? as u16;
        Ok(((high << 8) | low) & 0x3FFF)
    }

    pub fn get_system_voltage<I: I2c>(&self, i2c: &mut I) -> Result<u16, Error<I::Error>> {
        let raw = self.get_system_voltage_raw(i2c)?;
        Ok((raw as u32 * 11 / 10) as u16)
    }


    // STATUS & CHARGING DETECTION
    /// CHECK IF VBUS (USB) IS CONNECTED (BIT 5 OF STATUS1)
    pub fn is_vbus_in<I: I2c>(&self, i2c: &mut I) -> Result<bool, Error<I::Error>> {
        let status = self.read_reg(i2c, REG_STATUS1)?;
        Ok(status & 0x20 != 0)
    }

    /// CHARGING STATUS FROM STATUS2 BITS 5‑7
    pub fn get_charge_status<I: I2c>(&self, i2c: &mut I) -> Result<ChargeStatus, Error<I::Error>> {
        let status = self.read_reg(i2c, REG_STATUS2)?;
        let chg = (status >> 5) & 0x07;
        match chg {
            0 => Ok(ChargeStatus::NotCharging),
            1 | 2 | 3 => Ok(ChargeStatus::Charging),
            4 => Ok(ChargeStatus::ChargingDone),
            _ => Ok(ChargeStatus::Unknown),
        }
    }

    pub fn is_charging<I: I2c>(&self, i2c: &mut I) -> Result<bool, Error<I::Error>> {
        let status = self.read_reg(i2c, REG_STATUS2)?;
        let chg = (status >> 5) & 0x07;
        Ok(chg >= 1 && chg <= 3)
    }


    // CHIP IDENTIFICATION
    pub fn read_chip_id<I: I2c>(&self, i2c: &mut I) -> Result<u8, Error<I::Error>> {
        self.read_reg(i2c, REG_IC_TYPE)
    }


    // ADC CONTROL
    /// ENABLE SPECIFIC ADC CHANNELS (BITMASK AS DEFINED DATASHEET)
    /// EXAMPLE `0b00011101` ENABLES VBAT, VBUS, VSYS & TEMPERATURE
    pub fn set_adc_enable<I: I2c>(&self, i2c: &mut I, mask: u8) -> Result<(), Error<I::Error>> {
        self.write_reg(i2c, REG_ADC_ENABLE, mask)
    }

    pub fn get_adc_enable<I: I2c>(&self, i2c: &mut I) -> Result<u8, Error<I::Error>> {
        self.read_reg(i2c, REG_ADC_ENABLE)
    }
    
    
    // ── PWRON key power‑off behaviour ───────────────────────
    pub fn set_pwron_key_poweroff<I: I2c>(&self, i2c: &mut I, value: bool) -> Result<(), Error<I::Error>> {
        if value {
            self.set_bit(i2c, REG_POWEROFF_EN_BEHAVIOR, 1)
        } else {
            self.clear_bit(i2c, REG_POWEROFF_EN_BEHAVIOR, 1)
        }
    }

    pub fn set_pwron_key_poweroff_but_restart<I: I2c>(&self, i2c: &mut I, value: bool) -> Result<(), Error<I::Error>> {
        if value {
            self.set_bit(i2c, REG_POWEROFF_EN_BEHAVIOR, 0)
        } else {
            self.clear_bit(i2c, REG_POWEROFF_EN_BEHAVIOR, 0)
        }
    }

    // ── Key duration for IRQ ────────────────────────────────
    pub fn key_duration_irq<I: I2c>(&self, i2c: &mut I) -> Result<KeyDurationIrq, Error<I::Error>> {
        let reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        let bits = (reg >> 4) & 0b11;
        Ok(KeyDurationIrq::from_bits(bits))
    }

    pub fn set_key_duration_irq<I: I2c>(&self, i2c: &mut I, value: KeyDurationIrq) -> Result<(), Error<I::Error>> {
        let mut reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        reg &= !(0b11 << 4);
        reg |= (value as u8) << 4;
        self.write_reg(i2c, REG_KEY_EVENT_TIME, reg)
    }

    // ── Key duration for power‑off ──────────────────────────
    pub fn key_duration_power_off<I: I2c>(&self, i2c: &mut I) -> Result<KeyDurationPowerOff, Error<I::Error>> {
        let reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        let bits = (reg >> 2) & 0b11;
        Ok(KeyDurationPowerOff::from_bits(bits))
    }

    pub fn set_key_duration_power_off<I: I2c>(&self, i2c: &mut I, value: KeyDurationPowerOff) -> Result<(), Error<I::Error>> {
        let mut reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        reg &= !(0b11 << 2);
        reg |= (value as u8) << 2;
        self.write_reg(i2c, REG_KEY_EVENT_TIME, reg)
    }

    // ── Key duration for power‑on ───────────────────────────
    pub fn key_duration_power_on<I: I2c>(&self, i2c: &mut I) -> Result<KeyDurationPowerOn, Error<I::Error>> {
        let reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        let bits = reg & 0b11;
        Ok(KeyDurationPowerOn::from_bits(bits))
    }

    pub fn set_key_duration_power_on<I: I2c>(&self, i2c: &mut I, value: KeyDurationPowerOn) -> Result<(), Error<I::Error>> {
        let mut reg = self.read_reg(i2c, REG_KEY_EVENT_TIME)?;
        reg &= !0b11;
        reg |= value as u8;
        self.write_reg(i2c, REG_KEY_EVENT_TIME, reg)
    }
}

impl Default for Axp2101 {
    fn default() -> Self {
        Self::new()
    }
}


