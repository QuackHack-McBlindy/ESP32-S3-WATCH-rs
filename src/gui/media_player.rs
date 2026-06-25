// GUI/MEDIA_PLAYER
// DRAWS THE _QWACKIFY_ APPLICATION IN THE GUI
// WRITES PNG PIXELS DIRECTLY INTO THE RAW FRAMEBUFFER
// PREV/NEXT TRACK+PLAY/PAUSE CONTROL BUTTONS
// TRACK TITLE + PROGRESS BAR FOR CURRENTLY PLAYING SONG
// MARK CURRENT SONG WITH A HEART MOVING IT TO FAVOURITES PLAYLIST
// TRASHCAN BUTTON TO CLEAR CURRENT PLAYLIST (NO CONFIRMATION!)
// TAP THE QWACKIFY LOGO TO SPLIT THE MEDIA PLAYER INTO TWO PIECES THAT SLIDES APART AND REVEAL THE PLAYLIST VIEW


// ───────────────────────────────────────────────────────────
// CONSTANTS
const PLAYLIST_BUFFER_HEIGHT: usize = 1000;
const PLAYLIST_LINE_HEIGHT: i32 = 70;
const PLAYLIST_MARGIN: i32 = 20;

// ───────────────────────────────────────────────────────────
// CACHED PIXEL DATA (DECODED ONCE IN init())
static mut PNG_PREV: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_PLAY: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_PAUSE: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_NEXT: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_CLEAR: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_HEART: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_HEART_FILLED: Option<(&'static [u16], u32, u32)> = None;
static mut PNG_ALBUM: Option<(&'static [u16], u32, u32)> = None;


// ───────────────────────────────────────────────────────────
// DECODE A PNG FROM BYTES, CONVERT TO RAW RGB565 PIXELS, AND LEAK THE BUFFER
// SO IT LIVES FOREVER. RETURNS (PIXEL SLICE, WIDTH, HEIGHT).
unsafe fn decode_and_leak(bytes: &[u8]) -> (&'static [u16], u32, u32) {
    let png = embedded_png::Png::load_from_bytes(bytes)
        .expect("PNG FAILED TO DECODE");
    let w = png.width();
    let h = png.height();
    let mut pixels: alloc::vec::Vec<u16> = alloc::vec::Vec::with_capacity((w * h) as usize);
    for pixel in png.pixels() {
        pixels.push(pixel.map(|c| embedded_graphics::pixelcolor::IntoStorage::into_storage(c)).unwrap_or(0));
    }
    let leaked = pixels.leak(); // 'STATIC LIFETIME
    (leaked, w, h)
}


// ───────────────────────────────────────────────────────────
// CACHE BUFFERS AND DIRTY FLAGS
static BASE_CACHE: critical_section::Mutex<core::cell::RefCell<Option<alloc::vec::Vec<u16>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
static BASE_DIRTY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);

static PLAYLIST_CACHE: critical_section::Mutex<core::cell::RefCell<Option<alloc::vec::Vec<u16>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
static PLAYLIST_DIRTY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);


// TRACK METADATA TO DETECT CHANGES THAT INVALIDATE THE BASE CACHE
static LAST_TITLE: critical_section::Mutex<core::cell::RefCell<Option<alloc::string::String>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));
static LAST_PLAYING: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static LAST_LIKED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);


// ───────────────────────────────────────────────────────────
// INIT THE MEDIA PLAYER (DECODE PNGs, BUILD INITIAL CACHE)
pub fn init() {
    // DECODE ALL PNGs FIRST (NO DMA CONFLICTS YET – MOST OF SYSTEM STILL IDLE)
    unsafe {
        PNG_PREV         = Some(decode_and_leak(crate::base::assets::MEDIA_PREVIOUS_PNG));
        PNG_PLAY         = Some(decode_and_leak(crate::base::assets::MEDIA_PLAY_PNG));
        PNG_PAUSE        = Some(decode_and_leak(crate::base::assets::MEDIA_PAUSE_PNG));
        PNG_NEXT         = Some(decode_and_leak(crate::base::assets::MEDIA_NEXT_PNG));
        PNG_CLEAR        = Some(decode_and_leak(crate::base::assets::MEDIA_CLEAR_PNG));
        PNG_HEART        = Some(decode_and_leak(crate::base::assets::MEDIA_HEART_PNG));
        PNG_HEART_FILLED = Some(decode_and_leak(crate::base::assets::MEDIA_HEART_FILLED_PNG));
        PNG_ALBUM        = Some(decode_and_leak(crate::base::assets::QWACKIFY_PNG));
    }

    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;

    let title_opt = crate::applications::media_player::current_track_title();
    let mut buf = alloc::vec![0u16; screen_w * screen_h];
    render_base_to_buffer(&mut buf, screen_w, screen_h, &title_opt, false, false);

    critical_section::with(|cs| {
        *BASE_CACHE.borrow_ref_mut(cs) = Some(buf);
    });
    BASE_DIRTY.store(false, core::sync::atomic::Ordering::Release);

    critical_section::with(|cs| {
        *LAST_TITLE.borrow_ref_mut(cs) = title_opt;
    });
    LAST_PLAYING.store(false, core::sync::atomic::Ordering::Release);
    LAST_LIKED.store(false, core::sync::atomic::Ordering::Release);

    set_hit_areas_closed();
}


