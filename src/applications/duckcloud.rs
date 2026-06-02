// APPLICATIONS/DUCKCLOUD
// DUCKCLOUD APPLICATION
// PERSONAL LOCAL CLOUD STORAGE

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "duckcloud",
    description: "duckcloud provides access to the homeservers storage on the watch",
    launch: open_app,
    icon: crate::base::assets::DUCKCLOUD_PNG,
};


pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 13);
}
