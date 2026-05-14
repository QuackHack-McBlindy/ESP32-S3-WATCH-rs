// GET /INDEX.HTML - SERVES WEB FRONTEND 
pub fn index_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
   tinyapi::Response::html(include_str!("./../../../www/index.html"))
}

// GET /FAVICON.ICO - SERVES FRONTEND FAVICON
pub fn favicon_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::favicon(include_bytes!("./../../../www/favicon.ico"))
}
