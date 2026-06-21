// BASE/ROUTES/API/MEDIA/SEARCH/SONG/FUZZY_SONG

use alloc::format;


// FUZZY SEARCH (FINDS BEST MATCH)
pub async fn fuzzy_song_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if !crate::load!(crate::state::SD_READY) {
        crate::components::storage::ensure_sd_ready();
        embassy_time::Timer::after_secs(1).await;
    }
    match crate::components::storage::search_song(value) {
        Some((path, _score)) => {
            if crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
                crate::set_speaker_volume(65);
                embassy_time::Timer::after_secs(1).await;
            }
            crate::store!(crate::gui::pages::CURRENT_PAGE, 10);
            tinyapi::Response::text(&format!("Found song: {}", path))
        }
        None => tinyapi::Response::text("Song not found or error generating playlist"),
    }
}
