// GUI/MOD

// ───────────────────────────────────────────────────────────────────────
// LOAD MODULES
pub mod colors;
pub mod pages;
pub mod apps;
pub mod control_center;
pub mod time;
pub mod battery;
pub mod media_player; // QWACKIFY
pub mod duck_tv;
pub mod duckcloud;
pub mod call;
pub mod text;
pub mod settings;
pub mod options;
pub mod weather;
pub mod input;
pub mod gallery;


// ───────────────────────────────────────────────────────────────────────
// SHARED HELPERS
pub(crate) static mut ROBOTO_BOLD_FONT: Option<rusttype::Font<'static>> = None;

// DRAW A TEXT
// USAGE: crate::gui::draw_text(fb, x, y, font_size, string);
pub fn draw_text(
    fb: &mut crate::components::framebuffer::Framebuffer,
    x: i32,
    y: i32,
    font_size: u32,
    text: &str,
) {
    let font = critical_section::with(|_| unsafe {
        let ptr = core::ptr::addr_of!(ROBOTO_BOLD_FONT);
        (*ptr).as_ref().unwrap().clone()
    });

    let ttf_style = embedded_ttf::FontTextStyleBuilder::new(font)
        .font_size(font_size)
        .text_color(crate::gui::colors::WHITE)
        .build();

    let text_pos = embedded_graphics::geometry::Point::new(x, y);
    let text_render = embedded_graphics::text::Text::new(text, text_pos, ttf_style);

    <embedded_graphics::text::Text<
        embedded_ttf::FontTextStyle<embedded_graphics::pixelcolor::Rgb565>,
    > as embedded_graphics::prelude::Drawable>::draw(&text_render, fb)
        .ok();
}



// HIT AREA (X, Y, WIDTH, HEIGHT MAP > ACTION)
#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub struct HitArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub action: TouchAction,
}


#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum TouchAction {
    None,
    // APP CENTER QUICK ACTIONS
    AppCenterApp(usize),
    AppLaunch(usize),
    // CONTROL CENTER QUICK ACTIONS
    ControlCenterBox1,
    ControlCenterBox2,
    ControlCenterBox3,
    ControlCenterBox4,
    // OPEN APPS
    OpenQwackify,
    OpenSettings,
    OpenDuckTv,
    OpenDuckCloud,
    // CALL PAGE
    CallAccept,
    CallDecline,
    // TEXT INPUT PAGE
    TextInputCacel,
    TextInputOk,
    // MEDIA PLAYER PAGE
    MediaPrev,
    MediaPlayPause,
    MediaNext,
    MediaHeart,
    MediaClear,
    MediaSplitView,
    // SETTINGS
    SettingsToggle,
    SettingsToggleWifi,
    SettingsToggleWifiScan,
    SettingsToggleBle,    
    SettingsToggleApi,
    SettingsToggleAmp,
    SettingsToggleMic,
    SettingsToggleSpeaker,
    SettingsToggleStreaming,
    SettingsToggleSsh,
    SettingsToggleVpn,
    SettingsToggleWakeWord,
    SettingsToggleDisplay,            
}

pub fn hit_test(x: i32, y: i32, area: &HitArea) -> bool {
    x >= area.x && x < area.x + area.width as i32 &&
    y >= area.y && y < area.y + area.height as i32
}


// ───────────────────────────────────────────────────────────────────────
// ASYNC TE-SYNCED FLUSH FOR SMOOTH, TEAR-FREE FRAMES

// WAITS FOR TE (tearing effect) VBLANK SIGNAL, THE FLUSH FRAMEBUFFER.
pub async fn flush_vsync_async(
    fb: &mut crate::components::framebuffer::Framebuffer,
    display: &mut crate::components::co5300::Co5300Display<'_>,
    te: &esp_hal::gpio::Input<'_>,
) {
    // WAIT FOR TE LOW > HIGH (START OF OF VBLANK)
    while te.is_high() {
        embassy_time::Timer::after_micros(100).await;
    }
    while te.is_low() {
        embassy_time::Timer::after_micros(100).await;
    }
    crate::dirty!();
    fb.flush(display);
    crate::dirty!();
}


