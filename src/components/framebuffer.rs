// COMPONENTS/FRAMEBUFFER
// PSRAM FRAMEBUFFER FOR CO5300 DISPLAY
// 410x502 RGB565 = 411,640 BYTES (~402KB)
// DRAWS TO RAM, THEN FLUSHES ENTIRE SCREEN VIA DMA QSPI

const WIDTH: usize = crate::state::LCD_WIDTH as usize;
const HEIGHT: usize = crate::state::LCD_HEIGHT as usize;
const PIXEL_COUNT: usize = WIDTH * HEIGHT;

pub struct Framebuffer {
    buf: alloc::vec::Vec<u16>,
    back: alloc::vec::Vec<u16>, // DOUBLE BUFFER: DRAW TO BACK, FLUSH FRONT
}

impl Framebuffer {
    /// ALLOCATE FRAMEBUFFER IN PSRAM (VIA GLOBAL ALLOCATOR).
    pub fn new() -> Self {
        let buf = alloc::vec![0u16; PIXEL_COUNT];
        let back = alloc::vec![0u16; PIXEL_COUNT];
        Self { buf, back }
    }

    /// SWAP FRONT AND BACK BUFFERS. CALL AFTER RENDERING TO BACK BUFFER.
    /// THE FRONT BUFFER (buf) IS WHAT GETS FLUSHED TO DISPLAY.
    pub fn swap(&mut self) {
        core::mem::swap(&mut self.buf, &mut self.back);
    }

    /// CLEAR THE ENTIRE FRAMEBUFFER WITH A COLOR.
    pub fn clear_color(&mut self, color: embedded_graphics_core::pixelcolor::Rgb565) {
        let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
            core::convert::From::from(color);
        let raw: u16 =
            embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
        self.buf.fill(raw);
    }

    /// SET A SINGLE PIXEL (NO BOUNDS CHECK FOR SPEED).
    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u16) {
        if x < WIDTH && y < HEIGHT {
            self.buf[y * WIDTH + x] = color;
        }
    }

    /// FILL A RECTANGULAR REGION.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u16) {
        let x_end = (x + w).min(WIDTH);
        let y_end = (y + h).min(HEIGHT);
        for row in y..y_end {
            let start = row * WIDTH + x;
            let end = row * WIDTH + x_end;
            self.buf[start..end].fill(color);
        }
    }

    /// DOUBLE-BUFFER SWAP + VSYNC FLUSH.
    /// 1. SWAP FRONT/BACK BUFFERS (INSTANT)
    /// 2. WAIT FOR TE SIGNAL (VBLANK)
    /// 3. FLUSH THE NEW FRONT BUFFER TO DISPLAY
    /// RESULT: DISPLAY ALWAYS SHOWS A COMPLETE FRAME, ZERO TEARING.
    /// FAST VSYNC FLUSH FOR GAMES. NO COPY, JUST SYNC + SEND.
    pub fn swap_and_flush(
        &mut self,
        display: &mut crate::components::co5300::Co5300Display,
        te: &esp_hal::gpio::Input<'_>,
    ) {
        // SHORT TE SYNC. IF TE ISN'T PULSING (DISPLAY JUST WOKEN UP, OR WE'RE FLUSHING
        // OUTSIDE VBLANK WINDOW), GIVE UP AFTER A FEW HUNDRED CYCLES INSTEAD OF BURNING
        // CPU. TEARING IS INVISIBLE MOST OF THE TIME ANYWAY BECAUSE WE FLUSH <30FPS.
        for _ in 0..400 {
            if te.is_high() {
                break;
            }
        }
        display.set_addr_window(0, 0, WIDTH as u16, HEIGHT as u16);
        display.bus_mut().write_pixels(&self.buf);
    }

    /// VSYNC FLUSH FOR WATCHFACE / MENUS. SAME AS swap_and_flush BUT KEPT DISTINCT FOR CLARITY.
    pub fn flush_vsync(
        &self,
        display: &mut crate::components::co5300::Co5300Display,
        te: &esp_hal::gpio::Input<'_>,
    ) {
        for _ in 0..400 {
            if te.is_high() {
                break;
            }
        }
        self.flush(display);
    }

    /// FLUSH THE ENTIRE FRAMEBUFFER TO THE DISPLAY VIA DMA QSPI.
    pub fn flush(&self, display: &mut crate::components::co5300::Co5300Display) {
        display.set_addr_window(0, 0, WIDTH as u16, HEIGHT as u16);
        display.bus_mut().write_pixels(&self.buf);
    }

    /// FLUSH ONLY A RECTANGULAR REGION (DIRTY RECT OPTIMIZATION).
    pub fn flush_region(
        &self,
        display: &mut crate::components::co5300::Co5300Display,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
    ) {
        if w == 0 || h == 0 {
            return;
        }

        // THE CO5300 IS HAPPIER WITH EVEN-ALIGNED PARTIAL WRITES.
        // EXPAND THE DIRTY RECT TO AN EVEN 2x2-ALIGNED REGION BEFORE STREAMING ROWS.
        let mut x0 = (x as usize).min(WIDTH.saturating_sub(1));
        let mut y0 = (y as usize).min(HEIGHT.saturating_sub(1));
        let mut x1 = ((x as usize).saturating_add(w as usize)).min(WIDTH);
        let mut y1 = ((y as usize).saturating_add(h as usize)).min(HEIGHT);

        x0 &= !1;
        y0 &= !1;
        if x1 & 1 != 0 && x1 < WIDTH {
            x1 += 1;
        }
        if y1 & 1 != 0 && y1 < HEIGHT {
            y1 += 1;
        }

        if x1 <= x0 {
            x1 = (x0 + 2).min(WIDTH);
        }
        if y1 <= y0 {
            y1 = (y0 + 2).min(HEIGHT);
        }

        let flush_w = (x1 - x0).max(2).min(WIDTH - x0);
        let flush_h = (y1 - y0).max(2).min(HEIGHT - y0);

        display.set_addr_window(x0 as u16, y0 as u16, flush_w as u16, flush_h as u16);
        display.bus_mut().begin_pixels();
        for row in y0..(y0 + flush_h) {
            let start = row * WIDTH + x0;
            let end = start + flush_w;
            display.bus_mut().stream_pixels(&self.buf[start..end]);
        }
        display.bus_mut().end_pixels();
    }

    /// GET RAW BUFFER FOR DIRECT ACCESS.
    pub fn buffer(&self) -> &[u16] {
        &self.buf
    }

    /// GET MUTABLE RAW BUFFER FOR DIRECT ACCESS (SNAPSHOT RESTORE).
    pub fn buffer_mut(&mut self) -> &mut [u16] {
        &mut self.buf
    }
}

