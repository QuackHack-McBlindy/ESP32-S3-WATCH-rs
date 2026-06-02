// APPLICATIONS/DUCK_TV
// DUCK-TV APPLICATION -- DUCK-TV IS MY PERSONAL TVOS/ANDROID/BROWSER LOCAL MEDIA STREAMING APP
// HERE I WILL TRY TO IMPLEMENT ANY USECASES THAT ARISES FOR IT. 
// AS FULL CONTROL IS ALREADYT AVAILABLE THROUGH VOICE COMMANDS - THE USECASE SCENARIO IS THE CHALLANGE. 

// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "duck-tv",
    description: "duck-tv watch controller",
    launch: open_app,
    icon: crate::base::assets::DUCK_TV_PNG,
};


pub fn open_app() {
    crate::store!(crate::gui::pages::CURRENT_PAGE, 11);
}
