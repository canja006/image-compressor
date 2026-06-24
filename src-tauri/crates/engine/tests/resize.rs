//! Resize wrapper: caps the long edge, preserves aspect ratio, and never upscales.

use engine::resize::{downscale_by_factor, downscale_to_long_edge};
use image::{DynamicImage, Rgb, RgbImage};

fn solid(w: u32, h: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, Rgb([120, 130, 140])))
}

#[test]
fn long_edge_is_capped_and_aspect_preserved() {
    let out = downscale_to_long_edge(&solid(800, 400), 200).unwrap();
    assert_eq!(out.width().max(out.height()), 200);
    assert_eq!((out.width(), out.height()), (200, 100));
}

#[test]
fn never_upscales() {
    let out = downscale_to_long_edge(&solid(100, 50), 1000).unwrap();
    assert_eq!((out.width(), out.height()), (100, 50));
}

#[test]
fn factor_clones_when_ge_one_and_halves_at_one_half() {
    let same = downscale_by_factor(&solid(200, 100), 1.0).unwrap();
    assert_eq!((same.width(), same.height()), (200, 100));
    let half = downscale_by_factor(&solid(200, 100), 0.5).unwrap();
    assert_eq!((half.width(), half.height()), (100, 50));
}
