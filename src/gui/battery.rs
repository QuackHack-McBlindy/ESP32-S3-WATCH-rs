// GUI/BATTERY
// CLEAN BIG BATTERY GAUGE

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

// BATTERY BODY PROPERTIES
const BODY_W: i32 = (W as f32 * 0.8) as i32;
const BODY_H: i32 = (H as f32 * 0.4) as i32;
const CORNER_R: u32 = 12;
const TERMINAL_W: i32 = 12;
const TERMINAL_H: i32 = 30;

// FILL COLOR THREASHOLD
const GREEN_THRESHOLD: u8 = 60;
const ORANGE_THRESHOLD: u8 = 20;


pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    let white = embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255);
    let black = embedded_graphics::pixelcolor::Rgb565::new(0, 0, 0);
    let gray = embedded_graphics::pixelcolor::Rgb565::new(0x80, 0x80, 0x80);
    let yellow = embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0xFF, 0x00);

    // CLEAR SCREEN
    let full_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::zero(),
        embedded_graphics::geometry::Size::new(W as u32, H as u32),
    );
    let styled_clear = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        full_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(black),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_clear, fb)
    .ok();

    // READ GLOBAL BATTERY STATE
    let (percent, voltage_mv, charging) = critical_section::with(|_cs| {
        let p = crate::load!(crate::state::BATTERY_PERCENT);
        let v = crate::load!(crate::state::BATTERY_VOLTAGE);
        let c = crate::load!(crate::state::BATTERY_CHARGING);
        (p, v, c)
    });

    // PICK FILL COLOR
    let fill_color = if charging {
        embedded_graphics::pixelcolor::Rgb565::new(0x00, 0xFF, 0xFF)         // cyan
    } else if percent > GREEN_THRESHOLD {
        embedded_graphics::pixelcolor::Rgb565::new(0x00, 0xFF, 0x00)         // green
    } else if percent > ORANGE_THRESHOLD {
        embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0xA5, 0x00)         // orange
    } else {
        embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0x00, 0x00)         // red
    };

    // LAYOUT – BODY CENTERED, TERMINAL ATTACHED ON THE RIGHT
    let body_x = (W - BODY_W) / 2;
    let body_y = (H - BODY_H) / 2 - 10;
    let terminal_x = body_x + BODY_W;
    let terminal_y = body_y + (BODY_H - TERMINAL_H) / 2;

    // FILL (ROUNDED, PROPERTIONAL)
    let fill_w = (BODY_W as u32 * percent as u32 / 100).max(1);
    if percent > 0 {
        let fill_rect = embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(body_x, body_y),
            embedded_graphics::geometry::Size::new(fill_w, BODY_H as u32),
        );
        let rrect = embedded_graphics::primitives::RoundedRectangle::new(
            fill_rect,
            embedded_graphics::primitives::CornerRadii::new(
                embedded_graphics::geometry::Size::new(CORNER_R, CORNER_R),
            ),
        );
        let styled_fill = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
            rrect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(fill_color),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::RoundedRectangle,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled_fill, fb)
        .ok();
    }

    // OUTLINE (WHITE)
    let outline_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(body_x, body_y),
        embedded_graphics::geometry::Size::new(BODY_W as u32, BODY_H as u32),
    );
    let rrect_outline = embedded_graphics::primitives::RoundedRectangle::new(
        outline_rect,
        embedded_graphics::primitives::CornerRadii::new(
            embedded_graphics::geometry::Size::new(CORNER_R, CORNER_R),
        ),
    );
    let styled_outline = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
        rrect_outline,
        embedded_graphics::primitives::PrimitiveStyle::with_stroke(white, 2),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::RoundedRectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_outline, fb)
    .ok();

    // TERMINAL (POSITIVE NUB)
    let term_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(terminal_x, terminal_y),
        embedded_graphics::geometry::Size::new(TERMINAL_W as u32, TERMINAL_H as u32),
    );
    let styled_term = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        term_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(white),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_term, fb)
    .ok();

    // CHARGING BOLT
    if charging {
        let cx = body_x + BODY_W / 2;
        let cy = body_y + BODY_H / 2;
        let bolt_style = embedded_graphics::primitives::PrimitiveStyle::with_stroke(yellow, 2);

        let lines = [
            ((cx - 10, cy - 20), (cx + 5, cy - 5)),
            ((cx + 5, cy - 5), (cx, cy - 10)),
            ((cx - 5, cy + 5), (cx + 15, cy + 20)),
            ((cx + 15, cy + 20), (cx, cy + 10)),
        ];
        for &(p1, p2) in &lines {
            let line = embedded_graphics::primitives::Line::new(
                embedded_graphics::geometry::Point::new(p1.0, p1.1),
                embedded_graphics::geometry::Point::new(p2.0, p2.1),
            );
            let styled_line = <embedded_graphics::primitives::Line as embedded_graphics::prelude::Primitive>::into_styled(
                line,
                bolt_style,
            );
            <embedded_graphics::primitives::Styled<
                embedded_graphics::primitives::Line,
                embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable>::draw(&styled_line, fb)
            .ok();
        }
    }

    // PERCENTAGE TEXT
    let font = embedded_graphics::mono_font::ascii::FONT_10X20;
    let mut pct_style = embedded_graphics::mono_font::MonoTextStyle::new(&font, black);
    if percent <= ORANGE_THRESHOLD {
        pct_style = embedded_graphics::mono_font::MonoTextStyle::new(&font, white);
    }
    let mut pct_buf = [0u8; 4];
    let pct_str = format_percent(&mut pct_buf, percent);
    let text = embedded_graphics::text::Text::with_alignment(
        pct_str,
        embedded_graphics::geometry::Point::new(body_x + BODY_W / 2, body_y + BODY_H / 2 + 8),
        pct_style,
        embedded_graphics::text::Alignment::Center,
    );
    <
        embedded_graphics::text::Text<embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>>
        as embedded_graphics::prelude::Drawable
    >::draw(&text, fb)
    .ok();

    // VOLTAGE BELOW BODY
    let volt_style = embedded_graphics::mono_font::MonoTextStyle::new(&font, gray);
    let mut volt_buf = [0u8; 8];
    let volt_str = format_voltage(&mut volt_buf, voltage_mv);
    let volt_text = embedded_graphics::text::Text::with_alignment(
        volt_str,
        embedded_graphics::geometry::Point::new(W / 2, body_y + BODY_H + 20),
        volt_style,
        embedded_graphics::text::Alignment::Center,
    );
    <
        embedded_graphics::text::Text<embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>>
        as embedded_graphics::prelude::Drawable
    >::draw(&volt_text, fb)
    .ok();
}

