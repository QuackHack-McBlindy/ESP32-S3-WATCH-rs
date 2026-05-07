// SRC/BASE/ROUTES/API/MEDIA/SEARCH/SONGS/FUZZY_SONGS

use alloc::format;

// 
pub fn fuzzy_songs_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    match crate::components::storage::search_song(value) {
        Some((name, score)) =>
            tinyapi::Response::text(&format!("{} ({})", name, score)),
            // PLAY THE BEST MATCH
            //crate::spawn!(spawner, crate::applications::media_player::play_mp3_task(alloc::string::ToString::to_string(name)));
        None =>
            tinyapi::Response::text("Song not found"),
    }
}
