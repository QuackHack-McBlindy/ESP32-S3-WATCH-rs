// SRC/BASE/ROUTES/API/SETTINGS/WIFI/SCAN


pub async fn scan_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Scan).await;
    tinyapi::Response::text("scanning for wifi networks...")
}
