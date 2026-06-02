// BASE/ROUTES/API/MEDIA/PLAY_PAUSE

pub async fn play_pause_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::state::MEDIA_COMMAND.store(
        crate::state::MediaCommand::PlayPause as u8,
        core::sync::atomic::Ordering::Relaxed,
    );
    tinyapi::Response::text("Play/Pause toggled")
}
