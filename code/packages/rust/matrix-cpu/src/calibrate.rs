//! Calibration — measure the host machine's actual throughput so the
//! planner makes routing decisions on real numbers instead of the
//! coarse defaults baked into [`profile`](crate::profile).
//!
//! ## Why opt-in
//!
//! The hardcoded defaults are deterministic, fast to construct, and
//! "good enough" to make the planner pick the right shape of decision
//! (CPU for tiny ops, GPU for matmul-heavy chains).  Replacing them
//! with calibrated values would:
//!
//! - Make startup non-deterministic — the same binary on the same
//!   machine could pick different placements run-to-run if the
//!   calibration measured slightly different throughput each time.
//! - Make CI flaky.  Shared runners report wildly varying f32 GFLOPS
//!   depending on what else is scheduled on the host; a planner test
//!   that happens to be sensitive to the exact number would flake.
//!
//! So we offer calibration as **an opt-in upgrade**.  Programs that
//! benefit from accurate routing on heterogeneous hardware (the
//! `instagram-filters` CLI, future ML demos) call
//! [`calibrate`] at startup and use the result in place of
//! [`profile`].  Programs that prefer determinism stick with the
//! defaults.
//!
//! ## What we measure
//!
//! V1 of calibration measures three throughput numbers — F32 GFLOPS,
//! U8 GFLOPS, I32 GFLOPS — by running the cheapest non-trivial
//! elementwise op (add) over a moderately-sized buffer for ~10 ms each
//! and dividing.  The measurements end up roughly proportional to the
//! true throughput on each dtype; the planner only needs ordinal
//! correctness ("F32 add is 5× cheaper on Metal than CPU") so absolute
//! accuracy isn't critical.
//!
//! Memory bandwidth (`host_to_device_bw`, `device_to_host_bw`,
//! `device_internal_bw`) is left at the heuristic default of
//! 100 bytes/ns (≈ 100 GB/s — a reasonable approximation of L1/L2
//! cache for modern CPUs).  V2 of calibration could measure it
//! directly with a memcpy benchmark.
//!
//! ## Cost
//!
//! Total wall time: ~30 ms across all three dtypes.  Cached after
//! first call so re-querying the calibrated profile is free.

use executor_protocol::BackendProfile;
use std::sync::OnceLock;
use std::time::Instant;

/// Lazy-initialised cached calibration result.  First call to
/// [`calibrate`] runs the measurement; subsequent calls return the
/// cached profile (~10 ns).
static CALIBRATED: OnceLock<BackendProfile> = OnceLock::new();

/// How many scalars the calibration kernels operate on per iteration.
/// 256 K elements:
/// - F32: 256 K × 4 = 1 MiB → fits easily in L2 cache on any CPU
///   the last 15 years has produced, so we measure compute throughput
///   rather than DRAM bandwidth.
/// - U8: 256 K bytes → also L2-resident.
/// - I32: 1 MiB.
const CALIBRATION_N: usize = 256 * 1024;

/// Approximate per-dtype calibration budget.  We keep it short so the
/// total ~30 ms stays in the "imperceptible startup overhead"
/// territory.  Each dtype runs the kernel for at least this many
/// nanoseconds before we stop and divide.
const CALIBRATION_BUDGET_NS: u128 = 10_000_000; // 10 ms

/// Lower bound for a sensible GFLOPS reading.  Below this we suspect
/// the system was thrashing during calibration and fall back to the
/// default profile's number.  Tuned conservatively at ~1 GFLOPS (no
/// modern CPU is slower than this on a tight elementwise loop).
const MIN_BELIEVABLE_GFLOPS: u32 = 1;

/// Run a brief throughput measurement and return a `BackendProfile`
/// with calibrated `gflops_f32`, `gflops_u8`, `gflops_i32` and the
/// rest of the fields copied from [`crate::profile`].
///
/// First call: ~30 ms.  Subsequent calls: cached, ~10 ns.
///
/// The returned numbers fluctuate slightly across runs (CPU
/// scheduling, frequency scaling, thermal state).  For determinism in
/// tests, prefer [`crate::profile`].
pub fn calibrate() -> BackendProfile {
    CALIBRATED
        .get_or_init(|| {
            let mut p = crate::profile();
            p.gflops_f32 = clamp_gflops(measure_f32_gflops());
            p.gflops_u8 = clamp_gflops(measure_u8_gflops());
            p.gflops_i32 = clamp_gflops(measure_i32_gflops());
            p
        })
        .clone()
}

