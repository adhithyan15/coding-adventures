//! Pseudo-noise must stay finite for every sample index and seed.
//!
//! `trig::sin` returns NaN beyond 2^29 radians (PHY00a 3.4.1).  The noise phase
//! `(i + 1) * (seed * 0.1618 + 0.618)` grows without bound, so it is reduced to
//! [0, 1) cycles first (PHY00a 7).  These are the seeds and counts at which the
//! unreduced phase used to fail.

use percussion_kit::pseudo_noise;

const CASES: [(usize, u64); 5] = [
    (0, 1_000_000_000),
    (50_000, 12_345),
    (5_700_000, 89),
    (1_000_000_000_000, 89),
    (7, 4_294_967_295),
];

#[test]
fn pseudo_noise_is_finite_and_in_range() {
    for (index, seed) in CASES {
        let v = pseudo_noise(index, seed);
        assert!(v.is_finite() && (-1.0..=1.0).contains(&v), "{index} {seed} {v}");
    }
}

#[test]
fn pseudo_noise_over_a_long_run_never_fails() {
    for i in (0..200_000usize).step_by(997) {
        assert!(pseudo_noise(i, 12_345).is_finite());
    }
}

/// The same values are asserted in the Python and TypeScript lanes.
const EXPECTED: [f64; 5] = [0.06019736691951039, -0.578681446131668, -0.08452169376437269, -0.22354608627210837, -0.9668349004277843];

#[test]
fn cross_lane_values() {
    for ((index, seed), want) in CASES.iter().zip(EXPECTED) {
        assert_eq!(pseudo_noise(*index, *seed), want);
    }
}
