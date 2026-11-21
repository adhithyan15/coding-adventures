//! # VER08b runtime capability conformance for `swf-conformance-corpus`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use swf_conformance_corpus::{
    run_all_corpus_cases, run_corpus_case, validate_corpus_bit_width, validate_corpus_digest_hex,
    validate_corpus_frame_count, validate_corpus_swf_bytes, CorpusId, CorpusRunner, SwfBitWriter,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = vec![
            check(
                "validate_corpus_swf_bytes",
                validate_corpus_swf_bytes(4096) == Ok(true),
            ),
            check(
                "validate_corpus_frame_count",
                validate_corpus_frame_count(16) == Ok(true),
            ),
            check(
                "validate_corpus_bit_width",
                validate_corpus_bit_width(15) == Ok(true),
            ),
            check(
                "validate_corpus_digest_hex",
                validate_corpus_digest_hex(
                    b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                ) == Ok(true),
            ),
        ];

        let mut bw = SwfBitWriter::bit_writer_new();
        let bw_ok =
            bw.bit_writer_write_rect(0, 800, 0, 600).is_ok() && bw.bit_writer_into_bytes().is_ok();
        checks.push(check("swf_bit_writer_rect", bw_ok));

        let single_ok = run_corpus_case(CorpusId::Shapes).is_ok();
        checks.push(check("run_corpus_case_shapes", single_ok));

        let all_ok = run_all_corpus_cases().is_ok();
        checks.push(check("run_all_corpus_cases", all_ok));

        let adv_ok = CorpusRunner::runner_new()
            .runner_execute_all_adversarial()
            .is_ok();
        checks.push(check("runner_execute_all_adversarial", adv_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/swf-conformance-corpus",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
