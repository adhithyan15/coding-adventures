//! Bounded SAT miter for 64 action slots and three ordered button levels.
//!
//! The CNF independently tabulates the specified per-slot transition truth
//! table. It is a finite Boolean model, not extracted Rust and not a proof of
//! the record-based reducer. Its UNSAT result is DRAT-checked; SAT mutants are
//! decoded into three ordered 64-event rounds and replayed through the real crate.

use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver, Var};
use game_input_frame::{reduce_frame, ActionChange, ButtonActionId, InputState, PlayerId};

const BUTTON_SLOTS: usize = 64;
const TRANSITIONS: usize = 3;
const INPUTS_PER_SLOT: usize = 1 + TRANSITIONS;
const OUTPUTS_PER_SLOT: usize = 6;
const DIFFS_PER_SLOT: usize = 3;
const VARS_PER_SLOT: usize = INPUTS_PER_SLOT + OUTPUTS_PER_SLOT + DIFFS_PER_SLOT;

#[derive(Clone, Copy)]
enum Candidate {
    ExactOrdered,
    EndpointOnly,
    DropBit63,
}

#[derive(Clone, Copy)]
struct SlotVars {
    levels: [Var; INPUTS_PER_SLOT],
    expected: [Var; 3],
    candidate: [Var; 3],
    different: [Var; 3],
}

fn slot_vars(slot: usize) -> SlotVars {
    let base = slot * VARS_PER_SLOT;
    SlotVars {
        levels: std::array::from_fn(|offset| Var::from_index(base + offset)),
        expected: std::array::from_fn(|offset| Var::from_index(base + INPUTS_PER_SLOT + offset)),
        candidate: std::array::from_fn(|offset| {
            Var::from_index(base + INPUTS_PER_SLOT + 3 + offset)
        }),
        different: std::array::from_fn(|offset| {
            Var::from_index(base + INPUTS_PER_SLOT + OUTPUTS_PER_SLOT + offset)
        }),
    }
}

fn truth_table(levels: [bool; INPUTS_PER_SLOT]) -> [bool; 3] {
    let mut held = levels[0];
    let mut pressed = false;
    let mut released = false;
    for next in levels.into_iter().skip(1) {
        pressed |= !held && next;
        released |= held && !next;
        held = next;
    }
    [held, pressed, released]
}

/// Direct occurrence predicates over adjacent levels, deliberately expressed
/// separately from the stepwise expected-side fold above.
fn ordered_predicates(levels: [bool; INPUTS_PER_SLOT]) -> [bool; 3] {
    let pressed =
        (!levels[0] && levels[1]) || (!levels[1] && levels[2]) || (!levels[2] && levels[3]);
    let released =
        (levels[0] && !levels[1]) || (levels[1] && !levels[2]) || (levels[2] && !levels[3]);
    [levels[3], pressed, released]
}

/// Encode `output == f(inputs)` by forbidding each of the sixteen wrong rows.
fn encode_output(
    cnf: &mut Cnf,
    inputs: &[Var; INPUTS_PER_SLOT],
    output: Var,
    value: impl Fn([bool; INPUTS_PER_SLOT]) -> bool,
) {
    for row in 0_u8..(1 << INPUTS_PER_SLOT) {
        let bits = std::array::from_fn(|index| row & (1 << index) != 0);
        let mut clause: Vec<Lit> = inputs
            .iter()
            .enumerate()
            .map(|(index, var)| var.lit(bits[index]))
            .collect();
        clause.push(output.lit(!value(bits)));
        cnf.add_clause(&clause);
    }
}

fn candidate_value(
    candidate: Candidate,
    slot: usize,
    levels: [bool; INPUTS_PER_SLOT],
) -> [bool; 3] {
    match candidate {
        Candidate::ExactOrdered => ordered_predicates(levels),
        Candidate::EndpointOnly => [
            levels[INPUTS_PER_SLOT - 1],
            !levels[0] && levels[INPUTS_PER_SLOT - 1],
            levels[0] && !levels[INPUTS_PER_SLOT - 1],
        ],
        Candidate::DropBit63 if slot == 63 => [false, false, false],
        Candidate::DropBit63 => ordered_predicates(levels),
    }
}

