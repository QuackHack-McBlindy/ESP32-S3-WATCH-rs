// GUI/INPUT
// DISPLAYS A BIG BOX WITH A WHITE TEXT INPUT FIELD AWAITING TEXT TO BE TYPED...
// ++ CANCEL & OK COLOR CODED BUTTONS INSIDE IT

// ───────────────────────────────────────────────────────────────────────
// CONFIGURABLE GAP BETWEEN THE TWO BUTTONS (IN PIXELS)

const BUTTON_GAP: i32 = 60;
const MAX_CHARS_PER_LINE: usize = 13;
const LABEL_FONT_SIZE: u32 = 82;

// ───────────────────────────────────────────────────────────────────────
// STATE
#[derive(Clone)]
pub struct InputState {
    pub prompt: heapless::String<64>,
    pub text: heapless::String<128>,
    pub cursor_pos: usize,
    pub callback: Option<fn(&str)>, // CALLED WHEN OK IS PRESSED
    pub previous_page: u8,
}

pub(crate) static INPUT_STATE: critical_section::Mutex<
    core::cell::RefCell<Option<InputState>>
> = critical_section::Mutex::new(core::cell::RefCell::new(None));

// ───────────────────────────────────────────────────────────────────────
// OPEN THE INPUT PAGE WITH CUSTOM PARAMETERS
pub fn open(
    prompt: &str,
    initial_text: &str,
    on_ok: fn(&str),
    return_page: crate::gui::pages::Page,
) {
    let mut state = InputState {
        prompt: heapless::String::<64>::new(),
        text: heapless::String::<128>::new(),
        cursor_pos: initial_text.len(),
        callback: Some(on_ok),
        previous_page: return_page.as_raw(),
    };
    state.prompt.push_str(prompt).ok();
    state.text.push_str(initial_text).ok();

    critical_section::with(|cs| {
        INPUT_STATE.borrow(cs).replace(Some(state));
    });
    crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::TextInput.as_raw());
    crate::dirty!();
}

// ───────────────────────────────────────────────────────────────────────
// DRAWING
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    // ENSURE WE ALWAYS HAVE A STATE
    let state = critical_section::with(|cs| {
        let mut opt = INPUT_STATE.borrow(cs).borrow_mut();
        if opt.is_none() {
            *opt = Some(InputState {
                prompt: heapless::String::<64>::new(),
                text: heapless::String::<128>::new(),
                cursor_pos: 0,
                callback: None,
                previous_page: crate::gui::pages::Page::Clock.as_raw(),
            });
        }
        opt.as_ref().cloned()
    });
    let state = state.unwrap();

    let screen_w = crate::state::LCD_WIDTH as i32;
    let screen_h = crate::state::LCD_HEIGHT as i32;

    // CLEAR TO BLACK
    fb.buffer_mut().fill(0x0000);

    // FONT
    let font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(crate::gui::ROBOTO_BOLD_FONT);
        (*ptr).as_ref().cloned()
    }).expect("FONT NOT INITIALISED");

    // PROMPT (TOP‑LEFT, LIGHT GREY)
    let prompt_text = if state.prompt.is_empty() { " " } else { &state.prompt };
    crate::gui::draw_text(fb, 15, 15, 82, prompt_text);

    // INPUT TEXT WITH WRAPPING EVERY 10 CHARACTERS
    let text = &state.text;
    let text_style = embedded_ttf::FontTextStyleBuilder::new(font.clone())
        .font_size(82)
        .text_color(crate::gui::colors::WHITE)
        .build();

    let base_x = 10;
    let base_y = 110;
    let line_height = 90;

    let mut cursor_x = base_x;
    let mut cursor_y = base_y;

    for (line_idx, line) in text.as_str().as_bytes().chunks(MAX_CHARS_PER_LINE).enumerate() {
        let line_str = core::str::from_utf8(line).unwrap_or("");
        let line_y = base_y + line_idx as i32 * line_height;
        if !line_str.is_empty() {
            let mut text_obj = embedded_graphics::text::Text::new(
                line_str,
                embedded_graphics::geometry::Point::new(base_x, line_y),
                text_style.clone(),
            );
            embedded_graphics::prelude::Drawable::draw(&mut text_obj, fb).ok();
        }

        // IF THE CURSOR LIES IN THIS LINE, COMPUTE ITS X POSITION
        let line_start = line_idx * MAX_CHARS_PER_LINE;
        let line_end = (line_start + MAX_CHARS_PER_LINE).min(text.len());
        if state.cursor_pos >= line_start && state.cursor_pos < line_end {
            let chars_in_this_line = state.cursor_pos - line_start;
            let part = &text[line_start..state.cursor_pos];
            cursor_x = base_x + approximate_text_width(part, 82);
            cursor_y = line_y;
        }
    }

    // IF CURSOR IS EXACTLY AT THE END
    if state.cursor_pos == text.len() && !text.is_empty() {
        let last_line_idx = (text.len().saturating_sub(1)) / MAX_CHARS_PER_LINE;
        cursor_y = base_y + last_line_idx as i32 * line_height;
        let chars_before = text.len() - (last_line_idx * MAX_CHARS_PER_LINE);
        let part = &text[last_line_idx * MAX_CHARS_PER_LINE..];
        cursor_x = base_x + approximate_text_width(part, 82);
    }

    // CURSOR (WHITE BAR)
    let mut cursor_rect = embedded_graphics::prelude::Primitive::into_styled(
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(cursor_x, cursor_y - 70),
            embedded_graphics::geometry::Size::new(5, 80),
        ),
        embedded_graphics::primitives::PrimitiveStyle::with_fill(crate::gui::colors::WHITE),
    );
    embedded_graphics::prelude::Drawable::draw(&mut cursor_rect, fb).ok();

    // BUTTONS (AT THE VERY BOTTOM, CENTERED LABELS)
    let btn_width = 160;
    let btn_height = 110;
    let total_btn_w = 2 * btn_width + BUTTON_GAP;
    let btn_start_x = (screen_w - total_btn_w) / 2;
    let btn_y = screen_h - btn_height - 10;

    let cancel_x = btn_start_x;
    let ok_x = cancel_x + btn_width + BUTTON_GAP;

    draw_button(fb, cancel_x, btn_y, btn_width, btn_height, "X", 95, 440, crate::gui::colors::RED, font.clone());
    draw_button(fb, ok_x, btn_y, btn_width, btn_height, "OK", 315, 450, crate::gui::colors::GREEN, font);
}