// ───────────────────────────────────────────────────────────
// HIT AREAS FOR TOUCH INPUT
static mut HIT_AREAS: core::option::Option<[crate::gui::HitArea; 6]> = core::option::Option::None;


// SET THE HIT AREAS FOR THE MEDIA PLAYER WHEN THE SPLIT IS CLOSED!
// CALL THIS ONCE WHEN THE PLAYLIST APPEARS! & AGAIN WHEN THE SPLIT ANIMATION CLOSES!!
pub fn set_hit_areas_closed() {
    const AREAS: [crate::gui::HitArea; 6] = [
        // PREVIOUS TRACK BUTTON
        crate::gui::HitArea {
            x: 20, y: 382,
            width: 90, height: 90,
            action: crate::gui::TouchAction::MediaPrev,
        },
        // PLAY / PAUSE BUTTON
        crate::gui::HitArea {
            x: 160, y: 382,
            width: 90, height: 90,
            action: crate::gui::TouchAction::MediaPlayPause,
        },
        // NEXT TRACK BUTTON
        crate::gui::HitArea {
            x: 300, y: 382,
            width: 90, height: 90,
            action: crate::gui::TouchAction::MediaNext,
        },
        // TRASH CAN (CLEAR PLAYLIST)
        crate::gui::HitArea {
            x: 5, y: 130,
            width: 90, height: 90,
            action: crate::gui::TouchAction::MediaClear,
        },
        // HEART (LIKE / FAVOURITE)
        crate::gui::HitArea {
            x: 315, y: 130,
            width: 90, height: 90,
            action: crate::gui::TouchAction::MediaHeart,
        },        
        // ALBUM ART (QWACKIFY LOGO) – SPLITS THE MEDIA PLAYER VIEW
        crate::gui::HitArea {
            x: 115, y: 85,
            width: 180, height: 180,
            action: crate::gui::TouchAction::MediaSplitView,
        },
    ];
    critical_section::with(|_cs| unsafe {
        HIT_AREAS = Some(AREAS);
    });
}


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
        let mut split = MEDIA_SPLIT.borrow_ref_mut(cs);
        split.target_offset = crate::state::LCD_HEIGHT as i32 / 2;
    });
}

pub fn close_split() {
    critical_section::with(|cs| {
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

            // CHECK IF ANIMATION JUST FINISHED
            if split.current_offset == split.target_offset {
                if split.target_offset > 0 {
                    // SPLIT IS FULLY OPEN > DISABLE REGULAR HIT AREAS
                    unsafe { HIT_AREAS = None; }
                } else {
                    // SPLIT CLOSED > RESTORE STATIC HARDCODED HIT AREAS (LESS HEAVY LIFTING)
                    set_hit_areas_closed();
                }
            }
        }
    });
}

pub fn is_split_open() -> bool {
    critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset > 0)
}


// ───────────────────────────────────────────────────────────
// PLAYLIST SCROLLING
pub struct PlaylistScroll {
    pub offset: i32,
    pub target: i32,
    pub max_scroll: i32,
}

pub(crate) static PLAYLIST_SCROLL: critical_section::Mutex<core::cell::RefCell<PlaylistScroll>> =
    critical_section::Mutex::new(core::cell::RefCell::new(PlaylistScroll {
        offset: 0,
        target: 0,
        max_scroll: 0,
    }));

pub fn animate_playlist_scroll(speed: i32) {
    critical_section::with(|cs| {
        let mut scroll = PLAYLIST_SCROLL.borrow_ref_mut(cs);
        let diff = scroll.target - scroll.offset;
        if diff != 0 {
            let step = diff.clamp(-speed, speed);
            scroll.offset += step;
            defmt::info!("scroll offset={} target={} (diff={}, step={})", scroll.offset, scroll.target, diff, step);
        }
    });
}


// ───────────────────────────────────────────────────────────
// FORCE REFRESH OF PLAYLIST CACHE
pub fn invalidate_playlist() {
    PLAYLIST_DIRTY.store(true, core::sync::atomic::Ordering::Release);
}

