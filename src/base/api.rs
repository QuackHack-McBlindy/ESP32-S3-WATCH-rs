// BASE/API
// CONFIGURES `GET` ENDPOINTS VIA `tinyapi`
// FOR CONTROLLING/CONFIGURING THE DEVICE EXTERNALLY
// ++ SERVE WEBSERVER AT `http://0.0.0.0:80`
// EXAMPLE USAGE: (SET DISPLAY BRIGHTNESS TO `70%` USING `curl`) 
// `curl 192.168.1.11:80/api/settings/display/brightness/70`


// FUNCTION TO INIT ENDPOINTS
pub async fn init_routes() {
    // SERVE THE WEB FRONTEND
    tinyapi::register_route("/", crate::base::routes::index::index_handler).await;   
    tinyapi::register_route("/favicon.ico", crate::base::routes::index::favicon_handler).await;    

    // LIST AVAILABLE ENDPOINTS
    tinyapi::register_route("/api", crate::base::routes::api::list::handle).await;

    // /API/MEDIA/SEARCH/SONGS/{query}
    tinyapi::register_route("/api/media/search/songs/{value}", crate::base::routes::api::media::search::songs::fuzzy_songs::fuzzy_songs_handler).await;    

    // /API/SETTINGS/MIC    
    tinyapi::register_route("/api/settings/mic/volume/{value}", crate::base::routes::api::settings::mic::volume::mic_volume_handler).await;
    tinyapi::register_route("/api/settings/mic/mute/{value}", crate::base::routes::api::settings::mic::mute::mic_mute_handler).await;

    // /API/SETTINGS/SPEAKER    
    tinyapi::register_route("/api/settings/speaker/volume/{value}", crate::base::routes::api::settings::speaker::volume::speaker_volume_handler).await;    
    tinyapi::register_route("/api/settings/speaker/mute/{value}", crate::base::routes::api::settings::speaker::mute::speaker_mute_handler).await;  

    // /API/SETTINGS/DISPLAY
    tinyapi::register_route("/api/settings/display/brightness/{value}", crate::base::routes::api::settings::display::brightness::brightness_handler).await;    

    tinyapi::register_route("/api/settings/display/page/{value}", crate::base::routes::api::settings::display::page::page_handler).await;    
    
  
    // RETURNS ALL SENSOR DATA
    tinyapi::register_route("/api/sensors", crate::base::routes::api::sensor::handle_sensors).await;
    // RETURNS SPECIFIC SENSOR DATA
    tinyapi::register_route("/api/sensor/{value}", crate::base::routes::api::sensor::handle_sensor).await;

}
