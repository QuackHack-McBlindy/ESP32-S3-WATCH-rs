// BASE/ROUTES/API/MEDIA/PLAYLIST/APPEND

use alloc::format;

// APPEND A SONG TO THE MUSIC PLAYLIST.
pub fn add_to_playlist_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let song = match req.param("value") {
        Some(s) if !s.is_empty() => s,
        _ => return tinyapi::Response::text("Missing song name"),
    };

    match crate::components::storage::append_to_playlist(song) {
        Ok(()) => tinyapi::Response::text(&format!("Added '{}' to playlist", song)),
        Err(_) => tinyapi::Response::text("Failed to add song to playlist"),
    }
}