// CLEAR PLAYLIST CACHE
pub fn release_playlist_cache() {
    critical_section::with(|cs| {
        *PLAYLIST_CACHE.borrow_ref_mut(cs) = None;
    });
    PLAYLIST_DIRTY.store(true, core::sync::atomic::Ordering::Release);
}

// ───────────────────────────────────────────────────────────────────────
// HELPERS FOR CACHED RENDERING

// RENDERS ALL STATIC ELEMENTS (ALBUM ART, BUTTONS, TITLE, HEADER, ETC ETC ETC)
// INTO THE PROVIDED BUFFER. DOES NOT DRAW THE PROGRESS BAR OR TIME LABELS
fn render_base_to_buffer(
    buf: &mut [u16],
    screen_w: usize,
    screen_h: usize,
    title_opt: &Option<alloc::string::String>,
    is_playing: bool,
    is_liked: bool,
) {
    // CLEAR TO BLACK
    buf.fill(0);

    // LOAD FONTS
    let bold_font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(crate::gui::ROBOTO_BOLD_FONT);
        (*ptr).as_ref().expect("FONT NOT INITIALISED").clone()
    });

    // COLORS
    let white = crate::gui::colors::WHITE;
    let cyan = crate::gui::colors::CYAN;
    let gray = crate::gui::colors::GRAY;

    // TEXT STYLES
    let header_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(62)
        .text_color(cyan)
        .build();
    let title_style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
        .font_size(48)
        .text_color(white)
        .build();

    // USE PRE‑DECODED PIXEL DATA (NO FLASH ACCESS)
    let (prev_data, prev_w_u32, prev_h_u32) = unsafe { PNG_PREV.unwrap() };
    let (play_data, play_w_u32, play_h_u32) = unsafe { PNG_PLAY.unwrap() };
    let (pause_data, pause_w_u32, pause_h_u32) = unsafe { PNG_PAUSE.unwrap() };
    let (next_data, next_w_u32, next_h_u32) = unsafe { PNG_NEXT.unwrap() };
    let (heart_filled_data, heart_filled_w_u32, heart_filled_h_u32) = unsafe { PNG_HEART_FILLED.unwrap() };
    let (clear_data, clear_w_u32, clear_h_u32) = unsafe { PNG_CLEAR.unwrap() };
    let (heart_data, heart_w_u32, heart_h_u32) = unsafe { PNG_HEART.unwrap() };
    let (album_data, album_w_u32, album_h_u32) = unsafe { PNG_ALBUM.unwrap() };

    // PICK PLAY OR PAUSE
    let (play_pause_data, pp_w_u32, pp_h_u32) = if is_playing {
        (pause_data, pause_w_u32, pause_h_u32)
    } else {
        (play_data, play_w_u32, play_h_u32)
    };

    // CONVERT TO i32 FOR CALCULATIONS
    let album_w = album_w_u32 as i32;
    let album_h = album_h_u32 as i32;
    let clear_w = clear_w_u32 as i32;
    let clear_h = clear_h_u32 as i32;
    let heart_w = heart_w_u32 as i32;
    let heart_h = heart_h_u32 as i32;
    let prev_w = prev_w_u32 as i32;
    let prev_h = prev_h_u32 as i32;
    let pp_w = pp_w_u32 as i32;
    let pp_h = pp_h_u32 as i32;
    let next_w = next_w_u32 as i32;
    let next_h = next_h_u32 as i32;

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    let w = screen_w as i32;
    let h = screen_h as i32;

    // ALBUM ART/QWACKIFY LOGO (SCALE 1.0)
    let img_w = album_w;
    let img_h = album_h;
    let album_y = (h as f32 * 0.35) as i32 - img_h / 2;
    let x = w / 2 - img_w / 2;
    draw_raw_pixels(buf, album_data, album_w_u32, album_h_u32, x, album_y, 1.1, screen_w, screen_h);

    // SIDE BUTTONS: CLEAR/HEART (SCALE 0.5)
    let side_scale: f32 = 0.5;
    let clear_w_scaled = (clear_w as f32 * side_scale) as i32;
    let clear_h_scaled = (clear_h as f32 * side_scale) as i32;
    let heart_w_scaled = (heart_w as f32 * side_scale) as i32;
    let heart_h_scaled = (heart_h as f32 * side_scale) as i32;
    let side_gap = 20;
    let clear_x = x - side_gap - clear_w_scaled;
    let clear_y = album_y + (img_h - clear_h_scaled) / 2;
    let heart_x = x + img_w + side_gap;
    let heart_y = album_y + (img_h - heart_h_scaled) / 2;

    draw_raw_pixels(buf, clear_data, clear_w_u32, clear_h_u32, clear_x, clear_y, side_scale, screen_w, screen_h);
    if is_liked {
        draw_raw_pixels_tinted(buf, heart_filled_data, heart_filled_w_u32, heart_filled_h_u32, heart_x, heart_y, side_scale, screen_w, screen_h, 0xF800);
    } else {
        draw_raw_pixels(buf, heart_data, heart_w_u32, heart_h_u32, heart_x, heart_y, side_scale, screen_w, screen_h);
    }

    // BOTTOM CONTROL BUTTONS (SCALED 0.5)
    let scale: f32 = 0.5;
    let gap = 50;
    let prev_w_scaled = (prev_w as f32 * scale) as i32;
    let pp_w_scaled   = (pp_w as f32 * scale) as i32;
    let next_w_scaled = (next_w as f32 * scale) as i32;
    let total_btn = prev_w_scaled + pp_w_scaled + next_w_scaled + 2 * gap;
    let start_x = (w - total_btn) / 2;
    let base_btn_y = h - (prev_h as f32 * scale) as i32 - 30;
    let btn_y = base_btn_y; // OFFSET = 0

    draw_raw_pixels(buf, prev_data, prev_w_u32, prev_h_u32, start_x, btn_y, scale, screen_w, screen_h);
    draw_raw_pixels(buf, play_pause_data, pp_w_u32, pp_h_u32, start_x + prev_w_scaled + gap, btn_y, scale, screen_w, screen_h);
    draw_raw_pixels(buf, next_data, next_w_u32, next_h_u32, start_x + prev_w_scaled + gap + pp_w_scaled + gap, btn_y, scale, screen_w, screen_h);

    // TEXT DRAWS VIA DRAWTARGET
    let mut target = RawBufferDrawTarget::new(buf, screen_w, screen_h);

    let header_text = embedded_graphics::text::Text::with_text_style(
        "QWACKIFY",
        embedded_graphics::geometry::Point::new(w / 2, 20),
        header_style,
        center_align,
    );
    embedded_graphics::Drawable::draw(&header_text, &mut target).ok();

    if let Some(title) = title_opt {
        let display = if title.len() > 25 { &title[..25] } else { title };
        let title_text = embedded_graphics::text::Text::with_text_style(
            display,
            embedded_graphics::geometry::Point::new(w / 2, (h as f32 * 0.55) as i32),
            title_style,
            center_align,
        );
        embedded_graphics::Drawable::draw(&title_text, &mut target).ok();
    }
}


