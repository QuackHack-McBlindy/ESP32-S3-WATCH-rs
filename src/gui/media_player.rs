// GUI/MEDIA_PLAYER
// DRAWS THE QWACKIFY APPLICATION IN THE GUI

static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 3]> = core::option::Option::None;

pub fn draw(
    fb: &mut impl embedded_graphics::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb565,
    >,
) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    // LOAD TTF FONT FROM ASSETS
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let regular_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_REGULAR).unwrap();

    // COLOR CONSTANTS (FROM CENTRALIZED MODULE)
    let white = crate::gui::colors::WHITE;
    let cyan = crate::gui::colors::CYAN;
    let gray = crate::gui::colors::GRAY;
    let dark_gray = crate::gui::colors::DARK_GRAY;

    // CREATE TEXT STYLES WITH DIFFERENT SIZES
    let header_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(62)
        .text_color(cyan)
        .build();

    let title_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(48)
        .text_color(white)
        .build();

    let time_style = embedded_ttf::FontTextStyleBuilder::new(regular_font)
        .font_size(16)
        .text_color(gray)
        .build();

    // LOAD PNGs FROM ASSETS
    let prev_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PREVIOUS_PNG).ok();
    let play_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PLAY_PNG).ok();
    let pause_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PAUSE_PNG).ok();
    let next_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_NEXT_PNG).ok();
    let album_png = embedded_png::Png::load_from_bytes(crate::base::assets::QWACKIFY_PNG).ok();

    let is_playing = crate::load!(crate::state::MEDIA_IS_PLAYING);
    let play_pause_png = if is_playing { &pause_png } else { &play_png };

    if prev_png.is_none() || next_png.is_none() || play_pause_png.is_none() {
        return;
    }

    let prev = prev_png.as_ref().unwrap();
    let next = next_png.as_ref().unwrap();
    let play_pause = play_pause_png.as_ref().unwrap();

    let total_duration = crate::load!(crate::applications::media_player::TRACK_DURATION_MS);
    let current_position = crate::load!(crate::applications::media_player::TRACK_POSITION_MS);
    let title_opt = crate::applications::media_player::current_track_title();

    // SCREEN
    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    // HEADER "QWACKIFY"
    let header_text = embedded_graphics::text::Text::with_text_style(
        "QWACKIFY",
        Point::new(w / 2, 20),
        header_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&header_text, fb)
        .ok();

    // QWACKIFY ICON (or album art if you wish)
    if let Some(album) = &album_png {
        let scale = 1;
        let img_w = album.width() as i32;
        let img_h = album.height() as i32;
        let scaled_w = img_w * scale;
        let scaled_h = img_h * scale;
        let center_y = 150;
        let x = w / 2 - scaled_w / 2;
        let y = center_y - scaled_h / 2;

        for sy in 0..img_h {
            for sx in 0..img_w {
                let idx = (sy * img_w + sx) as usize;
                if let Some(color) = album.pixels()[idx] {
                    let px = x + sx * scale;
                    let py = y + sy * scale;
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let pixel = embedded_graphics::Pixel(
                                Point::new(px + dx, py + dy),
                                color,
                            );
                            <embedded_graphics::Pixel<Rgb> as embedded_graphics::Drawable>::draw(
                                &pixel, fb,
                            )
                            .ok();
                        }
                    }
                }
            }
        }
    }

    // TRACK TITLE
    if let Some(ref title) = title_opt {
        let display = if title.len() > 25 { &title[..25] } else { title };
        let title_text = embedded_graphics::text::Text::with_text_style(
            display,
            Point::new(w / 2, 220),
            title_style,
            center_align,
        );
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::Drawable>::draw(&title_text, fb)
            .ok();
    }

    // PROGRESS BAR
    let bar_y = 310;
    let bar_height = 24;
    let bar_width = w - 40;
    let bar_x = 20;

    let bg_rect = embedded_graphics::primitives::Rectangle::new(
        Point::new(bar_x, bar_y),
        Size::new(bar_width as u32, bar_height as u32),
    );
    let bg_styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        bg_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(dark_gray),
    );
    <embedded_graphics::primitives::Styled<
        embedded_graphics::primitives::Rectangle,
        embedded_graphics::primitives::PrimitiveStyle<Rgb>,
    > as embedded_graphics::Drawable>::draw(&bg_styled, fb)
        .ok();

    let progress = if total_duration > 0 {
        (current_position as f32 / total_duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled_w = (bar_width as f32 * progress) as u32;
    if filled_w > 0 {
        let fill_rect = embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            embedded_graphics::primitives::Rectangle::new(
                Point::new(bar_x, bar_y),
                Size::new(filled_w, bar_height as u32),
            ),
            Size::new(6, 6),
        );
        let fill_styled = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
            fill_rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(cyan),
        );
        <embedded_graphics::primitives::Styled<
            embedded_graphics::primitives::RoundedRectangle,
            embedded_graphics::primitives::PrimitiveStyle<Rgb>,
        > as embedded_graphics::Drawable>::draw(&fill_styled, fb)
            .ok();
    }

    // TIME LABELS
    let time_y = bar_y + bar_height + 8;
    let cur_str = format_time(current_position);
    let tot_str = format_time(total_duration);

    let cur_text = embedded_graphics::text::Text::with_text_style(
        &cur_str,
        Point::new(bar_x, time_y),
        time_style.clone(),
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&cur_text, fb)
        .ok();

    let tot_text = embedded_graphics::text::Text::with_text_style(
        &tot_str,
        Point::new(bar_x + bar_width, time_y),
        time_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&tot_text, fb)
        .ok();

    // CONTROL BUTTONS
    let scale = 7;
    let gap = 50;
    let prev_w = prev.width() as i32 * scale;
    let play_w = play_pause.width() as i32 * scale;
    let next_w = next.width() as i32 * scale;

    let total_btn = prev_w + play_w + next_w + 2 * gap;
    let start_x = (w - total_btn) / 2;
    let btn_y = h - (prev.height() as i32 * scale) - 30;

    let prev_x = start_x;
    let play_x = prev_x + prev_w + gap;
    let next_x = play_x + play_w + gap;

    let btn_area_w = prev.width() as u32 * scale as u32;
    let btn_area_h = prev.height() as u32 * scale as u32;

    let areas = [
        crate::gui::HitArea {
            x: prev_x,
            y: btn_y,
            width: btn_area_w,
            height: btn_area_h,
            action: crate::gui::TouchAction::MediaPrev,
        },
        crate::gui::HitArea {
            x: play_x,
            y: btn_y,
            width: btn_area_w,
            height: btn_area_h,
            action: crate::gui::TouchAction::MediaPlayPause,
        },
        crate::gui::HitArea {
            x: next_x,
            y: btn_y,
            width: btn_area_w,
            height: btn_area_h,
            action: crate::gui::TouchAction::MediaNext,
        },
    ];
    critical_section::with(|_cs| unsafe {
        core::ptr::addr_of_mut!(HIT_AREAS).write(core::option::Option::Some(areas));
    });

    draw_scaled_png(fb, prev, prev_x, btn_y, scale).ok();
    draw_scaled_png(fb, play_pause, play_x, btn_y, scale).ok();
    draw_scaled_png(fb, next, next_x, btn_y, scale).ok();
}

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
                        <embedded_graphics::Pixel<Rgb> as embedded_graphics::Drawable>::draw(
                            &pixel, display,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn format_time(ms: u32) -> heapless::String<16> {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let mut s = heapless::String::new();
    core::fmt::Write::write_fmt(&mut s, format_args!("{}:{:02}", mins, secs)).ok();
    s
}

pub fn handle_touch(x: i32, y: i32) -> core::option::Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let core::option::Option::Some(areas) =
            core::ptr::addr_of!(HIT_AREAS).read().as_ref()
        {
            for area in areas {
                if crate::gui::hit_test(x, y, area) {
                    return core::option::Option::Some(area.action);
                }
            }
        }
        core::option::Option::None
    })
}
