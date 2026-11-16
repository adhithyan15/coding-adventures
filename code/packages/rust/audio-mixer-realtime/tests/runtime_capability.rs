//! # VER08b runtime capability conformance for `audio-mixer-realtime`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use audio_mixer_realtime::{
    mix_channels_into, PcmRingBuffer, RealtimeMixerState, UNITY_GAIN_Q16,
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

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        let mut state = RealtimeMixerState::new(44_100).expect("valid sample rate");
        checks.push(check("state initialized", state.sample_rate_hz == 44_100));

        let alloc_res = state.allocate_voice(1, 10, UNITY_GAIN_Q16, false);
        checks.push(check("allocate voice", alloc_res.is_ok()));

        let source = [1000i16; 16];
        let sources = [&source[..]];
        let mut out = [0i16; 8];
        let mix_res = mix_channels_into(&mut state, &sources, &mut out);
        checks.push(check("mix channels into out", mix_res.is_ok()));

        let mut ring: PcmRingBuffer<16> = PcmRingBuffer::empty();
        let push_res = ring.push(500);
        checks.push(check("ring push", push_res.is_ok()));
        let popped = ring.pop();
        checks.push(check("ring pop", popped == Some(500)));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/audio-mixer-realtime",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
