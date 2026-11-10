//! VER08b runtime capability conformance for `ct-compare`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the *full public API* is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//! An independent `strace` observer must agree, negative controls must be
//! caught, and (when Docker works) the same policy is run in a container with
//! no network, a read-only root and no capabilities.
//!
//! Phase A (`prepare`, unconstrained) builds every input vector. Phase B (the
//! returned closure, armed) only calls the library and returns booleans: no
//! file, socket, clock, environment or process access is possible there, and
//! none is allowed to appear.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};
use std::panic::catch_unwind;
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// Phase A: allocate all vectors and silence the panic hook (the expected
/// panic must not need anything but `write` to fd 2, which we avoid entirely).
fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    // ct_eq vectors: equal and unequal at several lengths, difference at
    // the first, middle and last byte, plus the length-mismatch case.
    let lengths = [0usize, 1, 2, 31, 32, 33, 255, 1024];
    let mut eq_pairs: Vec<(Vec<u8>, Vec<u8>, bool)> = Vec::new();
    for &len in &lengths {
        let a: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        eq_pairs.push((a.clone(), a.clone(), true));
        for position in [0, len / 2, len.saturating_sub(1)] {
            if len > 0 {
                let mut b = a.clone();
                b[position] ^= 0x80;
                eq_pairs.push((a.clone(), b, false));
            }
        }
        let mut longer = a.clone();
        longer.push(0);
        eq_pairs.push((a.clone(), longer, false));
    }

    let u64_cases: Vec<(u64, u64, bool)> = {
        let mut cases = vec![
            (0, 0, true),
            (u64::MAX, u64::MAX, true),
            (0, u64::MAX, false),
        ];
        for bit in 0..64 {
            cases.push((
                0x1234_5678_9ABC_DEF0,
                0x1234_5678_9ABC_DEF0 ^ (1u64 << bit),
                false,
            ));
        }
        cases
    };

    let select_a = vec![0xAAu8; 40];
    let select_b = vec![0xBBu8; 40];
    // 1 MiB: above glibc's mmap threshold, so the armed phase really needs
    // mmap/munmap (the baseline justification for them is measured, not assumed).
    let big_a = vec![0x11u8; 1 << 20];
    let big_b = vec![0x22u8; 1 << 20];
    let mismatched = vec![0u8; 5];

    Box::new(move || {
        let mut checks = Vec::new();

        let ct_eq_ok = eq_pairs
            .iter()
            .all(|(a, b, expected)| ct_eq(a, b) == *expected);
        checks.push(check(
            "ct_eq: equal, single-byte-different and length-mismatch vectors",
            ct_eq_ok,
        ));

        checks.push(check(
            "ct_eq_fixed<16>",
            ct_eq_fixed(&[7u8; 16], &[7u8; 16]) && !ct_eq_fixed(&[7u8; 16], &[8u8; 16]),
        ));
        let mut one_off = [3u8; 32];
        one_off[31] ^= 1;
        checks.push(check(
            "ct_eq_fixed<32>",
            ct_eq_fixed(&[3u8; 32], &[3u8; 32]) && !ct_eq_fixed(&[3u8; 32], &one_off),
        ));
        checks.push(check(
            "ct_eq_fixed<64>",
            ct_eq_fixed(&[0xFFu8; 64], &[0xFFu8; 64]) && !ct_eq_fixed(&[0xFFu8; 64], &[0u8; 64]),
        ));
        checks.push(check("ct_eq_fixed<0>", ct_eq_fixed(&[], &[])));

        let select_ok = ct_select_bytes(&select_a, &select_b, true) == select_a
            && ct_select_bytes(&select_a, &select_b, false) == select_b
            && ct_select_bytes(&[], &[], true).is_empty()
            && ct_select_bytes(&big_a, &big_b, false) == big_b;
        checks.push(check(
            "ct_select_bytes: both choices and empty input",
            select_ok,
        ));

        // The panic path: unequal lengths must panic ("equal-length slices").
        let panicked = catch_unwind(|| ct_select_bytes(&[1, 2, 3], &[0u8; 5], true)).is_err();
        let also = catch_unwind(|| ct_select_bytes(&mismatched, &[], false)).is_err();
        checks.push(check(
            "ct_select_bytes: unequal lengths panic",
            panicked && also,
        ));

        let u64_ok = u64_cases
            .iter()
            .all(|(a, b, expected)| ct_eq_u64(*a, *b) == *expected);
        checks.push(check(
            "ct_eq_u64: equal and all 64 single-bit flips",
            u64_ok,
        ));
        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/ct-compare",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
