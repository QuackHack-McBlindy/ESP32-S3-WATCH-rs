// GUI/ROLEX
// ROLEX SUBMARINER‑STYLE WATCH FACE
// FLUTED BEZEL, CYCLOPS DATE, MERCEDES HANDS, LUME, CROWN LOGO.

// SCREEN DIMENSIONS
const SCREEN_W: i32 = crate::state::LCD_WIDTH as i32;
const SCREEN_H: i32 = crate::state::LCD_HEIGHT as i32;
const CX: i32 = SCREEN_W / 2;
const CY: i32 = SCREEN_H / 2;

// DIAL GEOMETRY
const DIAL_R: i32 = 104;         // MAIN DIAL RADIUS
const BEZEL_R: i32 = DIAL_R + 12;// OUTER EDGE OF FLUTED BEZEL
const FLUTES: i32 = 60;          // NUMBER OF FLUTES ON BEZEL

// HANDS LENGTHS (FRACTION OF DIAL)
const HOUR_HAND_LEN: i32 = 55;
const MINUTE_HAND_LEN: i32 = 80;
const SECOND_HAND_LEN: i32 = 90;
const CENTER_R: i32 = 7;

// MERCEDES HAND DIMENSIONS
const MERCEDES_WING_R: i32 = 14;     // RADIUS OF THE SMALL WINGS

// COLORS
const GOLD: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0xD7, 0x00);
const LUME: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0xA0, 0xA0, 0x80);
const DARK_GRAY: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0x20, 0x20, 0x20);
const VERY_DARK: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0x10, 0x10, 0x10);
const WHITE: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255);
const BLACK: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0, 0, 0);
const RED: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(255, 0, 0);
const CSS_GRAY: embedded_graphics::pixelcolor::Rgb565 = embedded_graphics::pixelcolor::Rgb565::new(0x80, 0x80, 0x80);


// WATCHFACE STRUCT
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOutcome {
    pub full_redraw: bool,
}

pub struct WatchFace {
    hours: u8,
    minutes: u8,
    seconds: u8,
    day: u8,
    month: u8,
    year: u8,
    full_redraw: bool,
    time_changed: bool,
}

impl WatchFace {
    pub fn new() -> Self {
        Self {
            hours: 0, minutes: 0, seconds: 0,
            day: 1, month: 1, year: 25,
            full_redraw: true,
            time_changed: false,
        }
    }

    pub fn update_time(&mut self, h: u8, m: u8, s: u8) {
        if self.hours != h || self.minutes != m || self.seconds != s {
            self.hours = h;
            self.minutes = m;
            self.seconds = s;
            self.time_changed = true;
        }
    }

    pub fn update_date(&mut self, day: u8, month: u8, year: u8) {
        self.day = day;
        self.month = month;
        self.year = year;
    }

    pub fn force_redraw(&mut self) { self.full_redraw = true; }
    pub fn needs_render(&self) -> bool { self.full_redraw || self.time_changed }

