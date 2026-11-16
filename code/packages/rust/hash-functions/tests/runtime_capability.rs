//! VER08b runtime capability conformance for `hash-functions`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time for the eight byte-string
//! hashes that `code/specs/hash-functions.warrant` covers (DT17a section 2):
//! each is exercised on published and independently computed known answers
//! inside a process that has been sandboxed, mechanically from that manifest, to
//! the minimal armed baseline (allocator + exit + TAP output).  An independent
//! `strace` observer must agree, negative controls must be caught, and (when
//! Docker works) the same policy is run in a container with no network, a
//! read-only root and no capabilities.
//!
//! The crate's `avalanche_score` draws OS randomness (getrandom) and is NOT
//! called here: it is outside the covered surface (DT17a section 3) and would,
//! correctly, be stopped by this jail.  The covered functions touch no file,
//! socket, clock, environment or process, and none is allowed to appear.
//!
//! Phase A (`prepare`, unconstrained) builds every input vector.  Phase B (the
//! returned closure, armed) only calls the library and returns named checks.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use coding_adventures_hash_functions::{
    djb2, fnv1a_32, fnv1a_64, murmur3_32, murmur3_32_with_seed, polynomial_rolling,
    polynomial_rolling_with_params, siphash_2_4,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// Phase A: build every input vector.  Nothing here is needed by phase B except
/// through the captured values.
fn prepare() -> Exercise {
    let empty: Vec<u8> = Vec::new();
    let a = b"a".to_vec();
    let abc = b"abc".to_vec();
    let hello = b"hello".to_vec();
    let foobar = b"foobar".to_vec();
    let hello_world = b"Hello, world!".to_vec();
    let fox = b"The quick brown fox jumps over the lazy dog".to_vec();
    // SipHash: the published key 00 01 .. 0f, and inputs 00 .. 0e (15 bytes) and
    // 00 .. 0f (16 bytes), the tail-only and the block-plus-no-tail paths.
    let key: [u8; 16] = core::array::from_fn(|i| i as u8);
    let zero = vec![0u8];
    let fifteen: Vec<u8> = (0..15u8).collect();
    let sixteen: Vec<u8> = (0..16u8).collect();
    // A longer buffer so the armed phase also runs several Murmur3 blocks and
    // SipHash blocks.
    let long: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();

    Box::new(move || {
        // Long inputs run, are deterministic, and depend on every input (the key,
        // the data); the result is not compared with a constant here.
        let mut other_key = key;
        other_key[0] ^= 1;
        vec![
            check(
                "fnv1a_32: published vectors",
                fnv1a_32(&empty) == 2_166_136_261
                    && fnv1a_32(&foobar) == 3_214_735_720
                    && fnv1a_32(&fox) == 76_545_936,
            ),
            check(
                "fnv1a_64: published vectors",
                fnv1a_64(&empty) == 14_695_981_039_346_656_037
                    && fnv1a_64(&foobar) == 9_625_390_261_332_436_968
                    && fnv1a_64(&fox) == 17_580_284_887_202_820_368,
            ),
            check(
                "djb2: known values",
                djb2(&empty) == 5381
                    && djb2(&hello) == 210_714_636_441
                    && djb2(&foobar) == 6_953_516_687_550,
            ),
            check(
                "polynomial_rolling: known values and agreement with the parameterised form",
                polynomial_rolling(&abc) == 96_354
                    && polynomial_rolling(&a)
                        == polynomial_rolling_with_params(&a, 31, (1u64 << 61) - 1)
                    && polynomial_rolling_with_params(&abc, 37, 1_000_000_007) == 136_518
                    && polynomial_rolling_with_params(&fox, 257, 65_521) == 420,
            ),
            check(
                "murmur3_32 / murmur3_32_with_seed: known values, all tail lengths",
                murmur3_32(&abc) == 3_017_643_002
                    && murmur3_32_with_seed(&hello_world, 0x9747_b28c) == 612_912_314
                    && murmur3_32_with_seed(&fox, 0x9747_b28c) == 799_549_133
                    && murmur3_32(&empty) == murmur3_32_with_seed(&empty, 0)
                    && (0..=8usize)
                        .all(|n| murmur3_32(&long[..n]) == murmur3_32_with_seed(&long[..n], 0)),
            ),
            check(
                "siphash_2_4: published vectors (lengths 0, 1, 15 and the block boundary 16)",
                siphash_2_4(&empty, &key) == 8_246_050_544_436_514_353
                    && siphash_2_4(&zero, &key) == 8_428_550_223_375_919_101
                    && siphash_2_4(&fifteen, &key) == 11_613_035_633_349_379_557
                    && siphash_2_4(&sixteen, &key) == 4_551_675_220_716_592_091,
            ),
            check(
                "long input: deterministic, and sensitive to the data and the key",
                fnv1a_32(&long) == fnv1a_32(&long)
                    && fnv1a_64(&long) != fnv1a_64(&long[1..])
                    && djb2(&long) != djb2(&long[1..])
                    && murmur3_32(&long) != murmur3_32(&long[1..])
                    && siphash_2_4(&long, &key) == siphash_2_4(&long, &key)
                    && siphash_2_4(&long, &key) != siphash_2_4(&long, &other_key),
            ),
        ]
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/hash-functions",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