// ───────────────────────────────────────────────────────────
// RENDERS THE PLAYLIST (SONG NAMES) INTO A BUFFER THAT FITS THE MAXIMUM GAP SIZE.
// THE BUFFER IS OF SIZE screen_w * PLAYLIST_BUFFER_HEIGHT (LARGE ENOUGH TO HOLD ALL TRACKS FOR SCROLLING)
fn render_playlist_to_buffer(
    buf: &mut [u16],
    screen_w: usize,
    total_height: usize,
    songs: &[&str],
    current_track_title: Option<&str>,
) -> i32 {
    // CLEAR THE BUFFER
    buf[..screen_w * total_height].fill(0);

    let bold_font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(crate::gui::ROBOTO_BOLD_FONT);
        (*ptr).as_ref().expect("FONT NOT INITIALISED").clone()
    });
    
    let white = crate::gui::colors::WHITE;
    let red = crate::gui::colors::RED;
    let cyan = crate::gui::colors::CYAN;
    
    let left_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Left)
        .build();

    let mut target = RawBufferDrawTarget::new(buf, screen_w, total_height);

    let line_height = PLAYLIST_LINE_HEIGHT;
    let margin = PLAYLIST_MARGIN;
    let mut y = margin;

    for song in songs {
        if y + line_height > total_height as i32 {
            break; // BUFFER FULL
        }

        // CURRENTLY PLAYING TRACK HAS DIFFERENT TEXT COLOR
        let is_current = current_track_title.map_or(false, |t| t == *song);
        let text_color = if is_current { cyan } else { white };

        let style = embedded_ttf::FontTextStyleBuilder::new(bold_font.clone())
            .font_size(54)
            .text_color(text_color)
            .build();

        let text = embedded_graphics::text::Text::with_text_style(
            *song,
            embedded_graphics::geometry::Point::new(20, y),
            style,
            left_align,
        );
        embedded_graphics::Drawable::draw(&text, &mut target).ok();

        y += line_height;
    }

    y // RETURN TOTAL CONTENT HEIGHT
}

