// APPLICATIONS/SETTINGS
// CONTROL DEVICE OPTIONS - LIKE WiFi, BT, & TOGGLE TASKS ON/OFF, etc etc

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "Settings",
    description: "Tuggle device settings from within this application",
    grid_position: crate::applications::GridSlot::TopRight,
    launch: open_app,
    icon: crate::base::assets::SETTINGS_PNG,
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 10);
}

// ───────────────────────────────────────────────────────────────────────
// WORK IN PROGRESS
