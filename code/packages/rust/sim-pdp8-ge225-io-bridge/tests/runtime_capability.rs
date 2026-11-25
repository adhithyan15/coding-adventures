//! # VER08b runtime capability conformance for `sim-pdp8-ge225-io-bridge`.
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
use sim_pdp8_ge225_io_bridge::{
    ascii_to_ge225_bcd, ge225_bcd_to_ascii, new_pdp8_teletype_bridge,
    new_pdp8_teletype_gate_bridge, pdp8_dispatch_iot_functional, GateLevelIotDecoder, PDP8_DEV_KBD,
    PDP8_IOP1,
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

        let bridge = new_pdp8_teletype_bridge();
        checks.push(check(
            "new_pdp8_teletype_bridge initializes idle bridge",
            bridge.tp_flag() && !bridge.kb_flag(),
        ));

        let gate_bridge = new_pdp8_teletype_gate_bridge();
        checks.push(check(
            "new_pdp8_teletype_gate_bridge initializes idle gate bridge",
            gate_bridge.tp_flag() && !gate_bridge.kb_flag(),
        ));

        let func_res = pdp8_dispatch_iot_functional(PDP8_DEV_KBD, PDP8_IOP1, 0, false, true, 0);
        checks.push(check(
            "pdp8_dispatch_iot_functional handles KSF",
            func_res.handled && !func_res.skip,
        ));

        let gate_res = GateLevelIotDecoder::decode(PDP8_DEV_KBD, PDP8_IOP1, 0, false, true, 0);
        checks.push(check(
            "GateLevelIotDecoder decodes KSF",
            gate_res.dev_03 == 1 && gate_res.ksf == 1 && gate_res.skip == 0,
        ));

        let bcd = ascii_to_ge225_bcd(b'A');
        checks.push(check(
            "ascii_to_ge225_bcd maps 'A' to 0o21",
            bcd == Some(0o21),
        ));

        let ascii = ge225_bcd_to_ascii(0o21);
        checks.push(check(
            "ge225_bcd_to_ascii maps 0o21 to 'A'",
            ascii == Some(b'A'),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sim-pdp8-ge225-io-bridge",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
