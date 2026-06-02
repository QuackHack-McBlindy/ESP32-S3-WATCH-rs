// BASE/ROUTES/API/MEDIA/PLAYLIST/REMOVE

use alloc::format;

// REMOVE A SONG FROM THE MUSIC PLAYLIST.
pub fn remove_from_playlist_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let song = match req.param("value") {
        Some(s) if !s.is_empty() => s,
        _ => return tinyapi::Response::text("Missing song name"),
    };

    match crate::components::storage::remove_from_playlist(song) {
        Ok(()) => tinyapi::Response::text(&format!("Removed '{}' from playlist", song)),
        Err(_) => tinyapi::Response::text("Failed to remove song from playlist"),
    }
}
