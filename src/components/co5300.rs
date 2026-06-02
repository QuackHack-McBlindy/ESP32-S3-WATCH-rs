// COMPONENTS/CO5300
// CO5300 AMOLED DISPLAY DRIVER
// RESOLUTION: `410x502`, `col_offset=22` RGB565

use crate::components::qspi_bus::QspiBus;

// ───────────────────────────────────────────────────────────────────────
// INTERNAL FLAGS
crate::init_bool!(SHOULD_WAKE, false);
crate::init_bool!(SHOULD_SLEEP, false);
crate::init_bool!(FLASH_ON, false);


// ───────────────────────────────────────────────────────────────────────
// CO5300 COMMANDS
// SOFTWARE RESET
const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
// INVERT COLORS
const CMD_INVON: u8 = 0x21;
// TURN OFF INVERT COLORSS
const CMD_INVOFF: u8 = 0x20;
// TURNS OFF DISPLAY
const CMD_DISPOFF: u8 = 0x28;
// TURNS ON DISPLAY
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_PASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_PIXFMT: u8 = 0x3A;
const CMD_SPIMODECTL: u8 = 0xC4;
const CMD_WCTRLD1: u8 = 0x53;
const CMD_BRIGHTNESS: u8 = 0x51;
const CMD_BRIGHTNESS_HBM: u8 = 0x63;
const CMD_WCE: u8 = 0x58;

// MADCTL FLAGS
const MADCTL_RGB: u8 = 0x00;

// ───────────────────────────────────────────────────────────────────────
// DELAYS
const RST_DELAY_MS: u32 = 200;
const SLPOUT_DELAY_MS: u32 = 120;
const SLPIN_DELAY_MS: u32 = 120;

pub struct Co5300Display<'d> {
    bus: QspiBus<'d>,
    reset: esp_hal::gpio::Output<'d>,
    delay: esp_hal::delay::Delay,
    width: u16,
    height: u16,
    col_offset: u16,
    row_offset: u16,
}

#[derive(Debug)]
pub enum DisplayError {
    BusError,
}

impl<'d> Co5300Display<'d> {
    pub fn new(bus: QspiBus<'d>, reset: esp_hal::gpio::Output<'d>) -> Self {
        Self {
            bus,
            reset,
            delay: esp_hal::delay::Delay::new(),
            width: crate::state::LCD_WIDTH,
            height: crate::state::LCD_HEIGHT,
            col_offset: crate::state::LCD_COL_OFFSET,
            row_offset: crate::state::LCD_ROW_OFFSET,
        }
    }

    /// INITIALIZE THE DISPLAY. MUST BE CALLED BEFORE ANY DRAWING.
    pub fn init(&mut self) {
        // HARDWARE RESET
        self.reset.set_high();
        self.delay.delay_millis(10);
        self.reset.set_low();
        self.delay.delay_millis(RST_DELAY_MS);
        self.reset.set_high();
        self.delay.delay_millis(RST_DELAY_MS);

        // SLEEP OUT
        self.bus.write_command(CMD_SLPOUT);
        self.delay.delay_millis(SLPOUT_DELAY_MS);

        // INIT SEQUENCE FROM `co5300_init_operations[]`
        self.bus.write_c8d8(0xFE, 0x00); // VENDOR REGISTER ACCESS
        self.bus.write_c8d8(CMD_SPIMODECTL, 0x80); // SPI MODE CONTROL
        self.bus.write_c8d8(CMD_PIXFMT, 0x55); // 16-BIT RGB565
        self.bus.write_c8d8(CMD_WCTRLD1, 0x20); // WRITE CTRL DISPLAY
        self.bus.write_c8d8(CMD_BRIGHTNESS_HBM, 0xFF); // HBM BRIGHTNESS MAX
        self.bus.write_command(CMD_DISPON); // DISPLAY ON
        self.bus.write_c8d8(CMD_BRIGHTNESS, 0xD0); // NORMAL BRIGHTNESS
        self.bus.write_c8d8(CMD_WCE, 0x00); // CONTRAST ENHANCEMENT OFF

        // SET MADCTL FOR CORRECT COLOR ORDER (RGB, NO ROTATION)
        self.bus.write_c8d8(CMD_MADCTL, MADCTL_RGB);

        self.delay.delay_millis(10);

        // INVERSION OFF (STANDARD FOR THIS PANEL)
        self.bus.write_command(CMD_INVOFF);
        
        // ENABLE TEARING EFFECT (VBLANK ONLY)
        self.bus.write_c8d8(0x35, 0x00);

        self.delay.delay_millis(10);
    }

