// GUI/MOD

// ───────────────────────────────────────────────────────────────────────
// LOAD MODULES
pub mod colors;
pub mod pages;
pub mod apps;
pub mod time;
pub mod battery;
pub mod house;
pub mod media_player;
pub mod call;
pub mod text;
pub mod settings;

// ───────────────────────────────────────────────────────────────────────
// SHARED HELPERS

// DRAW A TEXT
// USAGE: crate::gui::draw_text(fb, x, y, font_size, string);
pub fn draw_text(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
    x: i32,
    y: i32,
    font_size: u32,
    text: &str,
) {
    let ttf_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();

    let ttf_style = embedded_ttf::FontTextStyleBuilder::new(ttf_font)
        .font_size(font_size)
        .text_color(crate::gui::colors::WHITE)
        .build();

    let text_pos = embedded_graphics::geometry::Point::new(x, y);
    let text_render = embedded_graphics::text::Text::new(text, text_pos, ttf_style);

    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&text_render, fb)
        .ok();
}

// HIT AREA (X, Y, WIDTH, HEIGHT MAP > ACTION)
pub struct HitArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub action: TouchAction,
}

#[derive(Clone, Copy, defmt::Format)]
pub enum TouchAction {
    None,
    // OPEN APPS
    OpenQwackify,
    OpenSettings,
    OpenApp3,
    OpenHouse,
    // CALL PAGE
    CallAccept,
    CallDecline,
    // HOUSE PAGE
    ZigbeeToggleLights,
    // MEDIA PLAYER PAGE
    MediaPrev,
    MediaPlayPause,
    MediaNext,
}

pub fn hit_test(x: i32, y: i32, area: &HitArea) -> bool {
    x >= area.x && x < area.x + area.width as i32 &&
    y >= area.y && y < area.y + area.height as i32
}
