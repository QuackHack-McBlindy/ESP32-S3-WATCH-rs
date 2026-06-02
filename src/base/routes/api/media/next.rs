// BASE/ROUTES/API/MEDIA/NEXT

pub async fn next_track_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::state::MEDIA_COMMAND.store(
        crate::state::MediaCommand::Next as u8,
        core::sync::atomic::Ordering::Relaxed,
    );
    tinyapi::Response::text("Next track")
}
