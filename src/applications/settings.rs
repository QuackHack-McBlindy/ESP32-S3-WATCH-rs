// APPLICATIONS/SETTINGS
// CONTROL DEVICE OPTIONS - LIKE WiFi, BT, & TOGGLE TASKS ON/OFF, etc etc

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "Settings",
    description: "Tuggle device settings from within this application",
    launch: open_app,
    icon: crate::base::assets::SETTINGS_PNG,
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 140);
}

// ───────────────────────────────────────────────────────────────────────
// WORK IN PROGRESS

pub async fn wifi_on() {
    defmt::info!("WIFI ON");
}

pub async fn wifi_off() {
    defmt::info!("WIFI OFF");
}
