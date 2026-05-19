// GUI/HOUSE
// BIG, ACCESSIBLE SMART HOME CONTROL – TAP A ROOM TO TOGGLE ALL LIGHTS
// (work in progresss...)

const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

const ROW_HEIGHT: i32 = 110;          // tall enough for easy tapping
const FONT_SIZE_ROOM: u32 = 48;       // big room name
const FONT_SIZE_LIGHTS: u32 = 32;     // smaller "Lights" label

// ───────────────────────────────────────────────────────────────────────
// ROOMS
const ROOMS: &[(&str, &str)] = &[
    ("Kitchen",    "kitchen"),
    ("Livingroom", "livingroom"),
    ("Hallway",    "hallway"),
    ("WC",         "wc"),
    ("Bedroom",    "bedroom"),
    ("TV Area",    "tv-area"),
    ("Other",      "other"),
];

// ───────────────────────────────────────────────────────────────────────
// SCROLL
struct ScrollState {
    offset: i32,
    target: i32,
}

static SCROLL: critical_section::Mutex<core::cell::RefCell<ScrollState>> =
    critical_section::Mutex::new(core::cell::RefCell::new(ScrollState {
        offset: 0,
        target: 0,
    }));


// SWIPE
pub fn handle_swipe(dir: crate::components::ft3168::SwipeDirection) {
    let total = ROOMS.len() as i32;
    let max_scroll = (total * ROW_HEIGHT - H).max(0);

    critical_section::with(|cs| {
        let mut state = SCROLL.borrow_ref_mut(cs);
        match dir {
            crate::components::ft3168::SwipeDirection::Up => {
                state.target = (state.target + ROW_HEIGHT).min(max_scroll);
            }
            crate::components::ft3168::SwipeDirection::Down => {
                state.target = (state.target - ROW_HEIGHT).max(0);
            }
            _ => {}
        }
    });
}


// TOUCH
pub fn handle_touch(x: i32, y: i32) {
    let (offset, _) = critical_section::with(|cs| {
        let state = SCROLL.borrow_ref(cs);
        (state.offset, state.target)
    });

    let tapped_y = y + offset;
    let row_index = tapped_y / ROW_HEIGHT;
    if row_index >= 0 && row_index < ROOMS.len() as i32 {
        let (display_name, room_id) = ROOMS[row_index as usize];
        toggle_room_lights(room_id);
        defmt::info!("Toggled lights in room: {}", display_name);
    }
}

// TODO
fn toggle_room_lights(room: &str) {
    defmt::info!("toggle_room_lights: {}", room);
}

// ───────────────────────────────────────────────────────────────────────
// MAIN DRAW
pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    // FONT
    let font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();

    // CLEAR SCREEN
    let full_rect = embedded_graphics::primitives::Rectangle::new(
        Point::zero(),
        Size::new(W as u32, H as u32),
    );
    let styled_clear = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        full_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(crate::gui::colors::BLACK),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&styled_clear, fb)
    .ok();

    // SMOOTH SCROLL ANIMATION
    let (offset, _) = critical_section::with(|cs| {
        let mut state = SCROLL.borrow_ref_mut(cs);
        let diff = state.target - state.offset;
        if diff.abs() > 2 {
            state.offset += diff / 3;
        } else {
            state.offset = state.target;
        }
        (state.offset, state.target)
    });

    let start_row = offset / ROW_HEIGHT;
    let end_row = ((offset + H) / ROW_HEIGHT).min(ROOMS.len() as i32 - 1);

    for i in start_row..=end_row {
        let idx = i as usize;
        let (display_name, _room_id) = ROOMS[idx];
        let y_base = i * ROW_HEIGHT - offset;

        // ALTERNATE BG
        let bg_color = if i % 2 == 0 {
            crate::gui::colors::DARK_GRAY
        } else {
            crate::gui::colors::BLACK
        };

        let bg_rect = embedded_graphics::primitives::Rectangle::new(
            Point::new(0, y_base),
            Size::new(W as u32, ROW_HEIGHT as u32),
        );
        let bg_styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
            bg_rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(bg_color),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Rectangle,
            embedded_graphics::primitives::PrimitiveStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&bg_styled, fb)
        .ok();

        // ROOM NAME
        let room_style = embedded_ttf::FontTextStyleBuilder::new(font.clone())
            .font_size(FONT_SIZE_ROOM)
            .text_color(crate::gui::colors::WHITE)
            .build();
        let room_pos = Point::new(20, y_base + 10);
        let room_text = embedded_graphics::text::Text::new(display_name, room_pos, room_style);
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&room_text, fb)
        .ok();

        // LABEL
        let lights_style = embedded_ttf::FontTextStyleBuilder::new(font.clone())
            .font_size(FONT_SIZE_LIGHTS)
            .text_color(crate::gui::colors::GRAY)
            .build();
        let lights_pos = Point::new(20, y_base + 55);
        let lights_text = embedded_graphics::text::Text::new("Lights", lights_pos, lights_style);
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&lights_text, fb)
        .ok();

        // IDICATOR TOGGLE
        let indicator_color = crate::gui::colors::CYAN;
        let indicator_rect = embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            embedded_graphics::primitives::Rectangle::new(
                Point::new(W - 50, y_base + 20),
                Size::new(30, 70),
            ),
            Size::new(6, 6),
        );
        let indicator_styled = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
            indicator_rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(indicator_color),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::RoundedRectangle,
            embedded_graphics::primitives::PrimitiveStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&indicator_styled, fb)
        .ok();
    }
}
