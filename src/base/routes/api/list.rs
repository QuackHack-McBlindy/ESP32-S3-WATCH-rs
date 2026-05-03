// GET /API - RETURN LIST OF ENDPOINTS 
pub fn handle(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    let endpoints = alloc::vec![
        "/",
        "/api/settings/power/state/{value}",
        "/api/settings/display/state/{value}",
        // ...
    ];
    tinyapi::Response::text(&endpoints.join("\n"))
}

