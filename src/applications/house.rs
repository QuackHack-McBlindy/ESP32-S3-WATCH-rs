// APPLICATIONS/HOUSE
// SIMPLE APP CONTROLLER FOR MY HOUSE & HOME DEVICES
// ++ SENSORS ++ MORE

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "House",
    description: "Smart Home controller application for Zigbee devices, TVs, PCs, sensors etc",
    launch: open_app,
    icon: crate::base::assets::HOUSE_PNG,
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 12);
}

// ───────────────────────────────────────────────────────────────────────
// WORK IN PROGRESS
