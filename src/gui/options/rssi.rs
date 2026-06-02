// GUI/OPTIONS/RSSI
// DRAWS READ‑ONLY RSSI PAGE – ARC GAUGE + WI‑FI ICON + dBm VALUE

use crate::components::framebuffer::Framebuffer;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::Point;
use embedded_graphics::primitives::{Arc, Rectangle, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, StrokeAlignment};
use embedded_graphics::text::Text;
use embedded_graphics::text::TextStyleBuilder;
use embedded_ttf::FontTextStyleBuilder;
use embedded_graphics_core::pixelcolor::IntoStorage;
use embedded_graphics::geometry::{Angle, Dimensions};

use rusttype;

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

// ───────────────────────────────────────────────────────────────────────
// DRAW FUNCTION
pub fn draw(fb: &mut Framebuffer) {
    let is_on = crate::load!(crate::state::WIFI_CONNECTED);
    type Rgb = embedded_graphics::pixelcolor::Rgb565;

    // CLEAR SCREEN
    let _ = Rectangle::new(Point::zero(), embedded_graphics_core::geometry::Size::new(W as u32, H as u32))
        .into_styled(PrimitiveStyle::with_fill(crate::gui::colors::BLACK))
        .draw(fb);

    // HEADER "RSSI"
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let header_style = FontTextStyleBuilder::new(bold_font.clone())
        .font_size(86)
        .text_color(crate::gui::colors::CYAN)
        .build();
    let header_align = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = Text::with_text_style("RSSI", Point::new(W / 2, 20), header_style, header_align)
        .draw(fb);

    // READ RSSI & MAP TO PERCENTAGE (0–100)
    let rssi: i32 = crate::load!(crate::state::RSSI).into();
    let rssi_clamped = rssi.clamp(-90, -30);
    let percent: u8 = ((rssi_clamped + 90) * 100 / 60) as u8;

    let wifi_on: bool = crate::load!(crate::state::WIFI_STATE);

    // ARC GEOMETRY
    let min_dim = if W < H { W } else { H } as u32;
    let diameter = min_dim * 7 / 10;
    let center_x = W / 2;
    let center_y = H / 2;
    let top_left = Point::new(center_x - diameter as i32 / 2, center_y - diameter as i32 / 2);
    let stroke_width = 5u32;

    // BACKGROUND ARC
    let bg_arc = Arc::new(top_left, diameter, Angle::from_degrees(270.0), Angle::from_degrees(360.0));
    let _ = bg_arc
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(crate::gui::colors::GRAY)
                .stroke_width(stroke_width)
                .stroke_alignment(StrokeAlignment::Inside)
                .build(),
        )
        .draw(fb);

    // FILL ARC
    let fill_color = crate::gui::colors::gradient_red_green(percent);
    if percent > 0 {
        let sweep_deg = -360.0 * percent as f32 / 100.0;
        let fill_arc = Arc::new(top_left, diameter, Angle::from_degrees(270.0), Angle::from_degrees(sweep_deg));
        let _ = fill_arc
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(fill_color)
                    .stroke_width(stroke_width)
                    .stroke_alignment(StrokeAlignment::Inside)
                    .build(),
            )
            .draw(fb);
    }

    // CENTER: WI‑FI ICON + RSSI VALUE
    // WI‑FI ICON
    let icon_bytes = if wifi_on {
        crate::base::assets::SETTINGS_WIFI_ON_PNG
    } else {
        crate::base::assets::SETTINGS_WIFI_OFF_PNG
    };

    if let Ok(icon_png) = embedded_png::Png::load_from_bytes(icon_bytes) {
        let img_w = icon_png.width() as i32;
        let img_h = icon_png.height() as i32;
        // SCALE TO FIT WITHIN THE ARC
        let max_icon_h = (diameter as f32 * 0.66) as i32;
        let scale = core::cmp::max(1, max_icon_h / img_h.max(1));
        let scaled_w = img_w * scale;
        let scaled_h = img_h * scale;
        let x = center_x - scaled_w / 2;
        let y = center_y - scaled_h / 2 - 15;

        let dest = fb.buffer_mut();
        let screen_w = W as usize;
        let screen_h = H as usize;
        for sy in 0..img_h {
            for sx in 0..img_w {
                let idx = (sy * img_w + sx) as usize;
                if let Some(color) = icon_png.pixels()[idx] {
                    // IF WIFI IS CONNECTED - KEEP ICON WHITE
                    let raw: u16 = if is_on {
                        color.into_storage()
                    } else {
                        // IF DISCONNECTED - MAKE IT RED FOR CLARITY
                        crate::gui::colors::RED.into_storage()
                    };
                    let px = x + sx * scale;
                    let py = y + sy * scale;
                    for dy in 0..scale {
                        let row = (py + dy) as usize;
                        if row >= screen_h { break; }
                        for dx in 0..scale {
                            let col = (px + dx) as usize;
                            if col < screen_w {
                                dest[row * screen_w + col] = raw;
                            }
                        }
                    }
                }
            }
        }
    }

    // RSSI TEXT BELOW THE ICON
    let rssi_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let rssi_style = FontTextStyleBuilder::new(rssi_font)
        .font_size(82)
        .text_color(crate::gui::colors::WHITE)
        .build();
    let rssi_text = format_rssi(rssi);
    let rssi_align = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = Text::with_text_style(&rssi_text, Point::new(center_x, center_y + 70), rssi_style, rssi_align)
        .draw(fb);
}


// ───────────────────────────────────────────────────────────────────────
// RSSI FORMATTING HELPER
fn format_rssi(rssi: i32) -> heapless::String<16> {
    let mut s = heapless::String::new();
    core::fmt::Write::write_fmt(&mut s, format_args!("{} dBm", rssi)).ok();
    s
}
