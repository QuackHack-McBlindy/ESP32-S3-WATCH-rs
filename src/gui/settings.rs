// GUI/SETTINGS.RS
// CONTROL SETTINGS & OPTIONS / TOGGLE TASKS ON & OFF IN GUI

use core::ptr::addr_of_mut;


const W: i32 = crate::state::LCD_WIDTH as i32;
const H: i32 = crate::state::LCD_HEIGHT as i32;

const ROW_HEIGHT: i32 = 100;
const FONT_SIZE_LABEL: u32 = 38;
const FONT_SIZE_VALUE: u32 = 44;

// ───────────────────────────────────────────────────────────────────────
// ICON CACHE (TODO)
fn load_icons() -> SettingsIcons {
    SettingsIcons {
        alert_triangle: embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_ALERT_TRIANGLE_PNG).ok(),
        bluetooth:      embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_BLUETOOTH_PNG).ok(),
        sun:            embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_SUN_PNG).ok(),
        slash:          embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_SLASH_PNG).ok(),
        toggle_left:    embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_TOGGLE_LEFT_PNG).ok(),
        toggle_right:   embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_TOGGLE_RIGHT_PNG).ok(),
        mic:            embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_MIC_PNG).ok(),
        mic_off:        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_MIC_OFF_PNG).ok(),
        speaker:        embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_SPEAKER_PNG).ok(),
        volume:         embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_VOLUME_PNG).ok(),
        volume_1:       embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_VOLUME_1_PNG).ok(),
        volume_2:       embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_VOLUME_2_PNG).ok(),
        volume_x:       embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_VOLUME_X_PNG).ok(),
        wifi:           embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_WIFI_PNG).ok(),
        wifi_off:       embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_WIFI_OFF_PNG).ok(),
        globe:          embedded_png::Png::load_from_bytes(crate::base::assets::SETTINGS_GLOBE_PNG).ok(),
    }
}

struct SettingsIcons {
    alert_triangle: core::option::Option<embedded_png::Png>,
    bluetooth:      core::option::Option<embedded_png::Png>,
    sun:            core::option::Option<embedded_png::Png>,
    slash:          core::option::Option<embedded_png::Png>,
    toggle_left:    core::option::Option<embedded_png::Png>,
    toggle_right:   core::option::Option<embedded_png::Png>,
    mic:            core::option::Option<embedded_png::Png>,
    mic_off:        core::option::Option<embedded_png::Png>,
    speaker:        core::option::Option<embedded_png::Png>,
    volume:         core::option::Option<embedded_png::Png>,
    volume_1:       core::option::Option<embedded_png::Png>,
    volume_2:       core::option::Option<embedded_png::Png>,
    volume_x:       core::option::Option<embedded_png::Png>,
    wifi:           core::option::Option<embedded_png::Png>,
    wifi_off:       core::option::Option<embedded_png::Png>,
    globe:          core::option::Option<embedded_png::Png>,
}

// ───────────────────────────────────────────────────────────────────────
// SETTING DESC
struct SettingItem {
    name: &'static str,
    icon: fn(&SettingsIcons) -> &core::option::Option<embedded_png::Png>,
    kind: SettingKind,
}

#[derive(Clone, Copy)]
enum SettingKind {
    ToggleDisplay,
    CycleBrightness,
    CycleVolume,
    ToggleSpeakerMute,
    CycleMicGain,
    ToggleMicMute,
    // TOGGLEBLUETOOTH,
    WiFiInfo,
}

const SETTINGS: &[SettingItem] = &[
    SettingItem { name: "Brightness", icon: |i| &i.sun, kind: SettingKind::CycleBrightness },
    SettingItem { name: "Display",    icon: |i| &i.sun, kind: SettingKind::ToggleDisplay },
    SettingItem { name: "Speaker",    icon: |i| &i.speaker, kind: SettingKind::CycleVolume },
    SettingItem { name: "Spk Mute",   icon: |i| &i.speaker, kind: SettingKind::ToggleSpeakerMute },
    SettingItem { name: "Mic Gain",   icon: |i| &i.mic, kind: SettingKind::CycleMicGain },
    SettingItem { name: "Mic Mute",   icon: |i| &i.mic, kind: SettingKind::ToggleMicMute },
    // SETTINGITEM { NAME: "BLUETOOTH",  ICON: |I| &I.BLUETOOTH, KIND: SETTINGKIND::TOGGLEBLUETOOTH },
    SettingItem { name: "Wi‑Fi",      icon: |i| &i.wifi, kind: SettingKind::WiFiInfo },
];


