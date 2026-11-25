//! Property-based testing for dsp-fft using SplitMix64 PRNG (DSP01, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_fft::{
    bluestein_scalar, bluestein_via_runtime, fft, fft_scalar, ifft, ifft_scalar, irfft, rfft,
    Direction,
};

fn sample_f32(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_u64();
    let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
    (mantissa * 2.0) - 1.0
}

fn assert_close(a: &[f32], b: &[f32], tol: f32) {
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        let scale = x.abs().max(y.abs()).max(1.0);
        assert!(
            (x - y).abs() <= scale * tol,
            "mismatch at index {i}: {x} vs {y} (tol {tol})"
        );
    }
}

#[test]
fn test_pbt_fft_ifft_roundtrip_pow2_and_arbitrary() {
    let mut rng = SplitMix64::new(0x1234_5678_9ABC_DEF0);
    for &n in &[1usize, 2, 3, 4, 5, 7, 8, 11, 16, 23, 32, 64, 100, 128] {
        let complex_sig: Vec<f32> = (0..(2 * n)).map(|_| sample_f32(&mut rng)).collect();
        let spec = fft(&complex_sig, true).unwrap();
        assert_eq!(spec.len(), n);
        let back = ifft(&spec).unwrap();
        assert_close(back.as_interleaved(), &complex_sig, 1e-4);

        let real_sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let spec_r = fft(&real_sig, false).unwrap();
        let back_r = ifft(&spec_r).unwrap();
        assert_close(&back_r.real(), &real_sig, 1e-4);
    }
}

#[test]
fn test_pbt_rfft_irfft_roundtrip_and_parseval_energy() {
    let mut rng = SplitMix64::new(0xFEED_FACE_CAFE_BEEF);
    for &n in &[1usize, 2, 3, 4, 5, 7, 8, 13, 16, 29, 32, 64, 99, 128] {
        let real_sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let half = rfft(&real_sig).unwrap();
        assert_eq!(half.len(), n / 2 + 1);
        let recovered = irfft(&half, n as u32).unwrap();
        assert_close(&recovered, &real_sig, 1e-4);

        // Parseval's theorem on full complex spectrum: sum |x[n]|^2 == (1/N) * sum |X[k]|^2
        let full = fft(&real_sig, false).unwrap();
        let time_energy: f64 = real_sig.iter().map(|&x| (x as f64) * (x as f64)).sum();
        let freq_energy: f64 = full
            .as_interleaved()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0] as f64) * (c[0] as f64) + (c[1] as f64) * (c[1] as f64))
            .sum::<f64>()
            / (n as f64);
        assert!(
            (time_energy - freq_energy).abs() <= time_energy.max(1.0) * 1e-3,
            "Parseval mismatch for N={n}: time={time_energy}, freq={freq_energy}"
        );
    }
}

#[test]
fn test_pbt_bluestein_matches_radix2_and_runtime() {
    let mut rng = SplitMix64::new(0x55AA_55AA_1122_3344);
    for &n in &[2usize, 4, 8, 16] {
        let sig: Vec<f32> = (0..(2 * n)).map(|_| sample_f32(&mut rng)).collect();
        let r2 = fft_scalar(&sig).unwrap();
        let bs = bluestein_scalar(&sig, Direction::Forward).unwrap();
        assert_close(&r2, &bs, 1e-4);
        let r2_inv = ifft_scalar(&r2).unwrap();
        let bs_inv = bluestein_scalar(&bs, Direction::Inverse).unwrap();
        assert_close(&r2_inv, &bs_inv, 1e-4);
    }

    for &n in &[3usize, 5, 7, 9] {
        let sig: Vec<f32> = (0..(2 * n)).map(|_| sample_f32(&mut rng)).collect();
        let bs_scalar = bluestein_scalar(&sig, Direction::Forward).unwrap();
        let bs_rt = bluestein_via_runtime(&sig, Direction::Forward).unwrap();
        assert_close(&bs_scalar, &bs_rt, 1e-4);
    }
}
