// BASE/ROUTES/API/SETTINGS/SLEEP/RESET


pub fn reset_timer_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    crate::base::timer::reset();
    tinyapi::Response::text("Reset the timer")
}
