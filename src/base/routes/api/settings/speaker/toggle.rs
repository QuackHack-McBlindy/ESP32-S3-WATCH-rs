// BASE/ROUTES/API/SETTINGS/SPEAKER/TOGGLE
pub async fn toggle_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "start" | "enable" | "enabled" => crate::store!(crate::state::SPEAKER_TASK_STATE, true),
        "0" | "off" | "stop" | "disable" | "disabled" => crate::store!(crate::state::SPEAKER_TASK_STATE, false),
        _ => { }
    }
    let new = crate::load!(crate::state::SPEAKER_TASK_STATE);
    let msg = match new {
        true => {
            yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Start).await;
            defmt::info!("Speaker task started!");      
            "Speaker task started"
        }
        false => {
            yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Stop).await;
            defmt::info!("Speaker task stopped!");
            "Speaker task stopped"
        }
    };
    tinyapi::Response::text(msg)
}