// ───────────────────────────────────────────────────────────────────────
// SETTING TOGGLE
fn apply(kind: SettingKind) {
    match kind {
        SettingKind::ToggleDisplay => {
            let current = crate::load!(crate::state::DISPLAY_STATE);
            crate::store!(crate::state::DISPLAY_STATE, !current);
            if !current {
                crate::components::co5300::wake_up();
            }
        }
        SettingKind::CycleBrightness => {
            let current = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
            let next = match current {
                0..=24  => 25,
                25..=49 => 50,
                50..=74 => 75,
                _       => 100,
            };
            crate::store!(crate::state::DISPLAY_BRIGHTNESS, next);
        }
        SettingKind::CycleVolume => {
            let current = crate::load!(crate::state::SPEAKER_VOLUME);
            let next = if current < 33 { 33 } else if current < 66 { 66 } else if current < 100 { 100 } else { 0 };
            crate::store!(crate::state::SPEAKER_VOLUME, next);
        }
        SettingKind::ToggleSpeakerMute => {
            let muted = crate::load!(crate::state::SPEAKER_MUTED);
            crate::store!(crate::state::SPEAKER_MUTED, !muted);
        }
        SettingKind::CycleMicGain => {
            let current = crate::load!(crate::state::MIC_VOLUME);  // MICROPHONE GAIN
            let next = if current < 33 { 33 } else if current < 66 { 66 } else if current < 100 { 100 } else { 0 };
            crate::store!(crate::state::MIC_VOLUME, next);
        }
        SettingKind::ToggleMicMute => {
            let muted = crate::load!(crate::state::MIC_MUTED);
            crate::store!(crate::state::MIC_MUTED, !muted);
        }
        // SETTINGKIND::TOGGLEBLUETOOTH => {
        //     // REQUIRE BLUETOOTH STATE VARIABLE (NOT YET DEFINED)
        // }
        SettingKind::WiFiInfo => { /* DISPLAY ONLY */ }
    }
}

// ───────────────────────────────────────────────────────────────────────
// GET DISPLAY
fn value_info(kind: SettingKind) -> (&'static str, embedded_graphics::pixelcolor::Rgb565) {
    match kind {
        SettingKind::ToggleDisplay => {
            if crate::load!(crate::state::DISPLAY_STATE) { ("ON", crate::gui::colors::GREEN) }
            else { ("OFF", crate::gui::colors::RED) }
        }
        SettingKind::CycleBrightness => {
            let b = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
            static mut BUF: [u8; 4] = [0; 4];
            let s = unsafe { format_percent(&mut *addr_of_mut!(BUF), b) };
            (s, crate::gui::colors::grayscale(b))
        }
        SettingKind::CycleVolume => {
            let v = crate::load!(crate::state::SPEAKER_VOLUME);
            static mut BUF: [u8; 4] = [0; 4];
            let s = unsafe { format_percent(&mut *addr_of_mut!(BUF), v) };
            (s, crate::gui::colors::gradient_blue_red(v))
        }
        SettingKind::ToggleSpeakerMute => {
            if crate::load!(crate::state::SPEAKER_MUTED) { ("MUTED", crate::gui::colors::RED) }
            else { ("ON", crate::gui::colors::GREEN) }
        }
        SettingKind::ToggleMicMute => {
            if crate::load!(crate::state::MIC_MUTED) { ("MUTED", crate::gui::colors::RED) }
            else { ("ON", crate::gui::colors::GREEN) }
        }
        // SETTINGKIND::TOGGLEBLUETOOTH => {
        //
        // }
        SettingKind::CycleMicGain => {
            let g = crate::load!(crate::state::MIC_VOLUME);
            static mut BUF: [u8; 4] = [0; 4];
            let s = unsafe { format_percent(&mut *addr_of_mut!(BUF), g) };
            (s, crate::gui::colors::gradient_cyan_magenta(g))
        }
        SettingKind::WiFiInfo => {
            let rssi = crate::load!(crate::state::RSSI);
            static mut BUF: [u8; 16] = [0; 16];
            let s = unsafe { format_rssi(&mut *addr_of_mut!(BUF), rssi) };
            let color = if rssi > -70 { crate::gui::colors::GREEN }
                        else if rssi > -85 { crate::gui::colors::YELLOW }
                        else { crate::gui::colors::RED };
            (s, color)
        }
    }
}

