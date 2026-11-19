//! VER08b capability gate for the pure GAME input reducer.
//!
//! The manifest declares no host capabilities. On Linux, the runner executes
//! representative success and refusal paths inside its manifest-derived
//! minimal sandbox. Other hosts report NOT-LIVE rather than a false pass.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use game_input_frame::{
    reduce_frame, ActionChange, AxisActionId, ButtonActionId, FrameError, InputState, PlayerId,
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
    Box::new(|| {
        let mut checks = Vec::new();
        let previous = InputState {
            player_id: PlayerId::new(u32::MAX),
            held_buttons: 0,
            axes: [0; 16],
        };
        let changes = [
            ActionChange::Button {
                action_id: ButtonActionId::new(63),
                down: true,
            },
            ActionChange::Button {
                action_id: ButtonActionId::new(63),
                down: false,
            },
            ActionChange::Axis {
                action_id: AxisActionId::new(15),
                value: -32767,
            },
        ];
        let frame = reduce_frame(&previous, u64::MAX, &changes);
        checks.push(check("valid frame accepted", frame.is_ok()));
        if let Ok(frame) = frame {
            checks.push(check("caller tick preserved", frame.tick == u64::MAX));
            checks.push(check(
                "highest button tap edges",
                frame.pressed_buttons == (1_u64 << 63) && frame.released_buttons == (1_u64 << 63),
            ));
            checks.push(check("axis endpoint preserved", frame.axes[15] == -32767));
        }
        let invalid = reduce_frame(
            &previous,
            0,
            &[ActionChange::Axis {
                action_id: AxisActionId::new(15),
                value: i16::MIN,
            }],
        );
        checks.push(check(
            "invalid axis refused with exact error",
            matches!(
                invalid,
                Err(FrameError::InvalidAxisValue {
                    change_index: 0,
                    axis_slot: 15,
                    value: i16::MIN
                })
            ),
        ));
        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-input-frame",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
