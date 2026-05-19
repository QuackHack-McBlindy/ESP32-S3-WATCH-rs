// APPLICATIONS/MOD


// ───────────────────────────────────────────────────────────────────────
// LOAD APP MODULES
pub mod media_player;
pub mod settings;
pub mod app3;
pub mod house;


// ───────────────────────────────────────────────────────────────────────
// DESCRIBES AN APPLICATION
pub struct AppDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub grid_position: GridSlot,
    pub launch: fn(),
    pub icon: &'static [u8],
}

// ENUM FOR APP ICON GRID LOCATION
#[derive(Clone, Copy, PartialEq)]
pub enum GridSlot {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}


// ───────────────────────────────────────────────────────────────────────
// FETCH ALL APPDESCRIPTIORS
pub static APPS: &[AppDescriptor] = &[
    media_player::APP_DESCRIPTOR,
    settings::APP_DESCRIPTOR,
    app3::APP_DESCRIPTOR,
    house::APP_DESCRIPTOR,
];    
