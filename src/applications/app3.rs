// APPLICATIONS/APP3
// PLACEHOLDER APPLICATION

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "App 3",
    description: "Placeholder application",
    grid_position: crate::applications::GridSlot::BottomLeft,
    launch: open_app,
    icon: crate::base::assets::APP3_PNG,
};


pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 10);
}
