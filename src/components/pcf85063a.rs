// COMPONENTS/PCF85063A
// PCF85063A - REAL TIME CLOCK DRIVER
// WITH NTP SYNC

use embedded_hal::i2c::I2c;

const PCF85063A_ADDR: u8 = 0x51;

// REGISTERS
const REG_CTRL1: u8 = 0x00;
const REG_CTRL2: u8 = 0x01;
const REG_SECONDS: u8 = 0x04;
const REG_MINUTES: u8 = 0x05;
const REG_HOURS: u8 = 0x06;
const REG_DAYS: u8 = 0x07;
const REG_WEEKDAYS: u8 = 0x08;
const REG_MONTHS: u8 = 0x09;
const REG_YEARS: u8 = 0x0A;

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub weekday: u8,
    pub month: u8,
    pub year: u8, // 0-99 (2000-2099)
}

impl DateTime {
    pub fn new(year: u8, month: u8, day: u8, hours: u8, minutes: u8, seconds: u8) -> Self {
        Self {
            seconds,
            minutes,
            hours,
            day,
            weekday: 0,
            month,
            year,
        }
    }
}

pub struct Pcf85063aRtc<'a, I: I2c> {
    i2c: &'a mut I,
}

impl<'a, I: I2c> Pcf85063aRtc<'a, I> {
    pub fn new(i2c: &'a mut I) -> Self {
        Self { i2c }
    }

