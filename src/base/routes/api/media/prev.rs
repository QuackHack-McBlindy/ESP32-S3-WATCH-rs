// BASE/ROUTES/API/MEDIA/PREV

pub async fn prev_track_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::state::MEDIA_COMMAND.store(
        crate::state::MediaCommand::Prev as u8,
        core::sync::atomic::Ordering::Relaxed,
    );
    tinyapi::Response::text("Previous track")
}
