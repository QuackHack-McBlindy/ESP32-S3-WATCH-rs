// BASE/ROUTES/OTA
// GET /OTA - OVER THE AIR UPDATES (TODO?)
pub fn ota_handler(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    defmt::info!("OTA update requested");
    tinyapi::Response::text("update started")
}
