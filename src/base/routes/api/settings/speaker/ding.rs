// BASE/ROUTES/API/SETTINGS/SPEAKER/DING
// PLAY DING SOUND (USED FOR TESTING)


pub async fn ding_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    yo_esp::play_ding().await;
    let msg = alloc::format!("Played ding sound");
    tinyapi::Response::text(&msg)
}
