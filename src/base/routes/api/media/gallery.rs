// BASE/ROUTERS/API/MEDIA/GALLERY

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;



pub async fn gallery_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let path = req.param("value").unwrap_or("/share/profile.png");

    crate::gui::gallery::set_current_image(path);
    crate::dirty!();
    crate::store!(crate::gui::pages::CURRENT_PAGE, crate::gui::pages::Page::Gallery as u8);
    tinyapi::Response::text("Image queued for display")
}
