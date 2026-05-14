// APPLICATIONS/APP2
// PLACEHOLDER APPLICATION

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "App 2",
    description: "Placeholder application",
    grid_position: crate::applications::GridSlot::TopRight,
    launch: open_app,
    icon: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/apps/app2.png")),
};

pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 10);
}
