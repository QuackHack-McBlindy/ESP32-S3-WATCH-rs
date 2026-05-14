// GUI/TIME
// FULL-SCREEN DIGITAL CLOCK – 24H HH:MM, BOLD WHITE ON BLACK.
// DIGIT SIZES ARE COMPUTED DYNAMICALLY TO FIT THE SCREEN WITH 5% MARGIN.

#[derive(Clone, Copy)]
struct SegmentRect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl SegmentRect {
    fn draw(
        &self,
        target: &mut impl embedded_graphics::draw_target::DrawTarget<
            Color = embedded_graphics::pixelcolor::Rgb565,
        >,
        color: embedded_graphics::pixelcolor::Rgb565,
    ) {
        let rect = embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(self.x, self.y),
            embedded_graphics::geometry::Size::new(self.w, self.h),
        );
        let styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
            rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(color),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::Rectangle,
            embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::prelude::Drawable>::draw(&styled, target)
        .ok();
    }
}

/// BUILD THE SEVEN SEGMENTS FOR A DIGIT WHOSE TOP‑LEFT CORNER IS `origin`.
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
        SegmentRect {
            x: x + g,
            y,
            w: (w - g * 2) as u32,
            h: s as u32,
        },
        // MIDDLE
        SegmentRect {
            x: x + g,
            y: y + (h / 2) - (s / 2),
            w: (w - g * 2) as u32,
            h: s as u32,
        },
        // BOTTOM
        SegmentRect {
            x: x + g,
            y: y + h - s,
            w: (w - g * 2) as u32,
            h: s as u32,
        },
        // UPPER‑LEFT
        SegmentRect {
            x,
            y: y + g,
            w: s as u32,
            h: (h / 2 - g) as u32,
        },
        // UPPER‑RIGHT
        SegmentRect {
            x: x + w - s,
            y: y + g,
            w: s as u32,
            h: (h / 2 - g) as u32,
        },
        // LOWER‑LEFT
        SegmentRect {
            x,
            y: y + (h / 2) + g,
            w: s as u32,
            h: (h / 2 - g - s) as u32,
        },
        // LOWER‑RIGHT
        SegmentRect {
            x: x + w - s,
            y: y + (h / 2) + g,
            w: s as u32,
            h: (h / 2 - g - s) as u32,
        },
    ]
}

const DIGIT_PATTERNS: [[bool; 7]; 10] = [
    [true, false, true, true, true, true, true], // 0
    [false, false, false, false, true, false, true], // 1
    [true, true, true, false, true, true, false], // 2
    [true, true, true, false, true, false, true], // 3
    [false, true, false, true, true, false, true], // 4
    [true, true, true, true, false, false, true], // 5
    [true, true, true, true, false, true, true], // 6
    [true, false, false, false, true, false, true], // 7
    [true, true, true, true, true, true, true], // 8
    [true, true, true, true, true, false, true], // 9
];

fn draw_digit(
    target: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
    digit: u8,
    x: i32,
    y: i32,
    digit_w: i32,
    digit_h: i32,
    stroke: i32,
    gap: i32,
) {
    if digit > 9 {
        return;
    }
    let segs = digit_segments(
        embedded_graphics::geometry::Point::new(x, y),
        digit_w,
        digit_h,
        stroke,
        gap,
    );
    let pattern = DIGIT_PATTERNS[digit as usize];
    for (i, on) in pattern.iter().enumerate() {
        if *on {
            segs[i].draw(
                target,
                embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255), // WHITE
            );
        }
    }
}

fn draw_colon(
    target: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
    x: i32,
    y: i32,
    digit_h: i32,
    stroke: i32,
) {
    let half_h = digit_h / 2;
    let dot = stroke as u32;
    let white = embedded_graphics::pixelcolor::Rgb565::new(255, 255, 255);

    // UPPER DOT
    let rect1 = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(x, y + half_h / 2),
        embedded_graphics::geometry::Size::new(dot, dot),
    );
    let styled1 = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        rect1,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(white),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled1, target)
    .ok();

    // LOWER DOT
    let rect2 = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(x, y + half_h + half_h / 2),
        embedded_graphics::geometry::Size::new(dot, dot),
    );
    let styled2 = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        rect2,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(white),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled2, target)
    .ok();
}

// PUBLIC DRAW FUNCTION
pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    let w = crate::state::LCD_WIDTH as i32;
    let h = crate::state::LCD_HEIGHT as i32;
    let black = embedded_graphics::pixelcolor::Rgb565::new(0, 0, 0);

    // CLEAR SCREEN
    let full_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::zero(),
        embedded_graphics::geometry::Size::new(w as u32, h as u32),
    );
    let styled_clear = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        full_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(black),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&styled_clear, fb)
    .ok();

    // DYNAMIC SIZING (FIT 95% OF SCREEN WIDTH)
    let total_desired = (w as f32 * 0.95) as i32;
    let digit_w = total_desired * 2 / 9;
    let stroke = digit_w / 6;
    if stroke < 1 {
        return;
    }
    let gap = stroke / 2;
    let colon_w = stroke;
    let digit_h = digit_w * 2 - 10;
    if digit_h < 20 {
        return;
    }

    // POSITIONING
    // BLOCK = H1 H2 : M1 M2
    let block_w = 4 * digit_w + 3 * colon_w + 4 * gap;
    let start_x = (w - block_w) / 2;
    let start_y = (h - digit_h) / 2;

    // READ CURRENT TIME
    let maybe_time = critical_section::with(|cs| crate::state::CURRENT_TIME.borrow(cs).get());
    if let Some(dt) = maybe_time {
        let hh = dt.hours;
        let mm = dt.minutes;

        let h1 = hh / 10;
        let h2 = hh % 10;
        let m1 = mm / 10;
        let m2 = mm % 10;

        draw_digit(fb, h1, start_x, start_y, digit_w, digit_h, stroke, gap);
        draw_digit(
            fb,
            h2,
            start_x + digit_w + gap,
            start_y,
            digit_w,
            digit_h,
            stroke,
            gap,
        );
        draw_colon(
            fb,
            start_x + 2 * (digit_w + gap),
            start_y,
            digit_h,
            stroke,
        );
        draw_digit(
            fb,
            m1,
            start_x + 2 * (digit_w + gap) + colon_w + gap,
            start_y,
            digit_w,
            digit_h,
            stroke,
            gap,
        );
        draw_digit(
            fb,
            m2,
            start_x + 3 * (digit_w + gap) + colon_w + gap,
            start_y,
            digit_w,
            digit_h,
            stroke,
            gap,
        );
    } else {
        // RTC NOT SET – SHOW NOTHING (BLACK SCREEN ALREADY DRAWN)
    }
}
