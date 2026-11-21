//! Property-based testing for dsp-dct using SplitMix64 PRNG (DSP02, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_dct::{dct, dct_2d, idct, idct_2d, DctNorm, DctType};

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
fn test_pbt_1d_dct_roundtrip_and_energy_conservation() {
    let mut rng = SplitMix64::new(0xA1B2_C3D4_E5F6_0718);
    for &n in &[1usize, 2, 3, 4, 5, 7, 8, 12, 16, 23, 31, 32, 64, 100] {
        let signal: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();

        // Ortho roundtrip
        let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
        assert_eq!(coeffs.len(), n);
        let recovered = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
        assert_close(&recovered, &signal, 1e-3);

        // Parseval energy conservation for Ortho DCT-II
        let e_time: f64 = signal.iter().map(|&x| (x as f64) * (x as f64)).sum();
        let e_freq: f64 = coeffs.iter().map(|&x| (x as f64) * (x as f64)).sum();
        assert!(
            (e_time - e_freq).abs() <= e_time.max(1.0) * 1e-3,
            "Ortho DCT energy mismatch for N={n}: time={e_time}, freq={e_freq}"
        );

        // None norm roundtrip
        let coeffs_none = dct(&signal, DctType::II, DctNorm::None).unwrap();
        let recovered_none = idct(&coeffs_none, DctType::III, DctNorm::None).unwrap();
        assert_close(&recovered_none, &signal, 1e-3);
    }
}

#[test]
fn test_pbt_2d_dct_roundtrip_and_energy_conservation() {
    let mut rng = SplitMix64::new(0xCA11_AB1E_D00D_FEED);
    for &(h, w) in &[(1u32, 1u32), (2, 2), (3, 4), (4, 4), (5, 3), (8, 8), (7, 6)] {
        let n = (h as usize) * (w as usize);
        let img: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let coeffs = dct_2d(&img, h, w, DctType::II, DctNorm::Ortho).unwrap();
        assert_eq!(coeffs.len(), n);
        let recovered = idct_2d(&coeffs, h, w, DctType::III, DctNorm::Ortho).unwrap();
        assert_close(&recovered, &img, 1e-3);

        let e_time: f64 = img.iter().map(|&x| (x as f64) * (x as f64)).sum();
        let e_freq: f64 = coeffs.iter().map(|&x| (x as f64) * (x as f64)).sum();
        assert!(
            (e_time - e_freq).abs() <= e_time.max(1.0) * 1e-3,
            "2-D Ortho DCT energy mismatch for {h}x{w}: time={e_time}, freq={e_freq}"
        );
    }
}
