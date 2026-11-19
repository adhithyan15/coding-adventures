//! Seeded model comparison and bounded typed-boundary fuzz campaigns.
//!
//! The reference holds one Boolean per action and counts rises/falls instead
//! of reusing the reducer's `u64` mask update helpers. This gives generated
//! ordered traces a second representation and a separate transition path.

use game_input_frame::{
    reduce_frame, ActionChange, ActionFrame, AxisActionId, ButtonActionId, FrameError, InputState,
    PlayerId, AXIS_SLOT_COUNT, BUTTON_SLOT_COUNT, MAX_CHANGES,
};

const PROPERTY_SEED: u64 = 0x4742_4c30_3550_4254;
const TYPED_FUZZ_SEED: u64 = 0x4742_4c30_3546_555a;
const PROPERTY_CASES_PER_STRATUM: usize = 256;
const STRATA: usize = 8;
const TYPED_FUZZ_CASES: usize = 4096;

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn axis(&mut self) -> i16 {
        (self.next() as i16).clamp(-32767, 32767)
    }
}

#[derive(Clone)]
struct Reference {
    player_id: u32,
    held: [bool; BUTTON_SLOT_COUNT],
    axes: [i16; AXIS_SLOT_COUNT],
}

fn reference_reduce(
    previous: &InputState,
    tick: u64,
    changes: &[ActionChange],
) -> Result<ActionFrame, FrameError> {
    if changes.len() > MAX_CHANGES {
        return Err(FrameError::TooManyChanges);
    }
    for (slot, value) in previous.axes.iter().copied().enumerate() {
        if value == i16::MIN {
            return Err(FrameError::InvalidPreviousAxisValue {
                axis_slot: slot as u8,
                value,
            });
        }
    }

    let mut reference = Reference {
        player_id: previous.player_id.get(),
        held: std::array::from_fn(|slot| (previous.held_buttons & (1_u64 << slot)) != 0),
        axes: previous.axes,
    };
    let mut rise_count = [0_u16; BUTTON_SLOT_COUNT];
    let mut fall_count = [0_u16; BUTTON_SLOT_COUNT];

    for (index, change) in changes.iter().copied().enumerate() {
        match change {
            ActionChange::Button { action_id, down } => {
                let slot = usize::from(action_id.get());
                if slot >= BUTTON_SLOT_COUNT {
                    return Err(FrameError::InvalidButtonActionId {
                        change_index: index as u8,
                        action_id: action_id.get(),
                    });
                }
                let before = reference.held[slot];
                if !before && down {
                    rise_count[slot] += 1;
                } else if before && !down {
                    fall_count[slot] += 1;
                }
                reference.held[slot] = down;
            }
            ActionChange::Axis { action_id, value } => {
                let slot = usize::from(action_id.get());
                if slot >= AXIS_SLOT_COUNT {
                    return Err(FrameError::InvalidAxisActionId {
                        change_index: index as u8,
                        action_id: action_id.get(),
                    });
                }
                if value == i16::MIN {
                    return Err(FrameError::InvalidAxisValue {
                        change_index: index as u8,
                        axis_slot: slot as u8,
                        value,
                    });
                }
                reference.axes[slot] = value;
            }
        }
    }

    let held_buttons = reference
        .held
        .iter()
        .enumerate()
        .filter(|(_, down)| **down)
        .map(|(slot, _)| 1_u64 << slot)
        .sum();
    let pressed_buttons = rise_count
        .iter()
        .enumerate()
        .filter(|(_, count)| **count != 0)
        .map(|(slot, _)| 1_u64 << slot)
        .sum();
    let released_buttons = fall_count
        .iter()
        .enumerate()
        .filter(|(_, count)| **count != 0)
        .map(|(slot, _)| 1_u64 << slot)
        .sum();

    Ok(ActionFrame {
        player_id: PlayerId::new(reference.player_id),
        tick,
        held_buttons,
        pressed_buttons,
        released_buttons,
        axes: reference.axes,
    })
}

fn button(slot: u8, down: bool) -> ActionChange {
    ActionChange::Button {
        action_id: ButtonActionId::new(slot),
        down,
    }
}

fn axis(slot: u8, value: i16) -> ActionChange {
    ActionChange::Axis {
        action_id: AxisActionId::new(slot),
        value,
    }
}

fn valid_state(rng: &mut Rng) -> InputState {
    let player_id = rng.next() as u32;
    let held_buttons = rng.next();
    let axes = std::array::from_fn(|_| rng.axis());
    InputState {
        player_id: PlayerId::new(player_id),
        held_buttons,
        axes,
    }
}