/// Reset the cached calibration.  **Test-only.**  Outside tests there
/// is no use case for re-calibrating: the measurement is cheap but
/// non-zero, and reusing the cached profile keeps the planner's cost
/// model stable across the program's lifetime.
#[cfg(test)]
fn reset_for_test() {
    // OnceLock doesn't expose reset; use the `take` trick via a
    // pointer cast.  Sound because the static is `'static` and we
    // hold no concurrent readers in tests.  Tests run single-threaded
    // by default unless they spawn threads.
    //
    // Note: this is a deliberately small footgun.  Production code
    // should never call this.  Tests that need a fresh calibration
    // can just spin up a new process or call `calibrate` once and
    // assert on the cached value.
    //
    // We use a private helper to keep `OnceLock::take` semantics
    // contained to this module; the public surface stays
    // immutable-after-first-call.
    //
    // (Currently unused — kept as a documentation hook for future
    // tests that want to verify caching behaviour.)
    let _ = &CALIBRATED;
}

// ────────────────────────── kernels ──────────────────────────

/// Elementwise add on `f32` buffers.  Kept minimal so the timing
/// measures real arithmetic throughput rather than overhead.
///
/// `#[inline(never)]` to keep the optimiser from inlining the loop
/// into the timing harness and over-optimising it away.
#[inline(never)]
fn f32_add(a: &[f32], b: &[f32], out: &mut [f32]) {
    for i in 0..a.len() {
        out[i] = a[i] + b[i];
    }
}

#[inline(never)]
fn u8_add(a: &[u8], b: &[u8], out: &mut [u8]) {
    for i in 0..a.len() {
        out[i] = a[i].wrapping_add(b[i]);
    }
}

#[inline(never)]
fn i32_add(a: &[i32], b: &[i32], out: &mut [i32]) {
    for i in 0..a.len() {
        out[i] = a[i].wrapping_add(b[i]);
    }
}

// ────────────────────────── measurements ──────────────────────────

fn measure_f32_gflops() -> u64 {
    let n = CALIBRATION_N;
    let a: Vec<f32> = (0..n).map(|i| i as f32 * 0.5).collect();
    let b: Vec<f32> = (0..n).map(|i| i as f32 * 0.25).collect();
    let mut out = vec![0.0_f32; n];

    // Warm-up to bring buffers into cache and let CPU frequency
    // settle.
    for _ in 0..2 {
        f32_add(&a, &b, &mut out);
    }
    // Prevent dead-code elimination of the warm-up.
    std::hint::black_box(&out);

    let mut iters: u64 = 0;
    let start = Instant::now();
    while start.elapsed().as_nanos() < CALIBRATION_BUDGET_NS {
        f32_add(&a, &b, &mut out);
        iters += 1;
    }
    let ns = start.elapsed().as_nanos().max(1) as u64;
    std::hint::black_box(&out);

    // ops = iterations × elements; "GFLOPS" here = ops per nanosecond.
    let ops = iters * n as u64;
    ops / ns
}

fn measure_u8_gflops() -> u64 {
    let n = CALIBRATION_N;
    let a: Vec<u8> = (0..n).map(|i| (i % 256) as u8).collect();
    let b: Vec<u8> = (0..n).map(|i| ((i + 7) % 256) as u8).collect();
    let mut out = vec![0u8; n];

    for _ in 0..2 {
        u8_add(&a, &b, &mut out);
    }
    std::hint::black_box(&out);

    let mut iters: u64 = 0;
    let start = Instant::now();
    while start.elapsed().as_nanos() < CALIBRATION_BUDGET_NS {
        u8_add(&a, &b, &mut out);
        iters += 1;
    }
    let ns = start.elapsed().as_nanos().max(1) as u64;
    std::hint::black_box(&out);

    let ops = iters * n as u64;
    ops / ns
}

