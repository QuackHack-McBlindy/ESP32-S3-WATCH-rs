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

    // PLAYLIST/ADD/{QUERY}
    tinyapi::register_route("/api/media/playlist/add/{value}", crate::base::routes::api::media::playlist::append::add_to_playlist_handler).await;    

    // PLAYLIST/REMOVE/{QUERY}
    tinyapi::register_route("/api/media/playlist/remove/{value}", crate::base::routes::api::media::playlist::remove::remove_from_playlist_handler).await;  

    // PLAYLIST/CLEAR
    tinyapi::register_route("/api/media/playlist/clear", crate::base::routes::api::media::playlist::clear::clear_playlist_handler).await;  



    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/API (GET)

    // OFF
    tinyapi::register_async_route("/api/settings/api/off", crate::base::routes::api::settings::api::off::disable_api).await;  


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/CPU (GET)
    
    // SET CPU FREQUENCY (80, 160, 240)
    tinyapi::register_async_route("/api/settings/cpu/{value}", crate::base::routes::api::settings::cpu::set::cpu_handler).await;
    


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/MIC (GET)
    
    // VOLUME    
    tinyapi::register_route("/api/settings/mic/volume/{value}", crate::base::routes::api::settings::mic::volume::mic_volume_handler).await;
    
    // MUTE
    tinyapi::register_route("/api/settings/mic/mute/{value}", crate::base::routes::api::settings::mic::mute::mic_mute_handler).await;


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/SPEAKER (GET)

    // ON/OFF
    tinyapi::register_async_route("/api/settings/speaker/{value}", crate::base::routes::api::settings::speaker::toggle::toggle_handler).await;
    
    // VOLUME (0-100)  
    tinyapi::register_route("/api/settings/speaker/volume/{value}", crate::base::routes::api::settings::speaker::volume::speaker_volume_handler).await;

    // MUTE (on/off)  
    tinyapi::register_route("/api/settings/speaker/mute/{value}", crate::base::routes::api::settings::speaker::mute::speaker_mute_handler).await;  

    // AMP (on/off/toggle)  
    tinyapi::register_route("/api/settings/speaker/amp/{value}", crate::base::routes::api::settings::speaker::amp::amp_handler).await;  


    // STREAM (on/off)
    tinyapi::register_async_route("/api/settings/speaker/stream/{value}", crate::base::routes::api::settings::speaker::stream::stream_handler).await;  

    // DING (PLAYS SOUND)
    tinyapi::register_async_route("/api/settings/speaker/play/ding", crate::base::routes::api::settings::speaker::ding::ding_handler).await;  


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/VOICE (GET)

    // ON/OFF/TOGGLE (THE ENTIRE PIPELINE)
    tinyapi::register_async_route("/api/settings/voice/{value}", crate::base::routes::api::settings::voice::state::voice_handler).await;
        
    // WAKEWORD (on/off) 
    tinyapi::register_async_route("/api/settings/voice/wakeword/{value}", crate::base::routes::api::settings::voice::wakeword::wake_word_handler).await;
    

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

    // REDRAW
    tinyapi::register_route("/api/settings/display/redraw", crate::base::routes::api::settings::display::redraw::display_redraw_handler).await;


    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/WIFI (GET)
    
    // OFF 
    tinyapi::register_route("/api/settings/wifi/off", crate::base::routes::api::settings::wifi::off::disable_wifi).await;

    // SET/SSID/{SSID}/PASSWORD/{PASSWORD}
    // ... (TODO)
    

    // ───────────────────────────────────────────────────────────────────────
    // /API/SETTINGS/BLUETOOTH (GET)

    // ... (TODO)    

    // ───────────────────────────────────────────────────────────────────────
    // ... (TODO MORE)


}
