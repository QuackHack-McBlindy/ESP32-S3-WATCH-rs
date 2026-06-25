// GUI/GALLERY
// DRAW IMAGES

use embedded_graphics::{
    prelude::*,
    pixelcolor::Rgb565,
    geometry::Point,
    Drawable,
};
use embedded_png::Png;
use crate::components::storage::SdError;
use heapless::String;


const MAX_PATH_LEN: usize = 128;


static CURRENT_IMAGE: critical_section::Mutex<core::cell::RefCell<Option<String<MAX_PATH_LEN>>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

pub fn set_current_image(path: &str) {
    critical_section::with(|cs| {
        let mut img = CURRENT_IMAGE.borrow_ref_mut(cs);
        if path.is_empty() {
            *img = None;
        } else {
            let mut s = String::new();
            if s.push_str(path).is_err() {
                defmt::warn!("Gallery image path too long, truncated");
            }
            *img = Some(s);
        }
    });
}

pub fn draw_png_file(
    path: &str,
    top_left: Point,
    target: &mut impl DrawTarget<Color = Rgb565>,
) -> Result<(), SdError> {
    let raw = crate::components::storage::read_file_to_vec(path)?;
    let png = Png::load_from_bytes(&raw).map_err(|_| SdError::File)?;
    let width = png.width() as i32;
    let height = png.height() as i32;
    let pixels = png.pixels();

    for y in 0..height {
        for x in 0..width {
            if let Some(color) = pixels[(y * width + x) as usize] {
                let _ = target.draw_iter(core::iter::once(Pixel(
                    Point::new(top_left.x + x, top_left.y + y),
                    color,
                )));
            }
        }
    }
    Ok(())
}


pub fn draw(target: &mut impl DrawTarget<Color = Rgb565>) {
    let path = critical_section::with(|cs| {
        CURRENT_IMAGE
            .borrow_ref(cs)
            .clone()
    });
    let file = if let Some(ref p) = path {
        p.as_str()
    } else {
        "default.png"
    };

    let _ = draw_png_file(file, Point::zero(), target);
}
