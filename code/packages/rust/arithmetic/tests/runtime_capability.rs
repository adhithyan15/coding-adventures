//! VER08b runtime capability conformance for `arithmetic`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time: EVERY public operation --
//! `half_adder`, `full_adder`, `ripple_carry_adder`, `ripple_carry_adder_with_carry`
//! and `alu` with each of its six operations -- is exercised inside a process
//! that has been sandboxed, mechanically from that manifest, to the minimal
//! armed baseline (allocator + exit + TAP output).  An independent `strace`
//! observer must agree, negative controls must be caught, and (when Docker
//! works) the same policy is run in a container with no network, a read-only
//! root and no capabilities.
//!
//! The jail's scope is the declared capabilities of this package AND of its
//! dependency closure (`logic-gates`, `transistors`, both pure), plus whatever
//! the test program itself declares (VER27 section 6.1): nothing here is allowed
//! beyond the empty set.
//!
//! Phase A (`prepare`, unconstrained) builds every operand.  Phase B (the
//! returned closure, armed) only calls the library and returns named checks: no
//! file, socket, clock, environment or process access is possible there, and
//! none is allowed to appear.  Phase B calls only VALID inputs: a rejected input
//! panics, and a panic prints to stderr, which the baseline does not allow; the
//! rejections are pinned unjailed, by `tests/bit_domain.rs` and the Warrant
//! harness.
//!
//! The oracle in phase B is the host language's own integer arithmetic (the
//! exact function the Warrant claims define, written a third time here, in Rust
//! `u64`, so that a disagreement between the library and the host is a failed
//! check, not a shared mistake).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use arithmetic::{
    alu, full_adder, half_adder, ripple_carry_adder, ripple_carry_adder_with_carry, AluOp,
};
use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// The bits of `value`, least significant first, `width` of them: the slice
/// convention of every operation in the crate.
fn bits(value: u64, width: usize) -> Vec<u8> {
    (0..width).map(|i| ((value >> i) & 1) as u8).collect()
}

/// `value` read back from its LSB-first bits.
fn value(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| acc | (u64::from(b) << i))
}

/// The two's-complement reading of the low `width` bits of `v`.
fn signed(v: u64, width: usize) -> i64 {
    let v = v & ((1u64 << width) - 1);
    if v >> (width - 1) & 1 == 1 {
        v as i64 - (1i64 << width)
    } else {
        v as i64
    }
}

/// Is `s` representable as a signed `width`-bit number?
fn fits(s: i64, width: usize) -> bool {
    let half = 1i64 << (width - 1);
    (-half..half).contains(&s)
}

