// BASE/ROUTES/API/SETTINGS/VPN/STATE


// TOGGLE WIREGUARD VPN ON/OFF
pub async fn vpn_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    let new_state = match value {
        "1" | "on" | "start" | "enable" | "enabled" => Some(true),
        "0" | "off" | "stop" | "disable" | "disabled" => Some(false),
        _ => None,
    };

    let final_state = new_state.unwrap_or_else(|| !crate::load!(crate::state::WG_STATE));
    crate::store!(crate::state::WG_STATE, final_state);

    if final_state {
        crate::base::wireguard::WG_CMD.send(crate::base::wireguard::WgCommand::Enable).await;
        defmt::info!("VPN on!");
    } else {
        crate::base::wireguard::WG_CMD.send(crate::base::wireguard::WgCommand::Disable).await;
        defmt::info!("VPN off!");
    }

    tinyapi::Response::text(if final_state { "VPN on!" } else { "VPN off!" })
}
