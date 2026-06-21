// GUI/TIME
// FULL-SCREEN DIGITAL CLOCK – 24H HH:MM, BOLD WHITE ON BLACK.
// DIGIT SIZES ARE COMPUTED DYNAMICALLY TO FIT THE SCREEN WITH 5% MARGIN.
// SEGMENTS ARE DRAWN DIRECTLY INTO THE RAW FRAMEBUFFER.
// QUICK ACTION CONTROL CENTER:
//   SWIPE DOWN FROM TOP OF THE DISPLAY
// "TINY" STATUS ICONS:
//   WI‑FI SIGNAL BARS (COLOR CODED & HIDDEN WHEN NOT CONNECTED)
//   BATTERY ICON (BATTERY LEVEL + CHARGING STATE + COLOR CODED)

use embedded_graphics::prelude::IntoStorage;

// ───────────────────────────────────────────────────────────
// STATUS ICON HEIGHT 
const ICON_HEIGHT: i32 = 16;

// ───────────────────────────────────────────────────────────
// SEGMENT DRAWING STRUCTS & FUNCTIONS
#[derive(Clone, Copy)]
struct SegmentRect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl SegmentRect {
    // DRAW THIS SEGMENT DIRECTLY INTO THE RAW BUFFER (WHITE)
    fn draw_raw(&self, dest: &mut [u16], screen_w: usize) {
        let x0 = self.x.max(0) as usize;
        let y0 = self.y.max(0) as usize;
        let w = self.w as usize;
        let h = self.h as usize;
        let white: u16 = 0xFFFF; // RGB565 white

        for row in 0..h {
            let y = y0 + row;
            if y >= (dest.len() / screen_w) { break; }
            let start = y * screen_w + x0;
            let end = start + w.min(screen_w - x0);
            dest[start..end].fill(white);
        }
    }
}

// BUILD THE SEGMENTS FOR A DIGIT WHOSE TOP‑LEFT CORNER IS `origin`
fn digit_segments(
    origin: embedded_graphics::geometry::Point,
    digit_w: i32,
    digit_h: i32,
    stroke: i32,
    gap: i32,
) -> [SegmentRect; 7] {
    let x = origin.x;
    let y = origin.y;
    let w = digit_w;
    let h = digit_h;
    let s = stroke;
    let g = gap;

    [
        // TOP
        SegmentRect { x: x + g, y, w: (w - g * 2) as u32, h: s as u32 },
        // MIDDLE
        SegmentRect { x: x + g, y: y + (h / 2) - (s / 2), w: (w - g * 2) as u32, h: s as u32 },
        // BOTTOM
        SegmentRect { x: x + g, y: y + h - s, w: (w - g * 2) as u32, h: s as u32 },
        // UPPER‑LEFT
        SegmentRect { x, y: y + g, w: s as u32, h: (h / 2 - g) as u32 },
        // UPPER‑RIGHT
        SegmentRect { x: x + w - s, y: y + g, w: s as u32, h: (h / 2 - g) as u32 },
        // LOWER‑LEFT
        SegmentRect { x, y: y + (h / 2) + g, w: s as u32, h: (h / 2 - g - s) as u32 },
        // LOWER‑RIGHT
        SegmentRect { x: x + w - s, y: y + (h / 2) + g, w: s as u32, h: (h / 2 - g - s) as u32 },
    ]
}

const DIGIT_PATTERNS: [[bool; 7]; 10] = [
    [true, false, true, true, true, true, true],     // 0
    [false, false, false, false, true, false, true], // 1
    [true, true, true, false, true, true, false],    // 2
    [true, true, true, false, true, false, true],    // 3
    [false, true, false, true, true, false, true],   // 4
    [true, true, true, true, false, false, true],    // 5
    [true, true, true, true, false, true, true],     // 6
    [true, false, false, false, true, false, true],  // 7
    [true, true, true, true, true, true, true],      // 8
    [true, true, true, true, true, false, true],     // 9
];

fn draw_digit(
    dest: &mut [u16],
    screen_w: usize,
    digit: u8,
    x: i32,
    y: i32,
    digit_w: i32,
    digit_h: i32,
    stroke: i32,
    gap: i32,
) {
    if digit > 9 { return; }
    let segs = digit_segments(
        embedded_graphics::geometry::Point::new(x, y),
        digit_w, digit_h, stroke, gap,
    );
    let pattern = DIGIT_PATTERNS[digit as usize];
    for (i, on) in pattern.iter().enumerate() {
        if *on {
            segs[i].draw_raw(dest, screen_w);
        }
    }
}

