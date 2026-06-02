// GUI/BATTERY
// CLEAN BIG COLORED BATTERY PROGRESS ARC THAT EVEN 🦆🧑‍🦯 CAN SEE (not really...)
// WRITES PNG PIXELS DIRECTLY INTO THE RAW FRAMEBUFFER

use crate::components::framebuffer::Framebuffer;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::Point;
use embedded_graphics::primitives::{Arc, Rectangle, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, StrokeAlignment};
use embedded_ttf::FontTextStyle;
use embedded_graphics::text::Text;
use embedded_graphics::geometry::Angle;
use embedded_graphics_core::pixelcolor::IntoStorage;

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

pub fn draw(fb: &mut Framebuffer) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Size = embedded_graphics::geometry::Size;

    // CLEAR SCREEN > BLACK
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
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&styled_clear, fb).ok();

    // BATTERY STATE
    let percent = crate::load!(crate::state::BATTERY_PERCENT);
    let usb_connected = crate::load!(crate::state::BATTERY_USB_CONNECTED);

    // ARC COLOR
    let fill_color = if usb_connected {
        crate::gui::colors::CYAN
    } else {
        crate::gui::colors::gradient_red_green(percent)
    };

    // ARC GEOMETRY
    let min_dim = if W < H { W } else { H } as u32;
    let diameter = min_dim * 7 / 10;
    let center_x = W / 2;
    let center_y = H / 2;
    let top_left = embedded_graphics::geometry::Point::new(
        center_x - diameter as i32 / 2,
        center_y - diameter as i32 / 2,
    );
    let stroke_width = 5u32;

    // BACKGROUND ARC (FULL CIRCLE, GRAY)
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
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&styled_bg, fb).ok();

    // FILL ARC (PROPORTIONAL TO BATTERY)
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
            embedded_graphics::primitives::PrimitiveStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&styled_fill, fb).ok();
    }

    // CENTER CONTENT: BOLT ICON (CHARGING & <100%) OR PERCENTAGE TEXT
    if usb_connected && percent < 100 {
        // LOAD BOLT PNG FROM ASSETS
        if let Some(bolt_png) = embedded_png::Png::load_from_bytes(crate::base::assets::BOLT_PNG).ok() {
            let scale = 2;
            let img_w = bolt_png.width() as i32;
            let img_h = bolt_png.height() as i32;
            let scaled_w = img_w * scale;
            let scaled_h = img_h * scale;
            let x = center_x - scaled_w / 2;
            let y = center_y - scaled_h / 2;

            let dest = fb.buffer_mut();
            let screen_w = W as usize;
            let screen_h = H as usize;

            // RAW PIXEL DRAWING (FAST)
            for src_row in 0..img_h {
                for src_col in 0..img_w {
                    let idx = (src_row * img_w + src_col) as usize;
                    if let Some(color) = bolt_png.pixels()[idx] {
                        let raw: u16 = color.into_storage();
                        for dy in 0..scale {
                            let row = (y + src_row * scale + dy) as usize;
                            if row >= screen_h { break; }
                            for dx in 0..scale {
                                let col = (x + src_col * scale + dx) as usize;
                                if col < screen_w {
                                    dest[row * screen_w + col] = raw;
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // PERCENTAGE TEXT
        let ttf_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
        let ttf_style = embedded_ttf::FontTextStyleBuilder::new(ttf_font)
            .font_size(88)
            .text_color(crate::gui::colors::WHITE)
            .build();

        let mut pct_buf = [0u8; 4];
        let pct_str = format_percent(&mut pct_buf, percent);

        // MEASURE AND CENTER TEXT
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
            > as embedded_graphics::Drawable
        >::draw(&pct_text, fb).ok();
    }
}

// ───────────────────────────────────────────────────────────────────────
// HELPER FORMAT PERCENTAGE
fn format_percent<'a>(buf: &'a mut [u8; 4], pct: u8) -> &'a str {
    let mut pos = 0;
    if pct >= 100 {
        buf[pos] = b'1'; pos += 1;
        buf[pos] = b'0'; pos += 1;
        buf[pos] = b'0'; pos += 1;
    } else {
        let tens = pct / 10;
        let ones = pct % 10;
        if tens > 0 {
            buf[pos] = b'0' + tens; pos += 1;
        }
        buf[pos] = b'0' + ones; pos += 1;
    }
    buf[pos] = b'%'; pos += 1;
    core::str::from_utf8(&buf[..pos]).unwrap_or("?")
}
