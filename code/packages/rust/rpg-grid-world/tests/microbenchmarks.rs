//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `rpg-grid-world`
//!
//! Evaluates `rpg-grid-world` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates, on the real heap via test-bootstrap's counting global allocator:
//! - zero allocation in the movement FSM (walk, turn, hop, surf, freeze),
//! - zero allocation in step encounter accumulation and table rolls,
//! - zero allocation in warp, NPC, interaction and sight queries,
//! - zero allocation in dialogue ticks and dialogue cell lowering,
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use rpg_grid_world::{
    count_dialogue_cells, find_npc_at, interact_target, lower_dialogue_cells_into, npc_blocks_tile,
    npc_sees_actor, resolve_warp, DialogueBox, DialogueInput, DialoguePage, DialogueStyle,
    EncounterEntry, EncounterState, EncounterTable, Facing, FrameTiles, GridActor, GridRng,
    GridWorld, Npc, NpcKind, TileBatchCell, TileCell, TileLayer, WarpPad, WarpTrigger, ATTR_HAZARD,
    ATTR_ONE_WAY, ATTR_SOLID, ATTR_WATER,
};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 10,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 100_000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, f);
    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!("{name} exceeded zero allocation budget"),
        );
    }
    Verdict::Pass
}

/// A 16 x 16 map mixing open ground, walls, ledges, water, hazards and grass.
fn fixture_cells() -> Vec<TileCell> {
    (0..16u32 * 16)
        .map(|i| {
            let (c, r) = (i % 16, i / 16);
            match (c * 5 + r * 3) % 13 {
                0 => TileCell::tile(1).with_attr(ATTR_SOLID),
                1 => TileCell::tile(2).with_attr(ATTR_ONE_WAY),
                2 => TileCell::tile(3).with_attr(ATTR_WATER),
                3 => TileCell::tile(7),
                4 => TileCell::tile(1).with_attr(ATTR_HAZARD),
                _ => TileCell::tile(1),
            }
        })
        .collect()
}

fn fixture_entries() -> [EncounterEntry; 2] {
    [
        EncounterEntry {
            species: 1,
            min_level: 2,
            max_level: 6,
            weight: 5,
        },
        EncounterEntry {
            species: 2,
            min_level: 5,
            max_level: 9,
            weight: 2,
        },
    ]
}

fn fixture_npcs() -> [Npc; 2] {
    [
        Npc {
            id: 1,
            x: 4,
            y: 4,
            facing: Facing::Right,
            kind: NpcKind::Trainer,
            script: 3,
            sight_range: 6,
            blocking: true,
        },
        Npc {
            id: 2,
            x: 9,
            y: 2,
            facing: Facing::Down,
            kind: NpcKind::Sign,
            script: 4,
            sight_range: 0,
            blocking: true,
        },
    ]
}

fn fixture_warps() -> [WarpPad; 2] {
    [
        WarpPad {
            x: 8,
            y: 8,
            trigger: WarpTrigger::OnEnter,
            target_map: 2,
            target_x: 1,
            target_y: 1,
            target_facing: Facing::Up,
        },
        WarpPad {
            x: 0,
            y: 15,
            trigger: WarpTrigger::OnBump(Facing::Down),
            target_map: 3,
            target_x: 5,
            target_y: 5,
            target_facing: Facing::Left,
        },
    ]
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_movement_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let layer = TileLayer::attach(&cells, 16, 16, 16, 16).unwrap();
    let entries = fixture_entries();
    let tables = [EncounterTable {
        tile_id: 7,
        min_steps: 2,
        max_steps: 9,
        rate_q8: 64,
        entries: &entries,
    }];
    let npcs = fixture_npcs();
    let warps = fixture_warps();
    let world = GridWorld::bind(&layer, &warps, &npcs, &tables).expect("world");
    let mut actor = GridActor::place(&layer, 7, 7, Facing::Down).expect("actor");
    actor.can_surf = true;
    actor.set_speed(5).expect("speed");
    actor.set_turn_delay(2).expect("turn");
    let mut enc = EncounterState::start(0xC0FFEE);
    let mut rng = GridRng::seeded(11);
    bench_zero_alloc("movement_zero_alloc", || {
        let input = match rng.roll_below(black_box(6)) {
            0 => Some(Facing::Up),
            1 => Some(Facing::Down),
            2 => Some(Facing::Left),
            3 => Some(Facing::Right),
            _ => None,
        };
        let r = actor.advance(&world, input, &mut enc).expect("advance");
        black_box((r, actor.pixel_position(&layer), actor.hop_arc(8)));
    })
}

