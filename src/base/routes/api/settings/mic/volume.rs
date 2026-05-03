// SRC/BASE/ROUTES/API/SETTINGS/MIC/VOLUME

use alloc::format; 

pub fn set_mic_gain(gain_db: u8) {
    critical_section::with(|cs| {
        let mut bus_ref = crate::I2C_BUS.borrow_ref_mut(cs);
        let bus = bus_ref.as_mut().expect("I2C BUS missing");
        let mut es7210_ref = crate::ES7210.borrow_ref_mut(cs);
        let es7210 = es7210_ref.as_mut().expect("ES7210 missing");
        if let Err(e) = es7210.gain_set(bus, gain_db as i8) {
            defmt::error!("Mic gain set failed: {:?}", defmt::Debug2Format(&e));
        }
        crate::store!(crate::state::MIC_VOLUME, gain_db);
    });
}

pub fn mic_volume_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        crate::store!(crate::state::MIC_VOLUME, vol);
        set_mic_gain(vol);
        defmt::info!("Mic volume set to {}%", vol);
    }
    tinyapi::Response::text(&format!("Mic volume {}", value))
}


