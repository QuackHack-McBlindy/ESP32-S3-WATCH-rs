// APPLICATIONS/MOD

// ───────────────────────────────────────────────────────────────────────
// LOAD APP MODULES
pub mod media_player; // QWACKIFY
pub mod duck_tv;
pub mod duckcloud;
pub mod settings;
pub mod house;
pub mod tinyweather;


// ───────────────────────────────────────────────────────────────────────
// DESCRIBES AN APPLICATION
pub struct AppDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub launch: fn(),
    pub icon: &'static [u8],
}

// ───────────────────────────────────────────────────────────────────────
// FETCH ALL APPDESCRIPTIORS
// APP LAUNCHER LIST APPS IN SAME ORDER AS THEY'RE LISTED HERE!
pub static APPS: &[AppDescriptor] = &[
    media_player::APP_DESCRIPTOR,
    duck_tv::APP_DESCRIPTOR,
    house::APP_DESCRIPTOR,
    duckcloud::APP_DESCRIPTOR,
    settings::APP_DESCRIPTOR,
    tinyweather::APP_DESCRIPTOR,
];    