/// Phase A: build every operand pair.  Nothing here is needed by phase B except
/// through the captured values.
fn prepare() -> Exercise {
    // Every pair of 4-bit operands (256), and a spread of 8-bit and 16-bit
    // pairs (the boundaries and a stride), as (a, b) values.
    let mut pairs4: Vec<(u64, u64)> = Vec::new();
    for a in 0..16u64 {
        for b in 0..16u64 {
            pairs4.push((a, b));
        }
    }
    let edge8 = [0u64, 1, 2, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff];
    let mut pairs8: Vec<(u64, u64)> = Vec::new();
    for &a in &edge8 {
        for &b in &edge8 {
            pairs8.push((a, b));
        }
    }
    let edge16 = [0u64, 1, 0x7fff, 0x8000, 0x8001, 0xfffe, 0xffff, 0x1234, 0xabcd];
    let mut pairs16: Vec<(u64, u64)> = Vec::new();
    for &a in &edge16 {
        for &b in &edge16 {
            pairs16.push((a, b));
        }
    }
    // A stride of the 8-bit space for the ALU (every 5th a against every 3rd b, 1,820
    // pairs); the Warrant harness enumerates all 65,536 unjailed.
    let mut all8: Vec<(Vec<u8>, Vec<u8>, u64, u64)> = Vec::new();
    for a in (0..256u64).step_by(5) {
        for b in (0..256u64).step_by(3) {
            all8.push((bits(a, 8), bits(b, 8), a, b));
        }
    }

    Box::new(move || {
        let mut checks = Vec::new();

        // The half adder and the full adder over every input, against `u8`
        // addition: sum is the low bit, carry the high bit.
        let mut single_ok = true;
        for a in 0..=1u8 {
            for b in 0..=1u8 {
                let (s, c) = half_adder(a, b);
                single_ok &= u8::from(s) + 2 * u8::from(c) == a + b;
                for cin in 0..=1u8 {
                    let (s, c) = full_adder(a, b, cin);
                    single_ok &= s + 2 * c == a + b + cin;
                }
            }
        }
        checks.push(check("half_adder, full_adder: every input equals a + b (+ cin)", single_ok));

        // 4-bit ripple-carry: every (a, b, cin) = 512 cases, sum mod 16 and carry.
        let mut rc4_ok = true;
        for &(a, b) in &pairs4 {
            let (ab, bb) = (bits(a, 4), bits(b, 4));
            let plain = ripple_carry_adder(&ab, &bb);
            rc4_ok &= value(&plain.sum) == (a + b) % 16 && u64::from(plain.carry_out) == (a + b) / 16;
            for cin in 0..=1u8 {
                let r = ripple_carry_adder_with_carry(&ab, &bb, cin);
                let total = a + b + u64::from(cin);
                rc4_ok &= value(&r.sum) == total % 16 && u64::from(r.carry_out) == total / 16;
                rc4_ok &= r.overflow == !fits(signed(a, 4) + signed(b, 4) + i64::from(cin), 4);
            }
        }
        checks.push(check("ripple_carry_adder(_with_carry): 4-bit, every (a, b, cin)", rc4_ok));

        // 8-bit and 16-bit: the boundary values against each other.
        let mut wide_ok = true;
        for (width, set) in [(8usize, &pairs8), (16usize, &pairs16)] {
            for &(a, b) in set.iter() {
                for cin in 0..=1u8 {
                    let r = ripple_carry_adder_with_carry(&bits(a, width), &bits(b, width), cin);
                    let total = a + b + u64::from(cin);
                    wide_ok &= value(&r.sum) == total & ((1u64 << width) - 1);
                    wide_ok &= u64::from(r.carry_out) == total >> width;
                    wide_ok &= r.overflow
                        == !fits(signed(a, width) + signed(b, width) + i64::from(cin), width);
                }
            }
        }
        checks.push(check("ripple_carry_adder_with_carry: 8-bit and 16-bit boundaries", wide_ok));

        // ALU, every operation, every 4-bit pair, against the host's integer
        // and bitwise operators, with all four flags.
        let mut alu4_ok = true;
        for &(a, b) in &pairs4 {
            let (ab, bb) = (bits(a, 4), bits(b, 4));
            for op in [AluOp::Add, AluOp::Sub, AluOp::And, AluOp::Or, AluOp::Xor, AluOp::Not] {
                let r = alu(&ab, &bb, op);
                let (want, carry, ovf) = match op {
                    AluOp::Add => (
                        (a + b) & 15,
                        a + b >= 16,
                        !fits(signed(a, 4) + signed(b, 4), 4),
                    ),
                    AluOp::Sub => (
                        a.wrapping_sub(b) & 15,
                        a >= b,
                        !fits(signed(a, 4) - signed(b, 4), 4),
                    ),
                    AluOp::And => (a & b, false, false),
                    AluOp::Or => (a | b, false, false),
                    AluOp::Xor => (a ^ b, false, false),
                    AluOp::Not => (!a & 15, false, false),
                };
                alu4_ok &= value(&r.result) == want
                    && r.zero == (want == 0)
                    && r.negative == (want >> 3 & 1 == 1)
                    && r.carry == carry
                    && r.overflow == ovf;
            }
        }
        checks.push(check("alu: six operations, every 4-bit pair, all four flags", alu4_ok));

        // ALU at 8 bits over a stride of the space (every 5th a, every 3rd b).
        let mut alu8_ok = true;
        for (ab, bb, a, b) in &all8 {
            let add = alu(ab, bb, AluOp::Add);
            alu8_ok &= value(&add.result) == (a + b) & 255 && add.carry == (a + b >= 256);
            let sub = alu(ab, bb, AluOp::Sub);
            alu8_ok &= value(&sub.result) == a.wrapping_sub(*b) & 255 && sub.carry == (a >= b);
            alu8_ok &= value(&alu(ab, bb, AluOp::And).result) == a & b;
            alu8_ok &= value(&alu(ab, bb, AluOp::Or).result) == a | b;
            alu8_ok &= value(&alu(ab, bb, AluOp::Xor).result) == a ^ b;
            alu8_ok &= value(&alu(ab, bb, AluOp::Not).result) == !a & 255;
        }
        checks.push(check("alu: 8-bit, a stride of the whole space, every operation", alu8_ok));

        // 16-bit ALU boundaries, and Not ignoring `b`.
        let mut alu16_ok = true;
        for &(a, b) in &pairs16 {
            let (ab, bb) = (bits(a, 16), bits(b, 16));
            alu16_ok &= value(&alu(&ab, &bb, AluOp::Add).result) == (a + b) & 0xffff;
            alu16_ok &= value(&alu(&ab, &bb, AluOp::Sub).result) == a.wrapping_sub(b) & 0xffff;
            alu16_ok &= alu(&ab, &bb, AluOp::Not) == alu(&ab, &[], AluOp::Not);
        }
        checks.push(check("alu: 16-bit boundaries; Not does not read b", alu16_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/arithmetic",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
