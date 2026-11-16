//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `rpg-battle-engine`
//!
//! Evaluates `rpg-battle-engine` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates, on the real heap via test-bootstrap's counting global allocator:
//! - zero allocation in `calc_damage`, type chart lookups and stat helpers,
//! - zero allocation in a full battle turn (`submit_turn` + `resolve_turn`),
//! - zero allocation in capture, flee and HUD helpers,
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
use rpg_battle_engine::{
    attempt_capture, calc_damage, catch_value, dual_multiplier, fade_params,
    fade_trigger_for_phase, flee_succeeds, hp_bar_blend_color, hp_bar_pixels, hp_bar_state,
    BaseStats, Battle, BattleRng, Category, Command, Creature, CreatureSpec, Effect, Move, Phase,
    Status, FIRE, GRASS, NORMAL, NO_TYPE, WATER,
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

const HIT: Move = Move {
    move_type: FIRE,
    category: Category::Special,
    power: 60,
    accuracy: 95,
    max_pp: 200,
    priority: 0,
    effect: Effect::Inflict {
        status: Status::Burn,
        chance: 20,
    },
};

fn creature(t1: u8, level: u8, v: u8) -> Creature {
    let moves = [HIT, HIT];
    Creature::from_spec(&CreatureSpec {
        type1: t1,
        type2: NO_TYPE,
        level,
        base: BaseStats {
            hp: v,
            atk: v,
            def: v,
            spa: v,
            spd: v,
            spe: v,
        },
        moves: &moves,
        catch_rate: 45,
        exp_yield: 64,
    })
    .expect("creature")
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_damage_zero_alloc(_ctx: &Ctx) -> Verdict {
    let a = creature(FIRE, 50, 100);
    let d = creature(GRASS, 50, 80);
    let v = bench_zero_alloc("calc_damage_zero_alloc", || {
        let r = calc_damage(
            black_box(&a),
            black_box(&d),
            black_box(&HIT),
            black_box(93),
            false,
        );
        black_box(r);
        black_box(dual_multiplier(black_box(FIRE), black_box(GRASS), black_box(WATER)).ok());
    });
    if !matches!(v, Verdict::Pass) {
        return v;
    }
    let mut c = creature(NORMAL, 5, 60);
    bench_zero_alloc("grant_xp_zero_alloc", || {
        black_box(c.grant_xp(black_box(50)));
        if c.level >= 100 {
            c = creature(NORMAL, 5, 60);
        }
    })
}

fn check_turn_zero_alloc(_ctx: &Ctx) -> Verdict {
    let a = creature(FIRE, 30, 100);
    let b = creature(WATER, 30, 100);
    bench_zero_alloc("battle_turn_zero_alloc", || {
        let mut battle = Battle::start(&[a, a], &[b, b], false, black_box(7)).expect("battle");
        battle.begin().expect("begin");
        let mut turns = 0;
        while battle.phase() == Phase::CommandSelect && turns < 8 {
            let enemy = battle.ai_choose();
            if battle
                .submit_turn(black_box(Command::Fight { slot: 0 }), enemy)
                .is_ok()
            {
                battle.resolve_turn().expect("resolve");
            }
            turns += 1;
        }
        black_box(battle.event_log().len());
    })
}

fn check_capture_flee_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut rng = BattleRng::seeded(9);
    let v = bench_zero_alloc("capture_zero_alloc", || {
        let a = catch_value(black_box(100), black_box(30), 45, 15, Status::Sleep);
        black_box(attempt_capture(&mut rng, a));
        black_box(flee_succeeds(
            black_box(40),
            black_box(90),
            2,
            rng.below(256),
        ));
    });
    if !matches!(v, Verdict::Pass) {
        return v;
    }
    let a = creature(FIRE, 20, 70);
    let mut wild = creature(WATER, 5, 50);
    wild.catch_rate = 255;
    bench_zero_alloc("battle_catch_zero_alloc", || {
        let mut battle = Battle::start(&[a], &[wild], true, black_box(3)).expect("battle");
        battle.begin().expect("begin");
        battle
            .submit_turn(
                Command::Catch { bonus_x10: 1000 },
                Command::Fight { slot: 0 },
            )
            .expect("submit");
        battle.resolve_turn().expect("resolve");
        black_box(battle.was_captured());
    })
}

fn check_hud_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("hud_zero_alloc", || {
        let hp = black_box(37u16);
        black_box(hp_bar_state(hp, 120));
        black_box(hp_bar_pixels(hp, 120, 48));
        black_box(hp_bar_blend_color(hp, 120));
        black_box(fade_params(fade_trigger_for_phase(black_box(
            Phase::Victory,
        ))));
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
        ("module", Json::str("rpg_battle_engine")),
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
    suite.add("bench/damage/zero_alloc", check_damage_zero_alloc);
    suite.add("bench/turn/zero_alloc", check_turn_zero_alloc);
    suite.add(
        "bench/capture_flee/zero_alloc",
        check_capture_flee_zero_alloc,
    );
    suite.add("bench/hud/zero_alloc", check_hud_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
