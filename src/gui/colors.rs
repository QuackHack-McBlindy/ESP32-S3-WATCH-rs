// GUI/COLORS
// COLOR DEFINITIONS & COLOR HELPERS
// GOT IT'S OWN MODULE FOR EASY QUICK ACCESS

// ───────────────────────────────────────────────────────────────────────
// GRADIENT HELPERS

// RED (0%) > GREEN (100%)
pub fn gradient_red_green(percent: u8) -> embedded_graphics::pixelcolor::Rgb565 {
    let r = ((100 - percent as u16) * 255 / 100) as u8;
    let g = (percent as u16 * 255 / 100) as u8;
    embedded_graphics::pixelcolor::Rgb565::new(r, g, 0)
}

// BLUE (0%) > RED (100%) – (GOOD FOR TEMPERATURES)
pub fn gradient_blue_red(percent: u8) -> embedded_graphics::pixelcolor::Rgb565 {
    let r = (percent as u16 * 255 / 100) as u8;
    let b = ((100 - percent as u16) * 255 / 100) as u8;
    embedded_graphics::pixelcolor::Rgb565::new(r, 0, b)
}

// CYAN (0%) > MAGENTA (100%) – (VIVID TRANSITIOS)
pub fn gradient_cyan_magenta(percent: u8) -> embedded_graphics::pixelcolor::Rgb565 {
    let r = (percent as u16 * 255 / 100) as u8;
    let g = ((100 - percent as u16) * 255 / 100) as u8;
    let b = 255;
    embedded_graphics::pixelcolor::Rgb565::new(r, g, b)
}

// GRAYSCALE – (EASY THEME INTEGRATION)
pub fn grayscale(percent: u8) -> embedded_graphics::pixelcolor::Rgb565 {
    let v = (percent as u16 * 255 / 100) as u8;
    embedded_graphics::pixelcolor::Rgb565::new(v, v, v)
}

// RAINBOW 
// RED > ORANGE > YELLOW > GREEN > BLUE > VIOLET
pub fn rainbow(progress: u8) -> embedded_graphics::pixelcolor::Rgb565 {
    let segment = progress as u16 * 6 / 256; // 0..5
    let remainder = (progress as u16 * 6 % 256) as u8;
    match segment {
        0 => embedded_graphics::pixelcolor::Rgb565::new(255, remainder, 0),
        1 => embedded_graphics::pixelcolor::Rgb565::new(255 - remainder, 255, 0),
        2 => embedded_graphics::pixelcolor::Rgb565::new(0, 255, remainder),
        3 => embedded_graphics::pixelcolor::Rgb565::new(0, 255 - remainder, 255),
        4 => embedded_graphics::pixelcolor::Rgb565::new(remainder, 0, 255),
        5 => embedded_graphics::pixelcolor::Rgb565::new(255, 0, 255 - remainder),
        _ => unreachable!(),
    }
}

// ───────────────────────────────────────────────────────────────────────

// BLEND 2 COLORS AT GIVEN RATIO
// 0 = COLOR_A
// 255 = COLOR_B
pub fn blend(
    color_a: embedded_graphics::pixelcolor::Rgb565,
    color_b: embedded_graphics::pixelcolor::Rgb565,
    ratio: u8,
) -> embedded_graphics::pixelcolor::Rgb565 {
    // Fully qualified RgbColor trait methods
    let r_a = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::r(&color_a);
    let g_a = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::g(&color_a);
    let b_a = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::b(&color_a);
    let r_b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::r(&color_b);
    let g_b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::g(&color_b);
    let b_b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::b(&color_b);

    let r = (r_a as u16 * (255 - ratio as u16) + r_b as u16 * ratio as u16) / 255;
    let g = (g_a as u16 * (255 - ratio as u16) + g_b as u16 * ratio as u16) / 255;
    let b = (b_a as u16 * (255 - ratio as u16) + b_b as u16 * ratio as u16) / 255;
    embedded_graphics::pixelcolor::Rgb565::new(r as u8, g as u8, b as u8)
}


