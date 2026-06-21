// BASE/ROUTES/API/SETTINGS/SSH/STATE


// TOGGLE SSHD ON/OFF
pub async fn ssh_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    let new_state = match value {
        "1" | "on" | "start" | "enable" | "enabled" => Some(true),
        "0" | "off" | "stop" | "disable" | "disabled" => Some(false),
        _ => None,
    };

    let final_state = new_state.unwrap_or_else(|| !crate::load!(crate::state::SSH_STATE));
    crate::store!(crate::state::SSH_STATE, final_state);

    if final_state {
        crate::base::ssh::SSH_CMD.send(crate::base::ssh::SshCommand::Enable).await;
        defmt::info!("sshd on!");
    } else {
        crate::base::ssh::SSH_CMD.send(crate::base::ssh::SshCommand::Disable).await;
        defmt::info!("sshd off!");
    }

    tinyapi::Response::text(if final_state { "sshd on!" } else { "sshd off!" })
}