    /// RENDER THE ENTIRE ROLEX DIAL.
    pub fn render<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        d: &mut D,
    ) -> Result<RenderOutcome, D::Error> {
        if !self.needs_render() {
            return Ok(RenderOutcome::default());
        }

        // 1. CLEAR SCREEN
        self.clear_screen(d)?;

        // 2. FLUTED BEZEL
        self.draw_fluted_bezel(d)?;

        // 3. DIAL BACKGROUND
        {
            let circle = embedded_graphics::primitives::Circle::new(
                embedded_graphics::geometry::Point::new(CX - DIAL_R, CY - DIAL_R),
                (DIAL_R * 2) as u32,
            );
            let styled = <embedded_graphics::primitives::Circle as embedded_graphics::prelude::Primitive>::into_styled(
                circle,
                embedded_graphics::primitives::PrimitiveStyle::with_fill(DARK_GRAY),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Circle,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }

        // 4. MINUTE MARKERS
        self.draw_minute_ticks(d)?;

        // 5. HOUR MARKERS
        self.draw_hour_markers(d)?;

        // 6. CROWN LOGO
        self.draw_crown_logo(d)?;

        // 7. HANDS
        self.draw_hour_hand(d)?;
        self.draw_minute_hand(d)?;
        self.draw_second_hand(d)?;

        // 8. CENTER CAP
        {
            let circle = embedded_graphics::primitives::Circle::new(
                embedded_graphics::geometry::Point::new(CX - CENTER_R, CY - CENTER_R),
                (CENTER_R * 2) as u32,
            );
            let styled = <embedded_graphics::primitives::Circle as embedded_graphics::prelude::Primitive>::into_styled(
                circle,
                embedded_graphics::primitives::PrimitiveStyle::with_fill(GOLD),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Circle,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }

        // 9. DATE WITH CYCLOPS
        self.draw_date_cyclops(d)?;

        // 10. "ROLEX" TEXT
        {
            let font = embedded_graphics::mono_font::ascii::FONT_10X20;
            let style = embedded_graphics::mono_font::MonoTextStyle::new(&font, GOLD);
            let text = embedded_graphics::text::Text::with_alignment(
                "ROLEX",
                embedded_graphics::geometry::Point::new(CX, CY + DIAL_R - 28),
                style,
                embedded_graphics::text::Alignment::Center,
            );
            <embedded_graphics::text::Text<
                embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&text, d)?;
        }

        self.full_redraw = false;
        self.time_changed = false;
        Ok(RenderOutcome { full_redraw: true })
    }

    fn clear_screen<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let rect = embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::zero(),
            embedded_graphics::geometry::Size::new(SCREEN_W as u32, SCREEN_H as u32),
        );
        let styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
            rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(VERY_DARK),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Rectangle,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)
    }

    fn draw_fluted_bezel<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let mut angle = 0.0f32;
        let step = 360.0 / FLUTES as f32;
        for _ in 0..FLUTES {
            let rad = angle.to_radians();
            let inner = (DIAL_R + 2) as f32;
            let outer = BEZEL_R as f32;
            let (x1, y1) = circle_point(CX, CY, inner, rad);
            let (x2, y2) = circle_point(CX, CY, outer, rad);
            let color = if ((angle / step) as i32) % 2 == 0 { GOLD } else { DARK_GRAY };
            let line = embedded_graphics::primitives::Line::new(
                embedded_graphics::geometry::Point::new(x1, y1),
                embedded_graphics::geometry::Point::new(x2, y2),
            );
            let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
                line,
                embedded_graphics::primitives::PrimitiveStyle::with_stroke(color, 1),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Line,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
            angle += step;
        }
        // OUTER BEZEL EDGE
        let circle = embedded_graphics::primitives::Circle::new(
            embedded_graphics::geometry::Point::new(CX - BEZEL_R, CY - BEZEL_R),
            (BEZEL_R * 2) as u32,
        );
        let styled = <embedded_graphics::primitives::Circle as embedded_graphics::prelude::Primitive>::into_styled(
            circle,
            embedded_graphics::primitives::PrimitiveStyle::with_stroke(GOLD, 3),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Circle,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)
    }

    fn draw_minute_ticks<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let tick_len = 6;
        for i in 0..60 {
            let angle = (i as f32) * 6.0 - 90.0;
            let rad = angle.to_radians();
            let inner = DIAL_R - tick_len;
            let outer = DIAL_R - 1;
            let (x1, y1) = circle_point(CX, CY, inner as f32, rad);
            let (x2, y2) = circle_point(CX, CY, outer as f32, rad);
            let color = if i % 5 == 0 { LUME } else { VERY_DARK };
            let line = embedded_graphics::primitives::Line::new(
                embedded_graphics::geometry::Point::new(x1, y1),
                embedded_graphics::geometry::Point::new(x2, y2),
            );
            let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
                line,
                embedded_graphics::primitives::PrimitiveStyle::with_stroke(color, 1),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Line,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }
        Ok(())
    }

    fn draw_hour_markers<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        for i in 0..12 {
            let angle = (i as f32) * 30.0 - 90.0;
            let rad = angle.to_radians();
            let inner = DIAL_R - 10;
            let outer = DIAL_R - 2;
            let (x1, y1) = circle_point(CX, CY, inner as f32, rad);
            let (x2, y2) = circle_point(CX, CY, outer as f32, rad);
            let line = embedded_graphics::primitives::Line::new(
                embedded_graphics::geometry::Point::new(x1, y1),
                embedded_graphics::geometry::Point::new(x2, y2),
            );
            let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
                line,
                embedded_graphics::primitives::PrimitiveStyle::with_stroke(LUME, 2),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Line,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }
        Ok(())
    }

    fn draw_crown_logo<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let top = CY - DIAL_R + 8;
        let triangle = embedded_graphics::primitives::Triangle::new(
            embedded_graphics::geometry::Point::new(CX, top - 6),
            embedded_graphics::geometry::Point::new(CX - 5, top),
            embedded_graphics::geometry::Point::new(CX + 5, top),
        );
        let styled = <embedded_graphics::primitives::Triangle as embedded_graphics::prelude::Primitive>::into_styled(
            triangle,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(GOLD),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Triangle,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)
    }

    fn draw_hour_hand<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let angle = ((self.hours % 12) as f32 * 30.0 + self.minutes as f32 * 0.5) - 90.0;
        let (hx, hy) = circle_point(CX, CY, HOUR_HAND_LEN as f32, angle.to_radians());

        // MAIN BATON
        let line = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(CX, CY),
            embedded_graphics::geometry::Point::new(hx, hy),
        );
        let style_main = embedded_graphics::primitives::PrimitiveStyle::with_stroke(LUME, 4);
        let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
            line,
            style_main,
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Line,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;

        // WINGS
        let wing_angle1 = angle + 45.0;
        let wing_angle2 = angle - 45.0;
        let (wx1, wy1) = circle_point(CX, CY, MERCEDES_WING_R as f32, wing_angle1.to_radians());
        let (wx2, wy2) = circle_point(CX, CY, MERCEDES_WING_R as f32, wing_angle2.to_radians());
        let style_wing = embedded_graphics::primitives::PrimitiveStyle::with_stroke(LUME, 2);

        let line1 = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(CX, CY),
            embedded_graphics::geometry::Point::new(wx1, wy1),
        );
        let styled1 = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
            line1,
            style_wing,
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Line,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled1, d)?;

        let line2 = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(CX, CY),
            embedded_graphics::geometry::Point::new(wx2, wy2),
        );
        let styled2 = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
            line2,
            style_wing,
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Line,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled2, d)
    }

    fn draw_minute_hand<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let angle = (self.minutes as f32 * 6.0) - 90.0;
        let (mx, my) = circle_point(CX, CY, MINUTE_HAND_LEN as f32, angle.to_radians());
        let line = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(CX, CY),
            embedded_graphics::geometry::Point::new(mx, my),
        );
        let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
            line,
            embedded_graphics::primitives::PrimitiveStyle::with_stroke(LUME, 3),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Line,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)
    }

    fn draw_second_hand<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let angle = (self.seconds as f32 * 6.0) - 90.0;
        let (sx, sy) = circle_point(CX, CY, SECOND_HAND_LEN as f32, angle.to_radians());
        let line = embedded_graphics::primitives::Line::new(
            embedded_graphics::geometry::Point::new(CX, CY),
            embedded_graphics::geometry::Point::new(sx, sy),
        );
        let styled = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
            line,
            embedded_graphics::primitives::PrimitiveStyle::with_stroke(RED, 1),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Line,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, d)
    }

    fn draw_date_cyclops<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        d: &mut D,
    ) -> Result<(), D::Error> {
        let wx = CX + DIAL_R - 30;
        let wy = CY - 10;
        // WHITE BACKGROUND
        {
            let rect = embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::geometry::Point::new(wx - 20, wy - 8),
                embedded_graphics::geometry::Size::new(40, 20),
            );
            let styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
                rect,
                embedded_graphics::primitives::PrimitiveStyle::with_fill(WHITE),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Rectangle,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }
        // DATE TEXT
        {
            let font = embedded_graphics::mono_font::ascii::FONT_10X20;
            let mut date_buf = [0u8; 8];
            let date_str = fmt_date_short(&mut date_buf, self.day, self.month, self.year);
            let style = embedded_graphics::mono_font::MonoTextStyle::new(&font, BLACK);
            let text = embedded_graphics::text::Text::with_alignment(
                date_str,
                embedded_graphics::geometry::Point::new(wx, wy + 10),
                style,
                embedded_graphics::text::Alignment::Center,
            );
            <embedded_graphics::text::Text<
                embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&text, d)?;
        }
        // CYCLOPS LENS
        {
            let lens_r = 16;
            let circle = embedded_graphics::primitives::Circle::new(
                embedded_graphics::geometry::Point::new(wx - lens_r, wy - lens_r),
                (lens_r * 2) as u32,
            );
            let styled = <embedded_graphics::primitives::Circle as embedded_graphics::prelude::Primitive>::into_styled(
                circle,
                embedded_graphics::primitives::PrimitiveStyle::with_stroke(GOLD, 2),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Circle,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }
        // REFLECTION
        {
            let lens_r = 16;
            let angle = -45.0f32.to_radians();
            let radius = (lens_r - 4) as f32;
            let x = wx + (radius * micromath::F32Ext::cos(angle)) as i32;
            let y = wy + (radius * micromath::F32Ext::sin(angle)) as i32;
            let circle = embedded_graphics::primitives::Circle::new(
                embedded_graphics::geometry::Point::new(x - 2, y - 2),
                4,
            );
            let styled = <embedded_graphics::primitives::Circle as embedded_graphics::prelude::Primitive>::into_styled(
                circle,
                embedded_graphics::primitives::PrimitiveStyle::with_fill(WHITE),
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Circle,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled, d)?;
        }
        Ok(())
    }
}


