// BASE/ROUTES/API/LIST


// GET /API - RETURN LIST OF ENDPOINTS 
pub fn handle(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    let endpoints = alloc::vec![
        "/",
        "/favicon.ico",
        "/www/{file}",
        "/api",
        "/api/shell/{value}",
        "/api/sensor/{value}",
        "/api/sensors",
        "/api/download/file/music/{filename}",
        "/api/download/file/share/{filename}",
        "/api/upload/file/music/{filename}",
        "/api/media/search/songs/{value}",
        "/api/media/playlist/add/{value}",
        "/api/media/playlist/remove/{value}",
        "/api/media/playlist/clear",      
        "/api/settings/mic/volume/{value}",
        "/api/settings/mic/mute/{value}",
        "/api/settings/speaker/{value}",
        "/api/settings/speaker/stream/{value}",        
        "/api/settings/speaker/volume/{value}",
        "/api/settings/speaker/mute/{value}",
        "/api/settings/voice/{value}",        
        "/api/settings/voice/wakeword/{value}",
        "/api/settings/display/brightness/{value}",
        "/api/settings/display/page/{value}",
        "/api/settings/display/text/{value}",
        "/api/settings/display/call/{value}",
        "/api/settings/wifi/off",
    ];
    tinyapi::Response::text(&endpoints.join("\n"))
}

