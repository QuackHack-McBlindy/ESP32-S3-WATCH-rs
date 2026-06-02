// GUI/MEDIA_PLAYER
// DRAWS THE _QWACKIFY_ APPLICATION IN THE GUI
// WRITES PNG PIXELS DIRECTLY INTO THE RAW FRAMEBUFFER
// PREV/NEXT TRACK+PLAY/PAUSE CONTROL BUTTONS
// TRACK TITLE + PROGRESS BAR FOR CURRENTLY PLAYING SONG
// MARK CURRENT SONG WITH A HEART MOVING IT TO FAVOURITES PLAYLIST
// TRASHCAN BUTTON TO CLEAR CURRENT PLAYLIST (NO CONFIRMATION!)
// TAP THE QWACKIFY LOGO TO SPLIT THE MEDIA PLAYER INTO TWO PIECES THAT SLIDES APART AND REVEAL THE PLAYLIST VIEW


use embedded_graphics::prelude::IntoStorage;
use embedded_graphics::geometry::Dimensions;

// ───────────────────────────────────────────────────────────────────────
// HIT AREAS FOR TOUCH INPUT
static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 6]> = core::option::Option::None;

// ───────────────────────────────────────────────────────────
// SPLIT ANIMATION STATE
pub struct MediaSplit {
    pub target_offset: i32,
    pub current_offset: i32,
}

pub(crate) static MEDIA_SPLIT: critical_section::Mutex<core::cell::RefCell<MediaSplit>> =
    critical_section::Mutex::new(core::cell::RefCell::new(MediaSplit {
        target_offset: 0,
        current_offset: 0,
    }));

pub fn open_split() {
    critical_section::with(|cs| {
        // LET'S BE SPLITTIN' IN HIGH REFRESH RATE WE BE LOOPIN'
        crate::dirty_loop_on!();
        let mut split = MEDIA_SPLIT.borrow_ref_mut(cs);
        split.target_offset = crate::state::LCD_HEIGHT as i32 / 2;   // MOVE APART BY HALF SCREEN
    });
}

pub fn close_split() {
    critical_section::with(|cs| {
        // LET'S BE SPLITTIN' IN HIGH REFRESH RATE WE BE LOOPIN'        
        crate::dirty_loop_on!();
        let mut split = MEDIA_SPLIT.borrow_ref_mut(cs);
        split.target_offset = 0;
    });
}

pub fn animate_split(anim_speed: i32) {
    critical_section::with(|cs| {
        let mut split = MEDIA_SPLIT.borrow_ref_mut(cs);
        let diff = split.target_offset - split.current_offset;
        if diff != 0 {
            let step = diff.clamp(-anim_speed, anim_speed);
            split.current_offset += step;
            // ANIMATION COMPLETE - STOP THE DIRTY LOOP
            if split.current_offset == split.target_offset {
                crate::dirty_loop_off!();
            }
        }
    });
}

pub fn is_split_open() -> bool {
    critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset > 0)
}

