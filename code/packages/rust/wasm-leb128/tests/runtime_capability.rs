//! VER08b runtime capability conformance for `wasm-leb128`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time: the *full public API* --
//! both encoders, both plain decoders and both bounded decoders, including the
//! `max_bits > 64` rejection (SEC-006) -- is exercised inside a process that has
//! been sandboxed, mechanically from that manifest, to the minimal armed
//! baseline (allocator + exit + TAP output).  An independent `strace` observer
//! must agree, negative controls must be caught, and (when Docker works) the
//! same policy is run in a container with no network, a read-only root and no
//! capabilities.
//!
//! Phase A (`prepare`, unconstrained) builds every input vector.  Phase B (the
//! returned closure, armed) only calls the library and returns named checks: no
//! file, socket, clock, environment or process access is possible there, and
//! none is allowed to appear.  Every decode error allocates a message `String`;
//! the baseline allows the allocator, so that path is exercised on purpose.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use std::path::PathBuf;
use wasm_leb128::{
    decode_signed, decode_signed_bounded, decode_unsigned, decode_unsigned_bounded,
    encode_signed, encode_unsigned,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// Phase A: build every input vector.  Nothing here is needed by phase B except
/// through the captured values.
fn prepare() -> Exercise {
    // Unsigned values: zero, each byte-length boundary (2^(7k) - 1 and 2^(7k)),
    // 32/64-bit extremes.
    let mut unsigned: Vec<u64> = vec![0, 1, 127, 128, 624_485, u32::MAX as u64, u64::MAX];
    for k in 1..=9u32 {
        unsigned.push((1u64 << (7 * k)) - 1);
        unsigned.push(1u64 << (7 * k));
    }
    // Signed values: both signs around each sign-byte boundary, and the extremes.
    let mut signed: Vec<i64> = vec![0, 1, -1, 63, 64, -64, -65, i32::MIN as i64, i32::MAX as i64, i64::MIN, i64::MAX];
    for k in 1..=8u32 {
        let b = 1i64 << (7 * k - 1);
        signed.extend([b - 1, b, -b, -b - 1]);
    }

    // Fixed byte strings (hex-free, so phase B needs no parsing).
    let padded_zero = vec![0x80u8, 0x00];
    let unterminated = vec![0x80u8];
    let u64_max = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
    let two_to_64 = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
    let eleven_overlong = vec![0x80u8, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00];
    let nine_ff_7f = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F];
    let eleven_80_01 = vec![0x80u8, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
    let u32_padded = vec![0x82u8, 0x80, 0x80, 0x80, 0x00];
    let u32_overlong = vec![0x82u8, 0x80, 0x80, 0x80, 0x80, 0x00];
    let two_to_32 = vec![0x80u8, 0x80, 0x80, 0x80, 0x10];
    let i32_min = vec![0x80u8, 0x80, 0x80, 0x80, 0x78];
    let two_to_31 = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0x0F];
    let prefixed = vec![0x00u8, 0x00, 0xE5, 0x8E, 0x26, 0xAA];
    // Include a large buffer so the armed phase also takes the allocator's
    // larger paths (the encoders allocate their own Vec).
    let widths_too_wide: Vec<u32> = vec![65, 66, 70, 71, 128, 1 << 16, u32::MAX - 1, u32::MAX];

    Box::new(move || {
        let mut checks = Vec::new();

        let enc_u_ok = unsigned.iter().all(|&v| {
            let bytes = encode_unsigned(v);
            !bytes.is_empty()
                && bytes.len() <= 10
                && decode_unsigned(&bytes, 0) == Ok((v, bytes.len()))
        });
        checks.push(check("encode_unsigned: canonical form round-trips (boundaries and extremes)", enc_u_ok));

        let enc_s_ok = signed.iter().all(|&v| {
            let bytes = encode_signed(v);
            !bytes.is_empty()
                && bytes.len() <= 10
                && decode_signed(&bytes, 0) == Ok((v, bytes.len()))
        });
        checks.push(check("encode_signed: canonical form round-trips (boundaries and extremes)", enc_s_ok));

        checks.push(check(
            "encode: known vectors",
            encode_unsigned(624_485) == [0xE5, 0x8E, 0x26]
                && encode_signed(-2) == [0x7E]
                && encode_signed(64) == [0xC0, 0x00],
        ));

        checks.push(check(
            "decode_unsigned: known value, padding, offset, trailing bytes",
            decode_unsigned(&[0xE5, 0x8E, 0x26], 0) == Ok((624_485, 3))
                && decode_unsigned(&padded_zero, 0) == Ok((0, 2))
                && decode_unsigned(&prefixed, 2) == Ok((624_485, 3))
                && decode_unsigned(&u64_max, 0) == Ok((u64::MAX, 10)),
        ));
        checks.push(check(
            "decode_unsigned: rejects 2^64, eleven-byte overlong, unterminated, empty, bad offset",
            decode_unsigned(&two_to_64, 0).is_err()
                && decode_unsigned(&eleven_overlong, 0).is_err()
                && decode_unsigned(&unterminated, 0).is_err()
                && decode_unsigned(&[], 0).is_err()
                && decode_unsigned(&[1], 5).is_err(),
        ));

        checks.push(check(
            "decode_signed: known values, extremes, offset",
            decode_signed(&[0x7F], 0) == Ok((-1, 1))
                && decode_signed(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7F], 0)
                    == Ok((i64::MIN, 10))
                && decode_signed(&prefixed, 2).is_ok(),
        ));
        checks.push(check(
            "decode_signed: rejects bad padding, unterminated, empty, bad offset",
            decode_signed(&u64_max, 0).is_err()
                && decode_signed(&unterminated, 0).is_err()
                && decode_signed(&[], 0).is_err()
                && decode_signed(&[1], 5).is_err(),
        ));

        checks.push(check(
            "decode_unsigned_bounded: u32 width accepts padding, rejects overlong and 2^32",
            decode_unsigned_bounded(&u32_padded, 0, 32) == Ok((2, 5))
                && decode_unsigned_bounded(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F], 0, 32)
                    == Ok((u32::MAX as u64, 5))
                && decode_unsigned_bounded(&u32_overlong, 0, 32).is_err()
                && decode_unsigned_bounded(&two_to_32, 0, 32).is_err(),
        ));
        checks.push(check(
            "decode_unsigned_bounded: width 0 admits only 0; width 64 equals decode_unsigned",
            decode_unsigned_bounded(&[0x00], 0, 0) == Ok((0, 1))
                && decode_unsigned_bounded(&[0x01], 0, 0).is_err()
                && decode_unsigned_bounded(&u64_max, 0, 64) == decode_unsigned(&u64_max, 0)
                && decode_unsigned_bounded(&two_to_64, 0, 64).is_err(),
        ));
        checks.push(check(
            "decode_signed_bounded: i32 extremes, 2^31 rejected, width 64 equals decode_signed",
            decode_signed_bounded(&i32_min, 0, 32) == Ok((i32::MIN as i64, 5))
                && decode_signed_bounded(&[0xFF, 0xFF, 0xFF, 0xFF, 0x07], 0, 32)
                    == Ok((i32::MAX as i64, 5))
                && decode_signed_bounded(&two_to_31, 0, 32).is_err()
                && decode_signed_bounded(&u64_max, 0, 64) == decode_signed(&u64_max, 0)
                && decode_signed_bounded(&[0x00], 0, 0) == Ok((0, 1))
                && decode_signed_bounded(&[0x7F], 0, 0).is_err(),
        ));

        // SEC-006: a width the u64 accumulator cannot hold is an error, never
        // a truncation (FF x9 7F) and never a shift overflow (80 x10 01).
        let wide_unsigned = widths_too_wide.iter().all(|&w| {
            decode_unsigned_bounded(&nine_ff_7f, 0, w).is_err()
                && decode_unsigned_bounded(&eleven_80_01, 0, w).is_err()
                && decode_unsigned_bounded(&[], 0, w).is_err()
        });
        checks.push(check("decode_unsigned_bounded: max_bits > 64 rejected", wide_unsigned));
        let wide_signed = widths_too_wide.iter().all(|&w| {
            decode_signed_bounded(&nine_ff_7f, 0, w).is_err()
                && decode_signed_bounded(&eleven_80_01, 0, w).is_err()
                && decode_signed_bounded(&[0x00], 9, w).is_err()
        });
        checks.push(check("decode_signed_bounded: max_bits > 64 rejected", wide_signed));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/wasm-leb128",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
