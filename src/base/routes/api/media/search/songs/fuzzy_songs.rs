// BASE/ROUTES/API/MEDIA/SEARCH/SONGS/FUZZY_SONGS

use alloc::format;
use crate::applications::media_player;

// FUZZY SEARCH SONGS (ADDS 10 CLOSEST MATCHES TO PLAYLIST)
pub async fn fuzzy_songs_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if !crate::load!(crate::state::SD_READY) {
        crate::components::storage::ensure_sd_ready();
        embassy_time::Timer::after_secs(1).await;
    }    
    match crate::components::storage::generate_playlist(value) {
        Ok(path) => {
            match media_player::load_playlist_from_sd(&path) {
                Ok(()) => {
                    defmt::debug!("Playlist written to: {}", path.as_str());
                    if crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
                        crate::set_speaker_volume(65);
                        embassy_time::Timer::after_millis(200).await;
                    }
                    crate::applications::media_player::play().await;
                    embassy_time::Timer::after_millis(500).await;
                    crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::MediaPlayer as u8);
                    //embassy_time::Timer::after_millis(100).await;
                    //crate::DISPLAY_CMD.send(crate::DisplayCommand::Start).await;
                    //crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
                    tinyapi::Response::text(&format!("Playlist created, playback started"))
                } 
                Err(e) => {
                    defmt::error!("Failed to load playlist from {}: {:?}", path.as_str(), e);
                    tinyapi::Response::text(&format!("Playlist file created but failed to load: {:?}", e))
                }    
            }
        }
        Err(e) => {
            defmt::error!("generate_playlist failed: {:?}", e);
            tinyapi::Response::text("Song not found or error generating playlist")
        }
    }
}
