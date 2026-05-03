// SRC/BASE/ROUTES/API/SETTINGS/SPEAKER/VOLUME
use alloc::format;

pub fn set_speaker_volume(vol: u8) {
    critical_section::with(|cs| {
        let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
        let bus = bus_ref.as_mut().unwrap();
        let mut es8311_ref = crate::ES8311.borrow_ref_mut(cs);
        let es8311 = es8311_ref.as_mut().unwrap();
        if let Err(e) = es8311.volume_set(bus, vol, None) {
            defmt::error!("Speaker vol set failed: {:?}", defmt::Debug2Format(&e));
        }
        crate::store!(crate::state::SPEAKER_VOLUME, vol);
    });
}

pub fn speaker_volume_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        crate::store!(crate::state::SPEAKER_VOLUME, vol);
        set_speaker_volume(vol);
        defmt::info!("Speaker volume set to {}%", vol);
    }
    tinyapi::Response::text(&format!("Speaker volume {}", value))
}