fn valid_changes(rng: &mut Rng, count: usize, stratum: usize) -> Vec<ActionChange> {
    (0..count)
        .map(|index| match stratum {
            0 => button(rng.below(64) as u8, rng.next() & 1 != 0),
            1 => axis(rng.below(16) as u8, rng.axis()),
            2 => {
                let slot = rng.below(64) as u8;
                button(slot, index % 2 == 0)
            }
            3 => {
                if index % 2 == 0 {
                    button(rng.below(64) as u8, rng.next() & 1 != 0)
                } else {
                    axis(rng.below(16) as u8, rng.axis())
                }
            }
            4 => button(if index % 2 == 0 { 0 } else { 63 }, rng.next() & 1 != 0),
            5 => axis(if index % 2 == 0 { 0 } else { 15 }, rng.axis()),
            6 => {
                if index % 3 == 0 {
                    button(63, index % 2 == 0)
                } else {
                    axis(15, rng.axis())
                }
            }
            _ => match rng.below(3) {
                0 => button(rng.below(64) as u8, rng.next() & 1 != 0),
                _ => axis(rng.below(16) as u8, rng.axis()),
            },
        })
        .collect()
}

fn arbitrary_state(rng: &mut Rng) -> InputState {
    InputState {
        player_id: PlayerId::new(rng.next() as u32),
        held_buttons: rng.next(),
        axes: std::array::from_fn(|_| rng.next() as i16),
    }
}

fn arbitrary_change(rng: &mut Rng) -> ActionChange {
    let slot = rng.next() as u8;
    match rng.below(2) {
        0 => button(slot, rng.next() & 1 != 0),
        _ => axis(slot, rng.next() as i16),
    }
}

#[test]
fn supplemental_stratified_model_checks_match_independent_reference() {
    let mut rng = Rng(PROPERTY_SEED);
    let mut seen = [0_usize; STRATA];
    for (stratum, seen_count) in seen.iter_mut().enumerate() {
        for case_index in 0..PROPERTY_CASES_PER_STRATUM {
            let previous = valid_state(&mut rng);
            let count = match stratum {
                0 => 0,
                1..=5 => rng.below(32),
                6 => [255, 256][case_index % 2],
                _ => rng.below(MAX_CHANGES + 1),
            };
            let changes = valid_changes(&mut rng, count, stratum);
            let tick = if case_index % 17 == 0 {
                u64::MAX
            } else {
                rng.next()
            };
            let expected = reference_reduce(&previous, tick, &changes);
            let actual = reduce_frame(&previous, tick, &changes);
            assert_eq!(actual, expected, "stratum={stratum}, case={case_index}");
            if let Ok(whole) = actual {
                // Splitting an ordered sequence preserves final levels; edge
                // summaries from both pieces combine by set union.
                let split = changes.len() / 2;
                let first = reduce_frame(&previous, tick, &changes[..split]).unwrap();
                let middle = InputState {
                    player_id: first.player_id,
                    held_buttons: first.held_buttons,
                    axes: first.axes,
                };
                let second = reduce_frame(&middle, tick, &changes[split..]).unwrap();
                assert_eq!(second.held_buttons, whole.held_buttons);
                assert_eq!(second.axes, whole.axes);
                assert_eq!(
                    first.pressed_buttons | second.pressed_buttons,
                    whole.pressed_buttons
                );
                assert_eq!(
                    first.released_buttons | second.released_buttons,
                    whole.released_buttons
                );
            }
            *seen_count += 1;
        }
    }
    assert_eq!(seen, [PROPERTY_CASES_PER_STRATUM; STRATA]);
    println!(
        "GBL050-MODEL-STRATA seed={PROPERTY_SEED:#018x} cases={} strata={STRATA} each={PROPERTY_CASES_PER_STRATUM} (supplemental custom generator, not VER05 engine)",
        seen.iter().sum::<usize>()
    );
}

