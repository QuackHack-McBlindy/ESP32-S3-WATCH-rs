// COMPONENTS/FT3168
// FT3168 TOUCH CONTROLLER DRIVER

use embedded_hal::i2c::I2c;
use heapless::Vec;

const FT3168_ADDR: u8 = 0x38;

// REGISTERS
const REG_FINGER_NUM: u8 = 0x02;
const REG_X1_H: u8 = 0x03;
const REG_X1_L: u8 = 0x04;
const REG_Y1_H: u8 = 0x05;
const REG_Y1_L: u8 = 0x06;
const REG_POWER_MODE: u8 = 0xA5;
const REG_GESTURE_ID: u8 = 0xD3;

const FT3X68_RD_DEVICE_GESTUREID: u8 = 0xD3;
const FT3X68_RD_DEVICE_FINGERNUM: u8 = 0x02;
const FT3X68_RD_DEVICE_X1POSH: u8 = 0x03;
const FT3X68_RD_DEVICE_X1POSL: u8 = 0x04;
const FT3X68_RD_DEVICE_Y1POSH: u8 = 0x05;
const FT3X68_RD_DEVICE_Y1POSL: u8 = 0x06;
const FT3X68_RD_DEVICE_X2POSH: u8 = 0x09;
const FT3X68_RD_DEVICE_X2POSL: u8 = 0x0A;
const FT3X68_RD_DEVICE_Y2POSH: u8 = 0x0B;
const FT3X68_RD_DEVICE_Y2POSL: u8 = 0x0C;
const FT3X68_RD_WR_DEVICE_GESTUREID_MODE: u8 = 0xD0;
const FT3X68_RD_WR_DEVICE_POWER_MODE: u8 = 0xA5;
const FT3X68_RD_WR_DEVICE_PROXIMITY_SENSING_MODE: u8 = 0xB0;
const FT3X68_RD_DEVICE_ID: u8 = 0xA0;

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum PowerMode {
    Active = 0x00,
    Monitor = 0x01,
    Standby = 0x02,
    Hibernate = 0x03,
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
    pub fingers: u8,
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum Gesture {
    None,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    SingleTap,
    DoubleTap,
    LongPress,
    Unknown(u8),
}

// DETECTED SWIPE GESTURE WITH START/END COORDINATES
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct SwipeEvent {
    pub direction: SwipeDirection,
    pub start_x: u16,
    pub start_y: u16,
    pub end_x: u16,
    pub end_y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
    Tap,
}

// REPRESENTS WHETHER A TOUCH POINT IS PRESSED OR RELEASED
#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum TouchState {
    Pressed(TouchPoint),
    Released,
}

pub struct Ft3168Touch<I> {
    i2c: I,
    // SWIPE TRACKING STATE
    tracking: bool,
    start_x: u16,
    start_y: u16,
    last_x: u16,
    last_y: u16,
}

