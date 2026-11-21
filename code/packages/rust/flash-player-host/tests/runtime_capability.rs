//! # VER08b runtime capability conformance for `flash-player-host`.
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
use flash_display_list::{PatchKind, PatchOp};
use flash_player_host::{
    compute_scene_digest_hex, evaluate_fscommand, is_fscommand_exec, run_headless_swf,
    sanitize_navigation_uri, validate_frame_rate_hz, validate_stage_dimension_px,
    validate_uri_length, validate_vm_step_budget, FlashPlayer, FlashPlayerConfig, FsCommandOutcome,
    HeadlessRunConfig,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn minimal_swf() -> Vec<u8> {
    let mut swf = Vec::new();
    swf.extend_from_slice(b"FWS");
    swf.push(8);
    swf.extend_from_slice(&0u32.to_le_bytes());
    swf.push(0x00); // 0-bit RECT
    swf.extend_from_slice(&[0x00, 24]); // 24 fps
    swf.extend_from_slice(&1u16.to_le_bytes()); // 1 frame
    swf.extend_from_slice(&[0x40, 0x00]); // ShowFrame (tag 1, len 0)
    swf.extend_from_slice(&[0x00, 0x00]); // End (tag 0, len 0)
    let len = swf.len() as u32;
    swf[4..8].copy_from_slice(&len.to_le_bytes());
    swf
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = vec![
            check(
                "validate_vm_step_budget",
                validate_vm_step_budget(2_000_000) == Ok(true),
            ),
            check(
                "validate_frame_rate_hz",
                validate_frame_rate_hz(24) == Ok(true),
            ),
            check(
                "validate_stage_dimension_px",
                validate_stage_dimension_px(800) == Ok(true),
            ),
            check("validate_uri_length", validate_uri_length(64) == Ok(true)),
            check(
                "sanitize_navigation_uri_allow_https",
                sanitize_navigation_uri("https://example.com/app").is_ok(),
            ),
            check(
                "sanitize_navigation_uri_block_js",
                sanitize_navigation_uri("javascript:alert(1)").is_err(),
            ),
            check(
                "fscommand_exec_blocked",
                is_fscommand_exec("exec")
                    && evaluate_fscommand("exec", "sh") == FsCommandOutcome::BlockedNoOp,
            ),
        ];

        let swf = minimal_swf();
        if let Ok(mut player) =
            FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard())
        {
            let dt_ns = player.player_clock().fixed_dt_ns;
            let mut ops = [PatchOp {
                kind: PatchKind::Added,
                old_index: 0,
                new_index: 0,
            }; 8];
            let stepped = player.player_step_with_clock(dt_ns, &mut ops, None).is_ok();
            let digest = compute_scene_digest_hex(player.player_last_scene());
            checks.push(check("player_step_with_clock", stepped));
            checks.push(check("compute_scene_digest_hex", digest.len() == 64));
        } else {
            checks.push(check("player_from_swf_bytes", false));
        }

        let headless_ok = run_headless_swf(
            &swf,
            &FlashPlayerConfig::cfg_standard(),
            &HeadlessRunConfig::headless_cfg_for_frames(1),
        )
        .is_ok();
        checks.push(check("run_headless_swf", headless_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/flash-player-host",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
