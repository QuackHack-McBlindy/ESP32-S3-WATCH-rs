// BASE/ROUTES/API/MEDIA/HEART

pub async fn heart_song_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::state::MEDIA_COMMAND.store(
        crate::state::MediaCommand::Heart as u8,
        core::sync::atomic::Ordering::Relaxed,
    );
    tinyapi::Response::text("Song added to favourites")
}
