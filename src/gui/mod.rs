// GUI/MOD

pub mod pages;
pub mod apps;
pub mod time;
pub mod battery;
pub mod house;
pub mod media_player;

// HIT AREA (X, Y, WIDTH, HEIGHT MAP > ACTION)
pub struct HitArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub action: TouchAction,
}

#[derive(Clone, Copy, defmt::Format)]
pub enum TouchAction {
    None,
    MediaPrev,
    MediaPlayPause,
    MediaNext,
    OpenQwackify,
    OpenApp2,
    OpenApp3,
    OpenHouse,
    ZigbeeToggleLights,
}

pub fn hit_test(x: i32, y: i32, area: &HitArea) -> bool {
    x >= area.x && x < area.x + area.width as i32 &&
    y >= area.y && y < area.y + area.height as i32
}
