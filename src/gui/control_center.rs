// GUI/CONTROL_CENTER
// A PANEL THAT SLIDES DOWN FROM THE TOP, HOLDING QUICK‑ACCESS TOGGLES.


pub(crate) static PREV_OFFSET: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(i32::MIN);

static PANEL_BUFFER: critical_section::Mutex<core::cell::RefCell<Option<alloc::vec::Vec<u16>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

static PANEL_DIRTY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);

// SNAPSHOT OF THE FRAMEBUFFER ROWS THAT THE FULLY‑OPEN PANEL COVERS
// USED TO RESTORE THE BACKGROUND WHILE THE PANEL IS SLIDING.
static BACKGROUND_SAVE: critical_section::Mutex<core::cell::RefCell<Option<alloc::vec::Vec<u16>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

// TRUE WHEN WE NEED TO CAPTURE A FRESH BACKGROUND SNAPSHOT BEFORE DRAWING.
static NEED_BG_SNAPSHOT: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

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
static mut OVERLAY_HIT_AREAS: Option<[crate::gui::HitArea; 4]> = None;

// ───────────────────────────────────────────────────────────────────────
// HELPERS
pub fn panel_height() -> i32 {
    (crate::state::LCD_HEIGHT as i32) * 49 / 100 // 49% OF DISPLAY HEIGHT
}

pub fn open() {
    critical_section::with(|cs| {
        let mut ol = OVERLAY.borrow_ref_mut(cs);
        ol.target_offset = 0; // PANEL SLIDES DOWN TO COVER ~49% OF SCREEN
    });
    NEED_BG_SNAPSHOT.store(true, core::sync::atomic::Ordering::Release);
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
            crate::dirty!();
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

// RENDER THE ENTIRE CONTROL PANEL INTO `BUF` (MUST BE SCREEN_W × PANEL_H U16).
fn render_panel_to_buffer(buf: &mut [u16], screen_w: usize, panel_h: usize) {
    let w = screen_w as i32;
    let h = panel_h as i32;
    let bg_color: u16 = 0x39E7;
    let box_color: u16 = 0x0841;
    let glow_color: u16 = 0x0410;

    // CLEAR TO BACKGROUND
    for y in 0..panel_h {
        let row_start = y * screen_w;
        buf[row_start..row_start + screen_w].fill(bg_color);
    }

    // BOX GEOMETRY (RELATIVE TO PANEL)
    let box_margin = 12i32;
    let box_spacing = 8i32;
    let total_boxes = 4;
    let usable_w = w - box_margin * 2 - box_spacing * (total_boxes - 1);
    let box_w = usable_w / total_boxes;
    let box_h = h * 3 / 5;
    let box_y = (h - box_h) / 2; // CENTER VERTICALLY

    // DRAW BOXES
    for i in 0..total_boxes {
        let x0 = box_margin + i * (box_w + box_spacing);
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

    // PRE‑LOAD ICONS ONCE
    let power_icon =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_AIRPLANE_PNG).ok();
    let llghts_on_icon =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_LIGHTBULB_ON_PNG).ok();
    let llghts_off_icon =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_LIGHTBULB_OFF_PNG).ok();
    let phone_icon =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_LOCATE_PHONE_PNG).ok();
    let settings_icon =
        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_PNG).ok();

    let power_state: bool = crate::load!(crate::state::LOW_POWER_MODE);
    let lights_state: bool = crate::load!(crate::state::LIGHTS_STATE);

    // HELPER TO DRAW AN ICON INTO THE BUFFER
    let mut draw_icon = |png: &embedded_png::Png, box_index: i32, red_tint: bool| {
        let x0 = box_margin + box_index * (box_w + box_spacing);
        let icon_scale = 0.5;
        let icon_w = (png.width() as f32 * icon_scale) as i32;
        let icon_h = (png.height() as f32 * icon_scale) as i32;
        let icon_x = x0 + (box_w - icon_w) / 2;
        let icon_y = box_y + (box_h - icon_h) / 2;

        if red_tint {
            draw_scaled_png_raw_tinted(
                buf,
                png,
                icon_x,
                icon_y,
                icon_scale,
                screen_w,
                panel_h, // THE BUFFER DIMENSIONS
                0xF800,  // PURE RED
            );
        } else {
            draw_scaled_png_raw(buf, png, icon_x, icon_y, icon_scale, screen_w, panel_h);
        }
    };

    // BOX 1 (INDEX 0) – POWER STATE: TINT IF ACTIVE, OTHERWISE NORMAL
    if let Some(power) = &power_icon {
        draw_icon(power, 0, power_state);
    }

    // BOX 2 (INDEX 1) – LIGHTS: CHOOSE ON/OFF ICON, NEVER TINTED
    let lights_icon = if lights_state {
        llghts_on_icon.as_ref()
    } else {
        llghts_off_icon.as_ref()
    };
    if let Some(lights) = lights_icon {
        draw_icon(lights, 1, false);
    }

    // BOX 3 (INDEX 2) – PHONE: NO TINT, NO STATE CHANGE
    if let Some(phone) = &phone_icon {
        draw_icon(phone, 2, false);
    }

    // BOX 4 (INDEX 3)
    if let Some(settings) = &settings_icon {
        draw_icon(settings, 3, false);
    }

    // GLOW BORDER AT THE BOTTOM OF THE PANEL (LAST 4 ROWS)
    let glow_top = panel_h.saturating_sub(4);
    for y in glow_top..panel_h {
        let row_start = y * screen_w;
        buf[row_start..row_start + screen_w].fill(glow_color);
    }
}

