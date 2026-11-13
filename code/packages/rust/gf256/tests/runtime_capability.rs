//! VER08b runtime capability conformance for `gf256`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time rather than by reading the
//! source: the whole public API -- the module-level `add`, `subtract`,
//! `multiply`, `divide`, `power`, `inverse` and the `Field` struct that carries
//! the same six operations for any primitive polynomial -- is exercised inside
//! a process that has been sandboxed, mechanically from that manifest, to the
//! minimal armed baseline (allocator + exit + TAP output).  Landlock denies the
//! filesystem, seccomp denies every syscall outside the baseline, an
//! independent `strace` observer must agree with the enforcement, negative
//! controls must be caught, and (when Docker works) the same policy is run in a
//! container with no network, a read-only root and no capabilities.
//!
//! The manifest's justification mentions a process-global pair of lookup tables
//! behind `std::sync::OnceLock`.  That is the one piece of shared state the
//! library has, so this test deliberately does NOT warm it in phase A: the
//! first armed call to `multiply` builds the tables inside the jail.  If table
//! construction needed a file, a clock or a socket, the jail would kill it.
//!
//! Phase A (`prepare`, unconstrained) builds the operand lists.  Phase B (the
//! returned closure, armed) only calls the library and returns named checks: no
//! file, socket, clock, environment or process access is possible there, and
//! none is allowed to appear.  Phase B calls only VALID inputs: division and
//! inversion of zero panic by contract, and a panic prints to stderr, which the
//! baseline does not allow; those rejections are pinned unjailed by
//! `tests/gf256_test.rs` and the generated Warrant harness.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use gf256::Field;
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// Phase A: build the operand lists.  Phase B only reads them through the
/// closure's captured values.
fn prepare() -> Exercise {
    // Every byte, as a list, so phase B can run the full 256 x 256 square.
    let bytes: Vec<u8> = (0..=255u8).collect();
    // Exponents that cover zero, one, the group order 255 and its neighbours,
    // and values far above it (the library reduces them modulo 255).
    let exps: Vec<u32> = vec![0, 1, 2, 7, 8, 254, 255, 256, 509, 510, 100_000, u32::MAX];

    Box::new(move || {
        let mut checks = Vec::new();

        // Characteristic 2: add and subtract are both XOR, for every pair.
        let add_ok = bytes.iter().all(|&a| {
            bytes
                .iter()
                .all(|&b| gf256::add(a, b) == a ^ b && gf256::subtract(a, b) == a ^ b)
        });
        checks.push(check("add/subtract: XOR for all 65,536 pairs", add_ok));

        // multiply: identity, absorber, commutativity.  This is the first call
        // that touches the OnceLock tables, so they are built inside the jail.
        let mul_ok = bytes.iter().all(|&a| {
            gf256::multiply(a, 1) == a
                && gf256::multiply(a, 0) == 0
                && bytes
                    .iter()
                    .all(|&b| gf256::multiply(a, b) == gf256::multiply(b, a))
        });
        checks.push(check(
            "multiply: identity, zero and commutativity for all pairs",
            mul_ok,
        ));
        checks.push(check(
            "multiply: known vectors (2*128 = 0x1D, 3*7 = 9)",
            gf256::multiply(2, 128) == 0x1D && gf256::multiply(3, 7) == 9,
        ));

        // inverse and divide on every non-zero element.
        let inv_ok = bytes
            .iter()
            .filter(|&&a| a != 0)
            .all(|&a| gf256::multiply(a, gf256::inverse(a)) == 1);
        checks.push(check("inverse: a * inverse(a) == 1 for all 255 a", inv_ok));
        let div_ok = bytes.iter().all(|&a| {
            bytes
                .iter()
                .filter(|&&b| b != 0)
                .all(|&b| gf256::multiply(gf256::divide(a, b), b) == a)
        });
        checks.push(check("divide: (a / b) * b == a for all b != 0", div_ok));

        // power: x^0 = 1, x^1 = x, the group order, and reduction mod 255.
        let pow_ok = bytes.iter().filter(|&&a| a != 0).all(|&a| {
            gf256::power(a, 0) == 1
                && gf256::power(a, 1) == a
                && gf256::power(a, 255) == 1
                && exps
                    .iter()
                    .all(|&e| gf256::power(a, e) == gf256::power(a, e % 255))
        }) && gf256::power(0, 0) == 1
            && gf256::power(0, 5) == 0;
        checks.push(check(
            "power: identity, group order 255 and exponent reduction",
            pow_ok,
        ));

        // Field: construct both well-known fields, inside the jail.
        let rs = Field::new(0x11D);
        let aes = Field::new(0x11B);
        let rs_ok = bytes.iter().all(|&a| {
            rs.add(a, 7) == gf256::add(a, 7)
                && rs.subtract(a, 7) == gf256::subtract(a, 7)
                && bytes
                    .iter()
                    .all(|&b| rs.multiply(a, b) == gf256::multiply(a, b))
        }) && bytes
            .iter()
            .filter(|&&a| a != 0)
            .all(|&a| rs.multiply(a, rs.inverse(a)) == 1 && rs.divide(a, a) == 1)
            && rs.power(2, 8) == 0x1D;
        checks.push(check(
            "Field(0x11D): agrees with the module-level operations",
            rs_ok,
        ));
        let aes_ok = aes.multiply(0x53, 0xCA) == 1
            && aes.inverse(0x53) == 0xCA
            && aes.divide(1, 0x53) == 0xCA
            && aes.power(3, 255) == 1
            && bytes
                .iter()
                .filter(|&&a| a != 0)
                .all(|&a| aes.multiply(a, aes.inverse(a)) == 1);
        checks.push(check(
            "Field(0x11B): FIPS 197 vectors (0x53 * 0xCA = 1) and inverses",
            aes_ok,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/gf256",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
