//! # VER08b runtime capability conformance for `sim-teletype-asr33`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use sim_teletype_asr33::{
    compute_even_parity, new_asr33, new_asr33_gate_level, parity_generator_7xor,
    render_paper_roll_ascii, to_paint_scene,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        let mut func = new_asr33();
        checks.push(check(
            "new_asr33 initializes idle functional state",
            func.tp_flag() && !func.kb_flag(),
        ));

        let gate = new_asr33_gate_level();
        checks.push(check(
            "new_asr33_gate_level initializes idle gate state",
            gate.tp_flag() && !gate.kb_flag(),
        ));

        let parity = compute_even_parity(0x41); // 'A'
        checks.push(check("compute_even_parity for 'A' is 0", parity == 0));

        let tree_parity = parity_generator_7xor(1, 0, 0, 0, 0, 0, 1);
        checks.push(check(
            "parity_generator_7xor for 1000001 is 0",
            tree_parity == 0,
        ));

        func.print_char(b'H');
        func.print_char(b'I');
        let text = func.paper_roll_text();
        checks.push(check(
            "paper_roll_text accumulates characters",
            text == "HI",
        ));

        let scene = to_paint_scene(&func);
        checks.push(check(
            "to_paint_scene produces valid scene",
            scene.width == 72.0 && scene.height >= 1.0,
        ));

        let ascii_render = render_paper_roll_ascii(&func);
        checks.push(check(
            "render_paper_roll_ascii produces rendered string",
            ascii_render.contains("HI"),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sim-teletype-asr33",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
