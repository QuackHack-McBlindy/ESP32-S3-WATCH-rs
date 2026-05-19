// GUI/APPS
// FULL-SIZED APP LAUNCHER, ONE APP PER "PAGE" , SWIPE UP/DOWN SMOOTH SCROLL ANIMATION

const PAGE_HEIGHT: i32 = crate::state::LCD_HEIGHT as i32;  // 502

pub struct Launcher {
    pub scroll_offset: i32,
    pub target_scroll: i32,
}

static LAUNCHER: critical_section::Mutex<core::cell::RefCell<Launcher>> =
    critical_section::Mutex::new(core::cell::RefCell::new(Launcher {
        scroll_offset: 0,
        target_scroll: 0,
    }));


// SWIPE UP === NEXT APP, SWIPE DOWN === PREVIIOUS APP.
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

// TAP ANYWHERE ON THE APPS PAGE go LAUNCH THE CURRENTLY DISPLAYED APP!"
pub fn handle_tap() {
    let total = crate::applications::APPS.len();
    if total == 0 { return; }

    let idx = critical_section::with(|cs| {
        let launcher = LAUNCHER.borrow_ref(cs);
        (launcher.scroll_offset / PAGE_HEIGHT).clamp(0, (total - 1) as i32) as usize
    });
    let app = &crate::applications::APPS[idx];
    (app.launch)();
}

pub fn handle_double_tap(_x: u16, _y: u16) {
    crate::gui::apps::handle_tap();
}

pub fn draw(
    fb: &mut impl embedded_graphics_core::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    let total = crate::applications::APPS.len();
    if total == 0 { return; }

    let screen_w = crate::state::LCD_WIDTH as i32;  // 410
    let screen_h = crate::state::LCD_HEIGHT as i32; // 502

    // SMOOOOTH SCROLL ANIMATION
    let (offset, target) = critical_section::with(|cs| {
        let mut launcher = LAUNCHER.borrow_ref_mut(cs);
        let diff = launcher.target_scroll - launcher.scroll_offset;
        if diff.abs() > 2 {
            launcher.scroll_offset += diff / 3;
        } else {
            launcher.scroll_offset = launcher.target_scroll;
        }
        (launcher.scroll_offset, launcher.target_scroll)
    });

    // DETERMINE WHICH TWO PAGES MIGHT BE VISIBLE
    let current_page = offset / PAGE_HEIGHT;
    let progress = offset % PAGE_HEIGHT;  // 0 .. PAGE_HEIGHT-1
    let next_page = if progress == 0 { None } else { Some(current_page + 1) };
    let prev_page = if progress == 0 && offset > 0 { Some(current_page - 1) } else { None };

    // BACKGROUND: ALWAYS BLACK
    let bg_rect = embedded_graphics::primitives::Rectangle::new(
        Point::zero(),
        Size::new(screen_w as u32, screen_h as u32),
    );
    let bg_styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        bg_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(crate::gui::colors::BLACK),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&bg_styled, fb).ok();

    // HELPER TO DRAW ONE PAGE
    let mut draw_page = |page_idx: usize, base_y: i32| {
        let app = &crate::applications::APPS[page_idx];
        // DRAW ICON (maximized)
        if let Ok(png) = embedded_png::Png::load_from_bytes(app.icon) {
            let icon_w = png.width() as i32;
            let icon_h = png.height() as i32;
            // SCALE ICON TO FILL 90% OF SCREEN HEIGHT
            let target_h = (screen_h as f32 * 0.9) as i32;
            let scale = core::cmp::max(1, target_h / icon_h.max(1));
            let scaled_w = icon_w * scale;
            let scaled_h = icon_h * scale;
            let x = (screen_w - scaled_w) / 2;
            let y = base_y + (screen_h - scaled_h) / 2;  // vertically centered in page

            for sy in 0..icon_h {
                for sx in 0..icon_w {
                    let color = png.pixels()[(sy * png.width() as i32 + sx) as usize];
                    if let Some(c) = color {
                        let px = x + sx * scale;
                        let py = y + sy * scale;
                        for dy in 0..scale {
                            for dx in 0..scale {
                                let pixel = embedded_graphics::Pixel(Point::new(px + dx, py + dy), c);
                                <embedded_graphics::Pixel<Rgb> as embedded_graphics::Drawable>::draw(&pixel, fb).ok();
                            }
                        }
                    }
                }
            }
        }
        // APP NAME BELOW ICON
        let name_font = embedded_graphics::mono_font::MonoTextStyle::new(
            &embedded_graphics::mono_font::ascii::FONT_10X20,
            crate::gui::colors::WHITE,
        );
        let name_align = embedded_graphics::text::TextStyleBuilder::new()
            .alignment(embedded_graphics::text::Alignment::Center)
            .build();
        let name_y = base_y + screen_h * 4 / 5;
        let name_text = embedded_graphics::text::Text::with_text_style(
            app.name,
            Point::new(screen_w / 2, name_y),
            name_font,
            name_align,
        );
        <embedded_graphics::text::Text<'_, embedded_graphics::mono_font::MonoTextStyle<'_, Rgb>> as embedded_graphics::Drawable>::draw(&name_text, fb).ok();
    };

    // DRAW CURRENT PAGE (ALWAYS)
    draw_page(current_page as usize, -progress);

    // DRAW NEXT PAGE IF PARTIALLY VISIBLE
    if let Some(np) = next_page {
        if np < total as i32 {
            draw_page(np as usize, PAGE_HEIGHT - progress);
        }
    }
    // DRAW PREVIOUS PAGE IF PARTIALLY VISIBLE (rare due to snapping)
    if let Some(pp) = prev_page {
        draw_page(pp as usize, -PAGE_HEIGHT - progress);
    }
}
