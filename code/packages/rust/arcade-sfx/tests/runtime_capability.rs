//! # VER08b runtime capability conformance for `arcade-sfx`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use arcade_sfx::{
    mix_bank_into, render_sfx_into, validate_frequency, validate_sample_rate, SfxBank, SfxPreset,
    SfxVoice,
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

        checks.push(check("validate sample rate", validate_sample_rate(44_100).is_ok()));
        checks.push(check("validate frequency", validate_frequency(1000, 44_100).is_ok()));

        let mut voice = SfxVoice::new(44_100).expect("voice new");
        checks.push(check("voice initialized", voice.sample_rate_hz == 44_100));

        let trig_res = voice.trigger_preset(SfxPreset::PongPaddleHit);
        checks.push(check("voice trigger preset", trig_res.is_ok()));

        let mut out = [0i16; 64];
        let render_res = render_sfx_into(&mut voice, &mut out);
        checks.push(check("render sfx into out", render_res.is_ok()));

        let mut bank = SfxBank::<4>::empty(44_100).expect("bank empty");
        let bank_trig = bank.trigger_bank_preset(SfxPreset::CoinInsert);
        checks.push(check("bank trigger preset", bank_trig.is_ok()));

        let mix_res = mix_bank_into(&mut bank, &mut out);
        checks.push(check("mix bank into out", mix_res.is_ok()));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/arcade-sfx",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
