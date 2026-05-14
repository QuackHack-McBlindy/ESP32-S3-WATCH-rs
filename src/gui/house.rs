// GUI/HOUSE
// SMART HOME GUI PAGE

use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;

static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 1]> = core::option::Option::None;

pub fn draw(fb: &mut impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb565>) {
    let x = 100;
    let y = 100;
    let width = 80;
    let height = 80;

    let rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(x, y),
        embedded_graphics::geometry::Size::new(width, height),
    );
    let style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::CSS_YELLOW)
        .build();
    let _ = rect.into_styled(style).draw(fb);

    let area = crate::gui::HitArea {
        x,
        y,
        width,
        height,
        action: crate::gui::TouchAction::ZigbeeToggleLights,
    };

    critical_section::with(|_cs| {
        let ptr = core::ptr::addr_of_mut!(HIT_AREAS);
        unsafe {
            ptr.write(core::option::Option::Some([area]));
        }
    });
}

pub fn handle_touch(x: i32, y: i32) -> core::option::Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| {
        let ptr = core::ptr::addr_of!(HIT_AREAS);
        unsafe {
            if let Some(areas) = ptr.read().as_ref() {
                for area in areas {
                    if crate::gui::hit_test(x, y, area) {
                        return core::option::Option::Some(area.action);
                    }
                }
            }
        }
        core::option::Option::None
    })
}