    /// SET THE ADDRESS WINDOW FOR PIXEL WRITES.
    pub fn set_addr_window(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let x_start = x + self.col_offset;
        let x_end = x_start + w - 1;
        let y_start = y + self.row_offset;
        let y_end = y_start + h - 1;

        self.bus.write_c8d16d16(CMD_CASET, x_start, x_end);
        self.bus.write_c8d16d16(CMD_PASET, y_start, y_end);
        self.bus.write_command(CMD_RAMWR);
    }

    /// FILL THE ENTIRE SCREEN WITH A SINGLE COLOR.
    pub fn fill_screen(&mut self, color: embedded_graphics_core::pixelcolor::Rgb565) {
        let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
            core::convert::From::from(color);
        let raw: u16 = embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
        self.set_addr_window(0, 0, self.width, self.height);
        let total = self.width as u32 * self.height as u32;
        self.bus.write_repeat(raw, total);
    }

    /// FILL A RECTANGLE AREA WITH A SOLID COLOR
    pub fn write_pixels_area(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        color: embedded_graphics_core::pixelcolor::Rgb565,
    ) {
        let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
            core::convert::From::from(color);
        let raw: u16 = embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
        self.set_addr_window(x, y, w, h);
        self.bus.write_repeat(raw, w as u32 * h as u32);
    }

    /// GET MUTABLE REFERENCE TO BUS (FOR FRAMEBUFFER FLUSH)
    pub fn bus_mut(&mut self) -> &mut QspiBus<'d> {
        &mut self.bus
    }

    /// SET DISPLAY BRIGHTNESS - ACCEPTS ANY u8 VALUE (0–255)
    // BYTE === (PERCENTAGE * 255) / 100
    // 0% = 0x00
    // 10% ≈ 0x1A (26)
    // 20% ≈ 0x33 (51)
    // 30% = 0x4D (77)
    // 40% = 0x66 (102)
    // 50% = 0x80 (128)
    // 60% = 0x99 (153)
    // 70% = 0xB3 (179)
    // 80% = 0xCC (204)
    // 90% = 0xE6 (230)
    // 100% = 0xFF (255)
    pub fn set_brightness(&mut self, brightness: u8) {
        self.bus.write_c8d8(CMD_BRIGHTNESS, brightness);
    }

    // TURN DISPPLAY ON (EXIT SLEEP + DISPLAY ON)
    // MIPI DCS ORDER: SLPOUT -> 120MS -> DISPON -> 20MS
    pub fn display_on(&mut self) {
        self.bus.write_command(CMD_SLPOUT);
        self.delay.delay_millis(SLPOUT_DELAY_MS);
        self.bus.write_command(CMD_DISPON);
        self.delay.delay_millis(20);
    }

    /// TURN DISPLAY OFF (DISPOFF + ENTER SLEEP)
    /// MIPI DCS ORDER: DISPOFF -> 20MS -> SLPIN -> 120MS
    pub fn display_off(&mut self) {
        self.bus.write_command(CMD_DISPOFF);
        self.delay.delay_millis(20);
        self.bus.write_command(0x10); // SLPIN
        self.delay.delay_millis(SLPIN_DELAY_MS);
    }
}

impl embedded_graphics_core::geometry::OriginDimensions for Co5300Display<'_> {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

