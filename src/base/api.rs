// BASE/API
// CONFIGURES `GET` ENDPOINTS VIA `tinyapi`
// FOR CONTROLLING/CONFIGURING THE DEVICE EXTERNALLY
// ++ SERVE WEBSERVER AT `http://0.0.0.0:80`
// EXAMPLE USAGE: (SET DISPLAY BRIGHTNESS TO `70%` USING `curl`) 
// `curl 192.168.1.11:80/api/settings/display/brightness/70`


// ───────────────────────────────────────────────────────────────────────
// FUNCTION TO INIT ENDPOINTS
pub async fn init_routes() {
    // SERVE THE WEB FRONTEND
    tinyapi::register_route("/", crate::base::routes::index::index_handler).await;   
    tinyapi::register_route("/favicon.ico", crate::base::routes::index::favicon_handler).await;    

    tinyapi::register_route("/www/{file}", crate::base::routes::index::serve_file_handler).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API (GET)
    // LIST AVAILABLE ENDPOINTS
    tinyapi::register_route("/api", crate::base::routes::api::list::handle).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API/SHELL (GET)

    // SEND SHELL COMMANDS TO THE DEVICE
    tinyapi::register_route("/api/shell/{value}", crate::base::routes::api::shell::handle_shell).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API/SENSOR (GET)

    // RETURNS SPECIFIC SENSOR DATA
    tinyapi::register_route("/api/sensor/{value}", crate::base::routes::api::sensor::handle_sensor).await;
    
    // ───────────────────────────────────────────────────────────────────────
    // /API/SENSORS (GET)
    
    // RETURNS ALL SENSOR DATA
    tinyapi::register_route("/api/sensors", crate::base::routes::api::sensor::handle_sensors).await;


    // ───────────────────────────────────────────────────────────────────────
    // /API/DOWNLOAD
    
    // FILE/MUSIC/{FILENAME}
    tinyapi::register_stream("/api/download/file/music/{filename}", crate::base::routes::api::download::file::MusicDownload).await;

    // FILE/SHARE/{FILENAME}
    tinyapi::register_stream("/api/download/file/share/{filename}", crate::base::routes::api::download::file::ShareDownload).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API/UPLOAD (POST)
    
    // FILE/MUSIC/{FILENAME}
    tinyapi::register_upload("/api/upload/file/music/{filename}", crate::base::routes::api::upload::file::MusicUpload).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API/MEDIA (GET)
    
    // SEARCH/SONGS/{QUERY}
    tinyapi::register_route("/api/media/search/songs/{value}", crate::base::routes::api::media::search::songs::fuzzy_songs::fuzzy_songs_handler).await;    


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/MIC (GET)
    
    // VOLUME    
    tinyapi::register_route("/api/settings/mic/volume/{value}", crate::base::routes::api::settings::mic::volume::mic_volume_handler).await;
    
    // MUTE
    tinyapi::register_route("/api/settings/mic/mute/{value}", crate::base::routes::api::settings::mic::mute::mic_mute_handler).await;


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/SPEAKER (GET)
    
    // VOLUME    
    tinyapi::register_route("/api/settings/speaker/volume/{value}", crate::base::routes::api::settings::speaker::volume::speaker_volume_handler).await;

    // MUTE    
    tinyapi::register_route("/api/settings/speaker/mute/{value}", crate::base::routes::api::settings::speaker::mute::speaker_mute_handler).await;  

    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/DISPLAY (GET)
    
    // BRIGHTNESS
    tinyapi::register_route("/api/settings/display/brightness/{value}", crate::base::routes::api::settings::display::brightness::brightness_handler).await;    

    // STATE
    // ... (TODO)

    // PAGE
    tinyapi::register_route("/api/settings/display/page/{value}", crate::base::routes::api::settings::display::page::page_handler).await;

    // TEXT
    tinyapi::register_route("/api/settings/display/text/{value}", crate::base::routes::api::settings::display::text::display_string_handler).await;

    // CALL
    tinyapi::register_route("/api/settings/display/call/{value}", crate::base::routes::api::settings::display::call::call_handler).await;

    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/WIFI (GET)
    
    // OFF 
    // ... (TODO)

    // SET/SSID/{SSID}/PASSWORD/{PASSWORD}
    // ... (TODO)
    

    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/BLUETOOTH (GET)

    // ... (TODO)    

    // ───────────────────────────────────────────────────────────────────────
    // ... (TODO MORE)


}
