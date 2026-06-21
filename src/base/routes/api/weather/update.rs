// BASE/ROUTES/API/WEATHER/UPDATE


pub async fn weather_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::applications::tinyweather::update().await;
    tinyapi::Response::text("updated weather data")
}