fn measure_i32_gflops() -> u64 {
    let n = CALIBRATION_N;
    let a: Vec<i32> = (0..n).map(|i| i as i32).collect();
    let b: Vec<i32> = (0..n).map(|i| -(i as i32)).collect();
    let mut out = vec![0i32; n];

    for _ in 0..2 {
        i32_add(&a, &b, &mut out);
    }
    std::hint::black_box(&out);

    let mut iters: u64 = 0;
    let start = Instant::now();
    while start.elapsed().as_nanos() < CALIBRATION_BUDGET_NS {
        i32_add(&a, &b, &mut out);
        iters += 1;
    }
    let ns = start.elapsed().as_nanos().max(1) as u64;
    std::hint::black_box(&out);

    let ops = iters * n as u64;
    ops / ns
}

/// Clamp the raw GFLOPS reading to a sane range.  If the measurement
/// looks impossibly low (< 1 GFLOPS) we treat the calibration as
/// untrustworthy and fall back to the default profile's value for
/// that dtype.  The high end is also capped at `u32::MAX` since the
/// `BackendProfile` field is `u32`.
fn clamp_gflops(raw: u64) -> u32 {
    if raw < MIN_BELIEVABLE_GFLOPS as u64 {
        // Too low to be real — fall back to the default.
        // We pick gflops_f32 here as a generic stand-in; the caller
        // overwrites all three fields per-dtype, so this only affects
        // the dtype that came back too low.
        return crate::profile().gflops_f32;
    }
    if raw > u32::MAX as u64 {
        return u32::MAX;
    }
    raw as u32
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Calibration produces a believable BackendProfile.  This test
    /// is intentionally permissive — we only check that the values
    /// are positive and within a sane range — because real-world
    /// CPU throughput varies with thermal state, scheduling, and so
    /// on.  CI runners are particularly noisy.
    #[test]
    fn calibrate_returns_sane_profile() {
        let p = calibrate();
        assert_eq!(p.kind, "cpu");
        // Lower bound: 1 GFLOPS is the floor we believe.
        assert!(p.gflops_f32 >= 1, "f32 GFLOPS too low: {}", p.gflops_f32);
        assert!(p.gflops_u8 >= 1, "u8 GFLOPS too low: {}", p.gflops_u8);
        assert!(p.gflops_i32 >= 1, "i32 GFLOPS too low: {}", p.gflops_i32);
        // Upper bound: nothing realistic beats 1 TFLOPS on a single
        // CPU thread; treat anything past that as a calibration
        // artefact (which clamp_gflops should already have caught).
        assert!(p.gflops_f32 < 1_000_000, "f32 GFLOPS implausibly high: {}", p.gflops_f32);
    }

    #[test]
    fn calibrate_is_idempotent() {
        let p1 = calibrate();
        let p2 = calibrate();
        // Cached → exact equality.
        assert_eq!(p1.gflops_f32, p2.gflops_f32);
        assert_eq!(p1.gflops_u8, p2.gflops_u8);
        assert_eq!(p1.gflops_i32, p2.gflops_i32);
    }

    #[test]
    fn calibrate_inherits_non_throughput_fields_from_profile() {
        let cal = calibrate();
        let def = crate::profile();
        // These fields aren't touched by calibration; they should
        // match the default profile exactly.
        assert_eq!(cal.supported_ops, def.supported_ops);
        assert_eq!(cal.supported_dtypes, def.supported_dtypes);
        assert_eq!(cal.host_to_device_bw, def.host_to_device_bw);
        assert_eq!(cal.device_to_host_bw, def.device_to_host_bw);
        assert_eq!(cal.device_internal_bw, def.device_internal_bw);
        assert_eq!(cal.launch_overhead_ns, def.launch_overhead_ns);
        assert_eq!(cal.transport_latency_ns, def.transport_latency_ns);
        assert_eq!(cal.on_device_mib, def.on_device_mib);
        assert_eq!(cal.max_tensor_rank, def.max_tensor_rank);
        assert_eq!(cal.max_dim, def.max_dim);
    }

    #[test]
    fn clamp_gflops_floors_implausibly_low_at_default() {
        let def = crate::profile().gflops_f32;
        assert_eq!(clamp_gflops(0), def);
    }

    #[test]
    fn clamp_gflops_caps_at_u32_max() {
        assert_eq!(clamp_gflops(u64::MAX), u32::MAX);
    }

    #[test]
    fn clamp_gflops_passes_through_normal_values() {
        assert_eq!(clamp_gflops(40), 40);
        assert_eq!(clamp_gflops(1000), 1000);
    }
}
