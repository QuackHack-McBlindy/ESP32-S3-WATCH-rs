// GUI/HOMESCREEN


pub fn draw(
    fb: &mut impl embedded_graphics_core::draw_target::DrawTarget<
        Color = embedded_graphics_core::pixelcolor::Rgb565,
    >,
) {
    let white = embedded_graphics_core::pixelcolor::Rgb565::new(255, 255, 255);
    let font = embedded_graphics::mono_font::ascii::FONT_10X20;
    let style = embedded_graphics::mono_font::MonoTextStyle::new(&font, white);
    let text = embedded_graphics::text::Text::new(
        "Homescreen",
        embedded_graphics_core::geometry::Point::new(10, 50),
        style,
    );
    <embedded_graphics::text::Text<
        embedded_graphics::mono_font::MonoTextStyle<embedded_graphics_core::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&text, fb)
    .ok();
}
