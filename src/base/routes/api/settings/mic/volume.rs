// SRC/BASE/ROUTES/API/SETTINGS/MIC/VOLUME

use alloc::format; 

pub fn mic_volume_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        crate::set_mic_gain(vol);
    }
    tinyapi::Response::text(&format!("Mic gain {}", value))
}