// ───────────────────────────────────────────────────────────
// DRAWS THE DYNAMIC PROGRESS BAR AND TIME LABELS ON TOP OF THE BASE BUFFER
// THESE ARE DRAWN AT THE GIVEN OFFSET
fn draw_progress_and_time(
    dest: &mut [u16],
    screen_w: usize,
    screen_h: usize,
    offset: i32,
    total_duration: u32,
    current_position: u32,
) {
    let bold_font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(crate::gui::ROBOTO_BOLD_FONT);
        (*ptr).as_ref().expect("FONT NOT INITIALISED").clone()
    });
    let regular_font = bold_font.clone();

    let gray = crate::gui::colors::GRAY;
    let dark_gray = crate::gui::colors::DARK_GRAY;
    let cyan = crate::gui::colors::CYAN;

    let center_align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Center)
        .build();

    let time_style = embedded_ttf::FontTextStyleBuilder::new(regular_font)
        .font_size(16)
        .text_color(gray)
        .build();

    let w = screen_w as i32;
    let h = screen_h as i32;
    let split_line_y = h / 2;
    let bar_y = split_line_y + offset;
    let bar_height = 24;
    let bar_width = w - 40;
    let bar_x = 20;

    let mut target = RawBufferDrawTarget::new(dest, screen_w, screen_h);

    // BACKGROUND
    let bg_rect = embedded_graphics::primitives::Rectangle::new(
        embedded_graphics::geometry::Point::new(bar_x, bar_y),
        embedded_graphics::geometry::Size::new(bar_width as u32, bar_height as u32),
    );
    let bg_styled = <embedded_graphics::primitives::Rectangle as embedded_graphics::prelude::Primitive>::into_styled(
        bg_rect,
        embedded_graphics::primitives::PrimitiveStyle::with_fill(dark_gray),
    );
    embedded_graphics::Drawable::draw(&bg_styled, &mut target).ok();

    // FILL
    let progress = if total_duration > 0 {
        (current_position as f32 / total_duration as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled_w = (bar_width as f32 * progress) as u32;
    if filled_w > 0 {
        let fill_rect = embedded_graphics::primitives::RoundedRectangle::with_equal_corners(
            embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::geometry::Point::new(bar_x, bar_y),
                embedded_graphics::geometry::Size::new(filled_w, bar_height as u32),
            ),
            embedded_graphics::geometry::Size::new(6, 6),
        );
        let fill_styled = <embedded_graphics::primitives::RoundedRectangle as embedded_graphics::prelude::Primitive>::into_styled(
            fill_rect,
            embedded_graphics::primitives::PrimitiveStyle::with_fill(cyan),
        );
        embedded_graphics::Drawable::draw(&fill_styled, &mut target).ok();
    }


    // TIME LABELS
    let time_y = bar_y + bar_height + 8;
    let cur_str = format_time(current_position);
    let tot_str = format_time(total_duration);

    let cur_text = embedded_graphics::text::Text::with_text_style(
        &cur_str,
        embedded_graphics::geometry::Point::new(bar_x, time_y),
        time_style.clone(),
        center_align,
    );
    embedded_graphics::Drawable::draw(&cur_text, &mut target).ok();

    let tot_text = embedded_graphics::text::Text::with_text_style(
        &tot_str,
        embedded_graphics::geometry::Point::new(bar_x + bar_width, time_y),
        time_style,
        center_align,
    );
    embedded_graphics::Drawable::draw(&tot_text, &mut target).ok();
}


// DRAWS THE TEAL GLOW LINES AT THE SPLIT EDGES
fn draw_split_glow(dest: &mut [u16], screen_w: usize, screen_h: usize, split_line_y: i32, offset: i32) {
    if offset <= 0 {
        return;
    }
    let teal_bright: u16 = 0x07FF;
    let teal_dark: u16 = 0x020F;
    let glow_half_thickness = 2;

    let top_edge_y = split_line_y - offset;
    let bottom_edge_y = split_line_y + offset;

    for i in -glow_half_thickness..=glow_half_thickness {
        let color = if i == 0 { teal_bright } else { teal_dark };
        let y1 = top_edge_y + i;
        let y2 = bottom_edge_y + i;
        if y1 >= 0 && (y1 as usize) < screen_h {
            let row_start = (y1 as usize) * screen_w;
            for x in 0..screen_w {
                dest[row_start + x] = color;
            }
        }
        if y2 >= 0 && (y2 as usize) < screen_h {
            let row_start = (y2 as usize) * screen_w;
            for x in 0..screen_w {
                dest[row_start + x] = color;
            }
        }
    }
}



