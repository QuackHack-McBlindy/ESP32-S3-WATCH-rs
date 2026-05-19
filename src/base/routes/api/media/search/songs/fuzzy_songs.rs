// BASE/ROUTES/API/MEDIA/SEARCH/SONGS/FUZZY_SONGS

use alloc::format;

// FUZZY SEARCH
pub fn fuzzy_songs_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    match crate::components::storage::generate_playlist(value) {
        Ok(path) => tinyapi::Response::text(&format!("Playlist created at {}", path)),
        Err(_) => tinyapi::Response::text("Song not found or error generating playlist"),
    }
}
