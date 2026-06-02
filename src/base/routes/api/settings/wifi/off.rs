// SRC/BASE/ROUTES/API/SETTINGS/WIFI/OFF


pub fn disable_wifi(req: tinyapi::Request<'_>) -> tinyapi::Response {    
    crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Disable);
    crate::store!(crate::state::WIFI_STATE, false);
    defmt::info!("Disabled WiFi");
    tinyapi::Response::text("Disabled wifi")
}
