//! Property-based testing for `wave` using SplitMix64 PRNG (PHY01, VER15).

use trig::PI;
use wave::Wave;

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn next_f64_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }
}

#[test]
fn test_pbt_amplitude_bounded_and_finite() {
    let mut rng = SplitMix64::new(0x57A1_E001_CAFE_BEEF);
    for _ in 0..250 {
        let amp = rng.next_f64_unit() * 1000.0;
        let freq = 0.01 + rng.next_f64_unit() * 20_000.0;
        let phase = (rng.next_f64_unit() - 0.5) * 4.0 * PI;
        let wave = Wave::new(amp, freq, phase).unwrap();

        for _ in 0..16 {
            let t = (rng.next_f64_unit() - 0.5) * 100.0;
            let y = wave.try_evaluate(t).unwrap();
            assert!(y.is_finite());
            assert!(y.abs() <= amp + 1e-12);
        }
    }
}

#[test]
fn test_pbt_periodicity_and_half_period_inversion() {
    let mut rng = SplitMix64::new(0x1234_5678_ABCD_EF01);
    for _ in 0..150 {
        let amp = 0.1 + rng.next_f64_unit() * 100.0;
        let freq = 0.5 + rng.next_f64_unit() * 500.0;
        let phase = (rng.next_f64_unit() - 0.5) * 2.0 * PI;
        let wave = Wave::new(amp, freq, phase).unwrap();
        let period = wave.try_period().unwrap();
        let t = (rng.next_f64_unit() - 0.5) * 5.0;

        let y0 = wave.evaluate(t);
        let y_half = wave.evaluate(t + 0.5 * period);
        let y_full = wave.evaluate(t + period);

        let tol = 1e-9 * amp.max(1.0);
        assert!((y0 - y_full).abs() <= tol, "periodicity mismatch: {y0} vs {y_full}");
        assert!((y0 + y_half).abs() <= tol, "half-period inversion mismatch: {y0} vs {y_half}");
    }
}

#[test]
fn test_pbt_angular_frequency_identity_and_zero_amplitude() {
    let mut rng = SplitMix64::new(0xDEAD_BEEF_4242_1337);
    for _ in 0..100 {
        let freq = 0.001 + rng.next_f64_unit() * 10_000.0;
        let phase = (rng.next_f64_unit() - 0.5) * 100.0;
        let zero = Wave::new(0.0, freq, phase).unwrap();
        let t = (rng.next_f64_unit() - 0.5) * 1e6;
        assert_eq!(zero.try_evaluate(t).unwrap().to_bits(), 0);

        let w = Wave::new(1.0, freq, phase).unwrap();
        let omega = w.try_angular_frequency().unwrap();
        let period = w.try_period().unwrap();
        assert!(((omega * period) - 2.0 * PI).abs() <= 1e-12);
    }
}