// ───────────────────────────────────────────────────────────────────────
// PUBLIC DRAW FUNCTION (CACHED VERSION)
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    let offset = critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset);
    let is_playing = crate::load!(crate::state::MEDIA_IS_PLAYING);
    let is_liked = crate::load!(crate::state::MEDIA_IS_LIKED);
    let title_opt = crate::applications::media_player::current_track_title();
    let total_duration = crate::load!(crate::applications::media_player::TRACK_DURATION_MS);
    let current_position = crate::load!(crate::applications::media_player::TRACK_POSITION_MS);

    let screen_w = crate::state::LCD_WIDTH as usize;
    let screen_h = crate::state::LCD_HEIGHT as usize;


    // DETECT CHANGES THAT INVALIDATE THE BASE CACHE
    let mut title_changed = false;
    critical_section::with(|cs| {
        let mut last_title_ref = LAST_TITLE.borrow_ref_mut(cs);
        if let Some(ref title) = title_opt {
            if last_title_ref.as_ref().map_or(true, |t| t != title) {
                *last_title_ref = Some(title.clone());
                title_changed = true;
            }
        } else if last_title_ref.is_some() {
            *last_title_ref = None;
            title_changed = true;
        }
    });
    let playing_changed = LAST_PLAYING.load(core::sync::atomic::Ordering::Acquire) != is_playing;
    let liked_changed = LAST_LIKED.load(core::sync::atomic::Ordering::Acquire) != is_liked;

    if title_changed || playing_changed || liked_changed {
        BASE_DIRTY.store(true, core::sync::atomic::Ordering::Release);
        LAST_PLAYING.store(is_playing, core::sync::atomic::Ordering::Release);
        LAST_LIKED.store(is_liked, core::sync::atomic::Ordering::Release);
    }

    // REBUILD BASE CACHE IF DIRTY
    if BASE_DIRTY.swap(false, core::sync::atomic::Ordering::AcqRel) {
        critical_section::with(|cs| {
            let mut cache = BASE_CACHE.borrow_ref_mut(cs);
            let mut buf = alloc::vec![0u16; screen_w * screen_h];
            render_base_to_buffer(&mut buf, screen_w, screen_h, &title_opt, is_playing, is_liked);
            *cache = Some(buf);
        });
    }

    // REBUILD PLAYLIST CACHE IF DIRTY
    if PLAYLIST_DIRTY.swap(false, core::sync::atomic::Ordering::AcqRel) {
        critical_section::with(|cs| {
            let mut cache = PLAYLIST_CACHE.borrow_ref_mut(cs);

            // FETCH TITLES FROM THE MEDIA PLAYER
            let titles = crate::applications::media_player::playlist_titles();
            let title_strs: heapless::Vec<&str, 32> = titles.iter().map(|s| s.as_str()).collect();

            let mut buf = alloc::vec![0u16; screen_w * PLAYLIST_BUFFER_HEIGHT];

            let current_title = crate::applications::media_player::current_track_title();
            let content_height = render_playlist_to_buffer(
                &mut buf,
                screen_w,
                PLAYLIST_BUFFER_HEIGHT,
                &title_strs,
                current_title.as_deref(),
            );

            // UPDATE MAX SCROLL (GAP CAN BE UP TO screen_h WHEN FULLY OPEN)
            //let gap_height = screen_h;
            let gap_height = 200;
            let max_scroll = (content_height - gap_height as i32).max(0);
            let mut scroll = PLAYLIST_SCROLL.borrow_ref_mut(cs);
            scroll.max_scroll = max_scroll;
            scroll.target = scroll.target.clamp(0, max_scroll);
            scroll.offset = scroll.offset.clamp(0, max_scroll);

            *cache = Some(buf);
        });
    }

    // GET CACHED BUFFERS
    let base_buf = critical_section::with(|cs| {
        BASE_CACHE.borrow_ref(cs).as_ref().unwrap().as_ptr()
    });
    let playlist_buf = critical_section::with(|cs| {
        PLAYLIST_CACHE.borrow_ref(cs).as_ref().unwrap().as_ptr()
    });

    let dest = fb.buffer_mut();
    // USE FBACK FRAMEBUFFFER INSTEAD? ... NO REAL DIFFERENCE(?)
    //let dest = fb.back_buffer_mut(); 
    let split_line_y = screen_h as i32 / 2;


    // BLIT BASED ON OFFSET
    if offset == 0 {
        // FULL SCREEN BLIT FROM BASE CACHE
        let src = unsafe { core::slice::from_raw_parts(base_buf, screen_w * screen_h) };
        dest.copy_from_slice(src);
    } else {
        // TOP HALF: FROM 0 TO SPLIT_LINE - OFFSET
        let top_end = split_line_y - offset;
        if top_end > 0 {
            let src = unsafe { core::slice::from_raw_parts(base_buf, screen_w * screen_h) };
            let rows = top_end as usize;
            for row in 0..rows {
                let src_start = row * screen_w;
                let dst_start = row * screen_w;
                dest[dst_start..dst_start + screen_w].copy_from_slice(&src[src_start..src_start + screen_w]);
            }
        }

        // BOTTOM HALF: FROM SPLIT_LINE + OFFSET TO BOTTOM
        let bottom_start = split_line_y + offset;
        if bottom_start < screen_h as i32 {
            let rows = screen_h as i32 - bottom_start;
            let src = unsafe { core::slice::from_raw_parts(base_buf, screen_w * screen_h) };
            for row in 0..rows as usize {
                let src_row = (split_line_y + row as i32) as usize; // SRC IS AT OFFSET=0, SO ROWS CORRESPOND
                let dst_row = bottom_start as usize + row;
                let src_start = src_row * screen_w;
                let dst_start = dst_row * screen_w;
                dest[dst_start..dst_start + screen_w].copy_from_slice(&src[src_start..src_start + screen_w]);
            }
        }

        // GAP: FILL WITH PLAYLIST CACHE (SCROLLING ENABLED)
        let gap_top = split_line_y - offset;
        let gap_bottom = split_line_y + offset;
        if gap_top < gap_bottom {
            let gap_height = (gap_bottom - gap_top) as usize;

            let scroll_off = critical_section::with(|cs| {
                let s = PLAYLIST_SCROLL.borrow_ref(cs);
                s.offset.clamp(0, s.max_scroll)
            }) as usize;

            // SAFETY: playlist_buf POINTS TO A BUFFER OF SIZE screen_w * PLAYLIST_BUFFER_HEIGHT
            let src_start = scroll_off * screen_w;
            let playlist_src = unsafe {
                core::slice::from_raw_parts(
                    playlist_buf.add(src_start),
                    screen_w * gap_height,
                )
            };

            let dst_start_row = gap_top as usize;
            for row in 0..gap_height {
                let src_row = row * screen_w;
                let dst_row = (dst_start_row + row) * screen_w;
                dest[dst_row..dst_row + screen_w]
                    .copy_from_slice(&playlist_src[src_row..src_row + screen_w]);
            }
        }

        // DRAW TEAL GLOW LINES
        draw_split_glow(dest, screen_w, screen_h, split_line_y, offset);
    }

    // DRAW DYNAMIC PROGRESS BAR AND TIME LABELS
    draw_progress_and_time(dest, screen_w, screen_h, offset, total_duration, current_position);
}