// MAKE A COLOR BRIGHTER (INCREASE BY `amount` UP TO 255)
pub fn brighter(
    color: embedded_graphics::pixelcolor::Rgb565,
    amount: u8,
) -> embedded_graphics::pixelcolor::Rgb565 {
    let r = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::r(&color);
    let g = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::g(&color);
    let b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::b(&color);
    embedded_graphics::pixelcolor::Rgb565::new(
        r.saturating_add(amount),
        g.saturating_add(amount),
        b.saturating_add(amount),
    )
}

// MAKE A COLOR DARKER (REDUCE BY `amount`, FLOOR AT 0)
pub fn darker(
    color: embedded_graphics::pixelcolor::Rgb565,
    amount: u8,
) -> embedded_graphics::pixelcolor::Rgb565 {
    let r = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::r(&color);
    let g = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::g(&color);
    let b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::b(&color);
    embedded_graphics::pixelcolor::Rgb565::new(
        r.saturating_sub(amount),
        g.saturating_sub(amount),
        b.saturating_sub(amount),
    )
}

// ───────────────────────────────────────────────────────────────────────
// ACCESSIBILITY HELPERS

// RELATIVE LUMINANCE (0..255) – USEFUL FOR CONTRAST CHECKS
pub fn luminance(color: embedded_graphics::pixelcolor::Rgb565) -> u8 {
    // STANDARD FORMULA: 0.2126*R + 0.7152*G + 0.0722*B
    let r = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::r(&color) as u16;
    let g = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::g(&color) as u16;
    let b = <embedded_graphics::pixelcolor::Rgb565 as embedded_graphics::pixelcolor::RgbColor>::b(&color) as u16;
    ((r * 54 + g * 183 + b * 18) / 255) as u8
}

// CHOOSE BLACK OR WHITE BASED ON BACKGROUND LUMINANCE (FOR READABLE TEXT)
pub fn readable_text_color(bg: embedded_graphics::pixelcolor::Rgb565) -> embedded_graphics::pixelcolor::Rgb565 {
    if luminance(bg) > 128 {
        BLACK
    } else {
        WHITE
    }
}

// ENSURE A FOREGROUND COLOR HAS ENOUGH CONTRAST AGAINST BACKGROUND
pub fn ensure_contrast(
    bg: embedded_graphics::pixelcolor::Rgb565,
    fg: embedded_graphics::pixelcolor::Rgb565,
    min_diff: u8,
) -> embedded_graphics::pixelcolor::Rgb565 {
    let bg_lum = luminance(bg);
    let fg_lum = luminance(fg);
    if (bg_lum as i16 - fg_lum as i16).abs() >= min_diff as i16 {
        fg
    } else {
        // FALLBACK: BLACK OR WHITE WHICHEVER GIVES HIGHER CONTRAST
        readable_text_color(bg)
    }
}


// ───────────────────────────────────────────────────────────────────────
// STATIC COLORS
pub const WHITE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255);
pub const BLACK: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 0, 0);
pub const GRAY: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0x80, 0x80, 0x80);
pub const YELLOW: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0xFF, 0xFF, 0x00);
pub const CYAN: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0x00, 0xFF, 0xFF);
pub const RED: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 0, 0);
pub const GREEN: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 255, 0);
pub const BLUE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 0, 255);
pub const MAGENTA: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 0, 255);
pub const ORANGE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 165, 0);
pub const PURPLE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(128, 0, 128);
pub const LIME: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 255, 0);
pub const PINK: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 192, 203);
pub const TEAL: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 128, 128);
pub const NAVY: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(0, 0, 128);
pub const MAROON: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(128, 0, 0);
pub const OLIVE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(128, 128, 0);
pub const CORAL: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 127, 80);
pub const GOLD: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(255, 215, 0);
pub const SILVER: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(192, 192, 192);
pub const DARK_GRAY: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(64, 64, 64);
pub const LIGHT_GRAY: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(211, 211, 211);
pub const SKY_BLUE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(135, 206, 235);
pub const SALMON: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(250, 128, 114);
pub const TURQUOISE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(64, 224, 208);
pub const VIOLET: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(238, 130, 238);
pub const BROWN: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(139, 69, 19);
pub const BEIGE: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(245, 245, 220);
pub const CRIMSON: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(220, 20, 60);
pub const DARK_ORCHID: embedded_graphics::pixelcolor::Rgb565 =
    embedded_graphics::pixelcolor::Rgb565::new(153, 50, 204);