// ───────────────────────────────────────────────────────────────────────
// BUTTON DRAWING HELPER (CENTERED LABELS)
fn draw_button(
    fb: &mut crate::components::framebuffer::Framebuffer,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    labelx: i32,
    labely: i32,
    color: embedded_graphics::pixelcolor::Rgb565,
    font: rusttype::Font<'static>,
) {
    let style = embedded_graphics::primitives::PrimitiveStyle::with_fill(color);
    let mut button = embedded_graphics::prelude::Primitive::into_styled(
        embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::geometry::Point::new(x, y),
                embedded_graphics::geometry::Size::new(w as u32, h as u32),
            ),
            embedded_graphics::geometry::Size::new(42, 42),
        ),
        style,
    );
    embedded_graphics::prelude::Drawable::draw(&mut button, fb).ok();

    let text_style = embedded_ttf::FontTextStyleBuilder::new(font)
        .font_size(LABEL_FONT_SIZE)
        .text_color(crate::gui::colors::BLACK)
        .build();

    let mut text_obj = embedded_graphics::text::Text::with_alignment(
        label,
        embedded_graphics::geometry::Point::new(labelx, labely),
        text_style,
        embedded_graphics::text::Alignment::Center,
    );

    defmt::info!("Button '{}' at x={}, y={}", label, labelx, labely);
    embedded_graphics::prelude::Drawable::draw(&mut text_obj, fb).ok();
}

// ───────────────────────────────────────────────────────────────────────
// APPROXIMATE TEXT WIDTH
fn approximate_text_width(text: &str, font_size: u32) -> i32 {
    text.chars().count() as i32 * (font_size as f32 * 0.65) as i32
}


// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING – RETURNS TRUE IF THE TOUCH WAS CONSUMED
pub fn handle_touch(x: i32, y: i32) -> bool {
    let screen_w = crate::state::LCD_WIDTH as i32;
    let screen_h = crate::state::LCD_HEIGHT as i32;

    let btn_width: i32 = 160;
    let btn_height: i32 = 110;
    let total_btn_w = 2 * btn_width + BUTTON_GAP;
    let btn_start_x = (screen_w - total_btn_w) / 2;
    let btn_y = screen_h - btn_height - 10;
    let cancel_x = btn_start_x;
    let ok_x = cancel_x + btn_width + BUTTON_GAP;

    if x >= cancel_x && x < cancel_x + btn_width &&
       y >= btn_y && y < btn_y + btn_height {
        hit_cancel();
        return true;
    }
    if x >= ok_x && x < ok_x + btn_width &&
       y >= btn_y && y < btn_y + btn_height {
        hit_ok();
        return true;
    }
    false
}

pub fn hit_ok() {
    critical_section::with(|cs| {
        let state = INPUT_STATE.borrow(cs).borrow();
        if let Some(ref s) = *state {
            defmt::info!("Input OK: {}", s.text.as_str());
            if let Some(cb) = s.callback {
                cb(&s.text);
            }
            let prev_page = s.previous_page;
            drop(state);
            INPUT_STATE.borrow(cs).replace(None);
            crate::store!(crate::gui::pages::CURRENT_PAGE, prev_page);
            crate::dirty!();
        }
    });
}

pub fn hit_cancel() {
    critical_section::with(|cs| {
        let state = INPUT_STATE.borrow(cs).borrow();
        if let Some(ref s) = *state {
            let prev_page = s.previous_page;
            drop(state);
            INPUT_STATE.borrow(cs).replace(None);
            crate::store!(crate::gui::pages::CURRENT_PAGE, prev_page);
            crate::dirty!();
        }
    });
}

pub fn set_text(text: &str) {
    critical_section::with(|cs| {
        let mut opt = INPUT_STATE.borrow(cs).borrow_mut();
        if let Some(ref mut state) = *opt {
            state.text.clear();
            state.text.push_str(text).ok();
            state.cursor_pos = state.text.len();
            crate::dirty!();
        }
    });
}

pub fn push_char(c: char) {
    critical_section::with(|cs| {
        let mut opt = INPUT_STATE.borrow(cs).borrow_mut();
        if let Some(ref mut state) = *opt {
            if state.text.len() < state.text.capacity() {
                state.text.push(c).ok();
                state.cursor_pos = state.text.len();
                crate::dirty!();
            }
        }
    });
}
