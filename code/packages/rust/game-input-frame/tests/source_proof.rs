//! Bit-level proofs over the exact scalar helpers called by the reducer.
//!
//! `SLOT_BITS` is six because the helper's production precondition is the
//! reducer's validated `0..=63` slot. This intentionally does not claim that
//! Rust's checked shift has a meaningful production result for every `u8`.

use bit_blaster::bv::{self, BitVec};
use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;
use game_input_frame::{
    reduce_frame, ActionChange, ActionFrame, ButtonActionId, InputState, PlayerId,
};

const DIGITAL_SOURCE: &str = include_str!("../src/digital.rs");
const DIGITAL_SOURCE_PATH: &str = "code/packages/rust/game-input-frame/src/digital.rs";
const DIGITAL_SOURCE_SHA256: &str =
    "be1a4057993b31b35fabcda0f6a2873749afc331834c1b2180c5913542511ad9";
const SLOT_BITS: usize = 6;

fn production_function(source: &str, name: &str) -> Program {
    let parsed = parse_rust_file(source)
        .unwrap_or_else(|error| panic!("parse production source {DIGITAL_SOURCE_PATH}: {error:?}"));
    let program = lower_source_file(&parsed, &[(name, &["mask", "slot"])])
        .unwrap_or_else(|error| panic!("lower {name} from {DIGITAL_SOURCE_PATH}: {error:?}"));
    program
}

fn function<'a>(program: &'a Program, name: &str) -> &'a Function {
    program
        .find_function(name)
        .unwrap_or_else(|| panic!("lowered {name} missing"))
}

fn assert_mask_slot_signature(function: &Function, name: &str, result: WirType) {
    assert_eq!(function.name, name, "lowered helper name");
    assert_eq!(
        function
            .params
            .iter()
            .map(|(var, _, _)| var.0.as_str())
            .collect::<Vec<_>>(),
        ["mask", "slot"],
        "the extracted parameter names must match the production signature"
    );
    assert_eq!(
        function
            .params
            .iter()
            .map(|(_, ty, _)| ty.clone())
            .collect::<Vec<_>>(),
        [WirType::U64, WirType::U8],
        "the extracted arguments must match mask:u64, slot:u8"
    );
    assert_eq!(function.return_type, result);
}

fn lowered_slot_argument(slot: &BitVec) -> BitVec {
    assert_eq!(slot.width(), SLOT_BITS);
    let widened = coding_adventures_warrant_ir::sym::zero_extend(slot, 8);
    assert_eq!(
        widened.width(),
        8,
        "six free slot bits are zero-extended to u8"
    );
    widened
}

fn expected_slot_from_argument(slot: &BitVec) -> BitVec {
    assert_eq!(slot.width(), 8);
    let low = coding_adventures_warrant_ir::sym::truncate(slot, SLOT_BITS);
    assert_eq!(low.width(), SLOT_BITS);
    low
}

/// Independent 64-case truth-table circuit: for each slot, select exactly the
/// corresponding input bit. The construction never shifts a word by `slot`.
fn bit_predicate(
    aig: &mut bit_blaster::aig::Aig,
    mask: &BitVec,
    slot: &BitVec,
) -> bit_blaster::aig::Edge {
    let mut terms = Vec::with_capacity(64);
    for bit in 0_usize..64 {
        let selected = bv::eq(aig, slot, &BitVec::constant(SLOT_BITS, bit as u128));
        terms.push(aig.and(selected, mask.bits[bit]));
    }
    aig.or_all(&terms)
}

/// Independent truth-table output: each output bit is either the unchanged
/// input bit or the requested constant, selected by equality with that slot.
fn updated_mask(
    aig: &mut bit_blaster::aig::Aig,
    mask: &BitVec,
    slot: &BitVec,
    set: bool,
) -> BitVec {
    let bits = (0_usize..64)
        .map(|bit| {
            let selected = bv::eq(aig, slot, &BitVec::constant(SLOT_BITS, bit as u128));
            aig.mux(
                selected,
                bit_blaster::aig::Edge::constant(set),
                mask.bits[bit],
            )
        })
        .collect();
    BitVec { bits }
}

fn prove_mask_predicate(source: &str, name: &str) -> VerificationVerdict {
    let program = production_function(source, name);
    let mut prover = Prover::new();
    let mask = prover.executor.create_input_bits("mask", 64);
    // Six input bits represent exactly the validated action-slot domain.
    let slot = prover.executor.create_input_bits("slot", SLOT_BITS);
    let f = function(&program, name);
    assert_mask_slot_signature(f, name, WirType::Bool);
    let slot_argument = lowered_slot_argument(&slot);
    prover.prove_equivalence(
        f,
        vec![SymVal::Bits(mask), SymVal::Bits(slot_argument)],
        SymMemory::new(),
        |aig, args, _| {
            let mask = args[0].to_bits()?;
            let slot = expected_slot_from_argument(&args[1].to_bits()?);
            Ok(SymVal::Bool(bit_predicate(aig, &mask, &slot)))
        },
    )
}

