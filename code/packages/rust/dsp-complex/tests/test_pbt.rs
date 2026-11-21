//! Property-based testing for dsp-complex using SplitMix64 PRNG (DSP01, VER15).

use coding_adventures_test_bootstrap::prng::SplitMix64;
use dsp_complex::ComplexTensor;

fn sample_f32(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_u64();
    let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
    (mantissa * 2.0) - 1.0
}

#[test]
fn test_pbt_interleaved_and_component_roundtrip() {
    let mut rng = SplitMix64::new(0xC0FF_EE01_DEAD_BEEF);
    for iter in 0..100 {
        let len = (rng.bounded(128) + 1) as usize;
        let mut real = Vec::with_capacity(len);
        let mut imag = Vec::with_capacity(len);
        for _ in 0..len {
            real.push(sample_f32(&mut rng));
            imag.push(sample_f32(&mut rng));
        }

        let tensor = ComplexTensor::from_real_imag(&real, &imag)
            .unwrap_or_else(|e| panic!("iter {iter}: {e}"));
        assert_eq!(tensor.len(), len);
        assert_eq!(tensor.real(), real);
        assert_eq!(tensor.imag(), imag);

        let interleaved = tensor.as_interleaved().to_vec();
        let rebuilt = ComplexTensor::from_interleaved(interleaved.clone()).unwrap();
        assert_eq!(rebuilt, tensor);
        assert_eq!(rebuilt.into_interleaved(), interleaved);
    }
}

#[test]
fn test_pbt_conjugation_and_magnitude_phase_invariants() {
    let mut rng = SplitMix64::new(0xABCD_1234_5678_9EF0);
    for _ in 0..100 {
        let len = (rng.bounded(64) + 1) as usize;
        let mut real = Vec::with_capacity(len);
        let mut imag = Vec::with_capacity(len);
        for _ in 0..len {
            real.push(sample_f32(&mut rng) * 100.0);
            imag.push(sample_f32(&mut rng) * 100.0);
        }

        let tensor = ComplexTensor::from_real_imag(&real, &imag).unwrap();
        let conj = tensor.conjugate();
        assert_eq!(conj.conjugate(), tensor);
        assert_eq!(conj.real(), tensor.real());

        // |conj(z)| == |z|
        let mag = tensor.magnitude();
        let conj_mag = conj.magnitude();
        assert_eq!(mag.len(), len);
        for (a, b) in mag.iter().zip(conj_mag.iter()) {
            assert!(a.is_finite() && *a >= 0.0);
            assert!((a - b).abs() <= 1e-6 * a.max(1.0));
        }

        // Polar reconstruction: re ≈ r * cos(theta), im ≈ r * sin(theta)
        let phase = tensor.phase();
        for i in 0..len {
            let r = mag[i];
            let theta = phase[i];
            assert!(theta.is_finite());
            let rec_re = r * theta.cos();
            let rec_im = r * theta.sin();
            assert!((rec_re - real[i]).abs() <= 1e-4 * r.max(1.0));
            assert!((rec_im - imag[i]).abs() <= 1e-4 * r.max(1.0));
        }
    }
}
