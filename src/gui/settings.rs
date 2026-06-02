// GUI/SETTINGS/WIFI.RS
// DRAW A SETTINGS PAGE FOR TOGGLE WIFI ON/OFF
// BIG COLOR CODED TOGGLE SWITCH AND A STATE SWITCHING IMAGE

// GUI/SETTINGS/WIFI.RS
// WIFI TOGGLE – USES GLOBAL WIFI_STATE AS A BOOLEAN

use crate::components::framebuffer::Framebuffer;
use embedded_graphics::Drawable;
use embedded_graphics::prelude::Point;
use embedded_graphics::primitives::{Circle, Rectangle, Primitive, PrimitiveStyle};
use embedded_graphics_core::geometry::Size;
use embedded_graphics::text::Text;
use embedded_graphics::text::TextStyleBuilder;
use embedded_ttf::FontTextStyleBuilder;
use embedded_graphics_core::pixelcolor::IntoStorage;
use embedded_graphics::geometry::Dimensions;
use rusttype;
use crate::components::ft3168::SwipeDirection;
static mut HIT_AREA: Option<crate::gui::HitArea> = None;

pub fn handle_touch(x: i32, y: i32) -> Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let Some(area) = core::ptr::addr_of!(HIT_AREA).read().as_ref() {
            if crate::gui::hit_test(x, y, area) {
                // Current state – WIFI_STATE is an AtomicBool
                let is_on = crate::load!(crate::state::WIFI_STATE);
                let new_state = !is_on;

                // Update the state
                crate::store!(crate::state::WIFI_STATE, new_state);

                return Some(crate::gui::TouchAction::SettingsToggle);
            }
        }
        None
    })
}



pub fn handle_swipe(
    direction: SwipeDirection,
    _start_x: u16,
    start_y: u16,
    _last_x: u16,
    last_y: u16,
) {
    match direction {
        SwipeDirection::Up | SwipeDirection::Down => {
            // Screen y increases downward → up swipe gives negative delta
            let raw_delta = last_y as i32 - start_y as i32;   // negative for up
            let volume_change = -raw_delta;                    // positive for up, negative for down

            // Sensitivity: 1% per 2 pixels of vertical movement
            let sensitivity = 2;
            let delta_vol = volume_change / sensitivity;

            if delta_vol != 0 {
                let current: u8 = crate::load!(crate::state::MIC_VOLUME); // 0-100
                let new_val = (current as i32 + delta_vol).clamp(0, 100) as u8;
                crate::store!(crate::state::MIC_VOLUME, new_val);
                defmt::info!("Mic volume adjusted to {}%", new_val);
            }
        }
        _ => {}
    }
}

pub fn draw(fb: &mut Framebuffer) {
    let is_on = crate::load!(crate::state::WIFI_STATE);   // true/false

    // Screen dimensions
    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = w as usize;
    let screen_h = h as usize;

    // Clear to black
    let _ = Rectangle::new(Point::zero(), Size::new(w as u32, h as u32))
        .into_styled(PrimitiveStyle::with_fill(crate::gui::colors::BLACK))
        .draw(fb);

    // Header
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let header_style = FontTextStyleBuilder::new(bold_font.clone())
        .font_size(86)
        .text_color(crate::gui::colors::CYAN)
        .build();
    let header_align = TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = Text::with_text_style("WIFI", Point::new(w / 2, 20), header_style, header_align)
        .draw(fb);

    // Wi‑Fi icon
    let icon_bytes = if is_on {
        crate::base::assets::SETTINGS_WIFI_ON_PNG
    } else {
        crate::base::assets::SETTINGS_WIFI_OFF_PNG
    };

    if let Ok(icon_png) = embedded_png::Png::load_from_bytes(icon_bytes) {
        let img_w = icon_png.width() as i32;
        let img_h = icon_png.height() as i32;
        let target_h = (h as f32 * 0.66) as i32;
        let scale = core::cmp::max(1, target_h / img_h.max(1));
        let scaled_w = img_w * scale;
        let scaled_h = img_h * scale;
        let x = (w - scaled_w) / 2;
        let y = (h - scaled_h) / 2;
        let dest = fb.buffer_mut();
        for sy in 0..img_h {
            for sx in 0..img_w {
                let idx = (sy * img_w + sx) as usize;
                if let Some(color) = icon_png.pixels()[idx] {
                    let raw: u16 = color.into_storage();
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

    // Toggle at bottom centre
    let track_w = 120i32;
    let track_h = 60i32;
    let bottom_margin = 40i32;
    let toggle_left = (w - track_w) / 2;
    let toggle_top = h - track_h - bottom_margin;

    draw_toggle_switch(fb, Point::new(toggle_left, toggle_top), if is_on { 1.0 } else { 0.0 });

    let area = crate::gui::HitArea {
        x: toggle_left,
        y: toggle_top,
        width: track_w as u32,
        height: track_h as u32,
        action: crate::gui::TouchAction::SettingsToggle,
    };
    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(HIT_AREA).write(Some(area));
    });
}

// ── toggle drawing (unchanged, but now receives top‑left) ─
pub fn draw_toggle_switch(
    fb: &mut Framebuffer,
    top_left: Point,
    progress: f32,
) {
    let track_w = 120i32;
    let track_h = 60i32;
    let thumb_diameter = 52u32;
    let thumb_radius = thumb_diameter as i32 / 2;
    let margin = 4i32;

    let track_left = top_left.x;
    let track_top = top_left.y;

    let white = crate::gui::colors::WHITE;
    let dark_gray = crate::gui::colors::DARK_GRAY;
    let red = crate::gui::colors::RED;
    let green = crate::gui::colors::GREEN;

    let progress_u8 = (progress * 255.0) as u8;
    let track_fill = crate::gui::colors::blend(dark_gray, green, progress_u8);
    let glow_color = crate::gui::colors::blend(red, green, progress_u8);

    // Glow background
    let glow_expand = 3i32;
    let _ = Rectangle::new(
        Point::new(track_left - glow_expand, track_top - glow_expand),
        Size::new((track_w + 2 * glow_expand) as u32, (track_h + 2 * glow_expand) as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(glow_color))
    .draw(fb);

    // Track
    let _ = Rectangle::new(
        Point::new(track_left, track_top),
        Size::new(track_w as u32, track_h as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(track_fill))
    .draw(fb);

    // Thumb
    let thumb_left_max = track_left + margin;
    let thumb_left_min = track_left + track_w - margin - thumb_diameter as i32;
    let thumb_center_x = thumb_left_max
        + (progress * (thumb_left_min - thumb_left_max) as f32) as i32
        + thumb_radius;
    let thumb_center_y = track_top + track_h / 2;

    // Drop shadow
    let shadow_offset = 2i32;
    let _ = Circle::new(
        Point::new(
            thumb_center_x + shadow_offset - thumb_radius,
            thumb_center_y + shadow_offset - thumb_radius,
        ),
        thumb_diameter,
    )
    .into_styled(PrimitiveStyle::with_fill(dark_gray))
    .draw(fb);

    // White thumb
    let _ = Circle::new(
        Point::new(thumb_center_x - thumb_radius, thumb_center_y - thumb_radius),
        thumb_diameter,
    )
    .into_styled(PrimitiveStyle::with_fill(white))
    .draw(fb);
}
