#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use minipng::ColorType;

#[cfg(feature = "defmt")]
use defmt::info;

/// Convert 24‑bit RGB (0x00RRGGBB) to Rgb565.
fn from_rgb888(rgb: u32) -> Rgb565 {
    let r = ((rgb >> 16) & 0xFF) as u16;
    let g = ((rgb >> 8) & 0xFF) as u16;
    let b = (rgb & 0xFF) as u16;
    Rgb565::new((r >> 3) as u8, (g >> 2) as u8, (b >> 3) as u8)
}

/// A PNG image stored as pre‑converted pixels (None = transparent).
pub struct Png {
    width: u32,
    height: u32,
    pixels: Vec<Option<Rgb565>>,
}

impl Png {
    /// Load a PNG from a byte slice. Allocates the pixel buffer.
    pub fn load_from_bytes(png_data: &[u8]) -> Result<Self, minipng::Error> {
        let header = minipng::decode_png_header(png_data)?;
        let required = header.required_bytes();
        let mut buffer = vec![0u8; required];
        let data = minipng::decode_png(png_data, &mut buffer)?;

        let w = data.width();
        let h = data.height();
        let mut pixels = Vec::with_capacity((w * h) as usize);

        match header.color_type() {
            ColorType::Indexed => {
                for &idx in data.pixels() {
                    let idx = idx as usize;
                    if idx == 0 {
                        pixels.push(None);
                    } else {
                        let rgba = u32::from_be_bytes(data.palette(idx as u8));
                        pixels.push(Some(from_rgb888(rgba >> 8)));
                    }
                }
            }
            ColorType::Gray => {
                for &g in data.pixels() {
                    let rgb = u32::from_be_bytes([0, g, g, g]);
                    pixels.push(Some(from_rgb888(rgb)));
                }
            }
            ColorType::GrayAlpha => {
                for chunk in data.pixels().chunks_exact(2) {
                    let gray = chunk[0];
                    let alpha = chunk[1];
                    if alpha == 0 {
                        pixels.push(None);
                    } else {
                        let rgb = u32::from_be_bytes([0, gray, gray, gray]);
                        pixels.push(Some(from_rgb888(rgb)));
                    }
                }
            }
            ColorType::Rgb => {
                for chunk in data.pixels().chunks_exact(3) {
                    let rgb = u32::from_be_bytes([0, chunk[0], chunk[1], chunk[2]]);
                    pixels.push(Some(from_rgb888(rgb)));
                }
            }
            ColorType::Rgba => {
                for chunk in data.pixels().chunks_exact(4) {
                    let alpha = chunk[3];
                    if alpha == 0 {
                        pixels.push(None);
                    } else {
                        let rgb = u32::from_be_bytes([0, chunk[0], chunk[1], chunk[2]]);
                        pixels.push(Some(from_rgb888(rgb)));
                    }
                }
            }
        }

        #[cfg(feature = "defmt")]
        info!("PNG loaded: {}x{}", w, h);

        Ok(Self { width: w, height: h, pixels })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[Option<Rgb565>] {
        &self.pixels
    }
}

/// Original full‑size drawing – unchanged.
pub fn draw_png<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    png: &Png,
    x: i32,
    y: i32,
) -> Result<(), D::Error> {
    #[cfg(feature = "defmt")]
    info!("draw_png: at ({}, {}) size {}x{}", x, y, png.width(), png.height());

    let width = png.width();
    let height = png.height();
    let pixels = png.pixels();

    for row in 0..height {
        for col in 0..width {
            let idx = (row * width + col) as usize;
            if let Some(color) = pixels[idx] {
                let point = Point::new(x + col as i32, y + row as i32);
                Pixel(point, color).draw(display)?;
            }
        }
    }
    Ok(())
}




/// ------------------------------------------------------------------
/// New: draw PNG with integer nearest‑neighbour scaling.
/// `scale` must be ≥ 1.  `scale = 2` halves the width & height.
/// ------------------------------------------------------------------
pub fn draw_png_scaled<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    png: &Png,
    x: i32,
    y: i32,
    scale: u32,
) -> Result<(), D::Error> {
    if scale == 0 {
        // Avoid division by zero; treat as no scaling.
        return draw_png(display, png, x, y);
    }

    #[cfg(feature = "defmt")]
    info!(
        "draw_png_scaled: at ({}, {}) scale {} ({}x{} -> {}x{})",
        x,
        y,
        scale,
        png.width(),
        png.height(),
        png.width() / scale,
        png.height() / scale
    );

    let width = png.width();
    let height = png.height();
    let pixels = png.pixels();

    // Step through the source image in blocks of `scale × scale`.
    // Each block contributes one output pixel (its top‑left pixel).
    for row in (0..height).step_by(scale as usize) {
        for col in (0..width).step_by(scale as usize) {
            let idx = (row * width + col) as usize;
            if let Some(color) = pixels[idx] {
                let out_x = x + (col / scale) as i32;
                let out_y = y + (row / scale) as i32;
                Pixel(Point::new(out_x, out_y), color).draw(display)?;
            }
        }
    }
    Ok(())
}

/// ------------------------------------------------------------------
/// Convenience: decode bytes and draw scaled.
/// ------------------------------------------------------------------
pub fn draw_png_bytes_scaled<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    png_data: &[u8],
    x: i32,
    y: i32,
    scale: u32,
) -> Result<(), D::Error> {
    let png = Png::load_from_bytes(png_data).expect("PNG decoding failed");
    draw_png_scaled(display, &png, x, y, scale)
}

/// Convenience: draw full‑size from bytes (unchanged).
pub fn draw_png_bytes<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    png_data: &[u8],
    x: i32,
    y: i32,
) -> Result<(), D::Error> {
    let png = Png::load_from_bytes(png_data).expect("PNG decoding failed");
    draw_png(display, &png, x, y)
}

/// Same as `draw_png_bytes`, but at (0,0).
pub fn draw_png_bytes_at_origin<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    png_data: &[u8],
) -> Result<(), D::Error> {
    draw_png_bytes(display, png_data, 0, 0)
}

/// Version that ignores draw errors (only useful when D::Error = Infallible).
pub fn draw_png_bytes_unwrap<D: DrawTarget<Color = Rgb565, Error = core::convert::Infallible>>(
    display: &mut D,
    png_data: &[u8],
    x: i32,
    y: i32,
) {
    draw_png_bytes(display, png_data, x, y).unwrap();
}