impl embedded_graphics_core::draw_target::DrawTarget for Co5300Display<'_> {
    type Color = embedded_graphics_core::pixelcolor::Rgb565;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        // CO5300 REQUIRES MINIMUM 2x2 PIXEL WRITES
        // DRAW EACH PIXEL AS A 2x2 BLOCK.
        for embedded_graphics_core::Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < self.width as i32
                && coord.y >= 0
                && coord.y < self.height as i32
            {
                let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
                    core::convert::From::from(color);
                let raw: u16 = embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
                // WRITE 2x2 BLOCK (4 PIXELS)
                self.set_addr_window(coord.x as u16, coord.y as u16, 2, 2);
                self.bus.write_pixels(&[raw, raw, raw, raw]);
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics_core::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let area = area.intersection(
            &embedded_graphics_core::primitives::Rectangle::new(
                embedded_graphics_core::geometry::Point::zero(),
                embedded_graphics_core::geometry::Size::new(
                    self.width as u32,
                    self.height as u32,
                ),
            ),
        );

        if area.size.width == 0 || area.size.height == 0 {
            return Ok(());
        }

        self.set_addr_window(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
        );

        // CO5300 REQUIRES MINIMUM 2-LINE WRITES.
        // IF HEIGHT IS 1, DOUBLE IT & DUPLICATE EACH ROW.
        let actual_h = if area.size.height < 2 {
            2
        } else {
            area.size.height as u16
        };
        let needs_row_dup = area.size.height < 2;

        self.set_addr_window(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            actual_h,
        );

        self.bus.begin_pixels();
        let w = area.size.width as usize;
        let mut row_buf = [0u16; 128]; // MAX WIDTH WE SUPPORT PER ROW
        let mut col = 0usize;

        for color in colors.into_iter() {
            if col < 128 {
                let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
                    core::convert::From::from(color);
                row_buf[col] =
                    embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
            }
            col += 1;

            // END OF ROW
            if col >= w {
                let slice = &row_buf[..w.min(128)];
                self.bus.stream_pixels(slice);
                if needs_row_dup {
                    // DUPLICATE THE ROW FOR MINIMUM 2-LINE REQUIREMENT
                    self.bus.stream_pixels(slice);
                }
                col = 0;
            }
        }
        // FLUSH REMAINING PARTIAL ROW
        if col > 0 {
            let slice = &row_buf[..col.min(128)];
            self.bus.stream_pixels(slice);
            if needs_row_dup {
                self.bus.stream_pixels(slice);
            }
        }
        self.bus.end_pixels();

        Ok(())
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics_core::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        let area = area.intersection(
            &embedded_graphics_core::primitives::Rectangle::new(
                embedded_graphics_core::geometry::Point::zero(),
                embedded_graphics_core::geometry::Size::new(
                    self.width as u32,
                    self.height as u32,
                ),
            ),
        );

        if area.size.width == 0 || area.size.height == 0 {
            return Ok(());
        }

        let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
            core::convert::From::from(color);
        let raw: u16 = embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
        self.set_addr_window(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
        );
        self.bus.write_repeat(raw, area.size.width * area.size.height);
        Ok(())
    }
}


// ───────────────────────────────────────────────────────────────────────
// CALL WHEN WAKE WORD IS DETECTED.
pub fn wake_up() {
    crate::store!(SHOULD_SLEEP, false);
    crate::store!(SHOULD_WAKE, true);
}


// ───────────────────────────────────────────────────────────────────────
// CALL WHEN SERVER STARTS TRANSCRIPTION.
pub fn start_flash() {
    crate::store!(FLASH_ON, true);
}


// ───────────────────────────────────────────────────────────────────────
// CALL WHEN TRANSCRIPTION IS DONE.
pub fn stop_flash() {
    crate::store!(FLASH_ON, false);
}


// ───────────────────────────────────────────────────────────────────────
// CALL WHEN VOICE SESSION ENDS.
pub fn sleep_now() {
    crate::store!(SHOULD_WAKE, false);
    crate::store!(SHOULD_SLEEP, true);
}

// ───────────────────────────────────────────────────────────────────────
// FUNCTIONS USED BY THE DISPLAY TASK TO READ FLAGS
pub fn consume_wake() -> bool {
    if SHOULD_WAKE.swap(false, core::sync::atomic::Ordering::Relaxed) {
        true
    } else {
        false
    }
}

pub fn consume_sleep() -> bool {
    if SHOULD_SLEEP.swap(false, core::sync::atomic::Ordering::Relaxed) {
        true
    } else {
        false
    }
}

pub fn is_flashing() -> bool {
    crate::load!(FLASH_ON)
}
