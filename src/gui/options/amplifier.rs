// GUI/OPTIONS/AMPLIFIER
// DRAW A SETTINGS PAGE FOR TOGGLE AMPLIFIER ON/OFF
// BIG COLOR CODED TOGGLE SWITCH AND A STATE ICON

use embedded_graphics::prelude::Primitive;

// ───────────────────────────────────────────────────────────────────────
// HIT AREA
static mut HIT_AREA: Option<crate::gui::HitArea> = None;

pub fn handle_touch(x: i32, y: i32) -> Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let core::option::Option::Some(area) =
            core::ptr::addr_of!(HIT_AREA).read().as_ref()
        {
            if crate::gui::hit_test(x, y, area) {
                // TOGGLE AMPLIFIER STATE (ON/OFF)
                let is_on: bool = crate::load!(crate::state::AMPLIFIER_STATE);
                defmt::info!("Amplifier state: {}", !is_on);
                crate::store!(crate::state::AMPLIFIER_STATE, !is_on);
                return core::option::Option::Some(
                    crate::gui::TouchAction::SettingsToggleAmp,
                );
            }
        }
        None
    })
}

// ───────────────────────────────────────────────────────────────────────
// THE DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    let is_on: bool = crate::load!(crate::state::AMPLIFIER_STATE);

    let bbox = embedded_graphics::geometry::Dimensions::bounding_box(fb);
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = w as usize;
    let screen_h = h as usize;

    // CLEAR TO BLACK
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::zero(),
            embedded_graphics_core::geometry::Size::new(w as u32, h as u32),
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            crate::gui::colors::BLACK,
        )),
        fb,
    );

    // HEADER
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD)
        .unwrap();
    let header_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(86)
        .text_color(crate::gui::colors::CYAN)
        .build();
    let header_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::text::Text::with_text_style(
            "AMP",
            embedded_graphics::prelude::Point::new(w / 2, 20),
            header_style,
            header_align,
        ),
        fb,
    );

    // STATIC AMP ICON
    let icon_bytes = crate::base::assets::SETTINGS_AMP_PNG;

    if let core::result::Result::Ok(icon_png) =
        embedded_png::Png::load_from_bytes(icon_bytes)
    {
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
                if let core::option::Option::Some(color) =
                    icon_png.pixels()[idx]
                {
                    // IF STATE IS ON - KEEP ICON WHITE
                    let raw: u16 = if is_on {
                        embedded_graphics_core::pixelcolor::IntoStorage::into_storage(
                            color,
                        )
                    } else {
                        // OTHERWISE MAKE IT RED - FOR CLARITY
                        embedded_graphics_core::pixelcolor::IntoStorage::into_storage(
                            crate::gui::colors::RED,
                        )
                    };
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

    // TOGGLE SWITCH CENTERED AT BOTTOM 
    let track_w = 120i32;
    let track_h = 60i32;
    let bottom_margin = 40i32;
    let toggle_left = (w - track_w) / 2;
    let toggle_top = h - track_h - bottom_margin;

    draw_toggle_switch(
        fb,
        embedded_graphics::prelude::Point::new(toggle_left, toggle_top),
        if is_on { 1.0 } else { 0.0 },
    );

    let area = crate::gui::HitArea {
        x: toggle_left,
        y: toggle_top,
        width: track_w as u32,
        height: track_h as u32,
        action: crate::gui::TouchAction::SettingsToggle,
    };
    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(HIT_AREA).write(core::option::Option::Some(area));
    });
}

// ───────────────────────────────────────────────────────────────────────
// TOGGLE SWITCH DRAWING
pub fn draw_toggle_switch(
    fb: &mut crate::components::framebuffer::Framebuffer,
    top_left: embedded_graphics::prelude::Point,
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
    let track_fill =
        crate::gui::colors::blend(dark_gray, green, progress_u8);
    let glow_color =
        crate::gui::colors::blend(red, green, progress_u8);

    let glow_expand = 3i32;
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::new(
                track_left - glow_expand,
                track_top - glow_expand,
            ),
            embedded_graphics_core::geometry::Size::new(
                (track_w + 2 * glow_expand) as u32,
                (track_h + 2 * glow_expand) as u32,
            ),
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            glow_color,
        )),
        fb,
    );

    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::new(track_left, track_top),
            embedded_graphics_core::geometry::Size::new(
                track_w as u32,
                track_h as u32,
            ),
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            track_fill,
        )),
        fb,
    );

    let thumb_left_max = track_left + margin;
    let thumb_left_min =
        track_left + track_w - margin - thumb_diameter as i32;
    let thumb_center_x = thumb_left_max
        + (progress * (thumb_left_min - thumb_left_max) as f32) as i32
        + thumb_radius;
    let thumb_center_y = track_top + track_h / 2;

    let shadow_offset = 2i32;
    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Circle::new(
            embedded_graphics::prelude::Point::new(
                thumb_center_x + shadow_offset - thumb_radius,
                thumb_center_y + shadow_offset - thumb_radius,
            ),
            thumb_diameter,
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            dark_gray,
        )),
        fb,
    );

    let _ = embedded_graphics::Drawable::draw(
        &embedded_graphics::primitives::Circle::new(
            embedded_graphics::prelude::Point::new(
                thumb_center_x - thumb_radius,
                thumb_center_y - thumb_radius,
            ),
            thumb_diameter,
        )
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            white,
        )),
        fb,
    );
}