// ───────────────────────────────────────────────────────────────────────
// PUBLIC DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    type Rgb = embedded_graphics::pixelcolor::Rgb565;
    type Point = embedded_graphics::geometry::Point;
    type Size = embedded_graphics::geometry::Size;

    let offset = critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset);

    // LOAD TTF FONTS
    // FETCH THE ALREADY PARSED FONT
    let bold_font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(crate::gui::ROBOTO_BOLD_FONT);
        (*ptr).as_ref().expect("FONT NOT INITIALISED").clone()
    });
    // TODO CACHE REGULAR FONT
    let regular_font = bold_font.clone();

    // COLOR CONSTANTS
    let white = crate::gui::colors::WHITE;
    let cyan = crate::gui::colors::CYAN;
    let gray = crate::gui::colors::GRAY;
    let dark_gray = crate::gui::colors::DARK_GRAY;

    // TEXT STYLES
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

    // LOAD PNG ICONS
    let prev_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PREVIOUS_PNG).ok();
    let play_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PLAY_PNG).ok();
    let pause_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_PAUSE_PNG).ok();
    let next_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_NEXT_PNG).ok();
    let album_png = embedded_png::Png::load_from_bytes(crate::base::assets::QWACKIFY_PNG).ok();
    let clear_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_CLEAR_PNG).ok();
    let heart_png = embedded_png::Png::load_from_bytes(crate::base::assets::MEDIA_HEART_PNG).ok();

    let is_playing = crate::load!(crate::state::MEDIA_IS_PLAYING);
    let play_pause_png = if is_playing { &pause_png } else { &play_png };

    if prev_png.is_none() || next_png.is_none() || play_pause_png.is_none()
        || clear_png.is_none() || heart_png.is_none() {
        return;
    }

    let prev = prev_png.as_ref().unwrap();
    let next = next_png.as_ref().unwrap();
    let play_pause = play_pause_png.as_ref().unwrap();
    let clear = clear_png.as_ref().unwrap();
    let heart = heart_png.as_ref().unwrap();

    // DYNAMIC DATA
    let total_duration = crate::load!(crate::applications::media_player::TRACK_DURATION_MS);
    let current_position = crate::load!(crate::applications::media_player::TRACK_POSITION_MS);
    let title_opt = crate::applications::media_player::current_track_title();

    // SCREEN DIMENSIONS
    let bbox = fb.bounding_box();
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;

    // SPLIT LINE IS THE EXACT VERTICAL CENTER
    let split_line_y = h / 2;

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    // CONTROL BUTTONS COORDINATE CALCULATION
    let scale: f32 = 0.5;
    let gap = 50;
    let prev_w = (prev.width() as f32 * scale) as i32;
    let play_w = (play_pause.width() as f32 * scale) as i32;
    let next_w = (next.width() as f32 * scale) as i32;

    let total_btn = prev_w + play_w + next_w + 2 * gap;
    let start_x = (w - total_btn) / 2;
    let base_btn_y = h - (prev.height() as f32 * scale) as i32 - 30;
    let btn_y = base_btn_y + offset;

    let prev_x = start_x;
    let play_x = prev_x + prev_w + gap;
    let next_x = play_x + play_w + gap;

    let btn_area_w = (prev.width() as f32 * scale) as u32;
    let btn_area_h = (prev.height() as f32 * scale) as u32;
    // ───────────────────────────────────────────────────────────────────────────

    // HEADER "QWACKIFY" 
    let header_text = embedded_graphics::text::Text::with_text_style(
        "QWACKIFY",
        Point::new(w / 2, 20 - offset),
        header_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&header_text, fb).ok();

    // ALBUM ART (RAW PIXEL WRITE)
    if let Some(album) = &album_png {
        let img_w = album.width() as i32;
        let img_h = album.height() as i32;
        let scaled_w = img_w;
        let scaled_h = img_h;
        let album_y_base = (h as f32 * 0.35) as i32 - scaled_h / 2;
        let album_y = album_y_base - offset;
        let x = w / 2 - scaled_w / 2;

        let scale = 1i32;
        let dest = fb.buffer_mut();
        for sy in 0..img_h {
            for sx in 0..img_w {
                let idx = (sy * img_w + sx) as usize;
                if let Some(color) = album.pixels()[idx] {
                    let raw: u16 = color.into_storage();
                    let px = x + sx * scale;
                    let py = album_y + sy * scale;
                    for dy in 0..scale {
                        let row = (py + dy) as usize;
                        if row >= screen_h { break; }
                        for dx in 0..scale {
                            let col = (px + dx) as usize;
                            if col < screen_w {
                                dest[row * screen_w + col] = raw;
                            }
                        }
                    }
                }
            }
        }

        // ───────────────────────────────────────────────────────────
        // SIDE BUTTONS (CLEAR PLAYLIST & HEART CURRENT TRACK)
        let side_scale: f32 = 0.5;
        let clear_w = (clear.width() as f32 * side_scale) as i32;
        let clear_h = (clear.height() as f32 * side_scale) as i32;
        let heart_w = (heart.width() as f32 * side_scale) as i32;
        let heart_h = (heart.height() as f32 * side_scale) as i32;

        // GAP BETWEEN BUTTONS
        let side_gap = 20;

        // CLEAR
        let clear_x = x - side_gap - clear_w;
        let clear_y = album_y + (img_h - clear_h) / 2;

        // HEART BUTTON
        let heart_x = x + img_w + side_gap;
        let heart_y = album_y + (img_h - heart_h) / 2;

        // DRAW THE CLEAR BUTTON
        draw_scaled_png_raw(fb.buffer_mut(), clear, clear_x, clear_y, side_scale, screen_w, screen_h);

        // ───────────────────────────────────────────────────────────
        // DRAW HEART BUTTON WITH TINT (RED IF LIKED)
        let is_liked = crate::load!(crate::state::MEDIA_IS_LIKED);
        let heart_tint = if is_liked { core::option::Option::Some(0xF800) } else { core::option::Option::None }; // 0xF800 = PURE RED IN RGB565
        draw_scaled_png_raw_tinted(
            fb.buffer_mut(),
            heart,
            heart_x,
            heart_y,
            side_scale,
            screen_w,
            screen_h,
            heart_tint,
        );

        // STORE HIT AREAS
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
            crate::gui::HitArea {
                x: clear_x,
                y: clear_y,
                width: clear_w as u32,
                height: clear_h as u32,
                action: crate::gui::TouchAction::MediaClear,
            },
            crate::gui::HitArea {
                x: heart_x,
                y: heart_y,
                width: heart_w as u32,
                height: heart_h as u32,
                action: crate::gui::TouchAction::MediaHeart,
            },
            crate::gui::HitArea {
                x,
                y: album_y,
                width: img_w as u32,
                height: img_h as u32,
                action: crate::gui::TouchAction::MediaSplitView,
            },
        ];
        critical_section::with(|_cs| unsafe {
            core::ptr::addr_of_mut!(HIT_AREAS).write(core::option::Option::Some(areas));
        });
    } else {
        let dummy = crate::gui::HitArea { x: 0, y: 0, width: 0, height: 0, action: crate::gui::TouchAction::MediaPrev };
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
            dummy,
            dummy,
            dummy,
        ];
        critical_section::with(|_cs| unsafe {
            core::ptr::addr_of_mut!(HIT_AREAS).write(core::option::Option::Some(areas));
        });
    }

    // TRACK TITLE (TEXT, BIG ENOUGH)
    if let Some(ref title) = title_opt {
        let display = if title.len() > 25 { &title[..25] } else { title };
        let title_text = embedded_graphics::text::Text::with_text_style(
            display,
            Point::new(w / 2, (h as f32 * 0.55) as i32 - offset),
            title_style,
            center_align,
        );
        <embedded_graphics::text::Text<
            embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
        > as embedded_graphics::Drawable>::draw(&title_text, fb).ok();
    }

    // PROGRESS BAR BACKGROUND – ANCHORED TO THE SPLIT LINE
    let bar_y = split_line_y + offset;
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
    > as embedded_graphics::Drawable>::draw(&bg_styled, fb).ok();

    // PROGRESS BAR FILL
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
        > as embedded_graphics::Drawable>::draw(&fill_styled, fb).ok();
    }

    // TIME LABELS (TEXT)
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
    > as embedded_graphics::Drawable>::draw(&cur_text, fb).ok();

    let tot_text = embedded_graphics::text::Text::with_text_style(
        &tot_str,
        Point::new(bar_x + bar_width, time_y),
        time_style,
        center_align,
    );
    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::Drawable>::draw(&tot_text, fb).ok();

    // DRAW CONTROL BUTTONS AT THE BOTTOM
    draw_scaled_png_raw(fb.buffer_mut(), prev, prev_x, btn_y, scale, screen_w, screen_h);
    draw_scaled_png_raw(fb.buffer_mut(), play_pause, play_x, btn_y, scale, screen_w, screen_h);
    draw_scaled_png_raw(fb.buffer_mut(), next, next_x, btn_y, scale, screen_w, screen_h);

    // SPLIT GLOW & PLAYLIST (ONLY WHEN APART)
    if offset > 0 {
        // TEAL GLOW LINES
        let teal_bright: u16 = 0x07FF; // BRIGHT TEAL
        let teal_dark: u16 = 0x020F;   // DARKER TEAL
        let glow_half_thickness = 2;   // 5 PIXEL TOTAL THICKNESS

        let top_edge_y = split_line_y - offset;
        let bottom_edge_y = split_line_y + offset;

        let draw_h_line = |buf: &mut [u16], y: i32, color: u16| {
            if y >= 0 && (y as usize) < screen_h {
                let row_start = (y as usize) * screen_w;
                for x in 0..screen_w {
                    buf[row_start + x] = color;
                }
            }
        };

        let dest = fb.buffer_mut();
        for i in -glow_half_thickness..=glow_half_thickness {
            let brightness = if i == 0 { teal_bright } else { teal_dark };
            draw_h_line(dest, top_edge_y + i, brightness);
            draw_h_line(dest, bottom_edge_y + i, brightness);
        }

        // PLAYLIST TEXT – POSITIONS SCALED WITH GAP SIZE
        if offset > 30 {
            let left_align = embedded_graphics::text::TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Left)
                .build();

            let playlist_font = bold_font.clone();
            let playlist_style = embedded_ttf::FontTextStyleBuilder::new(playlist_font)
                .font_size(54)
                .text_color(white)
                .build();

            let songs = [
                "duck-song1.mp3",
                "ducksong-2.mp3",
                "duck-song3.mp3",
                "duck-song4.mp3",
            ];

            let margin = 20;
            let available_height = 2 * offset - 2 * margin;
            let line_height = if songs.len() > 1 {
                available_height / (songs.len() as i32 - 1)
            } else {
                0
            };
            let start_y = top_edge_y + margin;

            for (i, song) in songs.iter().enumerate() {
                let y = start_y + i as i32 * line_height;
                let text = embedded_graphics::text::Text::with_text_style(
                    *song,
                    Point::new(20, y),
                    playlist_style.clone(),
                    left_align,
                );
                <embedded_graphics::text::Text<
                    embedded_ttf::FontTextStyle<Rgb>,
                > as embedded_graphics::Drawable>::draw(&text, fb).ok();
            }
        }
    }
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
            if let Some(color) = png.pixels()[idx] {
                let raw: u16 = color.into_storage();
                dest[row * screen_w + col] = raw;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING WITH TINT
fn draw_scaled_png_raw_tinted(
    dest: &mut [u16],
    png: &embedded_png::Png,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
    tint: core::option::Option<u16>,
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
            if let Some(color) = png.pixels()[idx] {
                let raw = if let core::option::Option::Some(t) = tint { t } else { color.into_storage() };
                dest[row * screen_w + col] = raw;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────
// TIME FORMATTING
fn format_time(ms: u32) -> heapless::String<16> {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let mut s = heapless::String::new();
    core::fmt::Write::write_fmt(&mut s, core::format_args!("{}:{:02}", mins, secs)).ok();
    s
}

// ───────────────────────────────────────────────────────────────────────
// TOUCH HANDLING
pub fn handle_touch(x: i32, y: i32) -> core::option::Option<crate::gui::TouchAction> {
    critical_section::with(|_cs| unsafe {
        if let core::option::Option::Some(areas) = core::ptr::addr_of!(HIT_AREAS).read().as_ref() {
            for area in areas {
                if crate::gui::hit_test(x, y, area) {
                    return core::option::Option::Some(area.action);
                }
            }
        }
        core::option::Option::None
    })
}