fn miter(
    candidate: Candidate,
    forced: Option<(usize, [bool; INPUTS_PER_SLOT])>,
) -> (Cnf, Vec<SlotVars>) {
    let mut cnf = Cnf::new(BUTTON_SLOTS * VARS_PER_SLOT);
    let variables: Vec<_> = (0..BUTTON_SLOTS).map(slot_vars).collect();
    let mut mismatch_terms = Vec::with_capacity(BUTTON_SLOTS * DIFFS_PER_SLOT);

    for (slot, vars) in variables.iter().enumerate() {
        for output_index in 0..3 {
            encode_output(
                &mut cnf,
                &vars.levels,
                vars.expected[output_index],
                |levels| truth_table(levels)[output_index],
            );
            encode_output(
                &mut cnf,
                &vars.levels,
                vars.candidate[output_index],
                |levels| candidate_value(candidate, slot, levels)[output_index],
            );

            // `different` is the exclusive-or of expected and candidate.
            let left = vars.expected[output_index];
            let right = vars.candidate[output_index];
            let diff = vars.different[output_index];
            cnf.add_clause(&[left.positive(), right.positive(), diff.negative()]);
            cnf.add_clause(&[left.negative(), right.negative(), diff.negative()]);
            cnf.add_clause(&[left.positive(), right.negative(), diff.positive()]);
            cnf.add_clause(&[left.negative(), right.positive(), diff.positive()]);
            mismatch_terms.push(diff.positive());
        }
    }
    cnf.add_clause(&mismatch_terms);
    if let Some((slot, levels)) = forced {
        for (variable, value) in variables[slot].levels.iter().zip(levels) {
            cnf.add_clause(&[variable.lit(!value)]);
        }
    }
    (cnf, variables)
}

fn verify_unsat(cnf: &Cnf) {
    let formula = write_dimacs(cnf);
    let mut solver = Solver::from_cnf_with_proof(cnf);
    assert_eq!(
        solver.solve(),
        SolveResult::Unsat,
        "miter mismatch must be UNSAT"
    );
    let proof = solver
        .proof()
        .expect("UNSAT run records DRAT proof")
        .to_drat_text();
    drat_check::check(&formula, &proof).expect("independent DRAT checker accepts the UNSAT proof");
}

fn witness_levels(model: &[bool], variables: &SlotVars) -> [bool; INPUTS_PER_SLOT] {
    std::array::from_fn(|index| model[variables.levels[index].index()])
}

fn output_values(model: &[bool], variables: &[SlotVars], candidate: bool) -> Vec<[bool; 3]> {
    variables
        .iter()
        .map(|slot| {
            let output = if candidate {
                slot.candidate
            } else {
                slot.expected
            };
            std::array::from_fn(|index| model[output[index].index()])
        })
        .collect()
}

