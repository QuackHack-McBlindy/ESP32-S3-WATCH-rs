// GUI/BATTERY
// CLEAN BIG COLORED BATTERY PROGRESS ARC THAT EVEN 🦆🧑‍🦯 CAN SEE (not really...)

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    // LOAD FONT
    let ttf_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();

    // CLEAR SCREEN
    let full_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::zero(),
        embedded_graphics::geometry::Size::new(W as u32, H as u32),
    );
    let styled_clear = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        full_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(crate::gui::colors::BLACK),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_clear, fb)
    .ok();

    // READ GLOBAL BATTERY STATE
    let percent = crate::load!(crate::state::BATTERY_PERCENT);
    let voltage_mv = crate::load!(crate::state::BATTERY_VOLTAGE);
    let usb_connected = crate::load!(crate::state::BATTERY_USB_CONNECTED);

    // PICK FILL COLOR: CYAN WHILE CHARGING, OTHERWISE GREEN-TO-RED GRADIENT
    let fill_color = if usb_connected {
        crate::gui::colors::CYAN
    } else {
        crate::gui::colors::gradient_red_green(percent)
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
        .stroke_color(crate::gui::colors::GRAY)
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

    // DECIDE WHETHER TO SHOW BOLT (USB CONNECTED & NOT FULL) OR PERCENTAGE TEXT
    if usb_connected && percent < 100 {
        // LOAD BOLT PNG FROM ASSETS
        if let Some(bolt_png) = embedded_png::Png::load_from_bytes(crate::base::assets::BOLT_PNG).ok() {
            let scale = 2; // BIG ENOUGH?
            let img_w = bolt_png.width() as i32;
            let img_h = bolt_png.height() as i32;
            let scaled_w = img_w * scale;
            let scaled_h = img_h * scale;
            let x = center_x - scaled_w / 2;
            let y = center_y - scaled_h / 2;

            // DRAW SCALED BOLT
            for src_row in 0..img_h {
                for src_col in 0..img_w {
                    let idx = (src_row * img_w + src_col) as usize;
                    if let Some(color) = bolt_png.pixels()[idx] {
                        let px = x + src_col * scale;
                        let py = y + src_row * scale;
                        for dy in 0..scale {
                            for dx in 0..scale {
                                let pixel = embedded_graphics::Pixel(
                                    embedded_graphics::geometry::Point::new(px + dx, py + dy),
                                    color,
                                );
                                <embedded_graphics::Pixel<embedded_graphics::pixelcolor::Rgb565> as embedded_graphics::Drawable>::draw(&pixel, fb).ok();
                            }
                        }
                    }
                }
            }
        }
    } else {
        // PERCENTAGE TEXT – BIG CENTERED INSIDE THE ARC
        let ttf_style = embedded_ttf::FontTextStyleBuilder::new(ttf_font)
            .font_size(88)
            .text_color(crate::gui::colors::WHITE)
            .build();
        
        let mut pct_buf = [0u8; 4];
        let pct_str = format_percent(&mut pct_buf, percent);
        
        // MANUAL CENTERING OF THE TEXT
        let metrics = <
            embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>
            as embedded_graphics::text::renderer::TextRenderer
        >::measure_string(
            &ttf_style,
            pct_str,
            embedded_graphics::geometry::Point::zero(),
            embedded_graphics::text::Baseline::Top,
        );
        let text_w = metrics.bounding_box.size.width as i32;
        let text_h = metrics.bounding_box.size.height as i32;
        let text_pos = embedded_graphics::geometry::Point::new(
            center_x - text_w / 2,
            center_y - text_h / 2,
        );
        
        let pct_text = embedded_graphics::text::Text::new(pct_str, text_pos, ttf_style);
        <
            embedded_graphics::text::Text<
                embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
            > as embedded_graphics::prelude::Drawable
        >::draw(&pct_text, fb)
        .ok();
    }
}

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
