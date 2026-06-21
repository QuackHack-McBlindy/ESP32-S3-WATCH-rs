// GUI/OPTIONS/INFO
// DRAW A SETTINGS PAGE FOR DISPLAYING MISC INFORMATION
// SWIPING DOWN SMOOTH SCROLLS BETWEEN INFO PAGES
// LEFT/RIGHT DISABLED AFTER MOVED DOWN FROM FIRST INFO PAGE

// ───────────────────────────────────────────────────────────────────────
// SCROLL STATE
const PAGE_HEIGHT: i32 = crate::state::LCD_HEIGHT as i32;
const MAX_OFFSET: i32 = 4 * PAGE_HEIGHT;   // 4 info pages

pub struct InfoScroll {
    pub target_offset: i32,
    pub current_offset: i32,
}

pub(crate) static INFO_SCROLL: critical_section::Mutex<core::cell::RefCell<InfoScroll>> =
    critical_section::Mutex::new(core::cell::RefCell::new(InfoScroll {
        target_offset: 0,
        current_offset: 0,
    }));

pub fn animate_info(anim_speed: i32) {
    critical_section::with(|cs| {
        let mut scroll = INFO_SCROLL.borrow_ref_mut(cs);
        let diff = scroll.target_offset - scroll.current_offset;
        if diff != 0 {
            let step = diff.clamp(-anim_speed, anim_speed);
            scroll.current_offset += step;
        }
    });
}

// ───────────────────────────────────────────────────────────────────────
// IS THIS A INFO PAGE
pub fn is_on_info_page() -> bool {
    critical_section::with(|cs| {
        let scroll = INFO_SCROLL.borrow_ref(cs);
        scroll.current_offset > 0
    })
}

