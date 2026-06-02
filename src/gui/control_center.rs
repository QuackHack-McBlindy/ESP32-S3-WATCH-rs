// GUI/CONTROL_CENTER
// A PANEL THAT SLIDES DOWN FROM THE TOP, HOLDING QUICK‑ACCESS TOGGLES.

use embedded_graphics::geometry::Dimensions;
use embedded_graphics::prelude::IntoStorage;

use crate::gui::{HitArea, TouchAction, hit_test};

#[derive(Clone, Copy)]
pub struct Overlay {
    pub target_offset: i32,
    pub current_offset: i32,
}

pub(crate) static OVERLAY: critical_section::Mutex<core::cell::RefCell<Overlay>> =
    critical_section::Mutex::new(core::cell::RefCell::new(Overlay {
        target_offset: -(crate::state::LCD_HEIGHT as i32), // HIDDEN
        current_offset: -(crate::state::LCD_HEIGHT as i32),
    }));

// STORAGE FOR THE HIT AREAS OF ALL FOUR BOXES
static mut OVERLAY_HIT_AREAS: Option<[HitArea; 4]> = None;


pub fn panel_height() -> i32 {
    (crate::state::LCD_HEIGHT as i32) * 35 / 100
}

// ───────────────────────────────────────────────────────────────────────
// HELPERS
pub fn open() {
    critical_section::with(|cs| {
        let mut ol = OVERLAY.borrow_ref_mut(cs);
        ol.target_offset = 0;   // PANEL SLIDES DOWN TO COVER ~49% OF SCREEN
    });
}

pub fn close() {
    critical_section::with(|cs| {
        let mut ol = OVERLAY.borrow_ref_mut(cs);
        // 1% OF SCREEN HEIGHT
        let peek = (crate::state::LCD_HEIGHT as i32) / 100;
        ol.target_offset = -panel_height() + peek;
    });
}

pub fn animate(anim_speed: i32) {
    critical_section::with(|cs| {
        let mut ol = OVERLAY.borrow_ref_mut(cs);
        let diff = ol.target_offset - ol.current_offset;
        if diff != 0 {
            let step = diff.clamp(-anim_speed, anim_speed);
            ol.current_offset += step;
        }
    });
}

pub fn is_visible() -> bool {
    critical_section::with(|cs| {
        let ol = OVERLAY.borrow_ref(cs);
        // VISIBLE ONLY WHEN MORE THAN 1% IS ON THE SCREEN
        ol.current_offset > -panel_height() + (crate::state::LCD_HEIGHT as i32) / 100
    })
}

