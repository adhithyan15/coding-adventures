//! # VER08b runtime capability conformance for `rpg-battle-engine`.
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
use rpg_battle_engine::{
    attempt_capture, calc_damage, catch_value, dual_multiplier, fade_params,
    fade_trigger_for_phase, flee_succeeds, hp_bar_blend_color, hp_bar_pixels, hp_bar_state,
    BaseStats, Battle, BattleRng, Category, Command, Creature, CreatureSpec, Effect, HpBarState,
    Move, Phase, Status, FIRE, GRASS, NO_TYPE, WATER,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

const HIT: Move = Move {
    move_type: FIRE,
    category: Category::Special,
    power: 60,
    accuracy: 0,
    max_pp: 20,
    priority: 0,
    effect: Effect::NoEffect,
};

fn creature(t1: u8, level: u8) -> Option<Creature> {
    let moves = [HIT];
    Creature::from_spec(&CreatureSpec {
        type1: t1,
        type2: NO_TYPE,
        level,
        base: BaseStats {
            hp: 60,
            atk: 60,
            def: 60,
            spa: 60,
            spd: 60,
            spe: 60,
        },
        moves: &moves,
        catch_rate: 255,
        exp_yield: 64,
    })
    .ok()
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        checks.push(check(
            "dual_multiplier fire vs grass/bug is 4x",
            dual_multiplier(FIRE, GRASS, 11) == Ok(64),
        ));

        let (Some(a), Some(b)) = (creature(FIRE, 30), creature(WATER, 30)) else {
            checks.push(check("creatures construct", false));
            return checks;
        };
        let dmg = calc_damage(&a, &b, &HIT, 100, false);
        checks.push(check("calc_damage deals damage", dmg.damage > 0));

        match Battle::start(&[a], &[b], true, 42) {
            Ok(mut battle) => {
                let began = battle.begin().is_ok();
                let submitted = battle
                    .submit_turn(Command::Fight { slot: 0 }, Command::Fight { slot: 0 })
                    .is_ok();
                let resolved = battle.resolve_turn().is_ok();
                checks.push(check(
                    "battle turn runs",
                    began && submitted && resolved && battle.turn_number() == 1,
                ));
                let _ = battle.phase() == Phase::CommandSelect;
            }
            Err(_) => checks.push(check("battle starts", false)),
        }

        let mut rng = BattleRng::seeded(5);
        let av = catch_value(100, 10, 255, 10, Status::Sleep);
        checks.push(check("capture succeeds at a=255", {
            av == 255 && attempt_capture(&mut rng, av) == 4
        }));
        checks.push(check("flee when faster", flee_succeeds(10, 5, 0, 255)));

        checks.push(check(
            "hud helpers",
            hp_bar_state(10, 100) == HpBarState::Red
                && hp_bar_pixels(50, 100, 48) == 24
                && hp_bar_blend_color(100, 100)[3] == 255
                && fade_params(fade_trigger_for_phase(Phase::Defeat)).1 == 32,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/rpg-battle-engine",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
