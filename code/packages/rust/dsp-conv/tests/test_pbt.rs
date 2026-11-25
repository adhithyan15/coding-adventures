//! Property-based testing for `dsp-conv` using SplitMix64 PRNG (DSP04, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_conv::{
    conv1d, conv2d, laplacian_kernel, sep_conv2d, sobel_x_kernel, sobel_y_kernel,
    try_box_blur_kernel, try_gaussian_blur_kernel, try_sharpen_kernel, BoundaryMode,
};

const MODES: [BoundaryMode; 4] = [
    BoundaryMode::Zero,
    BoundaryMode::Replicate,
    BoundaryMode::Reflect,
    BoundaryMode::Wrap,
];

fn sample_f32(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_u64();
    let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
    (mantissa * 2.0) - 1.0
}

fn assert_close(a: &[f32], b: &[f32], tol: f32) {
    assert_eq!(a.len(), b.len());
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        let scale = x.abs().max(y.abs()).max(1.0);
        assert!(
            (x - y).abs() <= scale * tol,
            "mismatch at index {i}: {x} vs {y} (tol {tol})"
        );
    }
}

#[test]
fn test_pbt_conv1d_linearity_and_identity() {
    let mut rng = SplitMix64::new(0xD504_1111_2222_3333);
    for _ in 0..80 {
        let n = 1 + rng.bounded(64) as usize;
        let k = 1 + rng.bounded(11) as usize;
        let mode = MODES[rng.bounded(MODES.len() as u64) as usize];

        let x: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let y: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let h: Vec<f32> = (0..k).map(|_| sample_f32(&mut rng)).collect();
        let alpha = sample_f32(&mut rng);
        let beta = sample_f32(&mut rng);

        // Identity kernel
        let id = conv1d(&x, &[1.0], mode).expect("identity conv1d");
        assert_close(&id, &x, 1e-6);

        // Linearity: conv(a*x + b*y, h) == a*conv(x, h) + b*conv(y, h)
        let comb: Vec<f32> = x.iter().zip(&y).map(|(&xi, &yi)| alpha * xi + beta * yi).collect();
        let lhs = conv1d(&comb, &h, mode).expect("lhs");
        let cx = conv1d(&x, &h, mode).expect("cx");
        let cy = conv1d(&y, &h, mode).expect("cy");
        let rhs: Vec<f32> = cx.iter().zip(&cy).map(|(&a, &b)| alpha * a + beta * b).collect();
        assert_close(&lhs, &rhs, 1e-4);
    }
}

#[test]
fn test_pbt_sep_conv2d_matches_conv2d_outer_product() {
    let mut rng = SplitMix64::new(0xD504_4444_5555_6666);
    for _ in 0..60 {
        let h = 3 + rng.bounded(12) as usize;
        let w = 3 + rng.bounded(12) as usize;
        let kh = 1 + rng.bounded(h.min(5) as u64) as usize;
        let kw = 1 + rng.bounded(w.min(5) as u64) as usize;
        let mode = MODES[rng.bounded(MODES.len() as u64) as usize];

        let img: Vec<f32> = (0..h * w).map(|_| sample_f32(&mut rng)).collect();
        let hk: Vec<f32> = (0..kw).map(|_| sample_f32(&mut rng)).collect();
        let vk: Vec<f32> = (0..kh).map(|_| sample_f32(&mut rng)).collect();

        let mut outer = vec![0.0_f32; kh * kw];
        for r in 0..kh {
            for c in 0..kw {
                outer[r * kw + c] = vk[r] * hk[c];
            }
        }

        let via_sep = sep_conv2d(&img, &hk, &vk, h as u32, w as u32, mode).expect("sep_conv2d");
        let via_2d =
            conv2d(&img, &outer, h as u32, w as u32, kh as u32, kw as u32, mode).expect("conv2d");
        assert_close(&via_sep, &via_2d, 1e-4);
    }
}

#[test]
fn test_pbt_kernel_normalization_and_dc_preservation() {
    let mut rng = SplitMix64::new(0xD504_7777_8888_9999);
    for _ in 0..50 {
        let half = rng.bounded(7) as u32;
        let size = 2 * half + 1;
        let sigma = 0.25 + ((rng.next_u64() & 0xFFFF) as f32 / 65535.0) * 5.0;
        let gk = try_gaussian_blur_kernel(sigma, size).expect("gaussian kernel");
        assert_eq!(gk.len(), size as usize);
        assert!(gk.iter().all(|v| v.is_finite() && *v >= 0.0));
        let g_sum: f32 = gk.iter().sum();
        assert!((g_sum - 1.0).abs() < 1e-5, "gaussian sum {g_sum} != 1");

        let bk = try_box_blur_kernel(size).expect("box kernel");
        let b_sum: f32 = bk.iter().sum();
        assert!((b_sum - 1.0).abs() < 1e-5, "box sum {b_sum} != 1");

        let amount = ((rng.next_u64() & 0xFFFF) as f32 / 65535.0) * 4.0;
        let sk = try_sharpen_kernel(amount).expect("sharpen kernel");
        let s_sum: f32 = sk.iter().sum();
        assert!((s_sum - 1.0).abs() < 1e-5, "sharpen sum {s_sum} != 1");
    }

    assert!((sobel_x_kernel().iter().sum::<f32>()).abs() < 1e-6);
    assert!((sobel_y_kernel().iter().sum::<f32>()).abs() < 1e-6);
    assert!((laplacian_kernel().iter().sum::<f32>()).abs() < 1e-6);
}
