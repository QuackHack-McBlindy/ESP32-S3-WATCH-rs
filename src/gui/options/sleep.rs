// GUI/OPTIONS/SLEEP
// DRAWS SETTINGS PAGE FOR POWER DOWN TIMEOUT – ARC GAUGE + SLEEP ICON + SECONDS VALUE
// SWIPE UP/DOWN TO ADJUST THE POWER DOWN TIMEOUT DURATION 0 SECONDS (DISABLED) – 5 MIN

use embedded_graphics::prelude::Primitive;

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

// ───────────────────────────────────────────────────────────────────────
// DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    // CLEAR SCREEN
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::zero(),
            embedded_graphics_core::geometry::Size::new(W as u32, H as u32),
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            crate::gui::colors::BLACK,
        )),
        fb,
    );

    // HEADER "SLEEP"
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let header_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(86)
        .text_color(crate::gui::colors::CYAN)
        .build();
    let header_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::text::Text::with_text_style(
            "SLEEP",
            embedded_graphics::prelude::Point::new(W / 2, 20),
            header_style,
            header_align,
        ),
        fb,
    );

    // READ CURRENT TIMEOUT VALUE (0 – 300 SECONDS)
    let timeout: u32 = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
    let timeout_clamped = timeout.clamp(0, 300);
    // MAP TO 0–100 PERCENT FOR ARC (0 sec → 0%, 300 sec → 100%)
    let percent: u8 = ((timeout_clamped as u64 * 100 / 300) as u8).min(100);

    // ARC GEOMETRY
    let min_dim = if W < H { W } else { H } as u32;
    let diameter = min_dim * 7 / 10;
    let center_x = W / 2;
    let center_y = H / 2;
    let top_left = embedded_graphics::prelude::Point::new(
        center_x - diameter as i32 / 2,
        center_y - diameter as i32 / 2,
    );
    let stroke_width = 5u32;

    // BACKGROUND ARC
    let bg_arc = embedded_graphics::primitives::Arc::new(
        top_left,
        diameter,
        embedded_graphics::geometry::Angle::from_degrees(270.0),
        embedded_graphics::geometry::Angle::from_degrees(360.0),
    );
    let _ = embedded_graphics::Drawable::draw(
        &bg_arc.into_styled(
            embedded_graphics::primitives::PrimitiveStyleBuilder::new()
                .stroke_color(crate::gui::colors::GRAY)
                .stroke_width(stroke_width)
                .stroke_alignment(
                    embedded_graphics::primitives::StrokeAlignment::Inside,
                )
                .build(),
        ),
        fb,
    );

    // FILL ARC – GRADIENT FROM RED (0 SEC) TO GREEN (300 SEC)
    let fill_color = crate::gui::colors::gradient_red_green(percent);
    if percent > 0 {
        let sweep_deg = -360.0 * percent as f32 / 100.0;
        let fill_arc = embedded_graphics::primitives::Arc::new(
            top_left,
            diameter,
            embedded_graphics::geometry::Angle::from_degrees(270.0),
            embedded_graphics::geometry::Angle::from_degrees(sweep_deg),
        );
        let _ = embedded_graphics::Drawable::draw(
            &fill_arc.into_styled(
                embedded_graphics::primitives::PrimitiveStyleBuilder::new()
                    .stroke_color(fill_color)
                    .stroke_width(stroke_width)
                    .stroke_alignment(
                        embedded_graphics::primitives::StrokeAlignment::Inside,
                    )
                    .build(),
            ),
            fb,
        );
    }

    // CENTER: SLEEP ICON + SECONDS VALUE
    // SLEEP ICON
    if let core::result::Result::Ok(icon_png) =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_SLEEP_PNG)
    {
        let img_w = icon_png.width() as i32;
        let img_h = icon_png.height() as i32;
        // SCALE TO FIT WITHIN THE ARC
        let max_icon_h = (diameter as f32 * 0.55) as i32;
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
                if let core::option::Option::Some(color) = icon_png.pixels()[idx] {
                    // ALWAYS WHITE (NO RED TINT NEEDED)
                    let raw: u16 = embedded_graphics_core::pixelcolor::IntoStorage::into_storage(color);
                    let px = x + sx * scale;
                    let py = y + sy * scale;
                    for dy in 0..scale {
                        let row = (py + dy) as usize;
                        if row >= screen_h {
                            break;
                        }
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

    // TEXT: "X SEC" (OR "OFF" IF 0)
    let rssi_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let rssi_style = embedded_ttf::FontTextStyleBuilder::new(rssi_font)
        .font_size(82)
        .text_color(crate::gui::colors::WHITE)
        .build();
    let rssi_text = format_timeout(timeout_clamped);
    let rssi_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::text::Text::with_text_style(
            &rssi_text,
            embedded_graphics::prelude::Point::new(center_x, center_y + 70),
            rssi_style,
            rssi_align,
        ),
        fb,
    );
}

// ───────────────────────────────────────────────────────────────────────
// FORMAT TIMEOUT HELPER
fn format_timeout(secs: u32) -> heapless::String<16> {
    let mut s = heapless::String::new();
    if secs == 0 {
        s.push_str("OFF").ok();
    } else {
        core::fmt::Write::write_fmt(&mut s, format_args!("{} sec", secs)).ok();
    }
    s
}

// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING – NOTHING TO TOGGLE, SO RETURNS NONE
pub fn handle_touch(_x: i32, _y: i32) -> Option<crate::gui::TouchAction> {
    None
}

// ───────────────────────────────────────────────────────────────────────
// HANDLE SWIPE – ADJUST POWER DOWN TIMEOUT
pub fn handle_swipe(
    direction: crate::components::ft3168::SwipeDirection,
    _start_x: u16,
    start_y: u16,
    _last_x: u16,
    last_y: u16,
) {
    match direction {
        crate::components::ft3168::SwipeDirection::Up
        | crate::components::ft3168::SwipeDirection::Down => {
            let raw_delta = last_y as i32 - start_y as i32;
            let value_change = -raw_delta; // UP = POSITIVE

            // 1 SECOND PER 1 PIXEL OF VERTICAL MOVEMENT (FULL‑SCREEN SWIPE ~240 SEC)
            let delta_val = value_change;

            if delta_val != 0 {
                let current: u32 = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                let new_val = (current as i32 + delta_val).clamp(0, 300) as u32;
                crate::store!(crate::state::POWERDOWN_TIMEOUT_SECS, new_val);
                defmt::info!("Power down timeout adjusted to {} sec", new_val);
            }
        }
        _ => {}
    }
}