    fn read_reg(&mut self, reg: u8) -> Result<u8, I::Error> {
        let mut buf = [0u8];
        self.i2c.write_read(PCF85063A_ADDR, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), I::Error> {
        self.i2c.write(PCF85063A_ADDR, &[reg, val])
    }

    pub fn init(&mut self) -> Result<(), I::Error> {
        let ctrl1 = self.read_reg(REG_CTRL1)?;
        let new_ctrl1 = ctrl1 & !(0x20 | 0x04);
        if new_ctrl1 != ctrl1 {
            self.write_reg(REG_CTRL1, new_ctrl1)?;
        }
        Ok(())
    }

    pub fn get_time(&mut self) -> Result<DateTime, I::Error> {
        let mut buf = [0u8; 7];
        self.i2c.write_read(PCF85063A_ADDR, &[REG_SECONDS], &mut buf)?;

        Ok(DateTime {
            seconds: bcd_to_dec(buf[0] & 0x7F),
            minutes: bcd_to_dec(buf[1] & 0x7F),
            hours: bcd_to_dec(buf[2] & 0x3F),
            day: bcd_to_dec(buf[3] & 0x3F),
            weekday: buf[4] & 0x07,
            month: bcd_to_dec(buf[5] & 0x1F),
            year: bcd_to_dec(buf[6]),
        })
    }

    pub fn set_time(&mut self, dt: &DateTime) -> Result<(), I::Error> {
        let ctrl1 = self.read_reg(REG_CTRL1)?;
        self.write_reg(REG_CTRL1, ctrl1 | 0x20)?;
        self.write_reg(REG_SECONDS, dec_to_bcd(dt.seconds))?;
        self.write_reg(REG_MINUTES, dec_to_bcd(dt.minutes))?;
        self.write_reg(REG_HOURS, dec_to_bcd(dt.hours))?;
        self.write_reg(REG_DAYS, dec_to_bcd(dt.day))?;
        self.write_reg(REG_WEEKDAYS, dt.weekday)?;
        self.write_reg(REG_MONTHS, dec_to_bcd(dt.month))?;
        self.write_reg(REG_YEARS, dec_to_bcd(dt.year))?;
        self.write_reg(REG_CTRL1, ctrl1 & !0x20)?;
        Ok(())
    }
}

fn bcd_to_dec(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

fn dec_to_bcd(dec: u8) -> u8 {
    ((dec / 10) << 4) | (dec % 10)
}


// NTP SYNC
// SYNCRONIZE RTC TO NTP POOL
pub async fn ntp_sync(stack: &embassy_net::Stack<'static>) -> Result<(), &'static str> {
    let mut rx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 1];
    let mut rx_buf = [0u8; 256];
    let mut tx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 1];
    let mut tx_buf = [0u8; 256];

    let mut socket = embassy_net::udp::UdpSocket::new(
        stack.clone(), &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf
    );
    socket.bind(23456).map_err(|_| "bind failed")?;

    let mut ntp_request = [0u8; 48];
    ntp_request[0] = 0x1B;
    let ntp_addr = embassy_net::Ipv4Address::new(216, 239, 35, 0);
    socket.send_to(&ntp_request, (ntp_addr, 123)).await.map_err(|_| "send failed")?;

    let mut response = [0u8; 48];
    match embassy_time::with_timeout(embassy_time::Duration::from_secs(5), socket.recv_from(&mut response)).await {
        Ok(Ok((len, _addr))) if len >= 48 => {
            let ntp_secs = u32::from_be_bytes([response[40], response[41], response[42], response[43]]);
            let unix_secs = ntp_secs.wrapping_sub(2_208_988_800) as i64;
            let local_secs = unix_secs + timezone_offset(unix_secs) as i64;

            let (year, month, day, hour, minute, second) = unix_to_datetime(local_secs);
            defmt::info!("NTP time: {:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);

            critical_section::with(|cs| {
                let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
                if let Some(i2c_bus) = bus_ref.as_mut() {
                    let mut rtc = Pcf85063aRtc::new(i2c_bus);
                    let dt = DateTime {
                        seconds: second as u8,
                        minutes: minute as u8,
                        hours: hour as u8,
                        day: day as u8,
                        weekday: 0,
                        month: month as u8,
                        year: (year % 100) as u8,
                    };
                    let _ = rtc.set_time(&dt);
                }
            });
            Ok(())
        }
        _ => Err("NTP response timeout or invalid"),
    }
}

// RETURN OFFSET IN SECONDS (POSITIVE FOR UTC+1 OR +2)
fn timezone_offset(unix_secs: i64) -> i32 {
    let (year, month, day, hour, _, _) = unix_to_datetime(unix_secs);
    let is_summer = is_summer_time(year as i32, month as u32, day as u32, hour as u32);
    if is_summer { 7200 } else { 3600 }
}

fn is_summer_time(year: i32, month: u32, day: u32, hour: u32) -> bool {
    let mar_last_sun = last_sunday_of_month(year, 3);
    let oct_last_sun = last_sunday_of_month(year, 10);
    if month > 3 && month < 10 { return true; }
    if month == 3 {
        if day > mar_last_sun { return true; }
        if day == mar_last_sun && hour >= 2 { return true; }
    }
    if month == 10 {
        if day < oct_last_sun { return true; }
        if day == oct_last_sun && hour < 3 { return true; }
    }
    false
}

fn last_sunday_of_month(year: i32, month: u32) -> u32 {
    // HARDCODED FOR 2025‑2027
    match (year, month) {
        (2025, 3) => 30, (2025, 10) => 26,
        (2026, 3) => 29, (2026, 10) => 25,
        (2027, 3) => 28, (2027, 10) => 31,
        _ => 31, // SAFE FALLBACK (LAST DAY OF MARCH/OCTOBER IS AT LEAST 25)
    }
}

fn unix_to_datetime(secs: i64) -> (u32, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i32;
    let rem = secs % 86400;
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;
    let (year, month, day) = days_to_date(days);
    (year, month, day, hour, minute, second)
}

// CONVERT DAYS SINCE UNIX EPOCH (1970‑01‑01) TO YEAR, MONTH, DAY.
fn days_to_date(mut days: i32) -> (u32, u32, u32) {
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap_year(year);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    while month < 12 && days >= month_days[month] {
        days -= month_days[month];
        month += 1;
    }
    (year as u32, (month + 1) as u32, (days + 1) as u32)
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}


// BACKGROUND TASK: READ RTC AND STORE IN GLOBAL STATE
#[embassy_executor::task]
pub async fn rtc_update_task() {
    loop {
        critical_section::with(|cs| {
            let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
            if let Some(i2c_bus) = bus_ref.as_mut() {
                let mut rtc = Pcf85063aRtc::new(i2c_bus);
                if let Ok(dt) = rtc.get_time() {
                    crate::state::CURRENT_TIME.borrow(cs).set(Some(dt));
                }
            }
        });
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}
