// COMPONENTS/FT3168
// FT3168 TOUCH CONTROLLER DRIVER

use embedded_hal::i2c::I2c;
use critical_section;

use crate::I2C_BUS;

const FT3168_ADDR: u8 = 0x38;

// REGISTERS
const REG_FINGER_NUM: u8 = 0x02;
const REG_X1_H: u8 = 0x03;
const REG_X1_L: u8 = 0x04;
const REG_Y1_H: u8 = 0x05;
const REG_Y1_L: u8 = 0x06;
const REG_POWER_MODE: u8 = 0xA5;
const REG_GESTURE_ID: u8 = 0xD3;


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



/// DETECTED SWIPE GESTURE WITH START/END COORDINATES
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

    /// INITIALIZE TOUCH CONTROLLER IN MONITOR POWER MODE.
    pub fn init(&mut self) -> Result<(), I::Error> {
        // SET POWER MODE TO MONITOR (TRIGGERS ON TOUCH)
        self.write_reg(REG_POWER_MODE, 0x01)?;
        Ok(())
    }

    /// READ CURRENT TOUCH STATE. RETURNS `None` IF NO TOUCH.
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

        Ok(Some(TouchPoint {
            x,
            y,
            fingers,
        }))
    }

    /// POLL TOUCH & DETECT SWIPE GESTURES
    /// RETURNS Some(SwipeEvent) WHEN A FINGER IS LIFTED AFTER MOVEMENT
    /// RETURNS  CURRENT TOUCH POSITION FOR LIVE TRACKING
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
                } else { Ok((None, None)) }
            }
        }
    }

    /// READ GESTURE IDs
    pub fn read_gesture(&mut self) -> Result<Gesture, I::Error> {
        let id = self.read_reg(REG_GESTURE_ID)?;
        Ok(match id {
            0x00 => Gesture::None,
            0x01 => Gesture::SwipeUp,
            0x02 => Gesture::SwipeDown,
            0x03 => Gesture::SwipeLeft,
            0x04 => Gesture::SwipeRight,
            0x05 => Gesture::SingleTap,
            0x0B => Gesture::DoubleTap,
            0x0C => Gesture::LongPress,
            other => Gesture::Unknown(other),
        })
    }
}

// TASK THAT SIGNALS & SENDS TOUCH EVENTS TO GUI TASK
#[embassy_executor::task]
pub async fn touch_task() {
    let mut tracking = false;
    let mut start_x = 0u16;
    let mut start_y = 0u16;
    let mut last_x = 0u16;
    let mut last_y = 0u16;

    loop {
        let point = critical_section::with(|cs| {
            let mut bus_ref = I2C_BUS.borrow_ref_mut(cs);
            let i2c_bus = bus_ref.as_mut()?;
            let mut touch = Ft3168Touch::new(i2c_bus);
            touch.read().ok().flatten() // NOW `Option<TouchPoint>`
        });

        match point {
            Some(tp) => {
                if !tracking {        
                    // TOUCH DETECTED – WAKE UP DISPLAY (IF OFF)
                    if !crate::load!(crate::state::DISPLAY_STATE) {
                        crate::components::co5300::wake_up();
                        crate::store!(crate::state::DISPLAY_STATE, true);
                    }
                
                    tracking = true;
                    start_x = tp.x;
                    start_y = tp.y;
                }
                last_x = tp.x;
                last_y = tp.y;
                defmt::debug!("👆 X={} Y={}", tp.x, tp.y);
            }
            None => {
                if tracking {
                    tracking = false;
                    let dx = last_x as i32 - start_x as i32;
                    let dy = last_y as i32 - start_y as i32;
                    let abs_dx = dx.unsigned_abs();
                    let abs_dy = dy.unsigned_abs();

                    let direction = if abs_dx < 30 && abs_dy < 30 {
                        SwipeDirection::Tap
                    } else if abs_dx > abs_dy * 3 / 2 {
                        if dx > 0 { SwipeDirection::Right } else { SwipeDirection::Left }
                    } else if abs_dy > abs_dx * 3 / 2 {
                        if dy > 0 { SwipeDirection::Down } else { SwipeDirection::Up }
                    } else {
                        SwipeDirection::Tap
                    };

                    let event = match direction {
                        SwipeDirection::Tap => crate::gui::pages::TouchEvent::Tap { x: start_x, y: start_y },
                        dir => crate::gui::pages::TouchEvent::Swipe(dir, last_x, last_y),
                    };
                    crate::gui::pages::TOUCH_EVENTS.signal(event);

                    defmt::debug!("Swipe {:?}", direction);
                    defmt::debug!("  Start: ({},{}) -> End: ({},{})",
                        start_x, start_y, last_x, last_y);
                }
            }
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
