// GUI/APPS
// BIG APP LAUNCHER:
// 1x1 APP ICON GRID, SWIPE UP/DOWN FOR SMOOTH SCROLLING TRANSITIONS BETWEEN APPLICATIONS

// ───────────────────────────────────────────────────────────────────────
// TRAITS
use embedded_graphics::Drawable;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::{Text, TextStyleBuilder, Alignment};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics_core::pixelcolor::IntoStorage;
use alloc::vec;

// ───────────────────────────────────────────────────────────────────────
// CONSTANTS
const PAGE_HEIGHT: i32 = crate::state::LCD_HEIGHT as i32;  // 502

// ───────────────────────────────────────────────────────────────────────
// LAUNCHER STATE
pub struct Launcher {
    pub scroll_offset: i32,
    pub target_scroll: i32,
}

pub(crate) static LAUNCHER: critical_section::Mutex<core::cell::RefCell<Launcher>> =
    critical_section::Mutex::new(core::cell::RefCell::new(Launcher {
        scroll_offset: 0,
        target_scroll: 0,
    }));

// ───────────────────────────────────────────────────────────────────────
// SLICE DRAW TARGET (USED ONLY DURING PRE‑RENDERING)
struct SliceDrawTarget<'a> {
    buf: &'a mut [u16],
    width: usize,
    height: usize,
}

impl<'a> embedded_graphics_core::draw_target::DrawTarget for SliceDrawTarget<'a> {
    type Color = embedded_graphics::pixelcolor::Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        for embedded_graphics_core::Pixel(coord, color) in pixels.into_iter() {
            let x = coord.x as usize;
            let y = coord.y as usize;
            if x < self.width && y < self.height {
                let raw: u16 = color.into_storage();
                self.buf[y * self.width + x] = raw;
            }
        }
        Ok(())
    }
}