// ───────────────────────────────────────────────────────────────────────
// RAW PIXEL DRAWING
fn draw_raw_pixels(
    dest: &mut [u16],
    src: &[u16],
    src_w: u32,
    src_h: u32,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
) {
    if scale <= 0.0 { return; }
    let dst_w = (src_w as f32 * scale) as i32;
    let dst_h = (src_h as f32 * scale) as i32;
    for dst_row in 0..dst_h {
        let src_row = ((dst_row as f32 / scale) as u32).min(src_h.saturating_sub(1));
        let row = (y + dst_row) as usize;
        if row >= screen_h { break; }
        for dst_col in 0..dst_w {
            let src_col = ((dst_col as f32 / scale) as u32).min(src_w.saturating_sub(1));
            let col = (x + dst_col) as usize;
            if col >= screen_w { break; }
            dest[row * screen_w + col] = src[(src_row * src_w + src_col) as usize];
        }
    }
}

fn draw_raw_pixels_tinted(
    dest: &mut [u16],
    src: &[u16],
    src_w: u32,
    src_h: u32,
    x: i32,
    y: i32,
    scale: f32,
    screen_w: usize,
    screen_h: usize,
    tint: u16,
) {
    if scale <= 0.0 { return; }
    let dst_w = (src_w as f32 * scale) as i32;
    let dst_h = (src_h as f32 * scale) as i32;
    for dst_row in 0..dst_h {
        let src_row = ((dst_row as f32 / scale) as u32).min(src_h.saturating_sub(1));
        let row = (y + dst_row) as usize;
        if row >= screen_h { break; }
        for dst_col in 0..dst_w {
            let src_col = ((dst_col as f32 / scale) as u32).min(src_w.saturating_sub(1));
            let col = (x + dst_col) as usize;
            if col >= screen_w { break; }
            let pixel = src[(src_row * src_w + src_col) as usize];
            if pixel != 0 {
                dest[row * screen_w + col] = tint;
            }
        }
    }
}

