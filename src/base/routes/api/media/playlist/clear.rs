// BASE/ROUTES/API/MEDIA/PLAYLIST/CLEAR

use alloc::format;

// CLEAR THE ENTIRE PLAYLIST
pub fn clear_playlist_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    match crate::components::storage::clear_playlist() {
        Ok(()) => tinyapi::Response::text("Playlist cleared"),
        Err(_) => tinyapi::Response::text("Failed to clear playlist"),
    }
}