fn get_panel_buffer(screen_w: usize, panel_h: usize) -> *const u16 {
    let mut need_render = PANEL_DIRTY.swap(false, core::sync::atomic::Ordering::AcqRel);

    critical_section::with(|cs| {
        let mut opt = PANEL_BUFFER.borrow_ref_mut(cs);
        if opt.is_none() {
            let mut buf = alloc::vec![0u16; screen_w * panel_h];
            render_panel_to_buffer(&mut buf, screen_w, panel_h);
            *opt = Some(buf);
            need_render = false;
        } else if need_render {
            let buf = opt.as_mut().unwrap();
            render_panel_to_buffer(buf, screen_w, panel_h);
        }
        opt.as_ref().unwrap().as_ptr()
    })
}

pub fn invalidate_panel() {
    PANEL_DIRTY.store(true, core::sync::atomic::Ordering::Release);
}

// ───────────────────────────────────────────────────────────────────────
// DRAW THE SLIDING PANEL ON TOP OF THE CURRENT SCREEN
pub fn draw_overlay(fb: &mut crate::components::framebuffer::Framebuffer, offset: i32) {
    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;
    let panel_h = panel_height() as usize;

    let top = offset;
    let bottom = top + panel_h as i32;
    if bottom <= 0 || top >= screen_h as i32 {
        return;
    }

    // BACKGROUND SNAPSHOT
    let need_snap = NEED_BG_SNAPSHOT.swap(false, core::sync::atomic::Ordering::AcqRel);
    let mut bg_save = critical_section::with(|cs| BACKGROUND_SAVE.borrow_ref_mut(cs).take());

    if need_snap || bg_save.is_none() {
        // CAPTURE THE ROWS THAT THE FULLY‑OPEN PANEL WOULD COVER (OFFSET = 0)
        let buf = fb.buffer();
        let mut save = alloc::vec![0u16; screen_w * panel_h];
        for row in 0..panel_h {
            let src = row * screen_w;
            let dst = row * screen_w;
            save[dst..dst + screen_w].copy_from_slice(&buf[src..src + screen_w]);
        }
        bg_save = Some(save);
    }

    let bg = bg_save.as_ref().unwrap();
    let prev_offset = PREV_OFFSET.load(core::sync::atomic::Ordering::Acquire);

    // RESTORE VACATED BACKGROUND
    if prev_offset != i32::MIN && prev_offset != offset {
        // THE PANEL MOVED; RESTORE THE ROWS THAT WERE PREVIOUSLY COVERED
        let old_top = prev_offset;
        let old_bottom = old_top + panel_h as i32;
        let restore_start = old_top.max(0) as usize;
        let restore_end = old_bottom.min(screen_h as i32) as usize;
        let buf = fb.buffer_mut();
        for row in restore_start..restore_end {
            let bg_row = row; // SNAPSHOT WAS TAKEN AT OFFSET=0, SO ROW MATCHES
            let src = bg_row * screen_w;
            let dst = row * screen_w;
            buf[dst..dst + screen_w].copy_from_slice(&bg[src..src + screen_w]);
        }
    }

    // UPDATE THE PREVIOUS OFFSET FOR THE NEXT FRAME
    PREV_OFFSET.store(offset, core::sync::atomic::Ordering::Release);

    // DRAW PANEL AT NEW POSITION
    let panel_ptr = get_panel_buffer(screen_w, panel_h);
    let panel = unsafe { core::slice::from_raw_parts(panel_ptr, screen_w * panel_h) };
    let buf = fb.buffer_mut();
    let panel_start_row = if top < 0 { (-top) as usize } else { 0 };
    let screen_start_row = top.max(0) as usize;
    let screen_end_row = bottom.min(screen_h as i32) as usize;
    let rows_to_copy = screen_end_row.saturating_sub(screen_start_row);

    for row in 0..rows_to_copy {
        let panel_row = panel_start_row + row;
        let screen_row = screen_start_row + row;
        let src = panel_row * screen_w;
        let dst = screen_row * screen_w;
        buf[dst..dst + screen_w].copy_from_slice(&panel[src..src + screen_w]);
    }

    // STASH SNAPSHOT FOR NEXT FRAME
    critical_section::with(|cs| BACKGROUND_SAVE.borrow_ref_mut(cs).replace(bg_save.unwrap()));

    // HIT AREAS
    let box_margin = 12i32;
    let box_spacing = 8i32;
    let total_boxes = 4;
    let w = screen_w as i32;
    let usable_w = w - box_margin * 2 - box_spacing * (total_boxes - 1);
    let box_w = usable_w / total_boxes;
    let box_h = (panel_h as i32) * 3 / 5;
    let box_y_in_panel = (panel_h as i32 - box_h) / 2;
    let box_y_screen = offset + box_y_in_panel;

    let actions = [
        crate::gui::TouchAction::ControlCenterBox1,
        crate::gui::TouchAction::ControlCenterBox2,
        crate::gui::TouchAction::ControlCenterBox3,
        crate::gui::TouchAction::ControlCenterBox4,
    ];
    let mut hit_areas = [
        crate::gui::HitArea {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            action: crate::gui::TouchAction::ControlCenterBox1,
        };
        4
    ];
    for i in 0..4 {
        let x0 = box_margin + i * (box_w + box_spacing);
        hit_areas[i as usize] = crate::gui::HitArea {
            x: x0,
            y: box_y_screen,
            width: box_w as u32,
            height: box_h as u32,
            action: actions[i as usize],
        };
    }
    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(OVERLAY_HIT_AREAS).write(Some(hit_areas));
    });
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
                let raw: u16 = embedded_graphics::prelude::IntoStorage::into_storage(color);
                dest[row * screen_w + col] = raw;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING
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
pub fn handle_touch(x: i32, y: i32) -> Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let Some(areas) = core::ptr::addr_of!(OVERLAY_HIT_AREAS)
            .read()
            .as_ref()
        {
            for area in areas {
                if crate::gui::hit_test(x, y, area) {
                    return Some(area.action);
                }
            }
        }
        None
    })
}
