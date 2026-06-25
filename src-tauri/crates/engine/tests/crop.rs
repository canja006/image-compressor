//! Integration tests for the engine crate's cropping and resizing functionality.

use engine::{cover_crop_rect, cover_crop_resize, Anchor, CropRect};
use image::{DynamicImage, Rgb, RgbImage};

#[test]
fn cover_crop_rect_trims_width_of_a_wide_source() {
    let rect = cover_crop_rect(3000, 1000, 1000, 1000, Anchor::Center);
    assert_eq!(
        rect,
        CropRect {
            x: 1000,
            y: 0,
            w: 1000,
            h: 1000
        }
    );
}

#[test]
fn cover_crop_rect_trims_height_of_a_tall_source() {
    let rect = cover_crop_rect(1000, 3000, 1000, 1000, Anchor::Center);
    assert_eq!(
        rect,
        CropRect {
            x: 0,
            y: 1000,
            w: 1000,
            h: 1000
        }
    );
}

#[test]
fn cover_crop_resize_hits_exact_size() {
    let source = create_gradient_image(1600, 900);
    let resized = cover_crop_resize(&source, 800, 800, Anchor::Center, true).unwrap();
    assert_eq!(resized.width(), 800);
    assert_eq!(resized.height(), 800);
}

#[test]
fn anchor_end_pushes_crop_to_the_far_edge() {
    let rect_end = cover_crop_rect(3000, 1000, 1000, 1000, Anchor::End);
    assert_eq!(
        rect_end,
        CropRect {
            x: 2000,
            y: 0,
            w: 1000,
            h: 1000
        }
    );

    let rect_start = cover_crop_rect(3000, 1000, 1000, 1000, Anchor::Start);
    assert_eq!(
        rect_start,
        CropRect {
            x: 0,
            y: 0,
            w: 1000,
            h: 1000
        }
    );
}

#[test]
fn no_upscale_does_not_enlarge() {
    let source = create_gradient_image(600, 600);
    let resized = cover_crop_resize(&source, 2000, 2000, Anchor::Center, false).unwrap();
    assert_eq!(resized.width(), 600);
    assert_eq!(resized.height(), 600);
}

fn create_gradient_image(width: u32, height: u32) -> DynamicImage {
    let mut img = RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let r = (x * 255 / width) as u8;
            let g = (y * 255 / height) as u8;
            let b = 128; // constant blue channel
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
    DynamicImage::ImageRgb8(img)
}