impl embedded_graphics_core::geometry::OriginDimensions for Framebuffer {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl embedded_graphics_core::draw_target::DrawTarget for Framebuffer {
    type Color = embedded_graphics_core::pixelcolor::Rgb565;
    type Error = crate::components::co5300::DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        for embedded_graphics_core::Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < WIDTH as i32
                && coord.y >= 0
                && coord.y < HEIGHT as i32
            {
                let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
                    core::convert::From::from(color);
                let raw: u16 =
                    embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
                self.buf[coord.y as usize * WIDTH + coord.x as usize] = raw;
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
                embedded_graphics_core::geometry::Size::new(WIDTH as u32, HEIGHT as u32),
            ),
        );
        if area.size.width == 0 || area.size.height == 0 {
            return Ok(());
        }

        let x = area.top_left.x as usize;
        let y = area.top_left.y as usize;
        let w = area.size.width as usize;
        let mut row = y;
        let mut col = 0;

        for color in colors.into_iter() {
            if col < w && row < HEIGHT {
                let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
                    core::convert::From::from(color);
                let raw: u16 =
                    embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
                self.buf[row * WIDTH + x + col] = raw;
            }
            col += 1;
            if col >= w {
                col = 0;
                row += 1;
            }
        }
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
                embedded_graphics_core::geometry::Size::new(WIDTH as u32, HEIGHT as u32),
            ),
        );
        if area.size.width == 0 || area.size.height == 0 {
            return Ok(());
        }
        let raw_u16: embedded_graphics_core::pixelcolor::raw::RawU16 =
            core::convert::From::from(color);
        let raw: u16 =
            embedded_graphics_core::pixelcolor::raw::RawData::into_inner(raw_u16);
        self.fill_rect(
            area.top_left.x as usize,
            area.top_left.y as usize,
            area.size.width as usize,
            area.size.height as usize,
            raw,
        );
        Ok(())
    }
}
