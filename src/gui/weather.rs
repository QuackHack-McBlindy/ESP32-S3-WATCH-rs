// GUI/WEATHER
// DRAWS A CENTERED PNG WEATHER ICON AND A LARGE TEMPERATURE TEXT

use embedded_graphics::prelude::IntoStorage;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Dimensions;

// ───────────────────────────────────────────────────────────────────────
// PUBLIC DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;

    // LOAD TTF FONTS
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();

    // COLOR CONSTANTS
    let white = crate::gui::colors::WHITE;

    // SCREEN DIMENSIONS
    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;

    // LOAD WEATHER ICON PNG
    let png = embedded_png::Png::load_from_bytes(crate::base::assets::WEATHER_DRIZZLE_PNG).ok();
    if png.is_none() {
        return;
    }
    let png = png.unwrap();

    // CENTER THE ICON – USE A SCALE FACTOR TO MAKE IT BIG
    let scale: f32 = 1.8;
    let icon_w = (png.width() as f32 * scale) as i32;
    let icon_h = (png.height() as f32 * scale) as i32;
    let icon_x = (w - icon_w) / 2;
    let icon_y = (h - icon_h) / 2 - 60;

    draw_scaled_png_raw(fb.buffer_mut(), &png, icon_x, icon_y, scale, screen_w, screen_h);

    // TEMPERATURE TEXT – BIG, BELOW THE ICON
    let temperature = "18 °C";

    let temp_style = embedded_ttf::FontTextStyleBuilder::new(bold_font)
        .font_size(96)
        .text_color(white)
        .build();

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    let temp_text = embedded_graphics::text::Text::with_text_style(
        temperature,
        embedded_graphics::prelude::Point::new(w / 2, icon_y + icon_h + 20),  // 20PX GAP BELOW ICON
        temp_style,
        center_align,
    );
    temp_text.draw(fb).ok();
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING
fn draw_scaled_png_raw(
    dest: &mut [u16],
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
) {
    if scale <= 0.0 {
        return;
    }
    let src_w = png.width() as i32;
    let src_h = png.height() as i32;
    let dst_w = (src_w as f32 * scale) as i32;
    let dst_h = (src_h as f32 * scale) as i32;

    for dst_row in 0..dst_h {
        let src_row = ((dst_row as f32 / scale) as i32).clamp(0, src_h - 1);
        let row = (y + dst_row) as usize;
        if row >= screen_h {
            break;
        }
        for dst_col in 0..dst_w {
            let src_col = ((dst_col as f32 / scale) as i32).clamp(0, src_w - 1);
            let col = (x + dst_col) as usize;
            if col >= screen_w {
                break;
            }
            let idx = (src_row * src_w + src_col) as usize;
            if let Some(color) = png.pixels()[idx] {
                let raw: u16 = color.into_storage();
                dest[row * screen_w + col] = raw;
            }
        }
    }
}