fn format_percent<'a>(buf: &'a mut [u8; 4], pct: u8) -> &'a str {
    let mut pos = 0;
    if pct >= 100 {
        buf[pos] = b'1'; pos += 1;
        buf[pos] = b'0'; pos += 1;
        buf[pos] = b'0'; pos += 1;
    } else {
        let tens = pct / 10;
        let ones = pct % 10;
        if tens > 0 || pos > 0 {
            buf[pos] = b'0' + tens;
            pos += 1;
        }
        buf[pos] = b'0' + ones;
        pos += 1;
    }
    buf[pos] = b'%';
    pos += 1;
    core::str::from_utf8(&buf[..pos]).unwrap_or("?")
}

fn format_rssi<'a>(buf: &'a mut [u8; 16], rssi: i32) -> &'a str {
    let mut s: heapless::String<16> = heapless::String::new();
    core::fmt::Write::write_fmt(&mut s, format_args!("{} dBm", rssi)).ok();
    let bytes = s.as_bytes();
    let len = bytes.len().min(15);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    core::str::from_utf8(&buf[..len]).unwrap_or("? dBm")
}

// ───────────────────────────────────────────────────────────────────────
// DRAW ONE ICON SCALED
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
                        <embedded_graphics::Pixel<Rgb> as embedded_graphics::Drawable>::draw(&pixel, display)?;
                    }
                }
            }
        }
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────
// SCROLL STATE
struct ScrollState {
    offset: i32,
    target: i32,
}

static SCROLL: critical_section::Mutex<core::cell::RefCell<ScrollState>> =
    critical_section::Mutex::new(core::cell::RefCell::new(ScrollState {
        offset: 0,
        target: 0,
    }));


pub fn handle_swipe(dir: crate::components::ft3168::SwipeDirection) {
    let total_items = SETTINGS.len() as i32;
    let max_scroll = (total_items * ROW_HEIGHT - H).max(0);

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

// ───────────────────────────────────────────────────────────────────────
pub fn handle_touch(x: i32, y: i32) {
    let (offset, _) = critical_section::with(|cs| {
        let state = SCROLL.borrow_ref(cs);
        (state.offset, state.target)
    });

    let tapped_y = y + offset;
    let row_index = tapped_y / ROW_HEIGHT;
    if row_index >= 0 && row_index < SETTINGS.len() as i32 {
        let kind = SETTINGS[row_index as usize].kind;
        apply(kind);
    }
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

    let font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let icons = load_icons();

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
    let end_row = ((offset + H) / ROW_HEIGHT).min(SETTINGS.len() as i32 - 1);

    for i in start_row..=end_row {
        let idx = i as usize;
        let item = &SETTINGS[idx];
        let y_base = i * ROW_HEIGHT - offset;

        // ALTERNATE ROW BACKGROUND
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

        // ICON
        if let Some(icon) = (item.icon)(&icons).as_ref() {
            let scale = core::cmp::max(1, (ROW_HEIGHT - 20) / icon.height() as i32);
            let icon_x = 10;
            let icon_y = y_base + (ROW_HEIGHT - icon.height() as i32 * scale) / 2;
            draw_scaled_png(fb, icon, icon_x, icon_y, scale).ok();
        }

        // LABEL TEXT
        let label_style = embedded_ttf::FontTextStyleBuilder::new(font.clone())
            .font_size(FONT_SIZE_LABEL)
            .text_color(crate::gui::colors::WHITE)
            .build();
        let label_pos = Point::new(80, y_base + 10);
        let label_text = embedded_graphics::text::Text::new(item.name, label_pos, label_style);
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&label_text, fb)
        .ok();

        // VALUE BADGE
        let (val_text, val_color) = value_info(item.kind);
        let badge_w = 130;
        let badge_h = 60;
        let badge_x = W - badge_w - 10;
        let badge_y = y_base + (ROW_HEIGHT - badge_h) / 2;

        let badge_rect = embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            embedded_graphics::primitives::Rectangle::new(
                Point::new(badge_x, badge_y),
                Size::new(badge_w as u32, badge_h as u32),
            ),
            Size::new(8, 8),
        );
        let badge_styled = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
            badge_rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(val_color),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::RoundedRectangle,
            embedded_graphics::primitives::PrimitiveStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&badge_styled, fb)
        .ok();

        // TEXT IN BADGE
        let text_color = crate::gui::colors::readable_text_color(val_color);
        let value_style = embedded_ttf::FontTextStyleBuilder::new(font.clone())
            .font_size(FONT_SIZE_VALUE)
            .text_color(text_color)
            .build();
        let value_pos = Point::new(badge_x + 10, badge_y + 5);
        let value_text = embedded_graphics::text::Text::new(val_text, value_pos, value_style);
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&value_text, fb)
        .ok();
    }
}