// ───────────────────────────────────────────────────────────────────────
// HANDLE SWIPES
pub fn handle_swipe(
    direction: crate::components::ft3168::SwipeDirection,
    _start_x: u16,
    _start_y: u16,
    _last_x: u16,
    _last_y: u16,
) -> bool {
    match direction {
        // SWIPE UP - NEXT PAGE
        crate::components::ft3168::SwipeDirection::Up => {
            critical_section::with(|cs| {
                let mut scroll = INFO_SCROLL.borrow_ref_mut(cs);
                scroll.target_offset = (scroll.target_offset + PAGE_HEIGHT).min(MAX_OFFSET);
            });
            true
        }
        // SWIPE DOWN - PREVIOUS PAGE
        crate::components::ft3168::SwipeDirection::Down => {
            critical_section::with(|cs| {
                let mut scroll = INFO_SCROLL.borrow_ref_mut(cs);
                scroll.target_offset = (scroll.target_offset - PAGE_HEIGHT).max(0);
            });
            true
        }
        // LEFT/RIGHT BLOCKED WHEN ON ANY INFO PAGE
        crate::components::ft3168::SwipeDirection::Left
        | crate::components::ft3168::SwipeDirection::Right => {
            if is_on_info_page() {
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

// ───────────────────────────────────────────────────────────────────────
// DRAW FUNCTION
pub fn draw(fb: &mut crate::components::framebuffer::Framebuffer) {
    let bbox = embedded_graphics::geometry::Dimensions::bounding_box(fb);
    let w = bbox.size.width as i32;
    let h = bbox.size.height as i32;
    let screen_w = w as usize;
    let screen_h = h as usize;

    let current_offset = critical_section::with(|cs| {
        INFO_SCROLL.borrow_ref(cs).current_offset
    });

    // CLEAR SCREEN
    fb.buffer_mut().fill(0x0000);

    // LOAD TRUETYPE FONT
    let body_font = rusttype::Font::try_from_bytes(crate::base::assets::ROBOTO_BOLD).unwrap();

    // ───────────────────────────────────────────────────────────────────────
    // MAIN VIEW (ICON + HEADER)
    let main_view_top = -current_offset;
    if main_view_top + h > 0 && main_view_top < h {
        if main_view_top < 80 {
            // ───────────────────────────────────────────────────────────────────────
            // MAIN HEADER
            let header_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                .font_size(86)
                .text_color(crate::gui::colors::CYAN)
                .build();
            let header_align = embedded_graphics::text::TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Center)
                .build();
            let _ = embedded_graphics::Drawable::draw(
                &embedded_graphics::text::Text::with_text_style(
                    "INFO",
                    embedded_graphics::prelude::Point::new(w / 2, main_view_top + 20),
                    header_style,
                    header_align,
                ),
                fb,
            );
        }

        // ───────────────────────────────────────────────────────────────────────
        // MAIN ICON
        let icon_bytes = crate::base::assets::SETTINGS_INFO_PNG;
        if let core::result::Result::Ok(icon_png) =
            embedded_png::Png::load_from_bytes(icon_bytes)
        {
            let img_w = icon_png.width() as i32;
            let img_h = icon_png.height() as i32;
            let target_h = (h as f32 * 0.66) as i32;
            let scale = core::cmp::max(1, target_h / img_h.max(1));
            let scaled_w = img_w * scale;
            let scaled_h = img_h * scale;
            let x = (w - scaled_w) / 2;
            let y = main_view_top + (h - scaled_h) / 2;
            let dest = fb.buffer_mut();
            for sy in 0..img_h {
                let py = y + sy * scale;
                if py < 0 || py as usize >= screen_h { continue; }
                for sx in 0..img_w {
                    let idx = (sy * img_w + sx) as usize;
                    if let core::option::Option::Some(color) = icon_png.pixels()[idx] {
                        let raw: u16 =
                            embedded_graphics_core::pixelcolor::IntoStorage::into_storage(color);
                        let px = x + sx * scale;
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

            // ───────────────────────────────────────────────────────────────────────
            // MAIN ARROW DOWN INDICATOR
            let arrow_bytes = crate::base::assets::SETTINGS_ARROW_DOWN_PNG;
            if let core::result::Result::Ok(arrow_png) =
                embedded_png::Png::load_from_bytes(arrow_bytes)
            {
                let aw = arrow_png.width() as i32;
                let ah = arrow_png.height() as i32;
                let target_arrow_h = 16i32;
                let arrow_scale = core::cmp::max(1, target_arrow_h / ah.max(1));
                let scaled_aw = aw * arrow_scale;
                let scaled_ah = ah * arrow_scale;
                let ax = (w - scaled_aw) / 2;
                let ay = main_view_top + h - scaled_ah - 16;
                let dest = fb.buffer_mut();
                for sy in 0..ah {
                    let py = ay + sy * arrow_scale;
                    if py < 0 || py as usize >= screen_h { continue; }
                    for sx in 0..aw {
                        let idx = (sy * aw + sx) as usize;
                        if let core::option::Option::Some(color) = arrow_png.pixels()[idx] {
                            let raw: u16 =
                                embedded_graphics_core::pixelcolor::IntoStorage::into_storage(color);
                            let px = ax + sx * arrow_scale;
                            for dy in 0..arrow_scale {
                                let row = (py + dy) as usize;
                                if row >= screen_h { break; }
                                for dx in 0..arrow_scale {
                                    let col = (px + dx) as usize;
                                    if col < screen_w {
                                        dest[row * screen_w + col] = raw;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // INFO PAGES
    draw_info_page(fb, 1, PAGE_HEIGHT - current_offset, w, h, &body_font);
    draw_info_page(fb, 2, 2 * PAGE_HEIGHT - current_offset, w, h, &body_font);
    draw_info_page(fb, 3, 3 * PAGE_HEIGHT - current_offset, w, h, &body_font);
    draw_info_page(fb, 4, 4 * PAGE_HEIGHT - current_offset, w, h, &body_font);
}


// ───────────────────────────────────────────────────────────────────────
// DRAW AN INFO PAGE
fn draw_info_page(
    fb: &mut crate::components::framebuffer::Framebuffer,
    page_num: i32,
    top_y: i32,
    w: i32,
    h: i32,
    body_font: &rusttype::Font<'static>,
) {
    if top_y >= h || top_y <= -h {
        return;
    }

    // CLEAR PAGE AREA TO BLACK
    let screen_w = w as usize;
    let screen_h = h as usize;
    let buf = fb.buffer_mut();
    for y in top_y.max(0)..(top_y + h).min(h) {
        let row_start = (y as usize) * screen_w;
        let end = row_start + screen_w;
        buf[row_start..end].fill(0x0000);
    }

    let body_start_y = top_y + 80;

    // BODY TEXT STYLE
    const BODY_FONT_SIZE: u32 = 40;
    let body_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
        .font_size(BODY_FONT_SIZE)
        .text_color(crate::gui::colors::WHITE)
        .build();

    let red_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
        .font_size(48)
        .text_color(crate::gui::colors::RED)
        .build();

    let align = embedded_graphics::text::TextStyleBuilder::new()
        .alignment(embedded_graphics::text::Alignment::Left)
        .build();
    let line_height = (BODY_FONT_SIZE as i32) + 4; // font height + spacing

    // HELPER TO DRAW A LINE OF TEXT
    let draw_line = |line: &str, row: i32, fb: &mut crate::components::framebuffer::Framebuffer| {
        if row >= top_y && row + line_height <= top_y + h {
            let _ = embedded_graphics::Drawable::draw(
                &embedded_graphics::text::Text::with_text_style(
                    line,
                    embedded_graphics::prelude::Point::new(8, row),
                    body_style.clone(),
                    align,
                ),
                fb,
            );
        }
    };

    match page_num {
        // ─────────────────────────────────────────────────────────────────────── 
        // DEVICE INFORMATION   
        1 => {
            // HEADER
            let header_y = top_y + 8;
            if header_y >= 0 && header_y < h {
                let header_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                    .font_size(86)
                    .text_color(crate::gui::colors::CYAN)
                    .build();
                let header_align = embedded_graphics::text::TextStyleBuilder::new()
                    .alignment(embedded_graphics::text::Alignment::Center)
                    .build();
                let _ = embedded_graphics::Drawable::draw(
                    &embedded_graphics::text::Text::with_text_style(
                        "DEVICE",
                        embedded_graphics::prelude::Point::new(w / 2, header_y),
                        header_style,
                        header_align,
                    ),
                    fb,
                );
            }

            // UPTIME
            let uptime_secs: u64 = crate::load!(crate::state::UPTIME_SECS) as u64;
            let days = uptime_secs / 86400;
            let hours = (uptime_secs % 86400) / 3600;
            let minutes = (uptime_secs % 3600) / 60;

            let mut uptime_str;
            if days > 0 {
                uptime_str = alloc::format!("{}d {}h {}m", days, hours, minutes);
            } else if hours > 0 {
                uptime_str = alloc::format!("{}h {}m", hours, minutes);
            } else {
                uptime_str = alloc::format!("{}m", minutes);
            }
            let uptime_line = alloc::format!("Uptime: {}", uptime_str);
            defmt::info!("DEVICE: {}", &*uptime_line);

            // IP ADDRESS
            let ip_raw: u32 = crate::load!(crate::state::CURRENT_IP);
            let a = (ip_raw >> 24) as u8;
            let b = (ip_raw >> 16) as u8;
            let c = (ip_raw >> 8) as u8;
            let d = ip_raw as u8;
            let ip_str = alloc::format!("{}.{}.{}.{}", a, b, c, d);
            let ip_line = alloc::format!("IP:    {}", ip_str);
            defmt::info!("DEVICE: {}", &*ip_line);

            // TRY TO READ CURRENT CONNECTED SSID            
            let ssid_line = match crate::state::CONNECTED_SSID.try_lock() {
                Ok(guard) => {
                    if let Some(ssid) = guard.as_ref() {   // <-- borrow, don't move
                        alloc::format!("SSID:  {}", ssid)
                    } else {
                        alloc::string::String::from("SSID:  --")
                    }
                }
                Err(_) => alloc::string::String::from("SSID:  --"),
            };

            draw_line(&uptime_line, body_start_y, fb);
            draw_line(&ssid_line, body_start_y + 2 * line_height, fb);
            draw_line(&ip_line, body_start_y + line_height, fb);
        }
        
        // ─────────────────────────────────────────────────────────────────────── 
        // BACKEND INFORMATION
        2 => {
            // HEADER
            let header_y = top_y + 8;
            if header_y >= 0 && header_y < h {
                let header_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                    .font_size(86)
                    .text_color(crate::gui::colors::CYAN)
                    .build();
                let header_align = embedded_graphics::text::TextStyleBuilder::new()
                    .alignment(embedded_graphics::text::Alignment::Center)
                    .build();
                let _ = embedded_graphics::Drawable::draw(
                    &embedded_graphics::text::Text::with_text_style(
                        "BACKEND",
                        embedded_graphics::prelude::Point::new(w / 2, header_y),
                        header_style,
                        header_align,
                    ),
                    fb,
                );
            }

            // BACKEND HOST / PORT
            let host = crate::state::BACKEND_TCP_HOST;
            let port = crate::state::BACKEND_TCP_PORT_STR;
            let host_line = alloc::format!("Host: {}", host);
            let port_line = alloc::format!("Port: {}", port);
            defmt::info!("BACKEND: {}", &*host_line);
            defmt::info!("BACKEND: {}", &*port_line);

            draw_line(&host_line, body_start_y, fb);
            draw_line(&port_line, body_start_y + line_height, fb);
        }
        
        // ─────────────────────────────────────────────────────────────────────── 
        // AUDIO CONFIGURATION INFORMATION
        3 => {
            // HEADER
            let header_y = top_y + 8;
            if header_y >= 0 && header_y < h {
                let header_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                    .font_size(86)
                    .text_color(crate::gui::colors::CYAN)
                    .build();
                let header_align = embedded_graphics::text::TextStyleBuilder::new()
                    .alignment(embedded_graphics::text::Alignment::Center)
                    .build();
                let _ = embedded_graphics::Drawable::draw(
                    &embedded_graphics::text::Text::with_text_style(
                        "I2S",
                        embedded_graphics::prelude::Point::new(w / 2, header_y),
                        header_style,
                        header_align,
                    ),
                    fb,
                );
            }

            // I2S SETTINGS
            let sample_rate = crate::state::I2S_SAMPLE_RATE;
            let sample_count = crate::state::I2S_SAMPLE_COUNT;
            let bit_width = crate::state::I2S_BIT_WIDTH;
            let buffer_size = crate::state::I2S_BUFFER_SIZE;
            let data_format = crate::state::I2S_DATA_FORMAT;
            let endianness = crate::state::I2S_ENDIANNESS;
            let channels = crate::state::I2S_CHANNELS;
            let loopback = crate::state::I2S_SIGNAL_LOOPBACK;

            let data_format_str = alloc::format!("{:?}", data_format);
            let endianness_str = alloc::format!("{:?}", endianness);
            let channels_str = alloc::format!("{:?}", channels);

            let lines = [
                alloc::format!("Sample Rate: {}", sample_rate),
                alloc::format!("Sample Count: {}", sample_count),
                alloc::format!("Bit Width: {}", bit_width),
                alloc::format!("Buffer Size: {}", buffer_size),
                alloc::format!("Data Format: {}", data_format_str),
                alloc::format!("Endianness: {}", endianness_str),
                alloc::format!("Channels: {}", channels_str),
                alloc::format!("Loopback: {}", loopback),
            ];

            for (i, line) in lines.iter().enumerate() {
                defmt::info!("I2S: {}", line.as_str());
                draw_line(line, body_start_y + i as i32 * line_height, fb);
            }
        }
        
        // ─────────────────────────────────────────────────────────────────────── 
        // OS & FIRMWARE INFORMATION
        4 => {
            // HEADER - TWO LINES
            let header_start_y = top_y + 8;

            // PROJECT NAME
            let header_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                .font_size(56)
                .text_color(crate::gui::colors::CYAN)
                .build();
            let header_align = embedded_graphics::text::TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Center)
                .build();

            let _ = embedded_graphics::Drawable::draw(
                &embedded_graphics::text::Text::with_text_style(
                    crate::state::PROJECT_NAME,
                    embedded_graphics::prelude::Point::new(w / 2, header_start_y),
                    header_style,
                    header_align,
                ),
                fb,
            );

            // FIRMWARE VERSION
            let version_style = embedded_ttf::FontTextStyleBuilder::new(body_font.clone())
                .font_size(54)
                .text_color(crate::gui::colors::CYAN)
                .build();
            let version_y = header_start_y + 60;

            let _ = embedded_graphics::Drawable::draw(
                &embedded_graphics::text::Text::with_text_style(
                    &alloc::format!("VERSION: {}", crate::state::FW_VERSION),
                    embedded_graphics::prelude::Point::new(w / 2, version_y),
                    version_style,
                    header_align,
                ),
                fb,
            );

            // GITHUB ICON
            let gh_bytes = crate::base::assets::SETTINGS_GITHUB_PNG;
            if let core::result::Result::Ok(gh_png) =
                embedded_png::Png::load_from_bytes(gh_bytes)
            {
                let gw = gh_png.width() as i32;
                let gh = gh_png.height() as i32;
                let target_gh_h = 40i32;
                let gh_scale = core::cmp::max(1, target_gh_h / gh.max(1));
                let scaled_gw = gw * gh_scale;
                let scaled_gh = gh * gh_scale;
                let gx = (w - scaled_gw) / 2;
                let gy = body_start_y + 2 * line_height;
                let dest = fb.buffer_mut();
                for sy in 0..gh {
                    let py = gy + sy * gh_scale;
                    if py < 0 || py as usize >= screen_h { continue; }
                    for sx in 0..gw {
                        let idx = (sy * gw + sx) as usize;
                        if let core::option::Option::Some(color) = gh_png.pixels()[idx] {
                            let raw: u16 =
                                embedded_graphics_core::pixelcolor::IntoStorage::into_storage(color);
                            let px = gx + sx * gh_scale;
                            for dy in 0..gh_scale {
                                let row = (py + dy) as usize;
                                if row >= screen_h { break; }
                                for dx in 0..gh_scale {
                                    let col = (px + dx) as usize;
                                    if col < screen_w {
                                        dest[row * screen_w + col] = raw;
                                    }
                                }
                            }
                        }
                    }
                }

                // CREDIT LINE (CENTERED – BELOW ICON)
                let credit_y = gy + scaled_gh + 8;
                let credit_align = embedded_graphics::text::TextStyleBuilder::new()
                    .alignment(embedded_graphics::text::Alignment::Center)
                    .build();
                let _ = embedded_graphics::Drawable::draw(
                    &embedded_graphics::text::Text::with_text_style(
                        "QuackHack-McBlindy",
                        embedded_graphics::prelude::Point::new(w / 2, credit_y),
                        red_style,
                        credit_align,
                    ),
                    fb,
                );
            }
        }
        _ => {}
    }
}

