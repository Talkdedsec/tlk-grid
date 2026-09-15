//! Loading the images that go on an overlay: a crosshair PNG, artwork around
//! the letterbox, an animated GIF.
//!
//! Everything comes out as premultiplied BGRA because that is the one format
//! `UpdateLayeredWindow` accepts. Doing the multiply here rather than at draw
//! time means a held bind never pays for it.

use std::path::Path;

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// Premultiplied BGRA, top-down, 4 bytes per pixel.
    pub bgra: Vec<u8>,
    /// How long this frame is shown. Zero for a still image.
    pub delay_ms: u32,
}

#[derive(Clone)]
pub struct Picture {
    pub frames: Vec<Frame>,
}

impl Picture {
    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }

    pub fn size(&self) -> (u32, u32) {
        self.frames
            .first()
            .map(|f| (f.width, f.height))
            .unwrap_or((0, 0))
    }

    /// Total loop length, for anything that needs to know the cycle.
    pub fn duration_ms(&self) -> u32 {
        self.frames.iter().map(|f| f.delay_ms).sum()
    }
}

/// Straight RGBA to premultiplied BGRA. A layered window draws the raw bytes,
/// so a non-premultiplied image shows dark fringes around every soft edge.
pub fn premultiply_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut bgra = vec![0u8; rgba.len()];
    for (out, src) in bgra.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
        let alpha = src[3] as u32;
        out[0] = ((src[2] as u32 * alpha) / 255) as u8;
        out[1] = ((src[1] as u32 * alpha) / 255) as u8;
        out[2] = ((src[0] as u32 * alpha) / 255) as u8;
        out[3] = src[3];
    }
    debug_assert_eq!(bgra.len(), (width * height * 4) as usize);
    bgra
}

/// GIF frames can carry a zero delay, which browsers and Windows both read as
/// "as fast as possible". Clamp it the way every other renderer does.
const MIN_DELAY_MS: u32 = 20;

pub fn load(path: &Path) -> Result<Picture> {
    let bytes = std::fs::read(path).map_err(|_| Error::NoSuchImage(path.display().to_string()))?;
    let animated = image::guess_format(&bytes)
        .map(|format| format == image::ImageFormat::Gif)
        .unwrap_or(false);

    if animated {
        return load_gif(&bytes, path);
    }

    let decoded = image::load_from_memory(&bytes)
        .map_err(|_| Error::NoSuchImage(path.display().to_string()))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(Picture {
        frames: vec![Frame {
            width,
            height,
            bgra: premultiply_rgba(decoded.as_raw(), width, height),
            delay_ms: 0,
        }],
    })
}

fn load_gif(bytes: &[u8], path: &Path) -> Result<Picture> {
    use image::AnimationDecoder;

    let decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|_| Error::NoSuchImage(path.display().to_string()))?;

    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        let Ok(frame) = frame else { break };
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay_ms = numerator
            .checked_div(denominator)
            .unwrap_or(MIN_DELAY_MS)
            .max(MIN_DELAY_MS);
        let buffer = frame.into_buffer();
        let (width, height) = buffer.dimensions();
        frames.push(Frame {
            width,
            height,
            bgra: premultiply_rgba(buffer.as_raw(), width, height),
            delay_ms,
        });
    }

    if frames.is_empty() {
        return Err(Error::NoSuchImage(path.display().to_string()));
    }
    Ok(Picture { frames })
}

/// Box-filtered resize. Nearest neighbour is unusable for a crosshair scaled to
/// 40%, and a full Lanczos pass is more than an overlay needs.
pub fn resize(frame: &Frame, width: u32, height: u32) -> Frame {
    if width == frame.width && height == frame.height {
        return frame.clone();
    }
    let (width, height) = (width.max(1), height.max(1));
    let mut out = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        let y0 = (y as u64 * frame.height as u64 / height as u64) as u32;
        let y1 = (((y + 1) as u64 * frame.height as u64 / height as u64) as u32).max(y0 + 1);
        for x in 0..width {
            let x0 = (x as u64 * frame.width as u64 / width as u64) as u32;
            let x1 = (((x + 1) as u64 * frame.width as u64 / width as u64) as u32).max(x0 + 1);

            let mut sums = [0u32; 4];
            let mut count = 0u32;
            for sy in y0..y1.min(frame.height) {
                for sx in x0..x1.min(frame.width) {
                    let at = ((sy * frame.width + sx) * 4) as usize;
                    for (sum, byte) in sums.iter_mut().zip(&frame.bgra[at..at + 4]) {
                        *sum += *byte as u32;
                    }
                    count += 1;
                }
            }
            if count == 0 {
                continue;
            }
            let at = ((y * width + x) * 4) as usize;
            for (slot, sum) in out[at..at + 4].iter_mut().zip(sums) {
                *slot = (sum / count) as u8;
            }
        }
    }

    Frame {
        width,
        height,
        bgra: out,
        delay_ms: frame.delay_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Frame {
        Frame {
            width,
            height,
            bgra: premultiply_rgba(&rgba.repeat((width * height) as usize), width, height),
            delay_ms: 0,
        }
    }

    #[test]
    fn a_transparent_pixel_loses_its_colour() {
        let out = premultiply_rgba(&[200, 100, 50, 0], 1, 1);
        assert_eq!(out, vec![0, 0, 0, 0]);
    }

    #[test]
    fn an_opaque_pixel_only_swaps_to_bgra() {
        let out = premultiply_rgba(&[200, 100, 50, 255], 1, 1);
        assert_eq!(out, vec![50, 100, 200, 255]);
    }

    #[test]
    fn half_alpha_halves_the_colour() {
        let out = premultiply_rgba(&[255, 255, 255, 128], 1, 1);
        assert_eq!(&out[..3], &[128, 128, 128]);
        assert_eq!(out[3], 128);
    }

    #[test]
    fn resizing_keeps_a_solid_colour_solid() {
        let small = solid(4, 4, [10, 20, 30, 255]);
        let big = resize(&small, 16, 16);
        assert_eq!((big.width, big.height), (16, 16));
        assert_eq!(&big.bgra[..4], &[30, 20, 10, 255]);
        assert!(big.bgra.chunks_exact(4).all(|p| p == [30, 20, 10, 255]));
    }

    #[test]
    fn resizing_to_the_same_size_is_a_copy() {
        let frame = solid(8, 8, [1, 2, 3, 255]);
        assert_eq!(resize(&frame, 8, 8).bgra, frame.bgra);
    }

    #[test]
    fn a_still_picture_is_not_animated() {
        let picture = Picture {
            frames: vec![solid(2, 2, [0, 0, 0, 255])],
        };
        assert!(!picture.is_animated());
        assert_eq!(picture.duration_ms(), 0);
        assert_eq!(picture.size(), (2, 2));
    }
}
