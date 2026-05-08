// SRC/BASE/ROUTES/API/SETTINGS/SPEAKER/VOLUME
use alloc::format;

pub fn speaker_volume_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        crate::set_speaker_volume(vol);
    }
    tinyapi::Response::text(&format!("Speaker volume {}", value))
}
