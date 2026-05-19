// GUI/CALL
// SHOWS AN INCOMING CALL SCREEN WITH TWO BUTTONS (ACCEPT/DECLINE CALL)


static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 2]> = core::option::Option::None;

pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;

    let bg_rect = embedded_graphics::primitives::Rectangle::new(
        Point::zero(),
        Size::new(bbox.size.width, bbox.size.height),
    );
    let bg_styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        bg_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(crate::gui::colors::BLACK),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&bg_styled, fb)
        .ok();

    let accept_png = embedded_png::Png::load_from_bytes(crate::base::assets::CALL_ACCEPT_PNG).ok();
    let decline_png = embedded_png::Png::load_from_bytes(crate::base::assets::CALL_DECLINE_PNG).ok();
    let play_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PLAY_PNG).ok();

    if accept_png.is_none() || decline_png.is_none() || play_png.is_none() {
        return;
    }

    let accept = accept_png.as_ref().unwrap();
    let decline = decline_png.as_ref().unwrap();
    let play = play_png.as_ref().unwrap();

    let name = critical_section::with(|cs| { crate::state::CALLER_NAME.borrow(cs).borrow().clone() });        
    if let Some(name_str) = name.as_ref() { crate::gui::draw_text(fb, 150, 150, 106, name_str); }

    let scale = 1;
    let gap = 50;

    let accept_w = accept.width() as i32 * scale;
    let play_w = play.width() as i32 * scale;
    let decline_w = decline.width() as i32 * scale;

    let total_btn = accept_w + play_w + decline_w + 2 * gap;
    let start_x = (w - total_btn) / 2;
    let btn_y = h - (accept.height() as i32 * scale) - 30;

    let accept_x = start_x;
    let play_x = accept_x + accept_w + gap;
    let decline_x = play_x + play_w + gap;


    let btn_area_w = accept.width() as u32 * scale as u32;
    let btn_area_h = accept.height() as u32 * scale as u32;

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

    draw_scaled_png(fb, accept, accept_x, btn_y, scale).ok();
    draw_scaled_png(fb, decline, decline_x, btn_y, scale).ok();
}

fn draw_scaled_png<D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
    display: &mut D,
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: i32,
) -> Result<(), D::Error> {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;

    for src_row in 0..png.height() {
        for src_col in 0..png.width() {
            let idx = (src_row * png.width() + src_col) as usize;
            if let Some(color) = png.pixels()[idx] {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let point = Point::new(
                            x + src_col as i32 * scale + dx,
                            y + src_row as i32 * scale + dy,
                        );
                        let pixel = embedded_graphics::Pixel(point, color);
                        <embedded_graphics::Pixel<Rgb> as embedded_graphics::Drawable>::draw(
                            &pixel, display,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

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