impl<I: I2c> Ft3168Touch<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            tracking: false,
            start_x: 0,
            start_y: 0,
            last_x: 0,
            last_y: 0,
        }
    }

    fn read_reg(&mut self, reg: u8) -> Result<u8, I::Error> {
        let mut buf = [0u8];
        self.i2c.write_read(FT3168_ADDR, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), I::Error> {
        self.i2c.write(FT3168_ADDR, &[reg, val])
    }


    // INITIALIZES THE DEVICE DEFAULTING TO ACTIVE POWER MODE.
    // NO HARDWARE RESET IS PERFORMED (CALLER MUST HANDLE THAT IF REQUIRED)
    pub fn initialize(&mut self) -> Result<(), I::Error> {
        self.write_reg(FT3X68_RD_WR_DEVICE_POWER_MODE, PowerMode::Active as u8)
    }

    // INITIALIZES THE DEVICE WITH A SPECIFIC POWER MODE.
    pub fn initialize_with_mode(&mut self, mode: PowerMode) -> Result<(), I::Error> {
        self.write_reg(FT3X68_RD_WR_DEVICE_POWER_MODE, mode as u8)
    }

    // INITIALIZE TOUCH CONTROLLER IN MONITOR POWER MODE
    pub fn init(&mut self) -> Result<(), I::Error> {
        self.write_reg(REG_POWER_MODE, 0x01)
    }

    // SETS THE POWER MODE OF THE DEVICE
    pub fn set_power_mode(&mut self, mode: PowerMode) -> Result<(), I::Error> {
        self.i2c
            .write(FT3168_ADDR, &[FT3X68_RD_WR_DEVICE_POWER_MODE, mode as u8])
    }

    // ENABLES OR DISABLES THE PROXIMITY SENSING MODE.
    pub fn set_proximity_sensing_mode(&mut self, enable: bool) -> Result<(), I::Error> {
        self.i2c
            .write(FT3168_ADDR, &[FT3X68_RD_WR_DEVICE_PROXIMITY_SENSING_MODE, enable as u8])
    }

    // ENABLES OR DISABLES THE GESTURE RECOGNITION MODE.
    pub fn set_gesture_mode(&mut self, enable: bool) -> Result<(), I::Error> {
        self.i2c
            .write(FT3168_ADDR, &[FT3X68_RD_WR_DEVICE_GESTUREID_MODE, enable as u8])
    }

    // READ CURRENT TOUCH STATE. RETURNS `None` IF NO TOUCH
    pub fn read(&mut self) -> Result<Option<TouchPoint>, I::Error> {
        let fingers = self.read_reg(REG_FINGER_NUM)?;
        if fingers == 0 {
            return Ok(None);
        }

        let x_h = self.read_reg(REG_X1_H)? as u16;
        let x_l = self.read_reg(REG_X1_L)? as u16;
        let y_h = self.read_reg(REG_Y1_H)? as u16;
        let y_l = self.read_reg(REG_Y1_L)? as u16;

        let x = ((x_h & 0x0F) << 8) | x_l;
        let y = ((y_h & 0x0F) << 8) | y_l;

        Ok(Some(TouchPoint { x, y, fingers }))
    }

    // POLL TOUCH & DETECT SWIPE GESTURES
    // RETURNS Some(SwipeEvent) WHEN A FINGER IS LIFTED AFTER MOVEMENT
    // RETURNS CURRENT TOUCH POSITION FOR LIVE TRACKING
    pub fn poll(&mut self) -> Result<(Option<TouchPoint>, Option<SwipeEvent>), I::Error> {
        let point = self.read()?;

        match point {
            Some(tp) => {
                if !self.tracking {
                    // NEW TOUCH STARTED
                    self.tracking = true;
                    self.start_x = tp.x;
                    self.start_y = tp.y;
                }
                self.last_x = tp.x;
                self.last_y = tp.y;
                Ok((Some(tp), None))
            }
            None => {
                if self.tracking {
                    // TOUCH RELEASED (FINGER LIFTED) - DETERMINE SWIPE
                    self.tracking = false;
                    let dx = self.last_x as i32 - self.start_x as i32;
                    let dy = self.last_y as i32 - self.start_y as i32;
                    let abs_dx = dx.unsigned_abs();
                    let abs_dy = dy.unsigned_abs();

                    // REQUIRE DOMINANT AXIS TO BE AT LEAST 1.5x THE OTHER
                    // TO PREVENT DIAGONAL SWIPES FROM TRIGGERING LEFT/RIGHT
                    let direction = if abs_dx < 30 && abs_dy < 30 {
                        SwipeDirection::Tap
                    } else if abs_dx > abs_dy * 3 / 2 {
                        // CLEARLY HORIZONTAL
                        if dx > 0 { SwipeDirection::Right } else { SwipeDirection::Left }
                    } else if abs_dy > abs_dx * 3 / 2 {
                        // CLEARLY VERTICAL
                        if dy > 0 { SwipeDirection::Down } else { SwipeDirection::Up }
                    } else {
                        // DIAGONAL - TREAT AS TAP (IGNORE)
                        SwipeDirection::Tap
                    };

                    let event = SwipeEvent {
                        direction,
                        start_x: self.start_x,
                        start_y: self.start_y,
                        end_x: self.last_x,
                        end_y: self.last_y,
                    };
                    Ok((None, Some(event)))
                } else {
                    Ok((None, None))
                }
            }
        }
    }

    // READ GESTURE ID
    pub fn read_gesture(&mut self) -> Result<Gesture, I::Error> {
        let mut buffer = [0u8; 1];
        self.i2c.write_read(
            FT3168_ADDR,
            &[FT3X68_RD_DEVICE_GESTUREID],
            &mut buffer,
        )?;
        Ok(match buffer[0] {
            0x00 => Gesture::None,
            0x20 => Gesture::SwipeLeft,
            0x21 => Gesture::SwipeRight,
            0x22 => Gesture::SwipeUp,
            0x23 => Gesture::SwipeDown,
            0x24 => Gesture::DoubleTap,
            other => Gesture::Unknown(other),
        })
    }

    // READS THE NUMBER OF ACTIVE TOUCH POINTS
    pub fn finger_number(&mut self) -> Result<u8, I::Error> {
        let mut buffer = [0u8; 1];
        self.i2c.write_read(
            FT3168_ADDR,
            &[FT3X68_RD_DEVICE_FINGERNUM],
            &mut buffer,
        )?;
        Ok(buffer[0])
    }

    // READS THE STATE OF THE FIRST TOUCH POINT.
    pub fn touch1(&mut self) -> Result<TouchState, I::Error> {
        let fingers = self.finger_number()?;
        if fingers == 0 {
            return Ok(TouchState::Released);
        }

        let mut data = [0u8; 4];
        self.i2c
            .write_read(FT3168_ADDR, &[FT3X68_RD_DEVICE_X1POSH], &mut data)?;

        let x = ((data[0] as u16 & 0x0F) << 8) | data[1] as u16;
        let y = ((data[2] as u16 & 0x0F) << 8) | data[3] as u16;

        Ok(TouchState::Pressed(TouchPoint { x, y, fingers }))
    }

    // READS THE STATE OF THE SECOND TOUCH POINT.
    pub fn touch2(&mut self) -> Result<TouchState, I::Error> {
        let fingers = self.finger_number()?;
        if fingers < 2 {
            return Ok(TouchState::Released);
        }

        let mut data = [0u8; 4];
        self.i2c
            .write_read(FT3168_ADDR, &[FT3X68_RD_DEVICE_X2POSH], &mut data)?;

        let x = ((data[0] as u16 & 0x0F) << 8) | data[1] as u16;
        let y = ((data[2] as u16 & 0x0F) << 8) | data[3] as u16;

        Ok(TouchState::Pressed(TouchPoint { x, y, fingers }))
    }

    // RETURNS ALL ACTIVE TOUCH POINTS (UP TO 2).
    pub fn get_touches(&mut self) -> Result<Vec<TouchPoint, 2>, I::Error> {
        let mut touches: Vec<TouchPoint, 2> = Vec::new();
        let fingers = self.finger_number()?;

        if fingers >= 1 {
            if let TouchState::Pressed(point) = self.touch1()? {
                touches.push(point).ok();
            }
        }
        if fingers >= 2 {
            if let TouchState::Pressed(point) = self.touch2()? {
                touches.push(point).ok();
            }
        }
        Ok(touches)
    }
}
