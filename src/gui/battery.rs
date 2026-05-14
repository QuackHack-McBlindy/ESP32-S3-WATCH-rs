// GUI/BATTERY
// CLEAN BIG COLORED BATTERY PROGRESS ARC THAT EVEN 🦆🧑‍🦯 CAN SEE (not really...)

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    // DEFINE COLORS
    let white = embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255);
    let black = embedded_graphics::pixelcolor::Rgb565::new(0, 0, 0);
    let gray = embedded_graphics::pixelcolor::Rgb565::new(0x80, 0x80, 0x80);
    let yellow = embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0xFF, 0x00);
    let cyan = embedded_graphics::pixelcolor::Rgb565::new(0x00, 0xFF, 0xFF);

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

    // PICK FILL COLOR: CYAN WHILE CHARGING, OTHERWISE GREEN-TO-RED GRADIENT
    let fill_color = if charging {
        cyan
    } else {
        // LINEAR INTERPOLATION: 100% GREEN -> 0% RED
        let r = ((100u16 - percent as u16) * 255 / 100) as u8;
        let g = (percent as u16 * 255 / 100) as u8;
        embedded_graphics::pixelcolor::Rgb565::new(r, g, 0)
    };

    // PROGRESS ARC LAYOUT – CENTERED, TAKES 70% OF SMALLEST SCREEN DIMENSION
    let min_dim = if W < H { W } else { H } as u32;
    let diameter = min_dim * 7 / 10;
    let center_x = W / 2;
    let center_y = H / 2;
    let top_left = embedded_graphics::geometry::Point::new(
        center_x - diameter as i32 / 2,
        center_y - diameter as i32 / 2,
    );

    let stroke_width = 5u32;

    // BACKGROUND TRACK (FULL CIRCLE, GRAY)
    let bg_arc = embedded_graphics::primitives::Arc::new(
        top_left,
        diameter,
        embedded_graphics::geometry::Angle::from_degrees(270.0),
        embedded_graphics::geometry::Angle::from_degrees(360.0),
    );
    let bg_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
        .stroke_color(gray)
        .stroke_width(stroke_width)
        .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
        .build();
    let styled_bg = <embedded_graphics::primitives::Arc as embedded_graphics::prelude::Primitive>::into_styled(bg_arc, bg_style);
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Arc,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_bg, fb)
    .ok();

    // FILL ARC (CLOCKWISE FROM TOP, PROPORTIONAL TO BATTERY PERCENT)
    if percent > 0 {
        let sweep_deg = -360.0 * percent as f32 / 100.0;
        let fill_arc = embedded_graphics::primitives::Arc::new(
            top_left,
            diameter,
            embedded_graphics::geometry::Angle::from_degrees(270.0),
            embedded_graphics::geometry::Angle::from_degrees(sweep_deg),
        );
        let fill_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .stroke_color(fill_color)
            .stroke_width(stroke_width)
            .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
            .build();
        let styled_fill = <embedded_graphics::primitives::Arc as embedded_graphics::prelude::Primitive>::into_styled(fill_arc, fill_style);
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Arc,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled_fill, fb)
        .ok();
    }

    // CHARGING LIGHTNING BOLT (SIMPLE LINES)
    if charging {
        let bcx = center_x;
        let bcy = center_y;
        let bolt_style = embedded_graphics::primitives::PrimitiveStyle::with_stroke(yellow, 2);
        let bolt_lines: [( (i32, i32), (i32, i32) ); 4] = [
            ((bcx - 10, bcy - 20), (bcx + 5, bcy - 5)),
            ((bcx + 5, bcy - 5), (bcx, bcy - 10)),
            ((bcx - 5, bcy + 5), (bcx + 15, bcy + 20)),
            ((bcx + 15, bcy + 20), (bcx, bcy + 10)),
        ];
        for &(p1, p2) in &bolt_lines {
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

    // PERCENTAGE TEXT – PERFECTLY CENTERED INSIDE THE ARC
    let font = embedded_graphics::mono_font::ascii::FONT_10X20;
    let pct_style = embedded_graphics::mono_font::MonoTextStyle::new(&font, white);
    let text_style = embedded_graphics::text::TextStyleBuilder::new()
        .baseline(embedded_graphics::text::Baseline::Middle)
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let mut pct_buf = [0u8; 4];
    let pct_str = format_percent(&mut pct_buf, percent);
    let pct_text = embedded_graphics::text::Text::with_text_style(
        pct_str,
        embedded_graphics::geometry::Point::new(center_x, center_y),
        pct_style,
        text_style,
    );
    <embedded_graphics::text::Text<embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>> as embedded_graphics::prelude::Drawable>::draw(&pct_text, fb).ok();

    // VOLTAGE READOUT BELOW THE ARC
    let volt_style = embedded_graphics::mono_font::MonoTextStyle::new(&font, gray);
    let mut volt_buf = [0u8; 8];
    let volt_str = format_voltage(&mut volt_buf, voltage_mv);
    let volt_y = center_y + (diameter as i32 / 2) + 20;
    let volt_text = embedded_graphics::text::Text::with_alignment(
        volt_str,
        embedded_graphics::geometry::Point::new(center_x, volt_y),
        volt_style,
        embedded_graphics::text::Alignment::Center,
    );
    <embedded_graphics::text::Text<embedded_graphics::mono_font::MonoTextStyle<embedded_graphics::pixelcolor::Rgb565>> as embedded_graphics::prelude::Drawable>::draw(&volt_text, fb).ok();
}

// FORMATTING HELPERS (UNCHANGED FROM ORIGINAL)
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
