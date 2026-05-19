// GET /INDEX.HTML - SERVES WEB FRONTEND 
pub fn index_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
   tinyapi::Response::html(include_str!("./../../../www/index.html"))
}

// GET /FAVICON.ICO - SERVES FRONTEND FAVICON
pub fn favicon_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    tinyapi::Response::favicon(include_bytes!("./../../../www/favicon.ico"))
}


pub fn serve_file_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let filename = req.param("file").unwrap_or("unknown.mp3");
    let path = alloc::format!("/Music/{}", filename);

    match crate::components::storage::read_file_to_vec(&path) {
        Ok(data) => {
            let mime = if filename.ends_with(".mp3") {
                "audio/mpeg"
            } else if filename.ends_with(".wav") {
                "audio/wav"
            } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                "image/jpeg"
            } else if filename.ends_with(".png") {
                "image/png"
            } else {
                "application/octet-stream"
            };
            tinyapi::Response::binary(mime, data)
        }
        Err(_) => tinyapi::Response::not_found(),
    }
}
