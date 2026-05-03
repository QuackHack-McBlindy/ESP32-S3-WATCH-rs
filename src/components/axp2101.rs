// COMPONENTS/AXP2101
// AXP2101 - POWER MANAGEMENT UNIT DRIVER

use embedded_hal::i2c::I2c;

// REGISTER ADDRESSES
const AXP2101_ADDR: u8 = 0x34;

const REG_STATUS1: u8 = 0x00;
const REG_STATUS2: u8 = 0x01;
const REG_IC_TYPE: u8 = 0x03;
const REG_ADC_ENABLE: u8 = 0x30;
const REG_VBAT_H: u8 = 0x34;
const REG_VBAT_L: u8 = 0x35;
const REG_TS_H: u8 = 0x36;
const REG_TS_L: u8 = 0x37;
const REG_VBUS_H: u8 = 0x38;
const REG_VBUS_L: u8 = 0x39;
const REG_VSYS_H: u8 = 0x3A;
const REG_VSYS_L: u8 = 0x3B;
const REG_IRQ_ENABLE0: u8 = 0x40;
const REG_IRQ_ENABLE1: u8 = 0x41;
const REG_IRQ_ENABLE2: u8 = 0x42;
const REG_IRQ_STATUS0: u8 = 0x48;
const REG_IRQ_STATUS1: u8 = 0x49;
const REG_IRQ_STATUS2: u8 = 0x4A;
const REG_DC_ONOFF: u8 = 0x80;      // DC OUTPUT ON/OFF + DVM CONTROL
const REG_DC_VOL0: u8 = 0x82;       // DCDC1 VOLTAGE
// DCDC2 WOULD BE 0x83, DCDC3 0x84, ... 
const REG_LDO_ONOFF0: u8 = 0x90;    // ALDO1-4 ON/OFF
const REG_LDO_VOL0: u8 = 0x92;      // ALDO1 VOLTAGE
// ALDO2 WOULD BE 0x93, ALDO3 0x94, ALDO4 0x95
const REG_BAT_PERCENT: u8 = 0xA4;
const REG_CHG_STATUS: u8 = 0x01;    // REGISTER 0x01


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

/// CONFIGURATION FOR INIT
#[derive(Debug, Clone, Copy)]
pub struct Axp2101Config {
    /// ENABLE ADC FOR VBAT, VBUS, VSYS, & DIE TEMPERATURE (DEFAULT: TRUE)
    pub enable_adc: bool,
    /// TRIM ADC CHANNELS TO SAVE POWER (DISABLE TEMPERATURE TS)
    pub trim_adc: bool,
}

impl Default for Axp2101Config {
    fn default() -> Self {
        Self {
            enable_adc: true,
            trim_adc: true,
        }
    }
}

/// DRIVER ERRORS
#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
    InvalidVoltage,
    InvalidParameter,
}


// DRIVER STRUCT
/// AXP2101 PMU DRIVER (ESP32‑S3 REGISTER LAYOUT)
pub struct Axp2101;

impl Axp2101 {
    /// CREATE A NEW DRIVER INSTANCE.
    pub fn new() -> Self {
        Self
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
}

impl Default for Axp2101 {
    fn default() -> Self {
        Self::new()
    }
}