#[test]
fn supplemental_arbitrary_typed_smoke_matches_reference_and_keeps_refusals_atomic() {
    let mut rng = Rng(TYPED_FUZZ_SEED);
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    for case_index in 0..TYPED_FUZZ_CASES {
        let previous = arbitrary_state(&mut rng);
        let count = rng.below(MAX_CHANGES + 2);
        let changes: Vec<_> = (0..count).map(|_| arbitrary_change(&mut rng)).collect();
        let prior_before = previous.clone();
        let changes_before = changes.clone();
        let tick = if case_index % 13 == 0 {
            u64::MAX
        } else {
            rng.next()
        };
        let expected = reference_reduce(&previous, tick, &changes);
        let actual = reduce_frame(&previous, tick, &changes);
        assert_eq!(actual, expected, "fuzz case {case_index}");
        assert_eq!(
            previous, prior_before,
            "reducer mutated state at {case_index}"
        );
        assert_eq!(
            changes, changes_before,
            "reducer mutated changes at {case_index}"
        );
        if actual.is_ok() {
            accepted += 1;
        } else {
            refused += 1;
        }
    }
    assert!(
        accepted > 0 && refused > 0,
        "campaign must reach both outcomes"
    );
    println!(
        "GBL050-TYPED-SMOKE seed={TYPED_FUZZ_SEED:#018x} inputs={TYPED_FUZZ_CASES} accepted={accepted} refused={refused} (supplemental loop, not VER06 engine)"
    );
}

#[test]
fn disjoint_action_changes_commute_and_players_remain_isolated() {
    let previous = InputState {
        player_id: PlayerId::new(u32::MAX),
        held_buttons: 0,
        axes: [0; 16],
    };
    let left = [button(63, true), axis(15, -32767)];
    let right = [axis(15, -32767), button(63, true)];
    let frame_left = reduce_frame(&previous, u64::MAX, &left).unwrap();
    let frame_right = reduce_frame(&previous, u64::MAX, &right).unwrap();
    assert_eq!(frame_left, frame_right);
    assert_eq!(frame_left.player_id.get(), u32::MAX);
    assert_eq!(frame_left.tick, u64::MAX);
}

// The repository's VER05/VER06 engines are the campaign drivers below. The
// earlier xorshift strata remain supplemental fixed-seed model comparisons.
#[derive(Clone, Debug)]
struct EngineCase {
    previous: InputState,
    tick: u64,
    changes: Vec<ActionChange>,
}

fn engine_case(rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64) -> EngineCase {
    use coding_adventures_test_bootstrap::pbt::{gen_bool, gen_u64, gen_u8};

    let player = gen_u64(rng) as u32;
    let held = gen_u64(rng);
    let mut axes = [0_i16; AXIS_SLOT_COUNT];
    for axis_value in &mut axes {
        *axis_value = (rng.bounded(65_535) as i32 - 32_767) as i16;
    }
    let previous = InputState {
        player_id: PlayerId::new(player),
        held_buttons: held,
        axes,
    };
    // Bias toward accepted traces while deliberately preserving invalid prior
    // levels, invalid IDs/values, and over-limit batches in the same property.
    let invalid = rng.bounded(10) == 0;
    let count = if invalid && rng.bounded(2) == 0 {
        MAX_CHANGES + 1
    } else {
        rng.bounded((MAX_CHANGES + 1) as u64) as usize
    };
    let mut changes = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = rng.bounded(2);
        let slot = if invalid && rng.bounded(8) == 0 {
            64 + (gen_u8(rng) % 192)
        } else if kind == 0 {
            (rng.bounded(64)) as u8
        } else {
            rng.bounded(16) as u8
        };
        if kind == 0 {
            changes.push(button(slot, gen_bool(rng)));
        } else {
            let value = if invalid && rng.bounded(8) == 0 {
                i16::MIN
            } else {
                (rng.bounded(65_535) as i32 - 32_767) as i16
            };
            changes.push(axis(slot, value));
        }
    }
    EngineCase {
        previous,
        tick: gen_u64(rng),
        changes,
    }
}

fn valid_engine_case(rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64) -> EngineCase {
    use coding_adventures_test_bootstrap::pbt::{gen_bool, gen_u64};
    let previous = InputState {
        player_id: PlayerId::new(gen_u64(rng) as u32),
        held_buttons: gen_u64(rng),
        axes: std::array::from_fn(|_| (rng.bounded(65_535) as i32 - 32_767) as i16),
    };
    let count = rng.bounded((MAX_CHANGES + 1) as u64) as usize;
    let changes = (0..count)
        .map(|_| match rng.bounded(2) {
            0 => button(rng.bounded(64) as u8, gen_bool(rng)),
            _ => axis(
                rng.bounded(16) as u8,
                (rng.bounded(65_535) as i32 - 32_767) as i16,
            ),
        })
        .collect();
    EngineCase {
        previous,
        tick: gen_u64(rng),
        changes,
    }
}

