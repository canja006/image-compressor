//! Pure-Rust image-quality metrics: PSNR and SSIM. Implemented directly (no `dssim`/AGPL deps) so
//! the size search can enforce a perceptual floor and the preview can show a quality readout.

use image::RgbImage;

/// Peak signal-to-noise ratio in dB over the three RGB channels. Returns `f64::INFINITY` when the
/// images are identical (MSE == 0). Returns `f64::NAN` if dimensions differ.
pub fn psnr(a: &RgbImage, b: &RgbImage) -> f64 {
    if a.dimensions() != b.dimensions() {
        return f64::NAN;
    }
    let (width, height) = a.dimensions();
    let samples = f64::from(width) * f64::from(height) * 3.0;
    if samples == 0.0 {
        return f64::NAN;
    }

    let mut sum_squared_error = 0.0_f64;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for c in 0..3 {
            let diff = f64::from(pa[c]) - f64::from(pb[c]);
            sum_squared_error += diff * diff;
        }
    }

    let mse = sum_squared_error / samples;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        20.0 * 255.0_f64.log10() - 10.0 * mse.log10()
    }
}

/// Structural similarity (single-scale, luma) in `[-1.0, 1.0]`; 1.0 for identical images. Uses
/// non-overlapping 8x8 windows (partial edge blocks included). Returns `f64::NAN` if dimensions
/// differ or the image is empty.
pub fn ssim(a: &RgbImage, b: &RgbImage) -> f64 {
    if a.dimensions() != b.dimensions() {
        return f64::NAN;
    }
    let (width, height) = a.dimensions();
    if width == 0 || height == 0 {
        return f64::NAN;
    }

    let luma_a = to_luma(a);
    let luma_b = to_luma(b);
    let w = width as usize;

    let c1 = (0.01 * 255.0_f64).powi(2);
    let c2 = (0.03 * 255.0_f64).powi(2);

    let mut total_ssim = 0.0_f64;
    let mut block_count = 0.0_f64;

    for by in 0..height.div_ceil(8) {
        let y_start = (by * 8) as usize;
        let y_end = ((by * 8 + 8).min(height)) as usize;
        for bx in 0..width.div_ceil(8) {
            let x_start = (bx * 8) as usize;
            let x_end = ((bx * 8 + 8).min(width)) as usize;
            let n = ((x_end - x_start) * (y_end - y_start)) as f64;
            if n == 0.0 {
                continue;
            }

            let (mut sx, mut sy, mut sxy, mut sx2, mut sy2) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for y in y_start..y_end {
                let row_a = &luma_a[y * w..y * w + w];
                let row_b = &luma_b[y * w..y * w + w];
                for x in x_start..x_end {
                    let lx = row_a[x];
                    let ly = row_b[x];
                    sx += lx;
                    sy += ly;
                    sxy += lx * ly;
                    sx2 += lx * lx;
                    sy2 += ly * ly;
                }
            }

            let mean_x = sx / n;
            let mean_y = sy / n;
            let var_x = sx2 / n - mean_x * mean_x;
            let var_y = sy2 / n - mean_y * mean_y;
            let cov_xy = sxy / n - mean_x * mean_y;

            let numerator = (2.0 * mean_x * mean_y + c1) * (2.0 * cov_xy + c2);
            let denominator = (mean_x * mean_x + mean_y * mean_y + c1) * (var_x + var_y + c2);
            total_ssim += numerator / denominator;
            block_count += 1.0;
        }
    }

    if block_count == 0.0 {
        f64::NAN
    } else {
        total_ssim / block_count
    }
}

/// Flat row-major luma plane (`0.299R + 0.587G + 0.114B`) as f64 in [0, 255].
fn to_luma(image: &RgbImage) -> Vec<f64> {
    image
        .pixels()
        .map(|p| 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]))
        .collect()
}