// UTILITY FUNCTIONS
fn circle_point(cx: i32, cy: i32, radius: f32, rad: f32) -> (i32, i32) {
    let x = cx + (radius * micromath::F32Ext::cos(rad)) as i32;
    let y = cy + (radius * micromath::F32Ext::sin(rad)) as i32;
    (x, y)
}

fn fmt_date_short<'a>(buf: &'a mut [u8; 8], day: u8, month: u8, _year: u8) -> &'a str {
    let mut p = 0;
    buf[p] = b'0' + day / 10; p += 1;
    buf[p] = b'0' + day % 10; p += 1;
    buf[p] = b'/'; p += 1;
    buf[p] = b'0' + month / 10; p += 1;
    buf[p] = b'0' + month % 10; p += 1;
    core::str::from_utf8(&buf[..p]).unwrap_or("??/??")
}


// GLOBAL INSTANCE
static WATCHFACE: critical_section::Mutex<core::cell::RefCell<Option<WatchFace>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));


// PUBLIC DRAW FUNCTION (CALLED BY DISPLAY_TASK)
pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    let maybe_time = critical_section::with(|cs| crate::state::CURRENT_TIME.borrow(cs).get());

    if let Some(dt) = maybe_time {
        let h = dt.hours;
        let m = dt.minutes;
        let s = dt.seconds;
        let day = dt.day;
        let month = dt.month;
        let year = (dt.year % 100) as u8;

        critical_section::with(|cs| {
            let mut wf_ref = WATCHFACE.borrow_ref_mut(cs);
            let wf = wf_ref.get_or_insert_with(|| WatchFace::new());
            wf.update_time(h, m, s);
            wf.update_date(day, month, year);
            wf.force_redraw();
            let _ = wf.render(fb);
        });
    } else {
        let font = embedded_graphics::mono_font::ascii::FONT_10X20;
        let style = embedded_graphics::mono_font::MonoTextStyle::new(&font, WHITE);
        let text = embedded_graphics::text::Text::new(
            "No time",
            embedded_graphics::geometry::Point::new(10, 50),
            style,
        );
        <embedded_graphics::text::Text<
            embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&text, fb)
        .ok();
    }
}
