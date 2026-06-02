// BASE/ROUTES/API/SETTINGS/SPEAKER/STREAM


// TOGGLE THE STREAMING MODE ON OR OFF
pub async fn toggle_stream() {
    let current = crate::load!(crate::state::SPEAKER_ALLOW_STREAMING);
    if !current {
        // TURN STREAMING ON
        yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
        crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true);
    } else {
        // TURN STREAMING OFF
        yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Stop).await;
        crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, false);
    }
}



pub async fn stream_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "start" | "enable" | "enabled" => crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true),
        "0" | "off" | "stop" | "disable" | "disabled" => crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, false),
        _ => { }
    }
        
    let new = crate::load!(crate::state::SPEAKER_ALLOW_STREAMING);
    let msg = match new {
        true => {
            let speaker = crate::load!(crate::state::SPEAKER_TASK_STATE);
            let speaker_state = match speaker {
                true => { }
                false => { yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Start).await; }    
            };
            
            yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
            crate::amp_on();
            defmt::info!("Streaming audio to the speaker task is now allowed!");      
            "Streaming audio allowed"
        }
        false => {
            yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Stop).await;
            crate::amp_off();
            defmt::info!("Streaming audio to the speaker task is no longer allowed!");   
            "Streaming audio no longer allowed!"
        }
    };
    tinyapi::Response::text(msg)
}
