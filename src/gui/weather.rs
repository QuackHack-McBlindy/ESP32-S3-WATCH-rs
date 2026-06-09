// GUI/WEATHER
// DRAWS A CENTERED PNG WEATHER ICON AND A LARGE TEMPERATURE TEXT
// USES DATA FROM TINYWEATHER APP
// TAP CYCLES THROUGH: CURRENT WEATHER > TOMORROW > DAY AFTER TOMORROW > DAY AFTER THAT

use embedded_graphics::geometry::Dimensions;
extern crate alloc;

// ───────────────────────────────────────────────────────────────────────
// DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    let bold_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();
    let white = crate::gui::colors::WHITE;
    let red = crate::gui::colors::RED;

    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;

    // TRY TO GET WEATHER DATA
    let weather_opt = crate::applications::tinyweather::WEATHER
        .try_lock()
        .ok()
        .and_then(|guard| guard.clone());
    let day_index = crate::applications::tinyweather::WEATHER_DAY
        .try_lock()
        .map(|guard| *guard)
        .unwrap_or(0);

    if weather_opt.is_none() {
        return; // NO DATA YET
    }
    let weather = weather_opt.unwrap();

    // CHOOSE WHICH DAY TO DISPLAY: 0 = CURRENT, 1-3 = FORECAST DAYS
    let (code, temp, _desc) = if day_index == 0 {
        (
            weather.current_code.as_str(),
            weather.current_temp,
            weather.current_desc.as_str(),
        )
    } else if day_index <= 3 && weather.days[day_index - 1].is_some() {
        let day = weather.days[day_index - 1].as_ref().unwrap();
        (day.code.as_str(), day.maxtemp, day.desc.as_str())
    } else {
        return; // INVALID DAY
    };

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    // ───────────────────────────────────────────────────────────────────────
    // WEATHER ICON
    let png_bytes = crate::applications::tinyweather::weather_png(code)
        .unwrap_or(crate::base::assets::WEATHER_DRIZZLE_PNG);
    let png = embedded_png::Png::load_from_bytes(png_bytes).ok();
    if png.is_none() {
        return;
    }
    let png = png.unwrap();

    let scale: f32 = 1.25;
    let icon_w = (png.width() as f32 * scale) as i32;
    let icon_h = (png.height() as f32 * scale) as i32;
    let icon_x = (w - icon_w) / 2;
    let icon_y = (h - icon_h) / 2 - 60;

    draw_scaled_png_raw(fb.buffer_mut(), &png, icon_x, icon_y, scale, screen_w, screen_h);

    // ───────────────────────────────────────────────────────────────────────
    // BIG HEADER TEXT (WHAT DAY)
    let header_text = match day_index {
        0 => "NOW",
        1 => "TODAY",
        2 => "TOMORROW",
        3 => "IN 2 DAYS",
        _ => "?",
    };

    let header_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(78)
        .text_color(red)
        .build();

    let header = embedded_graphics::text::Text::with_text_style(
        header_text,
        embedded_graphics::prelude::Point::new(w / 2, 20),
        header_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&header, fb)
    .ok();

    // ───────────────────────────────────────────────────────────────────────
    // TEMPERATURE TEXT – DRAWN BELOW THE ICON
    let temperature_str = alloc::format!("{} °C", temp);

    let temp_style = embedded_ttf::FontTextStyleBuilder::new(bold_font)
        .font_size(86)
        .text_color(white)
        .build();

    let temp_text = embedded_graphics::text::Text::with_text_style(
        &temperature_str,
        embedded_graphics::prelude::Point::new(w / 2, icon_y + icon_h + 20),
        temp_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&temp_text, fb)
    .ok();
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING
fn draw_scaled_png_raw(
    dest: &mut [u16],
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
) {
    if scale <= 0.0 {
        return;
    }
    let src_w = png.width() as i32;
    let src_h = png.height() as i32;
    let dst_w = (src_w as f32 * scale) as i32;
    let dst_h = (src_h as f32 * scale) as i32;

    for dst_row in 0..dst_h {
        let src_row = ((dst_row as f32 / scale) as i32).clamp(0, src_h - 1);
        let row = (y + dst_row) as usize;
        if row >= screen_h {
            break;
        }
        for dst_col in 0..dst_w {
            let src_col = ((dst_col as f32 / scale) as i32).clamp(0, src_w - 1);
            let col = (x + dst_col) as usize;
            if col >= screen_w {
                break;
            }
            let idx = (src_row * src_w + src_col) as usize;
            if let core::option::Option::Some(color) = png.pixels()[idx] {
                let raw: u16 = embedded_graphics::prelude::IntoStorage::into_storage(color);
                dest[row * screen_w + col] = raw;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING – CYCLES THROUGH FORECAST
pub fn handle_touch(_x: i32, _y: i32) -> core::option::Option<crate::gui::TouchAction> {
    if let core::result::Result::Ok(mut guard) =
        crate::applications::tinyweather::WEATHER_DAY.try_lock()
    {
        *guard = (*guard + 1) % 4;
    }
    crate::dirty!(); // REDRAW DISPLAY NOW
    defmt::info!("tinyWeather: changed day!");
    core::option::Option::Some(crate::gui::TouchAction::None)
}