fn prove_mask_update(source: &str, name: &str, set: bool) -> VerificationVerdict {
    let program = production_function(source, name);
    let mut prover = Prover::new();
    let mask = prover.executor.create_input_bits("mask", 64);
    let slot = prover.executor.create_input_bits("slot", SLOT_BITS);
    let f = function(&program, name);
    assert_mask_slot_signature(f, name, WirType::U64);
    let slot_argument = lowered_slot_argument(&slot);
    prover.prove_equivalence(
        f,
        vec![SymVal::Bits(mask), SymVal::Bits(slot_argument)],
        SymMemory::new(),
        |aig, args, _| {
            let mask = args[0].to_bits()?;
            let slot = expected_slot_from_argument(&args[1].to_bits()?);
            Ok(SymVal::Bits(updated_mask(aig, &mask, &slot, set)))
        },
    )
}

fn counterexample_value(values: &[(String, u128)], name: &str) -> u64 {
    u64::try_from(
        values
            .iter()
            .find_map(|(key, value)| (key == name).then_some(*value))
            .unwrap_or_else(|| panic!("counterexample omitted {name}: {values:?}")),
    )
    .expect("counterexample fits a u64")
}

/// Replay one mask/slot through the real reducer and through the reducer's
/// exact branch semantics with the inverted `bit_is_set` result. Returning all
/// three frames lets each polarity check cover the complete transition.
fn replay_inverted_bit_test(mask: u64, slot: u64) -> (ActionFrame, ActionFrame, ActionFrame) {
    assert!(slot < 64);
    let bit = 1_u64 << slot;
    let initial = InputState {
        player_id: PlayerId::new(4),
        held_buttons: mask,
        axes: [0; 16],
    };
    let actual_frame = reduce_frame(
        &initial,
        17,
        &[ActionChange::Button {
            action_id: ButtonActionId::new(slot as u8),
            down: true,
        }],
    )
    .expect("decoded mask/slot is a valid reduction");

    let selected_is_held = (mask & bit) != 0;
    let expected_frame = ActionFrame {
        player_id: initial.player_id,
        tick: 17,
        held_buttons: if selected_is_held { mask } else { mask | bit },
        pressed_buttons: if selected_is_held { 0 } else { bit },
        released_buttons: 0,
        axes: [0; 16],
    };
    // The source mutant returns !selected_is_held. The reducer checks
    // `down && !was_down`, so both the held mask and pressed edge follow that
    // inverted result exactly.
    let faulty_was_held = !selected_is_held;
    let faulty_frame = ActionFrame {
        player_id: initial.player_id,
        tick: 17,
        held_buttons: if faulty_was_held { mask } else { mask | bit },
        pressed_buttons: if faulty_was_held { 0 } else { bit },
        released_buttons: 0,
        axes: [0; 16],
    };
    (actual_frame, expected_frame, faulty_frame)
}

#[test]
fn production_source_hash_and_all_three_u64_helpers_are_bound_and_proved() {
    assert_eq!(
        sha256_hex(DIGITAL_SOURCE.as_bytes()),
        DIGITAL_SOURCE_SHA256,
        "source proof is bound to {DIGITAL_SOURCE_PATH}"
    );

    assert_eq!(
        prove_mask_predicate(DIGITAL_SOURCE, "bit_is_set"),
        VerificationVerdict::Proved
    );
    assert_eq!(
        prove_mask_update(DIGITAL_SOURCE, "set_bit", true),
        VerificationVerdict::Proved
    );
    assert_eq!(
        prove_mask_update(DIGITAL_SOURCE, "clear_bit", false),
        VerificationVerdict::Proved
    );
}

#[test]
fn same_source_proof_route_falsifies_and_replays_mutated_bit_test() {
    assert_eq!(
        sha256_hex(DIGITAL_SOURCE.as_bytes()),
        DIGITAL_SOURCE_SHA256,
        "negative control starts from the bound production source"
    );
    let good = "(mask & (1_u64 << slot)) != 0";
    let mutant = "(mask & (1_u64 << slot)) == 0";
    assert!(
        DIGITAL_SOURCE.contains(good),
        "mutant anchor missing from production source"
    );
    let bad_source = DIGITAL_SOURCE.replacen(good, mutant, 1);

    let verdict = prove_mask_predicate(&bad_source, "bit_is_set");
    let VerificationVerdict::Falsified { counterexample } = verdict else {
        panic!("mutated production helper should falsify, got {verdict:?}");
    };
    let mask = counterexample_value(&counterexample, "mask");
    let slot = counterexample_value(&counterexample, "slot");
    assert!(
        slot < 64,
        "witness escaped the reducer-validated slot domain"
    );

    // Both controls pin each native branch outcome independent of SAT phase:
    // clear->down produces a press; held->down remains a duplicate no-op.
    for (control_mask, expected_pressed) in [(0_u64, 1_u64 << slot), (1_u64 << slot, 0)] {
        let (actual, expected, faulty) = replay_inverted_bit_test(control_mask, slot);
        assert_eq!(actual, expected, "native positive control");
        assert_eq!(expected.pressed_buttons, expected_pressed);
        assert_ne!(
            expected, faulty,
            "each polarity distinguishes the source mutant"
        );
    }

    let (actual, expected, faulty) = replay_inverted_bit_test(mask, slot);
    assert_eq!(
        actual, expected,
        "decoded witness through native production reducer"
    );
    assert_ne!(
        expected, faulty,
        "decoded witness must distinguish mutant semantics"
    );
    println!(
        "source-mutant witness: mask={mask:#018x}, slot={slot}, actual held={:#018x} pressed={:#018x}; faulty held={:#018x} pressed={:#018x}",
        actual.held_buttons,
        actual.pressed_buttons,
        faulty.held_buttons,
        faulty.pressed_buttons
    );
}