// FORMATTING HELPERS
fn format_percent<'a>(buf: &'a mut [u8; 4], pct: u8) -> &'a str {
    let mut pos = 0;
    if pct >= 100 {
        buf[pos] = b'1';
        pos += 1;
        buf[pos] = b'0';
        pos += 1;
        buf[pos] = b'0';
        pos += 1;
    } else {
        let tens = pct / 10;
        let ones = pct % 10;
        if tens > 0 || pos > 0 {
            buf[pos] = b'0' + tens;
            pos += 1;
        }
        buf[pos] = b'0' + ones;
        pos += 1;
    }
    buf[pos] = b'%';
    pos += 1;
    core::str::from_utf8(&buf[..pos]).unwrap_or("?")
}

fn format_voltage<'a>(buf: &'a mut [u8; 8], mv: u32) -> &'a str {
    let v = mv / 1000;
    let rem = (mv % 1000) / 100;
    let mut pos = 0;
    if v >= 10 {
        buf[pos] = b'0' + (v / 10) as u8;
        pos += 1;
    }
    buf[pos] = b'0' + (v % 10) as u8;
    pos += 1;
    buf[pos] = b'.';
    pos += 1;
    buf[pos] = b'0' + rem as u8;
    pos += 1;
    buf[pos] = b'V';
    pos += 1;
    core::str::from_utf8(&buf[..pos]).unwrap_or("?.?V")
}