// ───────────────────────────────────────────────────────────────────────
// DRAW THE SLIDING PANEL ON TOP OF THE CURRENT SCREEN
pub fn draw_overlay(
    fb: &mut crate::components::framebuffer::Framebuffer,
    offset: i32,
) {
    // LOAD ALL ICONS
    let wifi_icon     = embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_WIFI_ON_PNG).ok();
    let mic_icon      = embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_MIC_ON_PNG).ok();
    let api_icon      = embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_API_PNG).ok();
    let settings_icon = embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_PNG).ok();

    let w = fb.bounding_box().size.width as i32;
    let h = fb.bounding_box().size.height as i32;
    let panel_h = h * 35 / 100; // 35% OF SCREEN HEIGHT

    let top_y = offset;
    let bottom_y = top_y + panel_h;

    if bottom_y <= 0 || top_y >= h {
        return;
    }

    let screen_w = w as usize;
    let buf = fb.buffer_mut();

    // BACKGROUND: DARK GRAY
    let bg_color: u16 = 0x39E7;
    for y in top_y.max(0)..bottom_y.min(h) {
        let row_start = (y as usize) * screen_w;
        for x in 0..screen_w {
            buf[row_start + x] = bg_color;
        }
    }

    // FOUR PLACEHOLDER BOXES
    let box_margin = 12i32;
    let box_spacing = 8i32;
    let total_boxes = 4;
    let usable_w = w - box_margin * 2 - box_spacing * (total_boxes - 1);
    let box_w = usable_w / total_boxes;
    // BOXES OCCUPY 60% OF PANEL HEIGHT
    let box_h = panel_h * 3 / 5;
    let box_y = top_y + (panel_h - box_h) / 2;
    // CLAMP TO SCREEN
    let box_y = box_y.max(0).min(h - box_h);
    // DARK BLUE‑GRAY FOR BOXES
    let box_color: u16 = 0x0841;


    let peek = h / 100; // 1% OF SCREEN HIEIGHT
    if offset > -panel_h + peek {
        // DRAW ALL BOX BACKGROUNDS FIRST
        for i in 0..total_boxes {
            let x0 = box_margin + i as i32 * (box_w + box_spacing);
            let x0 = x0.max(0);
            let x1 = (x0 + box_w).min(w);
            for y in box_y..(box_y + box_h) {
                if y < 0 || y >= h {
                    continue;
                }
                let row_start = (y as usize) * screen_w;
                for x in x0..x1 {
                    if x >= 0 && (x as usize) < screen_w {
                        buf[row_start + x as usize] = box_color;
                    }
                }
            }
        }

        // HELPER TO DRAW AN ICON INSIDE A BOX
        let draw_icon = |buf: &mut [u16], png: &embedded_png::Png, box_index: i32, red_tint: bool| {
            let x0 = box_margin + box_index * (box_w + box_spacing);
            let icon_scale = 0.5;
            let icon_w = (png.width() as f32 * icon_scale) as i32;
            let icon_h = (png.height() as f32 * icon_scale) as i32;
            let icon_x = x0 + (box_w - icon_w) / 2;
            let icon_y = box_y + (box_h - icon_h) / 2;

            if red_tint {
                draw_scaled_png_raw_tinted(
                    buf, png, icon_x, icon_y, icon_scale,
                    screen_w, h as usize,
                    0xF800, // PURE RED
                );
            } else {
                draw_scaled_png_raw(
                    buf, png, icon_x, icon_y, icon_scale,
                    screen_w, h as usize,
                );
            }
        };

        // READ CURRENT STATES
        let wifi_state: bool  = crate::load!(crate::state::WIFI_STATE);
        let voice_state: bool = crate::load!(crate::state::VOICE_STATE);
        let api_state: bool   = crate::load!(crate::state::API_STATE);

        // DRAW ICONS ON TOP OF THE BOXES
        // BOX 1 – WI‑FI (TURNS RED IF WIFI IS OFF)
        if let Some(wifi) = &wifi_icon {
            draw_icon(buf, wifi, 0, !wifi_state);
        }

        // BOX 2 – MIC (TURNS RED IF VOICE IS OFF)
        if let Some(mic) = &mic_icon {
            draw_icon(buf, mic, 1, !voice_state);
        }

        // BOX 3 – API (TURNS RED IF API IS OFF)
        if let Some(api) = &api_icon {
            draw_icon(buf, api, 2, !api_state);
        }

        // BOX 4 – SETTINGS (ENTER SETTINGS)
        if let Some(settings) = &settings_icon {
            draw_icon(buf, settings, 3, false);
        }
    }

    // REGISTER HIT AREAS FOR ALL FOUR BOXES
    let actions = [
        TouchAction::ControlCenterBox1,
        TouchAction::ControlCenterBox2,
        TouchAction::ControlCenterBox3,
        TouchAction::ControlCenterBox4,
    ];

    let mut hit_areas = [HitArea {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        action: TouchAction::ControlCenterBox1,
    }; 4];

    for i in 0..4 {
        let x0 = box_margin + i as i32 * (box_w + box_spacing);
        hit_areas[i] = HitArea {
            x: x0,
            y: box_y,
            width: box_w as u32,
            height: box_h as u32,
            action: actions[i],
        };
    }

    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(OVERLAY_HIT_AREAS).write(Some(hit_areas));
    });

    // RED GLOWING BORDER AT BOTTOM (NOW TEAL)
    let glow_bottom = bottom_y.min(h);
    // 4‑PIXEL THICK GLOW
    let glow_top = glow_bottom - 4i32;
    let glow_color: u16 = 0x0410; // TEAL
    for y in glow_top..glow_bottom {
        if y < 0 || y >= h {
            continue;
        }
        let row_start = (y as usize) * screen_w;
        for x in 0..screen_w {
            buf[row_start + x] = glow_color;
        }
    }
}


// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING (NO TINT)
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

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING WITH A SINGLE REPLACEMENT TINT
fn draw_scaled_png_raw_tinted(
    dest: &mut [u16],
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
    tint: u16,
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
            if let Some(_color) = png.pixels()[idx] {
                dest[row * screen_w + col] = tint;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING
pub fn handle_touch(x: i32, y: i32) -> Option<TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let Some(areas) = core::ptr::addr_of!(OVERLAY_HIT_AREAS).read().as_ref() {
            for area in areas {
                if hit_test(x, y, area) {
                    return Some(area.action);
                }
            }
        }
        None
    })
}
