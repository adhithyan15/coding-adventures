//! Hostile input testing for `wave` (PHY01, VER15: CWE-20, CWE-190, CWE-400, CWE-674).

use trig::PI;
use wave::Wave;

#[test]
fn test_hostile_cwe_20_non_finite_parameters_rejected() {
    let bad_values = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for &bad in &bad_values {
        assert!(Wave::new(bad, 1.0, 0.0).is_err());
        assert!(Wave::new(1.0, bad, 0.0).is_err());
        assert!(Wave::new(1.0, 1.0, bad).is_err());
    }
}

#[test]
fn test_hostile_cwe_20_physical_domain_and_overflow_bounds() {
    assert!(Wave::new(-f64::MIN_POSITIVE, 1.0, 0.0).is_err());
    assert!(Wave::new(-1.0e100, 1.0, 0.0).is_err());
    assert!(Wave::new(1.0, 0.0, 0.0).is_err());
    assert!(Wave::new(1.0, -0.0, 0.0).is_err());
    assert!(Wave::new(1.0, -1.0, 0.0).is_err());
    assert!(Wave::new(1.0, f64::MAX, 0.0).is_err());
    assert!(Wave::new(1.0, f64::MAX / (2.0 * PI) * 1.0001, 0.0).is_err());

    let neg_zero = Wave::new(-0.0, 1.0, -0.0).unwrap();
    assert_eq!(neg_zero.amplitude.to_bits(), 0);
    assert_eq!(neg_zero.phase.to_bits(), 0);
}

#[test]
fn test_hostile_cwe_20_try_methods_fail_closed_without_panicking() {
    let w = Wave::new(1.0, 440.0, 0.0).unwrap();
    for &bad_t in &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(w.try_evaluate(bad_t).is_err());
    }

    // Mutated struct fields must fail closed in validate() and try_*() methods.
    let corrupted = Wave {
        amplitude: f64::NAN,
        frequency: 1.0,
        phase: 0.0,
    };
    assert!(corrupted.validate().is_err());
    assert!(corrupted.try_period().is_err());
    assert!(corrupted.try_angular_frequency().is_err());
    assert!(corrupted.try_evaluate(0.0).is_err());

    let overflow_freq = Wave {
        amplitude: 1.0,
        frequency: f64::MAX,
        phase: 0.0,
    };
    assert!(overflow_freq.try_angular_frequency().is_err());
}

#[test]
fn test_hostile_cwe_190_400_674_extreme_finite_and_subnormal_bounded() {
    let subnormal = Wave::new(1.0, f64::from_bits(1), f64::MAX).unwrap();
    assert!(subnormal.try_period().unwrap().is_infinite());
    for &t in &[-f64::MAX, -1e100, -1.0, 0.0, 1.0, 1e100, f64::MAX] {
        let y = subnormal.try_evaluate(t).unwrap();
        assert!(y.is_finite());
        assert!(y.abs() <= 1.0);
    }

    let huge = Wave::new(f64::MAX, 1e300, -f64::MAX).unwrap();
    for &t in &[-f64::MAX, -1e300, 0.0, 1e300, f64::MAX] {
        let y = huge.try_evaluate(t).unwrap();
        assert!(y.is_finite());
        assert!(y.abs() <= f64::MAX);
    }
}
