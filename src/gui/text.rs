// GUI/TEXT
// DISPLAYS A LARGE TEXT ON THE DISPLAY 
// THE STRING IN QUESTION IS PROVIDED BY THE TEXT API ENDPOINT 


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


    let string = critical_section::with(|cs| { crate::state::DISPLAY_STRING.borrow(cs).borrow().clone() });        
    if let Some(string_str) = string.as_ref() { crate::gui::draw_text(fb, 150, 150, 106, string_str); }

}