// ───────────────────────────────────────────────────────────
// TIME FORMATTING
fn format_time(ms: u32) -> heapless::String<16> {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let mut s = heapless::String::new();
    core::fmt::Write::write_fmt(&mut s, core::format_args!("{}:{:02}", mins, secs)).ok();
    s
}

// ───────────────────────────────────────────────────────────
// TOUCH HANDLING (HIT AREA)
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

// ───────────────────────────────────────────────────────────
// CHECK IF A Y COORDINATE IS INSIDE THE CURRENTLY VISIBLE PLAYLIST GAP
pub fn is_in_gap(y: i32) -> bool {
    let offset = critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset);
    if offset <= 0 {
        return false;
    }
    let split_line_y = crate::state::LCD_HEIGHT as i32 / 2;
    let gap_top = split_line_y - offset;
    let gap_bottom = split_line_y + offset;
    y >= gap_top && y <= gap_bottom
}

// ───────────────────────────────────────────────────────────
// INPUT HANDLERS
pub fn handle_swipe(dir: crate::components::ft3168::SwipeDirection) {
    if !is_split_open() {
        return;
    }
    let step = 70;

    critical_section::with(|cs| {
        let mut scroll = PLAYLIST_SCROLL.borrow_ref_mut(cs);
        match dir {
            crate::components::ft3168::SwipeDirection::Up => {
                scroll.target = (scroll.target + step).min(scroll.max_scroll);
                defmt::debug!("scroll target={}, max={}", scroll.target, scroll.max_scroll);
            }
            crate::components::ft3168::SwipeDirection::Down => {
                scroll.target = (scroll.target - step).max(0);
                defmt::debug!("scroll target={}, max={}", scroll.target, scroll.max_scroll);
            }
            _ => {}
        }
    });
}

// SINGLE TAP MARKS THE TAPPED TRACK - CHANGE BACKGROUND FOR THAT TRACK
pub fn handle_tap() {
    defmt::info!("👆");    
}

// DOUBLE-TAP PLAYS THAT TRACK
pub fn handle_double_tap(_x: u16, _y: u16) {
    defmt::info!("👆👆");
}


pub fn handle_playlist_tap(x: i32, y: i32) -> Option<usize> {
    // ONLY WORKS WHEN SPLIT VIEW (PLAYLIST) IS OPEN
    let offset = critical_section::with(|cs| MEDIA_SPLIT.borrow_ref(cs).current_offset);
    if offset <= 0 { return None; }

    let split_line_y = crate::state::LCD_HEIGHT as i32 / 2;
    let gap_top = split_line_y - offset;
    if y < gap_top || y > split_line_y + offset { return None; } // OUTSIDE GAP

    let scroll_off = critical_section::with(|cs| {
        PLAYLIST_SCROLL.borrow_ref(cs).offset
    });

    // THE FIRST VISIBLE TRACK IS AT y = gap_top + margin - scroll_off
    let relative_y = (y - gap_top) + scroll_off - PLAYLIST_MARGIN;
    if relative_y < 0 { return None; }
    let index = (relative_y / PLAYLIST_LINE_HEIGHT) as usize;

    // CLAMP TO THE ACTUAL PLAYLIST LENGTH
    let total = crate::applications::media_player::playlist_len();
    if index >= total { None } else { Some(index) }
}


// ───────────────────────────────────────────────────────────────────────
// RawBufferDrawTarget – A SIMPLE DRAWTARGET THAT WRITES TO A RAW u16 SLICE
// THIS IS USED FOR embedded_graphics DRAWING INTO THE CACHED BUFFERS
struct RawBufferDrawTarget<'a> {
    buf: &'a mut [u16],
    width: usize,
    height: usize,
}

impl<'a> RawBufferDrawTarget<'a> {
    fn new(buf: &'a mut [u16], width: usize, height: usize) -> Self {
        Self { buf, width, height }
    }
}

impl embedded_graphics::prelude::OriginDimensions for RawBufferDrawTarget<'_> {
    fn size(&self) -> embedded_graphics::geometry::Size {
        embedded_graphics::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

impl embedded_graphics::draw_target::DrawTarget for RawBufferDrawTarget<'_> {
    type Color = embedded_graphics::pixelcolor::Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter() {
            let point = pixel.0;
            if point.x >= 0 && point.x < self.width as i32 && point.y >= 0 && point.y < self.height as i32 {
                let idx = (point.y as usize) * self.width + (point.x as usize);
                self.buf[idx] = embedded_graphics::pixelcolor::IntoStorage::into_storage(pixel.1);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let raw = embedded_graphics::pixelcolor::IntoStorage::into_storage(color);
        for item in self.buf.iter_mut() {
            *item = raw;
        }
        Ok(())
    }
}
