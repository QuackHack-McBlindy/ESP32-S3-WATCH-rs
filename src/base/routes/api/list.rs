// BASE/ROUTES/API/LIST


// GET /API - RETURN LIST OF ENDPOINTS 
pub fn handle(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    let endpoints = alloc::vec![
        "/",
        "/favicon.ico",
        "/api",
        "/api/media/search/songs/{value}",
        "/api/settings/mic/volume/{value}",
        "/api/settings/mic/mute/{value}",
        "/api/settings/speaker/volume/{value}",
        "/api/settings/speaker/mute/{value}",
        "/api/settings/display/brightness/{value}",
        "/api/settings/display/page/{value}",
        "/api/sensors",
        "/api/sensor/{value}",
    ];
    tinyapi::Response::text(&endpoints.join("\n"))
}

