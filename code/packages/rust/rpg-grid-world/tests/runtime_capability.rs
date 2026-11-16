//! # VER08b runtime capability conformance for `rpg-grid-world`.
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
use rpg_grid_world::{
    count_dialogue_cells, face_npc_toward, find_npc_at, interact_target, lower_dialogue_cells_into,
    lower_dialogue_to_paint_scene, npc_blocks_tile, npc_sees_actor, resolve_warp,
    validate_encounter_table, validate_warp, DialogueBox, DialogueInput, DialoguePage,
    DialogueStyle, EncounterEntry, EncounterState, EncounterTable, Facing, FrameTiles, GridActor,
    GridRng, GridWorld, Npc, NpcKind, TileBatchCell, TileCell, TileLayer, WarpPad, WarpTrigger,
    ATTR_SOLID,
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

        let cells = [
            TileCell::tile(1),
            TileCell::tile(1),
            TileCell::tile(7),
            TileCell::tile(1).with_attr(ATTR_SOLID),
        ];
        let layer = TileLayer::attach(&cells, 2, 2, 8, 8);
        checks.push(check("TileLayer::attach succeeds", layer.is_ok()));
        let Ok(layer) = layer else {
            return checks;
        };

        let entries = [EncounterEntry {
            species: 1,
            min_level: 2,
            max_level: 4,
            weight: 3,
        }];
        let table = EncounterTable {
            tile_id: 7,
            min_steps: 1,
            max_steps: 2,
            rate_q8: 128,
            entries: &entries,
        };
        checks.push(check(
            "validate_encounter_table succeeds",
            validate_encounter_table(0, &table).is_ok(),
        ));
        let pad = WarpPad {
            x: 1,
            y: 0,
            trigger: WarpTrigger::OnEnter,
            target_map: 2,
            target_x: 0,
            target_y: 0,
            target_facing: Facing::Up,
        };
        checks.push(check(
            "validate_warp succeeds",
            validate_warp(&layer, &pad).is_ok(),
        ));
        let mut npcs = [Npc {
            id: 1,
            x: 1,
            y: 1,
            facing: Facing::Left,
            kind: NpcKind::Trainer,
            script: 0,
            sight_range: 2,
            blocking: true,
        }];
        checks.push(check(
            "npc queries succeed",
            find_npc_at(&npcs, 1, 1) == Some(0)
                && npc_blocks_tile(&npcs, 1, 1)
                && npc_sees_actor(&layer, &npcs[0], 0, 1)
                && interact_target(&layer, &npcs, 0, 1, Facing::Right, 1) == Ok(Some(0)),
        ));
        checks.push(check(
            "face_npc_toward turns the trainer",
            face_npc_toward(&mut npcs[0], Facing::Left),
        ));
        let warps = [pad];
        checks.push(check(
            "resolve_warp finds the pad",
            resolve_warp(&warps, 1, 0, WarpTrigger::OnEnter).is_some(),
        ));

        let tables = [table];
        let world = GridWorld::bind(&layer, &warps, &npcs, &tables);
        checks.push(check("GridWorld::bind succeeds", world.is_ok()));
        if let Ok(world) = world {
            let actor = GridActor::place(&layer, 0, 0, Facing::Right);
            checks.push(check("GridActor::place succeeds", actor.is_ok()));
            if let Ok(mut actor) = actor {
                let mut enc = EncounterState::start(7);
                actor.set_speed(8).ok();
                let started = actor
                    .advance(&world, Some(Facing::Right), &mut enc)
                    .map(|r| r.started);
                checks.push(check("advance starts a walk", started == Ok(true)));
                let arrived = actor.advance(&world, None, &mut enc).map(|r| r.arrived);
                checks.push(check("advance completes the walk", arrived == Ok(true)));
                actor.freeze_actor();
                actor.thaw_actor();
                checks.push(check(
                    "pixel_position and hop_arc succeed",
                    actor.pixel_position(&layer) == (8, 0) && actor.hop_arc(4) == 0,
                ));
            }
        }
        let mut rng = GridRng::seeded(3);
        checks.push(check(
            "GridRng rolls below its bound",
            rng.roll_below(5) < 5,
        ));

        let lines = ["Hi"];
        let pages = [DialoguePage {
            lines: &lines,
            choices: &[],
            choice_targets: &[],
        }];
        let dialogue = DialogueBox::open(&pages);
        checks.push(check("DialogueBox::open succeeds", dialogue.is_ok()));
        if let Ok(mut dialogue) = dialogue {
            checks.push(check(
                "tick_dialogue succeeds",
                dialogue.tick_dialogue(DialogueInput::Confirm).is_ok(),
            ));
            let style = DialogueStyle {
                col: 0,
                row: 0,
                inner_cols: 4,
                inner_rows: 2,
                glyph_base: 10,
                cursor_tile: 9,
                frame: FrameTiles {
                    tl: 1,
                    t: 2,
                    tr: 3,
                    l: 4,
                    fill: 5,
                    r: 6,
                    bl: 7,
                    b: 8,
                    br: 9,
                },
                tint: [255, 255, 255],
            };
            let n = count_dialogue_cells(&style);
            checks.push(check("count_dialogue_cells succeeds", n == Ok(24)));
            let mut out = [TileBatchCell::new(0, 0, 0); 24];
            checks.push(check(
                "lower_dialogue_cells_into succeeds",
                lower_dialogue_cells_into(&dialogue, &style, &mut out) == Ok(24),
            ));
            checks.push(check(
                "lower_dialogue_to_paint_scene succeeds",
                lower_dialogue_to_paint_scene(&dialogue, &style, 8, 8, 64, 64, "#000000").is_ok(),
            ));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/rpg-grid-world",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
