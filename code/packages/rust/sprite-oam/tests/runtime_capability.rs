//! # VER08b runtime capability conformance for `sprite-oam`.
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
use sprite_oam::{Animation, Animator, MetaFrame, OamEntry, OamTable, PlayMode};
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

        let mut table = OamTable::blank_table();
        let sprite = OamEntry {
            x: 8,
            y: 8,
            hidden: false,
            ..OamEntry::blank_entry()
        };
        checks.push(check(
            "set_entry succeeds",
            table.set_entry(3, sprite).is_ok(),
        ));
        checks.push(check(
            "set_entry rejects bad slot",
            table.set_entry(128, sprite).is_err(),
        ));

        let mut order = [0u8; 128];
        checks.push(check(
            "draw_order_into succeeds",
            table.draw_order_into(&mut order) == Ok(1) && order[0] == 3,
        ));

        let mut line = [0u8; 8];
        let report = table.scanline_sprites_into(10, 8, &mut line);
        checks.push(check(
            "scanline_sprites_into succeeds",
            report.map(|r| r.selected) == Ok(1),
        ));
        checks.push(check(
            "texel_at samples",
            table.texel_at(3, 9, 9) == Ok(Some((1, 1))),
        ));

        if let Ok(frame) = MetaFrame::frame_from_parts(&[sprite], 2) {
            let frames = [frame];
            match Animation::bind(&frames, PlayMode::Loop) {
                Ok(anim) => {
                    let mut animator = Animator::start();
                    checks.push(check(
                        "animator advances",
                        animator.advance(&anim, 5).is_ok(),
                    ));
                    checks.push(check(
                        "stamp_frame succeeds",
                        table.stamp_frame(10, &frame, 20, 20, false, false) == Ok(1),
                    ));
                }
                Err(_) => checks.push(check("animation binds", false)),
            }
        } else {
            checks.push(check("frame builds", false));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sprite-oam",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