#[test]
fn ver05_property_engine_checks_reference_with_replayable_seed_and_shrinking() {
    use coding_adventures_test_bootstrap::{
        pbt::{check_property, Gen, PbtOptions},
        Verdict,
    };

    const SEED: u64 = 0x4742_4c30_3550_4254;
    let options = PbtOptions {
        num_tests: 4096,
        max_retries: 100,
        max_shrink_steps: 128,
        is_ci: false,
    };
    let case_gen = Gen::new(|rng| Ok(engine_case(rng)));
    let generator =
        |rng: &mut coding_adventures_test_bootstrap::prng::SplitMix64| case_gen.sample(rng);
    let mixed_accepts = std::sync::atomic::AtomicUsize::new(0);
    let mixed_refusals = std::sync::atomic::AtomicUsize::new(0);
    let property = |case: &EngineCase| {
        if reduce_frame(&case.previous, case.tick, &case.changes).is_ok() {
            mixed_accepts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            mixed_refusals.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        reduce_frame(&case.previous, case.tick, &case.changes)
            == reference_reduce(&case.previous, case.tick, &case.changes)
    };
    let shrink = |case: EngineCase, fails: &dyn Fn(&EngineCase) -> bool| {
        let mut smallest = case.clone();
        let mut steps = 0;
        while !smallest.changes.is_empty() {
            let mut candidate = smallest.clone();
            candidate.changes.truncate(candidate.changes.len() / 2);
            if !fails(&candidate) {
                smallest = candidate;
                steps += 1;
            } else {
                break;
            }
        }
        (smallest, steps)
    };
    assert_eq!(
        check_property(SEED, &options, generator, property, shrink),
        Verdict::Pass
    );
    let valid_accepts = std::sync::atomic::AtomicUsize::new(0);
    let long_accepts = std::sync::atomic::AtomicUsize::new(0);
    let valid_gen = Gen::new(|rng| Ok(valid_engine_case(rng)));
    let valid_property = |case: &EngineCase| {
        let actual = reduce_frame(&case.previous, case.tick, &case.changes);
        if let Ok(frame) = &actual {
            valid_accepts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if case.changes.len() >= 64 {
                long_accepts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            frame.axes.iter().all(|value| *value != i16::MIN)
                && frame.player_id == case.previous.player_id
                && frame.tick == case.tick
        } else {
            false
        }
    };
    assert_eq!(
        check_property(
            SEED ^ 0xa5a5_5a5a,
            &options,
            |rng| valid_gen.sample(rng),
            valid_property,
            shrink,
        ),
        Verdict::Pass
    );
    assert_eq!(
        valid_accepts.load(std::sync::atomic::Ordering::Relaxed),
        options.num_tests
    );
    assert!(long_accepts.load(std::sync::atomic::Ordering::Relaxed) > 1000);
    // Prove the framework invokes the registered shrinker when a property
    // fails, without treating a fabricated failure as implementation evidence.
    let shrink_probe = EngineCase {
        previous: InputState {
            player_id: PlayerId::new(1),
            held_buttons: 0,
            axes: [0; AXIS_SLOT_COUNT],
        },
        tick: 0,
        changes: vec![button(0, true), button(0, false)],
    };
    let probe_gen = Gen::new(move |_| Ok(shrink_probe.clone()));
    let probe_failure = check_property(
        SEED,
        &PbtOptions {
            num_tests: 1,
            ..options.clone()
        },
        |rng| probe_gen.sample(rng),
        |_| false,
        shrink,
    );
    assert!(
        matches!(&probe_failure, Verdict::Fail { message, .. }
            if message.contains("after 2 shrink steps") && message.contains("changes: []")),
        "framework shrink negative control should minimize the trace: {probe_failure:?}"
    );
    println!(
        "VER05 game-input-frame mixed_seed={SEED:#018x} mixed_cases={} accepted={} refused={} valid_seed={:#018x} valid_cases={} accepted={} long_accepted_64plus={} shrink_negative_control=2_steps",
        options.num_tests,
        mixed_accepts.load(std::sync::atomic::Ordering::Relaxed),
        mixed_refusals.load(std::sync::atomic::Ordering::Relaxed),
        SEED ^ 0xa5a5_5a5a,
        options.num_tests,
        valid_accepts.load(std::sync::atomic::Ordering::Relaxed),
        long_accepts.load(std::sync::atomic::Ordering::Relaxed),
    );
}

// This deliberately small binary framing exists only to exercise the typed
// reducer boundary under VER06 mutations; it is not a public replay/wire ABI.
// The engine's bitmap records decoder/reducer semantic buckets, not compiler
// branch coverage. The test-bootstrap API has no automatic Rust branch
// instrumentation hook, so this is explicitly weaker campaign feedback.
fn encode_engine_case(case: &EngineCase) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(54 + case.changes.len() * 4);
    bytes.extend_from_slice(&case.previous.player_id.get().to_le_bytes());
    bytes.extend_from_slice(&case.tick.to_le_bytes());
    bytes.extend_from_slice(&case.previous.held_buttons.to_le_bytes());
    for axis_value in case.previous.axes {
        bytes.extend_from_slice(&axis_value.to_le_bytes());
    }
    bytes.extend_from_slice(&(case.changes.len() as u16).to_le_bytes());
    for change in &case.changes {
        match *change {
            ActionChange::Button { action_id, down } => {
                bytes.push(0);
                bytes.push(action_id.get());
                bytes.push(u8::from(down));
                bytes.push(0);
            }
            ActionChange::Axis { action_id, value } => {
                bytes.push(1);
                bytes.push(action_id.get());
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
    }
    bytes
}

fn decode_engine_case(bytes: &[u8]) -> Result<EngineCase, ()> {
    const HEADER: usize = 54;
    if bytes.len() < HEADER {
        return Err(());
    }
    let player = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| ())?);
    let tick = u64::from_le_bytes(bytes[4..12].try_into().map_err(|_| ())?);
    let held = u64::from_le_bytes(bytes[12..20].try_into().map_err(|_| ())?);
    let mut axes = [0_i16; AXIS_SLOT_COUNT];
    for (slot, value) in axes.iter_mut().enumerate() {
        let offset = 20 + slot * 2;
        *value = i16::from_le_bytes(bytes[offset..offset + 2].try_into().map_err(|_| ())?);
    }
    let count = u16::from_le_bytes(bytes[52..54].try_into().map_err(|_| ())?) as usize;
    if count > MAX_CHANGES || bytes.len() != HEADER + count * 4 {
        return Err(());
    }
    let mut changes = Vec::with_capacity(count);
    for index in 0..count {
        let record = &bytes[HEADER + index * 4..HEADER + (index + 1) * 4];
        changes.push(match record[0] {
            0 if record[2] <= 1 && record[3] == 0 => button(record[1], record[2] == 1),
            1 => axis(record[1], i16::from_le_bytes([record[2], record[3]])),
            _ => return Err(()),
        });
    }
    Ok(EngineCase {
        previous: InputState {
            player_id: PlayerId::new(player),
            held_buttons: held,
            axes,
        },
        tick,
        changes,
    })
}

#[test]
fn ver06_bounded_campaign_mutates_valid_biased_typed_inputs() {
    use coding_adventures_test_bootstrap::fuzz::{fuzz_target, FuzzOptions, FuzzOutcome};
    use coding_adventures_test_bootstrap::prng::SplitMix64;
    use std::cell::Cell;

    const SEED: u64 = 0x4742_4c30_3546_555a;
    let mut rng = SplitMix64::new(SEED);
    let mut corpus = vec![
        encode_engine_case(&EngineCase {
            previous: InputState {
                player_id: PlayerId::new(u32::MAX),
                held_buttons: 0,
                axes: [0; AXIS_SLOT_COUNT],
            },
            tick: u64::MAX,
            changes: vec![button(63, true), button(63, false)],
        }),
        vec![0; 4], // short-header refusal
    ];
    // Valid-biased seeds include long traces; malformed seeds exercise frame
    // framing and oversized-count refusal before mutation begins.
    for count in [1, 8, 64, 255, 256] {
        let mut case = engine_case(&mut rng);
        case.changes.truncate(count);
        while case.changes.len() < count {
            case.changes
                .push(button((case.changes.len() % 64) as u8, true));
        }
        corpus.push(encode_engine_case(&case));
    }
    let mut over = engine_case(&mut rng);
    over.changes.truncate(257);
    while over.changes.len() < 257 {
        over.changes.push(button(0, true));
    }
    corpus.push(encode_engine_case(&over));

    let accepted = Cell::new(0usize);
    let refused = Cell::new(0usize);
    let invocations = Cell::new(0usize);
    let buckets = Cell::new([0usize; 5]);
    let options = FuzzOptions {
        max_iterations: 4096,
        seed: SEED,
        max_input_len: 2048,
        base_energy: 4,
    };
    let report = fuzz_target(&corpus, &options, |input, coverage| {
        invocations.set(invocations.get() + 1);
        let decoded = decode_engine_case(input);
        let bucket = match &decoded {
            Err(()) if input.len() < 54 => 0,
            Err(()) => 1,
            Ok(case) if case.changes.len() > MAX_CHANGES => 2,
            Ok(case) => match reduce_frame(&case.previous, case.tick, &case.changes) {
                Ok(actual) => {
                    assert_eq!(
                        Ok(actual),
                        reference_reduce(&case.previous, case.tick, &case.changes)
                    );
                    accepted.set(accepted.get() + 1);
                    if case.changes.len() >= 64 {
                        3
                    } else {
                        2
                    }
                }
                Err(error) => {
                    assert_eq!(
                        Err(error),
                        reference_reduce(&case.previous, case.tick, &case.changes)
                    );
                    refused.set(refused.get() + 1);
                    4
                }
            },
        };
        let mut seen = buckets.get();
        seen[bucket] += 1;
        buckets.set(seen);
        // Semantic categories guide the corpus. These IDs describe the
        // explicit target buckets above and are not native branch counters.
        coverage.record_edge(0x500 + bucket);
        FuzzOutcome::Pass
    });
    assert!(report.is_clean(), "VER06 report: {report:?}");
    assert_eq!(report.iterations, options.max_iterations);
    assert!(accepted.get() > 0, "valid reducer traces were reached");
    assert!(refused.get() > 0, "reducer refusal paths were reached");
    assert!(buckets.get()[0] > 0, "decoder short-input refusal reached");
    assert!(buckets.get()[3] > 0, "long accepted trace reached");
    println!(
        "VER06 game-input-frame seed={SEED:#018x} scheduled_iterations={} actual_invocations={} corpus_seeds={} admitted={} semantic_buckets={:?} valid_invocations={} reducer_refusals={} edges={} (semantic buckets; not native branch coverage)",
        report.iterations,
        invocations.get(),
        corpus.len(),
        report.admitted_seeds,
        buckets.get(),
        accepted.get(),
        refused.get(),
        report.total_edges,
    );
}

#[test]
fn ver06_bounded_campaign_mutates_the_canonical_json_parser_boundary() {
    use coding_adventures_bounded_json::parse;
    use coding_adventures_test_bootstrap::fuzz::{fuzz_target, FuzzOptions, FuzzOutcome};
    use std::cell::Cell;

    const SEED: u64 = 0x4742_4c30_354a_534e;
    const CANONICAL: &[u8] =
        include_bytes!("../../../../specs/fixtures/game-input-frame-v1/cases.json");
    let mut corpus = vec![CANONICAL.to_vec()];
    corpus.push(b"{".to_vec());
    corpus.push(b"{\"version\":true}".to_vec());
    corpus.push(b"{\"version\":1,\"version\":1}".to_vec());
    let accepted = Cell::new(0usize);
    let refused = Cell::new(0usize);
    let invocations = Cell::new(0usize);
    let options = FuzzOptions {
        max_iterations: 1024,
        seed: SEED,
        max_input_len: 32 * 1024,
        base_energy: 1,
    };
    let report = fuzz_target(&corpus, &options, |input, coverage| {
        invocations.set(invocations.get() + 1);
        let bucket = match std::str::from_utf8(input) {
            Err(_) => 0,
            Ok(text) => match parse(text) {
                Ok(_) => {
                    accepted.set(accepted.get() + 1);
                    1
                }
                Err(_) => {
                    refused.set(refused.get() + 1);
                    2
                }
            },
        };
        // These are target outcomes from the real bounded-json parser, not
        // Rust compiler branch instrumentation.
        coverage.record_edge(0x600 + bucket);
        FuzzOutcome::Pass
    });
    assert!(report.is_clean(), "VER06 JSON report: {report:?}");
    assert_eq!(report.iterations, options.max_iterations);
    assert!(accepted.get() > 0, "canonical JSON seed must decode");
    assert!(
        refused.get() > 0,
        "malformed JSON seeds/mutations must refuse"
    );
    println!(
        "VER06 bounded-json seed={SEED:#018x} scheduled_iterations={} actual_invocations={} seeds={} admitted={} parser_accepted={} parser_refused={} semantic_edges={} (not native branch coverage)",
        report.iterations,
        invocations.get(),
        corpus.len(),
        report.admitted_seeds,
        accepted.get(),
        refused.get(),
        report.total_edges,
    );
}
