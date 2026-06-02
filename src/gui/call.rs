// GUI/CALL
// SHOWS AN INCOMING CALL SCREEN WITH TWO BUTTONS (ACCEPT/DECLINE CALL)

use embedded_graphics::prelude::IntoStorage;
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Dimensions;

static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 2]> = core::option::Option::None;

pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    // SCREEN DIMENSIONS
    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = w as usize;
    let screen_h = h as usize;

    // CLEAR SCREEN TO BLACK
    fb.buffer_mut().fill(0x0000);

    // LOAD BUTTON PNG
    let accept_png = embedded_png::Png::load_from_bytes(crate::base::assets::CALL_ACCEPT_PNG).ok();
    let decline_png = embedded_png::Png::load_from_bytes(crate::base::assets::CALL_DECLINE_PNG).ok();

    if accept_png.is_none() || decline_png.is_none() {
        return;
    }

    let accept = accept_png.as_ref().unwrap();
    let decline = decline_png.as_ref().unwrap();

    // LOAD CALLER NAME (SET WITH API CALL)
    let name = critical_section::with(|cs| crate::state::CALLER_NAME.borrow(cs).borrow().clone());
    if let core::option::Option::Some(name_str) = name.as_ref() {
        crate::gui::draw_text(fb, 150, 150, 106, name_str);
    }

    // BUTTON LAYOUT
    let scale = 1;
    let gap = 50;

    let accept_w = accept.width() as i32 * scale;
    let decline_w = decline.width() as i32 * scale;

    let total_btn = accept_w + decline_w + gap;
    let start_x = (w - total_btn) / 2;
    let btn_y = h - (accept.height() as i32 * scale) - 30;

    let accept_x = start_x;
    let decline_x = accept_x + accept_w + gap;

    let btn_area_w = accept.width() as u32 * scale as u32;
    let btn_area_h = accept.height() as u32 * scale as u32;

    // STORE HIT AREAS FOR TOUCH
    let areas = [
        crate::gui::HitArea {
            x: accept_x,
            y: btn_y,
            width: btn_area_w,
            height: btn_area_h,
            action: crate::gui::TouchAction::CallAccept,
        },
        crate::gui::HitArea {
            x: decline_x,
            y: btn_y,
            width: btn_area_w,
            height: btn_area_h,
            action: crate::gui::TouchAction::CallDecline,
        },
    ];
    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(HIT_AREAS).write(core::option::Option::Some(areas));
    });

    // DRAW BUTTONS
    draw_scaled_png_raw(fb.buffer_mut(), accept, accept_x, btn_y, scale, screen_w, screen_h);
    draw_scaled_png_raw(fb.buffer_mut(), decline, decline_x, btn_y, scale, screen_w, screen_h);
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING
fn draw_scaled_png_raw(
    dest: &mut [u16],
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: i32,
    screen_w: usize,
    screen_h: usize,
) {
    let img_w = png.width() as i32;
    let img_h = png.height() as i32;

    for src_row in 0..img_h {
        for src_col in 0..img_w {
            let idx = (src_row * img_w + src_col) as usize;
            if let core::option::Option::Some(color) = png.pixels()[idx] {
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

// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING
pub fn handle_touch(x: i32, y: i32) -> core::option::Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let core::option::Option::Some(areas) =
            core::ptr::addr_of!(HIT_AREAS).read().as_ref()
        {
            for area in areas {
                if crate::gui::hit_test(x, y, area) {
                    return core::option::Option::Some(area.action);
                }
            }
        }
        core::option::Option::None
    })
}
