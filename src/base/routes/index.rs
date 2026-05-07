// GET /INDEX.HTML - SERVES WEB FRONTEND 
pub fn index_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
   tinyapi::Response::html(include_str!("./../../../www/index.html"))
}

// GET /FAVICON.ICO - SERVES FRONTEND FAVICON
pub fn favicon_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::favicon(include_bytes!("./../../../www/favicon.ico"))
    //tinyapi::Response::not_found()    
}

// GET /STATIC/CSS/STYLES.CSS - SERVES FRONTEND STYLESHEETS (CSS)
pub fn css_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::stylesheet(include_str!("./../../../www/static/css/styles.css"))
}

// GET /SCRIPT.JS - SERVES FRONTEND JAVASCRIPT
pub fn js_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::script(include_str!("./../../../www/static/js/player.js"))
}
