// vendor/embedded-png/src/lib.rs
#![no_std]

use embedded_graphics::{pixelcolor::Rgb565, prelude::IntoStorage};

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    /// The PNG data is corrupt or unsupported.
    Png(minipng::Error),
    /// The provided output buffer is too small for the decoded image.
    OutputTooSmall,
    /// The provided work buffer is too small for the raw decoded data.
    WorkBufferTooSmall,
}

impl From<minipng::Error> for Error {
    fn from(e: minipng::Error) -> Self {
        Error::Png(e)
    }
}

// ---------------------------------------------------------------------------
// Helper: compute the size (in bytes) required by minipng to hold the raw
// decoded image.  Call this first to size your work buffer.
// ---------------------------------------------------------------------------

pub fn required_bytes(png_data: &[u8]) -> Result<usize, minipng::Error> {
    let header = minipng::decode_png_header(png_data)?;
    Ok(header.required_bytes())
}

// ---------------------------------------------------------------------------
// Main decode function – no allocations
// ---------------------------------------------------------------------------

/// Decode a PNG into a pre‑allocated `output` slice of `Rgb565` pixels.
///
/// `output` must be at least `image_width * image_height` elements long.
/// `work_buffer` must be at least `required_bytes(png_data)` bytes.
/// `background` is the colour used for fully‑transparent pixels.
///
/// Returns the image width and height on success.
pub fn decode_rgb565(
    png_data: &[u8],
    output: &mut [Rgb565],
    work_buffer: &mut [u8],
    background: Rgb565,
) -> Result<(u32, u32), Error> {
    let header = minipng::decode_png_header(png_data)?;
    let required = header.required_bytes();

    if work_buffer.len() < required {
        return Err(Error::WorkBufferTooSmall);
    }

    // minipng decodes the whole image into work_buffer
    let data = minipng::decode_png(png_data, work_buffer)?;

    let width = data.width();
    let height = data.height();
    let num_pixels = (width as usize) * (height as usize);

    if output.len() < num_pixels {
        return Err(Error::OutputTooSmall);
    }

    // Pre‑extract background into components for fast blending
    let bg_raw = background.into_storage();
    let bg_r = ((bg_raw >> 11) & 0x1F) as u8;   // 5‑bit
    let bg_g = ((bg_raw >> 5) & 0x3F) as u8;    // 6‑bit
    let bg_b = (bg_raw & 0x1F) as u8;            // 5‑bit

    match header.color_type() {
        minipng::ColorType::Indexed => {
            for (i, &idx) in data.pixels().iter().enumerate() {
                let idx = idx as usize;
                let rgba = u32::from_be_bytes(data.palette(idx as u8));
                let a = (rgba & 0xFF) as u8;
                let r = ((rgba >> 24) & 0xFF) as u8;
                let g = ((rgba >> 16) & 0xFF) as u8;
                let b = ((rgba >> 8) & 0xFF) as u8;
                output[i] = blend_rgb888_to_rgb565(r, g, b, a, bg_r, bg_g, bg_b);
            }
        }
        minipng::ColorType::Gray => {
            for (i, &g) in data.pixels().iter().enumerate() {
                output[i] = blend_rgb888_to_rgb565(g, g, g, 255, bg_r, bg_g, bg_b);
            }
        }
        minipng::ColorType::GrayAlpha => {
            let chunks = data.pixels().chunks_exact(2);
            let mut i = 0;
            for chunk in chunks {
                let gray = chunk[0];
                let alpha = chunk[1];
                output[i] = blend_rgb888_to_rgb565(gray, gray, gray, alpha, bg_r, bg_g, bg_b);
                i += 1;
            }
        }
        minipng::ColorType::Rgb => {
            let chunks = data.pixels().chunks_exact(3);
            let mut i = 0;
            for chunk in chunks {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];
                output[i] = blend_rgb888_to_rgb565(r, g, b, 255, bg_r, bg_g, bg_b);
                i += 1;
            }
        }
        minipng::ColorType::Rgba => {
            let chunks = data.pixels().chunks_exact(4);
            let mut i = 0;
            for chunk in chunks {
                let r = chunk[0];
                let g = chunk[1];
                let b = chunk[2];
                let a = chunk[3];
                output[i] = blend_rgb888_to_rgb565(r, g, b, a, bg_r, bg_g, bg_b);
                i += 1;
            }
        }
    }

    Ok((width, height))
}

// ---------------------------------------------------------------------------
// Internal: alpha‑blend a single RGBA pixel (channels 0..255) against a
// background colour given in 5‑6‑5 format, and return an Rgb565.
// ---------------------------------------------------------------------------

fn blend_rgb888_to_rgb565(
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
) -> Rgb565 {
    if a == 0 {
        // Fully transparent -> background
        return Rgb565::from_rgb565_inner((bg_r as u16) << 11 | (bg_g as u16) << 5 | bg_b as u16);
    }
    if a == 255 {
        // Fully opaque -> straight conversion
        return rgb888_to_rgb565(r, g, b);
    }

    // Partial transparency – blend against background
    let alpha = a as u32;
    let inv_alpha = 255u32 - alpha;

    // Upscale background to 8‑bit per channel
    let bg_r8 = ((bg_r as u32) * 255 + 15) / 31;   // 5‑bit -> 8‑bit
    let bg_g8 = ((bg_g as u32) * 255 + 31) / 63;   // 6‑bit -> 8‑bit
    let bg_b8 = ((bg_b as u32) * 255 + 15) / 31;   // 5‑bit -> 8‑bit

    let r_out = ((r as u32 * alpha + bg_r8 * inv_alpha) / 255) as u8;
    let g_out = ((g as u32 * alpha + bg_g8 * inv_alpha) / 255) as u8;
    let b_out = ((b as u32 * alpha + bg_b8 * inv_alpha) / 255) as u8;

    rgb888_to_rgb565(r_out, g_out, b_out)
}

fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> Rgb565 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    let raw = (r5 << 11) | (g6 << 5) | b5;
    Rgb565::from_rgb565_inner(raw)
}

// Helper trait to construct Rgb565 from raw u16 without requiring `RgbColor`
trait Rgb565FromRaw {
    fn from_rgb565_inner(raw: u16) -> Self;
}

impl Rgb565FromRaw for Rgb565 {
    fn from_rgb565_inner(raw: u16) -> Self {
        // Safety: Rgb565 is repr(transparent) over u16
        unsafe { core::mem::transmute::<u16, Rgb565>(raw) }
    }
}
