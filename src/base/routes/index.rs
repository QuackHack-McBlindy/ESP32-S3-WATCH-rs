// GET /INDEX.HTML - SERVES WEB FRONTEND 
pub fn index_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
   tinyapi::Response::html(include_str!("./../../../assets/index.html"))
}

// GET /FAVICON.ICO - SERVES FRONTEND FAVICON
pub fn favicon_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    //tinyapi::Response::file(include_bytes!("./../../assets/favicon.ico"));
    tinyapi::Response::not_found()    
}

// GET /SCRIPT.JS - SERVES FRONTEND JAVASCRIPT
pub fn js_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::script(include_str!("./../../../assets/script.js"))
}
