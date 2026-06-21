// BASE/ROUTES/API/SETTINGS/SLEEP/NOW


pub fn sleep_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    crate::deep_sleep_now();
    tinyapi::Response::text("sleepy time")
}