fn draw_colon(
    dest: &mut [u16],
    screen_w: usize,
    x: i32,
    y: i32,
    digit_h: i32,
    stroke: i32,
) {
    let half_h = digit_h / 2;
    let dot_w = stroke as u32;
    // UPPER DOT
    SegmentRect { x, y: y + half_h / 2, w: dot_w, h: dot_w }
        .draw_raw(dest, screen_w);
    // LOWER DOT
    SegmentRect { x, y: y + half_h + half_h / 2, w: dot_w, h: dot_w }
        .draw_raw(dest, screen_w);
}

// ───────────────────────────────────────────────────────────
// HELPER - DRAW A TINTED PNG ICON
fn draw_tinted_icon(
    dest: &mut [u16],
    screen_w: usize,
    screen_h: usize,
    icon_bytes: &[u8],
    target_color: u16,
    x: i32,
    y: i32,
    max_height: i32,
) {
    if max_height <= 0 {
        return;
    }

    if let core::result::Result::Ok(icon_png) = embedded_png::Png::load_from_bytes(icon_bytes) {
        let img_w = icon_png.width() as i32;
        let img_h = icon_png.height() as i32;
        if img_h == 0 || img_w == 0 {
            return;
        }

        // SCALE SO THAT ICONS HEIGHT = MAX_HEIGHT (UPSCALE)
        let scale = core::cmp::max(1, max_height / img_h.max(1));

        for sy in 0..img_h {
            let py = y + sy * scale;
            if py < 0 || py as usize >= screen_h { continue; }
            for sx in 0..img_w {
                let idx = (sy * img_w + sx) as usize;
                if let core::option::Option::Some(color) = icon_png.pixels()[idx] {
                    let pixel: u16 = embedded_graphics_core::pixelcolor::IntoStorage::into_storage(color);
                    if pixel != 0x0000 {
                        let px = x + sx * scale;
                        for dy in 0..scale {
                            let row = (py + dy) as usize;
                            if row >= screen_h { break; }
                            for dx in 0..scale {
                                let col = (px + dx) as usize;
                                if col < screen_w {
                                    dest[row * screen_w + col] = target_color;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ───────────────────────────────────────────────────────────
// PUBLIC DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    let w = crate::state::LCD_WIDTH as i32;
    let h = crate::state::LCD_HEIGHT as i32;
    let screen_w = w as usize;
    let screen_h = h as usize;

    // CLEAR
    fb.buffer_mut().fill(0x0000);

    // DYNAMIC SIZING (FIT 95% OF SCREEN WIDTH)
    let total_desired = (w as f32 * 0.95) as i32;
    let digit_w = total_desired * 2 / 9;
    let stroke = digit_w / 6;
    if stroke < 1 { return; }
    let gap = stroke / 2;
    let colon_w = stroke;
    let digit_h = digit_w * 2 - 10;
    if digit_h < 20 { return; }

    // POSITIONING
    let block_w = 4 * digit_w + 3 * colon_w + 4 * gap;
    let start_x = (w - block_w) / 2;

    // SHIFT CLOCK DOWN TO AVOID TOP STATUS ICONS
    const CLOCK_VERTICAL_OFFSET: i32 = 40;
    let start_y = ((h - digit_h) / 2 + CLOCK_VERTICAL_OFFSET)
        .min(h - digit_h)
        .max(0);

    // READ CURRENT TIME
    let maybe_time = critical_section::with(|cs| crate::state::CURRENT_TIME.borrow(cs).get());
    if let Some(dt) = maybe_time {
        let hh = dt.hours;
        let mm = dt.minutes;
        let h1 = hh / 10;
        let h2 = hh % 10;
        let m1 = mm / 10;
        let m2 = mm % 10;

        let dest = fb.buffer_mut();

        draw_digit(dest, screen_w, h1, start_x, start_y, digit_w, digit_h, stroke, gap);
        draw_digit(dest, screen_w, h2, start_x + digit_w + gap, start_y, digit_w, digit_h, stroke, gap);
        draw_colon(dest, screen_w, start_x + 2 * (digit_w + gap), start_y, digit_h, stroke);
        draw_digit(dest, screen_w, m1, start_x + 2 * (digit_w + gap) + colon_w + gap, start_y, digit_w, digit_h, stroke, gap);
        draw_digit(dest, screen_w, m2, start_x + 3 * (digit_w + gap) + colon_w + gap, start_y, digit_w, digit_h, stroke, gap);
    }

    // TOP STATUS ICONS
    let margin = 8;
    let battery_percent: u8 = crate::load!(crate::state::BATTERY_PERCENT);
    let battery_color: u16 = (if battery_percent >= 70 {
        crate::gui::colors::GREEN
    } else if battery_percent >= 50 {
        crate::gui::colors::YELLOW
    } else if battery_percent >= 30 {
        crate::gui::colors::ORANGE
    } else {
        crate::gui::colors::RED
    }).into_storage();

    // Choose the correct level icon, or the charging icon
    let battery_icon = if crate::load!(crate::state::BATTERY_USB_CONNECTED) {
        crate::base::assets::SETTINGS_BATTERY_CHARGING_PNG
    } else {
        match battery_percent {
            0..=9  => crate::base::assets::SETTINGS_BATTERY_WARNING_PNG, // lowest
            10..=29 => crate::base::assets::SETTINGS_BATTERY_EMPTY_PNG,
            30..=49 => crate::base::assets::SETTINGS_BATTERY_LOW_PNG,
            50..=69 => crate::base::assets::SETTINGS_BATTERY_MEDIUM_PNG,
            70..=89 => crate::base::assets::SETTINGS_BATTERY_HIGH_PNG,
            _       => crate::base::assets::SETTINGS_BATTERY_FULL_PNG,  // 90+
        }
    };

    // BAT ICON
    let bat_width = if let core::result::Result::Ok(png) = embedded_png::Png::load_from_bytes(battery_icon) {
        let img_w = png.width() as i32;
        let img_h = png.height() as i32;
        let scale = core::cmp::max(1, ICON_HEIGHT / img_h.max(1));
        img_w * scale
    } else { 0 };
    let bat_x = w - bat_width - margin;
    draw_tinted_icon(fb.buffer_mut(), screen_w, screen_h, battery_icon, battery_color, bat_x, margin, ICON_HEIGHT);


    // WIFI BARS (HIDE WHEN NOT CONNECTED)
    if crate::load!(crate::state::WIFI_CONNECTED) {
        // GET RAW RSSI (SIGNAL STRENGTH IN dBm, NEGATIVE VALUE)
        let rssi: i32 = crate::load!(crate::state::RSSI);

        // CHOOSE THE RIGHT ICON AND COLOR BASED ON RSSI RANGE
        let (wifi_icon, wifi_color) = match rssi {
            -90..=-80 => (crate::base::assets::SETTINGS_SIGNAL_VERY_LOW_PNG, crate::gui::colors::RED),
            -79..=-70 => (crate::base::assets::SETTINGS_SIGNAL_LOW_PNG,      crate::gui::colors::ORANGE),
            -69..=-60 => (crate::base::assets::SETTINGS_SIGNAL_MEDIUM_PNG,   crate::gui::colors::YELLOW),
            -59..=-50 => (crate::base::assets::SETTINGS_SIGNAL_HIGH_PNG,     crate::gui::colors::GREEN),
            _         => (crate::base::assets::SETTINGS_SIGNAL_FULL_PNG,     crate::gui::colors::GREEN),
        };

        // CALCULATE WIDTH FOR ICON (ADAPTIVE)
        let wifi_width = if let core::result::Result::Ok(png) = embedded_png::Png::load_from_bytes(wifi_icon) {
            let img_w = png.width() as i32;
            let img_h = png.height() as i32;
            let scale = core::cmp::max(1, ICON_HEIGHT / img_h.max(1));
            img_w * scale
        } else { 0 };

        // POSITION TO THE LEFT OF THE BATTERY ICON
        let wifi_x = bat_x - wifi_width - margin;

        // DRAW WITH THE PROPER COLOR
        draw_tinted_icon(
            fb.buffer_mut(),
            screen_w,
            screen_h,
            wifi_icon,
            wifi_color.into_storage(),
            wifi_x,
            margin,
            ICON_HEIGHT,
        );
    }

}
