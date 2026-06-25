use engine::metrics::{psnr, ssim};
use image::{Rgb, RgbImage};

fn solid(w: u32, h: u32, c: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb(c))
}

#[test]
fn identical_images_are_perfect() {
    let a = solid(32, 32, [123, 45, 200]);
    assert!(psnr(&a, &a).is_infinite());
    assert!((ssim(&a, &a) - 1.0).abs() < 1e-9);
}

#[test]
fn mismatched_dimensions_are_nan() {
    let a = solid(8, 8, [0, 0, 0]);
    let b = solid(8, 16, [0, 0, 0]);
    assert!(psnr(&a, &b).is_nan());
    assert!(ssim(&a, &b).is_nan());
}

#[test]
fn constant_offset_ssim_matches_closed_form() {
    // Two solid-gray images (R=G=B) so luma == the gray value. x=100, y=150 -> only the luminance
    // term survives (variances and covariance are 0): ssim = (2*100*150 + C1)/(100^2+150^2 + C1).
    let a = solid(16, 16, [100, 100, 100]);
    let b = solid(16, 16, [150, 150, 150]);
    let c1 = (0.01 * 255.0_f64).powi(2);
    let expected = (2.0 * 100.0 * 150.0 + c1) / (100.0_f64.powi(2) + 150.0_f64.powi(2) + c1);
    assert!(
        (ssim(&a, &b) - expected).abs() < 1e-6,
        "ssim {} != {}",
        ssim(&a, &b),
        expected
    );
}

#[test]
fn psnr_of_known_uniform_error() {
    // Every pixel differs by exactly 10 on each channel -> MSE = 100 -> psnr = 20*log10(255)-10*log10(100).
    let a = solid(10, 10, [100, 100, 100]);
    let b = solid(10, 10, [110, 110, 110]);
    let expected = 20.0 * 255.0_f64.log10() - 10.0 * 100.0_f64.log10();
    assert!(
        (psnr(&a, &b) - expected).abs() < 1e-9,
        "psnr {} != {}",
        psnr(&a, &b),
        expected
    );
}

#[test]
fn more_degradation_lowers_both_metrics() {
    let a = solid(24, 24, [120, 120, 120]);
    let near = solid(24, 24, [125, 125, 125]);
    let far = solid(24, 24, [170, 170, 170]);
    assert!(psnr(&a, &near) > psnr(&a, &far));
    assert!(ssim(&a, &near) > ssim(&a, &far));
}