fn check_encounter_zero_alloc(_ctx: &Ctx) -> Verdict {
    let entries = fixture_entries();
    let table = EncounterTable {
        tile_id: 7,
        min_steps: 1,
        max_steps: 4,
        rate_q8: 100,
        entries: &entries,
    };
    let mut enc = EncounterState::start(5);
    bench_zero_alloc("encounter_zero_alloc", || {
        black_box(enc.register_step(black_box(&table)));
    })
}

fn check_queries_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let layer = TileLayer::attach(&cells, 16, 16, 16, 16).unwrap();
    let npcs = fixture_npcs();
    let warps = fixture_warps();
    bench_zero_alloc("queries_zero_alloc", || {
        let x = black_box(3);
        let w = resolve_warp(&warps, black_box(8), black_box(8), WarpTrigger::OnEnter);
        let n = find_npc_at(&npcs, black_box(9), black_box(2));
        let b = npc_blocks_tile(&npcs, i64::from(x), 4);
        let t = interact_target(&layer, &npcs, x, black_box(4), Facing::Right, 2).expect("reach");
        let s = npc_sees_actor(&layer, &npcs[0], black_box(8), 4);
        black_box((w, n, b, t, s));
    })
}

fn check_dialogue_zero_alloc(_ctx: &Ctx) -> Verdict {
    let q = ["Do you want", "to continue?"];
    let choices = ["Yes", "No"];
    let end = ["Goodbye!"];
    let pages = [
        DialoguePage {
            lines: &q,
            choices: &choices,
            choice_targets: &[1, 2],
        },
        DialoguePage {
            lines: &end,
            choices: &[],
            choice_targets: &[],
        },
        DialoguePage {
            lines: &end,
            choices: &[],
            choice_targets: &[],
        },
    ];
    let style = DialogueStyle {
        col: 1,
        row: 10,
        inner_cols: 18,
        inner_rows: 4,
        glyph_base: 64,
        cursor_tile: 63,
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
    let n = count_dialogue_cells(&style).expect("count");
    let mut out = vec![TileBatchCell::new(0, 0, 0); n];
    let mut dlg = DialogueBox::open(&pages).expect("open");
    let inputs = [
        DialogueInput::Idle,
        DialogueInput::Idle,
        DialogueInput::Down,
        DialogueInput::Confirm,
        DialogueInput::Idle,
        DialogueInput::Confirm,
        DialogueInput::Up,
    ];
    let mut i = 0usize;
    bench_zero_alloc("dialogue_zero_alloc", || {
        i = (i + 1) % inputs.len();
        let ev = dlg.tick_dialogue(black_box(inputs[i])).expect("tick");
        if dlg.dialogue_state() == rpg_grid_world::DialogueState::Finished {
            dlg = DialogueBox::open(&pages).expect("reopen");
        }
        let written = lower_dialogue_cells_into(&dlg, &style, &mut out).expect("lower");
        black_box((ev, written, dlg.visible_line(0)));
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    if !matches!(
        leaky.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a closure allocating one byte did not fail the zero-allocation budget",
        );
    }
    let clean = harness.run_with_counter("clean_alloc", &budget, &counter, || {
        black_box(1u64) ^ black_box(2u64)
    });
    if !zero_alloc_ok(&clean) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a non-allocating closure did not pass the zero-allocation budget",
        );
    }
    Verdict::Pass
}

fn write_bench_manifest(outcomes: &[Outcome]) {
    let Some(path) = manifest::output_path("WARRANT_BENCH_OUT") else {
        return;
    };
    let mut tests: Vec<Json> = outcomes
        .iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id.clone())),
                ("kind", Json::str("bench")),
                (
                    "verdict",
                    Json::str(if o.passed() { "pass" } else { "fail" }),
                ),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("rpg-grid-world")),
        ("hw_instruction_counters", Json::str("not-live")),
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
                )
                .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add("bench/movement/zero_alloc", check_movement_zero_alloc);
    suite.add("bench/encounter/zero_alloc", check_encounter_zero_alloc);
    suite.add("bench/queries/zero_alloc", check_queries_zero_alloc);
    suite.add("bench/dialogue/zero_alloc", check_dialogue_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