impl<'a> embedded_graphics_core::geometry::OriginDimensions for SliceDrawTarget<'a> {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

// ───────────────────────────────────────────────────────────────────────
// PAGE CACHE (PRE‑RENDERED FULL‑SCREEN IMAGES)
static PAGE_CACHE: critical_section::Mutex<core::cell::RefCell<Option<alloc::vec::Vec<alloc::vec::Vec<u16>>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

fn get_page_cache() -> alloc::vec::Vec<alloc::vec::Vec<u16>> {
    critical_section::with(|cs| {
        let mut cache = PAGE_CACHE.borrow_ref_mut(cs);
        if let Some(ref c) = *cache {
            return c.clone();
        }

        let screen_w = crate::state::LCD_WIDTH as i32;
        let screen_h = crate::state::LCD_HEIGHT as i32;
        let buf_w = screen_w as usize;
        let buf_h = screen_h as usize;

        let mut pages = alloc::vec::Vec::with_capacity(crate::applications::APPS.len());
        for app in crate::applications::APPS.iter() {
            let mut buf = vec![0u16; buf_w * buf_h];

            // ICON (RAW PIXEL WRITE)
            if let Ok(png) = embedded_png::Png::load_from_bytes(app.icon) {
                let icon_w = png.width() as i32;
                let icon_h = png.height() as i32;
                let target_h = (screen_h as f32 * 0.9) as i32;
                let scale = core::cmp::max(1, target_h / icon_h.max(1));
                let scaled_w = icon_w * scale;
                let scaled_h = icon_h * scale;
                let x = (screen_w - scaled_w) / 2;
                let y = (screen_h - scaled_h) / 2;

                for sy in 0..icon_h {
                    for sx in 0..icon_w {
                        if let Some(color) = png.pixels()[(sy * png.width() as i32 + sx) as usize] {
                            let raw: u16 = color.into_storage();
                            let px = x + sx * scale;
                            let py = y + sy * scale;
                            for dy in 0..scale {
                                let row = (py + dy) as usize;
                                if row >= buf_h { break; }
                                for dx in 0..scale {
                                    let col = (px + dx) as usize;
                                    if col < buf_w {
                                        buf[row * buf_w + col] = raw;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // APP NAME (VIA SLICE DRAW TARGET)
            let mut slice_dt = SliceDrawTarget { buf: &mut buf, width: buf_w, height: buf_h };
            let name_font = embedded_graphics::mono_font::MonoTextStyle::new(
                &embedded_graphics::mono_font::ascii::FONT_10X20,
                crate::gui::colors::WHITE,
            );
            let name_align = embedded_graphics::text::TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Center)
                .build();
            let name_y = screen_h * 4 / 5;
            let _ = embedded_graphics::text::Text::with_text_style(
                app.name,
                embedded_graphics::geometry::Point::new(screen_w / 2, name_y),
                name_font,
                name_align,
            )
            .draw(&mut slice_dt);

            pages.push(buf);
        }

        *cache = Some(pages.clone());
        pages
    })
}

// ───────────────────────────────────────────────────────────────────────
// INPUT HANDLERS
// SWIPE UP === NEXT APP, SWIPE DOWN === PREVIOUS APP.
pub fn handle_swipe(dir: crate::components::ft3168::SwipeDirection) {
    let total = crate::applications::APPS.len();
    if total == 0 { return; }
    let max_scroll = (total - 1) as i32 * PAGE_HEIGHT;
    critical_section::with(|cs| {
        let mut launcher = LAUNCHER.borrow_ref_mut(cs);
        match dir {
            crate::components::ft3168::SwipeDirection::Up => {
                launcher.target_scroll = (launcher.target_scroll + PAGE_HEIGHT).min(max_scroll);
            }
            crate::components::ft3168::SwipeDirection::Down => {
                launcher.target_scroll = (launcher.target_scroll - PAGE_HEIGHT).max(0);
            }
            _ => {}
        }
    });
}

// SINGLE TAP DOES NOTHING - AVOIDS UNWANTED ACTIONS WHEN SCROLLING
pub fn handle_tap() {
    defmt::info!("👆");    
}

// DOUBLE-TAP ANYWHERE ON THE APPS PAGE > LAUNCH THE CURRENTLY DISPLAYED APP!
pub fn handle_double_tap(_x: u16, _y: u16) {
    defmt::info!("👆👆");
    let total = crate::applications::APPS.len();
    if total == 0 { return; }
    let idx = critical_section::with(|cs| {
        let launcher = LAUNCHER.borrow_ref(cs);
        (launcher.scroll_offset / PAGE_HEIGHT).clamp(0, (total - 1) as i32) as usize
    });
    let app = &crate::applications::APPS[idx];
    (app.launch)();
}

// ───────────────────────────────────────────────────────────────────────
// FRAME COMPOSITION (SCROLLING VIEW)
// COMPOSE THE CURRENT FRAME INTO `buf` USING THE CACHED PAGES.
// `scroll_offset` IS THE CURRENT SMOOTH‑ANIMATED OFFSET IN PIXELS.
pub fn compose(buf: &mut [u16], scroll_offset: i32) {
    let total = crate::applications::APPS.len();
    if total == 0 {
        buf.fill(0x0000);
        return;
    }

    let pages = get_page_cache();
    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;
    let page_h = screen_h as i32;

    buf.fill(0x0000); // CLEAR TO BLACK

    let current_page = scroll_offset / page_h;
    let progress = scroll_offset % page_h;

    // HELPER: COPY A RANGE OF ROWS FROM ONE BUFFER TO ANOTHER
    fn copy_rows(
        dest: &mut [u16],
        src: &[u16],
        src_y_start: usize,
        height: usize,
        dest_y_offset: usize,
        screen_w: usize,
        screen_h: usize,
    ) {
        for row in 0..height {
            let src_row = src_y_start + row;
            let dest_row = dest_y_offset + row;
            if src_row >= screen_h || dest_row >= screen_h { break; }
            let src_begin = src_row * screen_w;
            let src_end = src_begin + screen_w;
            let dest_begin = dest_row * screen_w;
            dest[dest_begin..dest_begin + screen_w].copy_from_slice(&src[src_begin..src_end]);
        }
    }

    // DRAW CURRENT PAGE (SHIFTED UP BY `progress`)
    if current_page >= 0 && (current_page as usize) < total {
        copy_rows(
            buf,
            &pages[current_page as usize],
            progress as usize,
            screen_h - progress as usize,
            0,
            screen_w,
            screen_h,
        );
    }

    // DRAW NEXT PAGE (APPEARS FROM BOTTOM) IF SCROLLING
    if progress > 0 && (current_page + 1) >= 0 && ((current_page + 1) as usize) < total {
        copy_rows(
            buf,
            &pages[(current_page + 1) as usize],
            0,
            progress as usize,
            screen_h - progress as usize,
            screen_w,
            screen_h,
        );
    }
}
