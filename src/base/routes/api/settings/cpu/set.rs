// BASE/ROUTES/API/SETTINGS/CPU/SET
// SET CPU FREQUENCY AT RUNTIME (MhZ)


pub async fn cpu_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let current = crate::load!(crate::state::CPU_FREQ);

    let new_mhz = match req.param("value").unwrap_or("toggle") {
        "80" => 80u16,
        "160" => 160u16,
        "240" => 240u16,
        "toggle" => {
            // CYCLE: 80 > 160 > 240 > 80
            match current {
                80 => 160,
                160 => 240,
                _ => 80,
            }
        },
        _ => current,
    };

    crate::components::frequency::set_cpu_mhz(new_mhz);
    crate::store!(crate::state::CPU_FREQ, new_mhz);

    let msg = alloc::format!("CPU set to: {} MHz", new_mhz);
    defmt::info!("🖥️ CPU frequency: {} MHz", new_mhz);

    tinyapi::Response::text(&msg)
}