fn replay_sat_mutant(
    candidate: Candidate,
    forced: Option<(usize, [bool; INPUTS_PER_SLOT])>,
) -> (usize, [bool; INPUTS_PER_SLOT], [bool; 3], [bool; 3]) {
    let (cnf, variables) = miter(candidate, forced);
    let mut solver = Solver::from_cnf(&cnf);
    assert_eq!(
        solver.solve(),
        SolveResult::Sat,
        "mutant must have a mismatch witness"
    );
    let assignment = solver.model();
    assert!(
        cnf.is_satisfied_by(assignment),
        "SAT witness satisfies the exact miter CNF"
    );
    let reference = output_values(assignment, &variables, false);
    let mutated = output_values(assignment, &variables, true);
    let slot = (0..BUTTON_SLOTS)
        .find(|&slot| reference[slot] != mutated[slot])
        .expect("the miter clause names a differing output");
    let levels = witness_levels(assignment, &variables[slot]);
    let expected_masks = (0..BUTTON_SLOTS).fold([0_u64; 3], |mut masks, action_slot| {
        let output = truth_table(witness_levels(assignment, &variables[action_slot]));
        let bit = 1_u64 << action_slot;
        for output_index in 0..3 {
            if output[output_index] {
                masks[output_index] |= bit;
            }
        }
        masks
    });
    let mutated_outputs = output_values(assignment, &variables, true);
    let mutant_masks = (0..BUTTON_SLOTS).fold([0_u64; 3], |mutant_masks, action_slot| {
        let output = mutated_outputs[action_slot];
        let bit = 1_u64 << action_slot;
        let mut masks = mutant_masks;
        for output_index in 0..3 {
            if output[output_index] {
                masks[output_index] |= bit;
            }
        }
        masks
    });

    let initial_mask = (0..BUTTON_SLOTS).fold(0_u64, |mask, action_slot| {
        if witness_levels(assignment, &variables[action_slot])[0] {
            mask | (1_u64 << action_slot)
        } else {
            mask
        }
    });
    let mut changes = Vec::with_capacity(BUTTON_SLOTS * TRANSITIONS);
    for step in 1..=TRANSITIONS {
        for (action_slot, slot_variables) in variables.iter().enumerate() {
            changes.push(ActionChange::Button {
                action_id: ButtonActionId::new(action_slot as u8),
                down: witness_levels(assignment, slot_variables)[step],
            });
        }
    }
    let frame = reduce_frame(
        &InputState {
            player_id: PlayerId::new(u32::MAX),
            held_buttons: initial_mask,
            axes: [0; 16],
        },
        u64::MAX,
        &changes,
    )
    .expect("decoded SAT witness is a valid public input");
    assert_eq!(
        [
            frame.held_buttons,
            frame.pressed_buttons,
            frame.released_buttons
        ],
        expected_masks,
        "replaying all 64 decoded level assignments matches every UNSAT-side output mask"
    );
    let mutant_frame = [mutant_masks[0], mutant_masks[1], mutant_masks[2]];
    assert_ne!(
        [
            frame.held_buttons,
            frame.pressed_buttons,
            frame.released_buttons
        ],
        mutant_frame,
        "full native frame differs from the full SAT mutant assignment"
    );
    (slot, levels, reference[slot], mutated[slot])
}

#[test]
fn ordered_three_step_mismatch_is_unsat_over_all_64_action_slots() {
    let (cnf, _) = miter(Candidate::ExactOrdered, None);
    verify_unsat(&cnf);
    println!("GBL050-SAT-MITER slots=64 transitions=3 mismatch=UNSAT drat=checked");
}

#[test]
fn endpoint_only_edge_mutant_has_sat_tap_witness_replayed_by_real_reducer() {
    let (slot, levels, expected, mutant) = replay_sat_mutant(
        Candidate::EndpointOnly,
        Some((0, [false, true, false, false])),
    );
    assert_eq!(slot, 0, "the forced tap slot is the first differing output");
    assert_eq!(levels, [false, true, false, false]);
    assert_ne!(expected, mutant);
    assert!(
        expected[1] && expected[2],
        "the witness carries both a press and release"
    );
    println!("GBL050-SAT-MUTANT endpoint-only slot={slot} levels={levels:?} replay=PASS");
}

#[test]
fn dropped_bit_63_mutant_has_sat_witness_replayed_by_real_reducer() {
    let (slot, levels, expected, mutant) =
        replay_sat_mutant(Candidate::DropBit63, Some((63, [false, true, true, true])));
    assert_eq!(
        slot, 63,
        "only the top action bit is corrupted by this mutant"
    );
    assert_eq!(levels, [false, true, true, true]);
    assert_ne!(expected, mutant);
    println!("GBL050-SAT-MUTANT drop-bit-63 slot={slot} levels={levels:?} replay=PASS");
}
