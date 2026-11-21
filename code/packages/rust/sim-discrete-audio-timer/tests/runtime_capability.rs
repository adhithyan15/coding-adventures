//! # VER08b runtime capability conformance for `sim-discrete-audio-timer`.
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
use sim_discrete_audio_timer::{
    calculate_555_frequency, render_noise_audio_into, render_system_audio_into,
    render_timer_audio_into, validate_sample_rate, CoinMechDebounce, DiscreteArcadeAudio,
    DiscreteNoiseFilter, Timer555Astable,
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

        checks.push(check(
            "validate sample rate",
            validate_sample_rate(44_100).is_ok(),
        ));
        checks.push(check(
            "calculate frequency",
            calculate_555_frequency(1000, 10000, 100_000).is_ok(),
        ));

        let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("timer");
        checks.push(check("timer initialized", timer.timer_frequency_hz() > 0));

        let mut out = [0i16; 64];
        let render_timer_res = render_timer_audio_into(&mut timer, &mut out);
        checks.push(check("render timer into out", render_timer_res.is_ok()));

        let mut noise = DiscreteNoiseFilter::explosion_filter(44_100).expect("noise");
        let render_noise_res = render_noise_audio_into(&mut noise, &mut out);
        checks.push(check("render noise into out", render_noise_res.is_ok()));

        let mut debouncer = CoinMechDebounce::new_debouncer(16, 99);
        let coin_res = debouncer.step_debounce(true);
        checks.push(check("step debounce", !coin_res));

        let mut system = DiscreteArcadeAudio::new_arcade_audio(44_100).expect("system");
        system.trigger_paddle_tone(100);
        let render_system_res = render_system_audio_into(&mut system, &mut out);
        checks.push(check("render system into out", render_system_res.is_ok()));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sim-discrete-audio-timer",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
