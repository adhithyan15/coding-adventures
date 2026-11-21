//! Property-based testing for `dsp-wavelets` using SplitMix64 PRNG (DSP06, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_wavelets::{
    cwt, dwt_1d, dwt_2d, idwt_1d, idwt_2d, slice_level, split_levels, Band, WaveletBoundary,
    WaveletType,
};

fn sample_f32(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_u64();
    let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
    (mantissa * 2.0) - 1.0
}

#[test]
fn test_pbt_1d_orthogonal_dwt_idwt_round_trip() {
    let mut rng = SplitMix64::new(0xD506_1111_2222_3333);
    let wavelets = [
        (WaveletType::Haar, 2usize, 1e-4f32),
        (WaveletType::Daubechies(2), 4, 1e-4),
        (WaveletType::Daubechies(4), 8, 1e-3),
        (WaveletType::Daubechies(6), 12, 1e-3),
        (WaveletType::Daubechies(8), 16, 1e-3),
        (WaveletType::Symlets(4), 8, 1e-3),
        (WaveletType::Coiflets(1), 6, 1e-3),
    ];

    for &(w, min_len, tol) in &wavelets {
        for _ in 0..20 {
            let levels = 1 + rng.bounded(3) as u32;
            let mult = 1usize << levels;
            // Exact multi-level Periodic PR requires each level's length to be even (n % 2^levels == 0).
            let base = min_len.max(16).div_ceil(mult) * mult;
            let n = base + (rng.bounded(8) as usize) * mult;
            let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
            let coeffs =
                dwt_1d(&sig, w, levels, WaveletBoundary::Periodic).expect("dwt_1d periodic");
            let recon = idwt_1d(&coeffs, w, levels, WaveletBoundary::Periodic, n as u32)
                .expect("idwt_1d periodic");
            assert_eq!(recon.len(), n);
            for (i, (&a, &b)) in sig.iter().zip(&recon).enumerate() {
                assert!(
                    (a - b).abs() < tol,
                    "{:?} levels={levels} idx={i}: {a} vs {b}",
                    w
                );
            }

            // Check split_levels / slice_level partition
            let offsets = split_levels(coeffs.len(), n, levels).expect("split_levels");
            assert_eq!(offsets.len(), levels as usize + 2);
            assert_eq!(offsets[0], 0);
            assert_eq!(*offsets.last().unwrap(), coeffs.len());
            let ca = slice_level(&coeffs, n, levels, levels, Band::Approximation).expect("ca");
            assert_eq!(ca.len(), offsets[1] - offsets[0]);
            for j in 1..=levels {
                let cd = slice_level(&coeffs, n, levels, j, Band::Detail).expect("cd");
                assert!(!cd.is_empty());
            }
        }
    }
}

#[test]
fn test_pbt_haar_symmetric_odd_and_even_round_trip() {
    let mut rng = SplitMix64::new(0xD506_4444_5555_6666);
    for _ in 0..40 {
        let n = 8 + rng.bounded(49) as usize;
        let levels = 1 + rng.bounded(3) as u32;
        let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();

        let coeffs =
            dwt_1d(&sig, WaveletType::Haar, levels, WaveletBoundary::Symmetric).expect("dwt sym");
        let recon = idwt_1d(
            &coeffs,
            WaveletType::Haar,
            levels,
            WaveletBoundary::Symmetric,
            n as u32,
        )
        .expect("idwt sym");
        assert_eq!(recon.len(), n);
        for (i, (&a, &b)) in sig.iter().zip(&recon).enumerate() {
            assert!((a - b).abs() < 1e-4, "sym idx={i}: {a} vs {b}");
        }
    }
}

#[test]
fn test_pbt_2d_dwt_idwt_round_trip() {
    let mut rng = SplitMix64::new(0xD506_7777_8888_9999);
    for &(w, tol) in &[
        (WaveletType::Haar, 1e-4f32),
        (WaveletType::Daubechies(2), 1e-3),
        (WaveletType::Symlets(4), 1e-3),
    ] {
        for _ in 0..15 {
            let levels = 1 + rng.bounded(2) as u32;
            let mult = 1usize << levels;
            let base = 16usize.div_ceil(mult) * mult;
            let rows = base + (rng.bounded(6) as usize) * mult;
            let cols = base + (rng.bounded(6) as usize) * mult;
            let img: Vec<f32> = (0..rows * cols).map(|_| sample_f32(&mut rng)).collect();

            let coeffs = dwt_2d(
                &img,
                rows as u32,
                cols as u32,
                w,
                levels,
                WaveletBoundary::Periodic,
            )
            .expect("dwt_2d");
            let recon = idwt_2d(
                &coeffs,
                rows as u32,
                cols as u32,
                w,
                levels,
                WaveletBoundary::Periodic,
            )
            .expect("idwt_2d");
            assert_eq!(recon.len(), img.len());
            for (i, (&a, &b)) in img.iter().zip(&recon).enumerate() {
                assert!((a - b).abs() < tol, "2d {:?} idx={i}: {a} vs {b}", w);
            }
        }
    }
}

#[test]
fn test_pbt_cwt_invariants() {
    let mut rng = SplitMix64::new(0xD506_AAAA_BBBB_CCCC);
    for _ in 0..20 {
        let n = 16 + rng.bounded(49) as usize;
        let sig: Vec<f32> = (0..n).map(|_| sample_f32(&mut rng)).collect();
        let scales: Vec<f32> = (1..=4)
            .map(|k| (k as f32) * (0.5 + (rng.next_u64() & 0xFF) as f32 / 255.0))
            .collect();

        let morlet = cwt(&sig, &scales, WaveletType::Morlet).expect("cwt morlet");
        assert_eq!(morlet.len(), scales.len() * n);
        assert!(morlet.iter().all(|&v| v.is_finite() && v >= 0.0));

        let mex = cwt(&sig, &scales, WaveletType::MexicanHat).expect("cwt mexican hat");
        assert_eq!(mex.len(), scales.len() * n);
        assert!(mex.iter().all(|v| v.is_finite()));
    }
}
