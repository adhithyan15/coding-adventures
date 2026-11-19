//! Bounded full-width SAT checks for raw Q16.16 addition and subtraction.
//!
//! The reference and candidate are separate Boolean circuits. The reference
//! computes an exact signed 33-bit result (with direct borrow for subtraction)
//! and checks its sign extension. The candidate computes a 32-bit result and
//! uses the signed-overflow predicates used by the public API contract. No
//! input patterns are excluded and no output cutpoints are assumed.

use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver, Var};
use coding_adventures_fixed_point::{
    checked_add, checked_sub, saturating_add, saturating_sub, Q16Error, Q16,
};
use drat_check::{check_with_limits, Limits as DratLimits};
use std::time::Instant;

const WIDTH: usize = 32;
const SOLVER_CONFLICT_LIMIT: u64 = 10_000_000;
const SOLVER_WORK_LIMIT: u64 = 1_000_000_000;
const DRAT_WORK_LIMIT: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Add,
    Sub,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutant {
    None,
    CheckedAlwaysOk,
    WrongClampDirection,
    MissingSubCarryIn,
    UnsignedOverflow,
    WrapSaturation,
    SpuriousEndpointFlag,
}

#[derive(Clone, Copy)]
struct Inputs {
    left: [Var; WIDTH],
    right: [Var; WIDTH],
}

#[derive(Clone)]
struct Observation {
    checked_ok: Var,
    checked_raw: [Var; WIDTH],
    saturated_raw: [Var; WIDTH],
    overflowed: Var,
}

struct Circuit {
    cnf: Cnf,
}

impl Circuit {
    fn new() -> Self {
        Self { cnf: Cnf::new(0) }
    }

    fn fresh(&mut self) -> Var {
        let var = Var::from_index(self.cnf.num_vars);
        self.cnf.num_vars += 1;
        var
    }

    fn input<const N: usize>(&mut self) -> [Var; N] {
        std::array::from_fn(|_| self.fresh())
    }

    fn constant(&mut self, value: bool) -> Var {
        let out = self.fresh();
        // `Var::lit` takes `negated`, so false selects the positive literal.
        self.cnf.add_clause(&[out.lit(!value)]);
        out
    }

    fn and(&mut self, a: Lit, b: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[!a, !b, out.positive()]);
        self.cnf.add_clause(&[a, out.negative()]);
        self.cnf.add_clause(&[b, out.negative()]);
        out
    }

    fn or(&mut self, a: Lit, b: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[a, b, out.negative()]);
        self.cnf.add_clause(&[!a, out.positive()]);
        self.cnf.add_clause(&[!b, out.positive()]);
        out
    }

    fn xor(&mut self, a: Lit, b: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[!a, !b, out.negative()]);
        self.cnf.add_clause(&[a, b, out.negative()]);
        self.cnf.add_clause(&[a, !b, out.positive()]);
        self.cnf.add_clause(&[!a, b, out.positive()]);
        out
    }

    fn not(&mut self, value: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[value, out.positive()]);
        self.cnf.add_clause(&[!value, out.negative()]);
        out
    }

    fn mux(&mut self, select: Lit, when_true: Lit, when_false: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[!select, !when_true, out.positive()]);
        self.cnf.add_clause(&[!select, when_true, out.negative()]);
        self.cnf.add_clause(&[select, !when_false, out.positive()]);
        self.cnf.add_clause(&[select, when_false, out.negative()]);
        out
    }

    fn full_add(&mut self, a: Lit, b: Lit, carry_in: Lit) -> (Var, Var) {
        let partial = self.xor(a, b);
        let sum = self.xor(partial.positive(), carry_in);
        let ab = self.and(a, b);
        let ac = self.and(a, carry_in);
        let bc = self.and(b, carry_in);
        let first = self.or(ab.positive(), ac.positive());
        let carry_out = self.or(first.positive(), bc.positive());
        (sum, carry_out)
    }

    /// One subtractor cell: `difference = a - b - borrow_in`.
    fn full_sub(&mut self, a: Lit, b: Lit, borrow_in: Lit) -> (Var, Var) {
        let axb = self.xor(a, b);
        let difference = self.xor(axb.positive(), borrow_in);
        let not_a_and_b = self.and(!a, b);
        let equal_and_borrow = self.and(!axb.positive(), borrow_in);
        let borrow_out = self.or(not_a_and_b.positive(), equal_and_borrow.positive());
        (difference, borrow_out)
    }

    fn raw_inputs(&mut self) -> Inputs {
        Inputs {
            left: self.input(),
            right: self.input(),
        }
    }
}

fn signed_extension(bits: &[Var; WIDTH]) -> Vec<Var> {
    let mut extended = bits.to_vec();
    extended.push(bits[WIDTH - 1]);
    extended
}

fn exact_reference(circuit: &mut Circuit, inputs: Inputs, operation: Operation) -> Observation {
    let left = signed_extension(&inputs.left);
    let right = signed_extension(&inputs.right);
    let mut exact = Vec::with_capacity(WIDTH + 1);
    match operation {
        Operation::Add => {
            let mut carry = circuit.constant(false);
            for i in 0..=WIDTH {
                let (sum, next) =
                    circuit.full_add(left[i].positive(), right[i].positive(), carry.positive());
                exact.push(sum);
                carry = next;
            }
        }
        Operation::Sub => {
            let mut borrow = circuit.constant(false);
            for i in 0..=WIDTH {
                let (difference, next) =
                    circuit.full_sub(left[i].positive(), right[i].positive(), borrow.positive());
                exact.push(difference);
                borrow = next;
            }
        }
    }

    // A signed 33-bit exact result is representable as i32 iff its top two
    // bits agree. The low 32 bits are the successful Q16 raw payload.
    let overflowed = circuit.xor(exact[WIDTH].positive(), exact[WIDTH - 1].positive());
    let checked_ok = circuit.not(overflowed.positive());
    let max: [Var; WIDTH] = std::array::from_fn(|i| circuit.constant(i < WIDTH - 1));
    let min: [Var; WIDTH] = std::array::from_fn(|i| circuit.constant(i == WIDTH - 1));
    let negative = exact[WIDTH].positive();
    let clamp: [Var; WIDTH] =
        std::array::from_fn(|i| circuit.mux(negative, min[i].positive(), max[i].positive()));
    let saturated_raw: [Var; WIDTH] = std::array::from_fn(|i| {
        circuit.mux(
            overflowed.positive(),
            clamp[i].positive(),
            exact[i].positive(),
        )
    });
    Observation {
        checked_ok,
        checked_raw: exact[..WIDTH].try_into().expect("32-bit result"),
        saturated_raw,
        overflowed,
    }
}

fn implementation_candidate(
    circuit: &mut Circuit,
    inputs: Inputs,
    operation: Operation,
    mutant: Mutant,
) -> Observation {
    // This is deliberately a different construction from the reference:
    // candidate arithmetic stays at 32 bits and derives signed overflow from
    // operand/result signs. Subtraction uses a + !b + 1.
    let mut carry =
        circuit.constant(operation == Operation::Sub && mutant != Mutant::MissingSubCarryIn);
    let mut result = Vec::with_capacity(WIDTH);
    for i in 0..WIDTH {
        let right_bit = match operation {
            Operation::Add => inputs.right[i].positive(),
            Operation::Sub => inputs.right[i].negative(),
        };
        let (sum, next) = circuit.full_add(inputs.left[i].positive(), right_bit, carry.positive());
        result.push(sum);
        carry = next;
    }
    let carry_out = carry;

    let sign_left = inputs.left[WIDTH - 1];
    let sign_right = inputs.right[WIDTH - 1];
    let sign_result = result[WIDTH - 1];
    let same_sign = circuit.xor(sign_left.positive(), sign_right.positive());
    let changed_sign = circuit.xor(sign_left.positive(), sign_result.positive());
    let sign_overflow = match operation {
        Operation::Add => circuit.and(!same_sign.positive(), changed_sign.positive()),
        Operation::Sub => circuit.and(same_sign.positive(), changed_sign.positive()),
    };
    let arithmetic_overflow = match (mutant, operation) {
        (Mutant::UnsignedOverflow, Operation::Add) => carry_out,
        (Mutant::UnsignedOverflow, Operation::Sub) => circuit.not(carry_out.positive()),
        _ => sign_overflow,
    };

    let checked_ok = if mutant == Mutant::CheckedAlwaysOk {
        circuit.constant(true)
    } else {
        circuit.not(arithmetic_overflow.positive())
    };

    let max: [Var; WIDTH] = std::array::from_fn(|i| circuit.constant(i < WIDTH - 1));
    let min: [Var; WIDTH] = std::array::from_fn(|i| circuit.constant(i == WIDTH - 1));
    let clamp_negative = if mutant == Mutant::WrongClampDirection {
        sign_left.negative()
    } else {
        sign_left.positive()
    };
    let clamp: [Var; WIDTH] =
        std::array::from_fn(|i| circuit.mux(clamp_negative, min[i].positive(), max[i].positive()));

    let mut saturated_raw: [Var; WIDTH] = std::array::from_fn(|i| {
        circuit.mux(
            arithmetic_overflow.positive(),
            clamp[i].positive(),
            result[i].positive(),
        )
    });
    let mut overflowed = arithmetic_overflow;
    if mutant == Mutant::WrapSaturation {
        saturated_raw = result.clone().try_into().expect("32-bit result");
        overflowed = circuit.constant(false);
    } else if mutant == Mutant::SpuriousEndpointFlag {
        let at_min = word_equals_constant(circuit, &result, 1_u32 << 31);
        let at_max = word_equals_constant(circuit, &result, i32::MAX as u32);
        let endpoint = circuit.or(at_min.positive(), at_max.positive());
        overflowed = circuit.or(arithmetic_overflow.positive(), endpoint.positive());
    }

    Observation {
        checked_ok,
        checked_raw: result.try_into().expect("32-bit result"),
        saturated_raw,
        overflowed,
    }
}

fn word_equals_constant(circuit: &mut Circuit, bits: &[Var], value: u32) -> Var {
    assert_eq!(bits.len(), WIDTH);
    let terms: Vec<_> = bits
        .iter()
        .enumerate()
        .map(|(i, bit)| bit.lit(((value >> i) & 1) == 0))
        .collect();
    let mut all = circuit.constant(true);
    for term in terms {
        let next = circuit.and(all.positive(), term);
        all = next;
    }
    all
}

fn any_difference(circuit: &mut Circuit, left: &[Var], right: &[Var]) -> Var {
    assert_eq!(left.len(), right.len());
    let mut any = circuit.constant(false);
    for (a, b) in left.iter().zip(right) {
        let diff = circuit.xor(a.positive(), b.positive());
        any = circuit.or(any.positive(), diff.positive());
    }
    any
}

fn output_mismatch(circuit: &mut Circuit, reference: &Observation, candidate: &Observation) -> Var {
    let tag_diff = circuit.xor(
        reference.checked_ok.positive(),
        candidate.checked_ok.positive(),
    );
    let raw_diff = any_difference(circuit, &reference.checked_raw, &candidate.checked_raw);
    let both_ok = circuit.and(
        reference.checked_ok.positive(),
        candidate.checked_ok.positive(),
    );
    let checked_raw_diff = circuit.and(both_ok.positive(), raw_diff.positive());
    let saturated_diff =
        any_difference(circuit, &reference.saturated_raw, &candidate.saturated_raw);
    let flag_diff = circuit.xor(
        reference.overflowed.positive(),
        candidate.overflowed.positive(),
    );
    let checked_diff = circuit.or(tag_diff.positive(), checked_raw_diff.positive());
    let with_sat = circuit.or(checked_diff.positive(), saturated_diff.positive());
    circuit.or(with_sat.positive(), flag_diff.positive())
}

struct Miter {
    cnf: Cnf,
    inputs: Inputs,
    reference: Observation,
    candidate: Observation,
}

fn encode_miter(operation: Operation, mutant: Mutant) -> Miter {
    let mut circuit = Circuit::new();
    let inputs = circuit.raw_inputs();
    let reference = exact_reference(&mut circuit, inputs, operation);
    let candidate = implementation_candidate(&mut circuit, inputs, operation, mutant);
    let mismatch = output_mismatch(&mut circuit, &reference, &candidate);
    circuit.cnf.add_clause(&[mismatch.positive()]);
    Miter {
        cnf: circuit.cnf,
        inputs,
        reference,
        candidate,
    }
}

fn constrain_word(cnf: &mut Cnf, bits: &[Var; WIDTH], value: u32) {
    for (i, bit) in bits.iter().enumerate() {
        // The literal must be true exactly when the bit has this value.
        cnf.add_clause(&[bit.lit(((value >> i) & 1) == 0)]);
    }
}

fn signed_word(model: &[bool], bits: &[Var; WIDTH]) -> i32 {
    bits.iter().enumerate().fold(0_u32, |word, (i, bit)| {
        word | ((model[bit.index()] as u32) << i)
    }) as i32
}

fn word_value(model: &[bool], bits: &[Var; WIDTH]) -> i32 {
    signed_word(model, bits)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PublicObservation {
    checked_ok: bool,
    checked_raw: i32,
    checked_error: Option<Q16Error>,
    saturated_raw: i32,
    overflowed: bool,
}

fn observe_model(model: &[bool], output: &Observation) -> PublicObservation {
    let checked_ok = model[output.checked_ok.index()];
    let checked_raw = if checked_ok {
        word_value(model, &output.checked_raw)
    } else {
        0
    };
    let saturated_raw = word_value(model, &output.saturated_raw);
    PublicObservation {
        checked_ok,
        checked_raw,
        checked_error: (!checked_ok).then_some(Q16Error::Overflow),
        saturated_raw,
        overflowed: model[output.overflowed.index()],
    }
}

fn scalar_oracle(operation: Operation, a: i32, b: i32) -> PublicObservation {
    let exact = match operation {
        Operation::Add => i128::from(a) + i128::from(b),
        Operation::Sub => i128::from(a) - i128::from(b),
    };
    let overflowed = exact < i128::from(i32::MIN) || exact > i128::from(i32::MAX);
    let bounded = exact.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
    PublicObservation {
        checked_ok: !overflowed,
        checked_raw: if overflowed { 0 } else { exact as i32 },
        checked_error: overflowed.then_some(Q16Error::Overflow),
        saturated_raw: bounded,
        overflowed,
    }
}

/// A scalar description of each deliberately faulty candidate predicate.
/// This does not inspect the CNF's internal wires: it evaluates wrapping
/// integer operations and the named faulty condition, then forms public
/// checked and saturating outcomes in Rust values.
fn scalar_mutant(operation: Operation, mutant: Mutant, a: i32, b: i32) -> PublicObservation {
    let a_bits = a as u32;
    let b_bits = b as u32;
    let (raw_bits, exact) = match (operation, mutant) {
        (Operation::Add, _) => (a_bits.wrapping_add(b_bits), i128::from(a) + i128::from(b)),
        (Operation::Sub, Mutant::MissingSubCarryIn) => (
            a_bits.wrapping_add(!b_bits),
            i128::from(a) - i128::from(b) - 1,
        ),
        (Operation::Sub, _) => (a_bits.wrapping_sub(b_bits), i128::from(a) - i128::from(b)),
    };
    let raw = raw_bits as i32;
    let exact_overflow = exact < i128::from(i32::MIN) || exact > i128::from(i32::MAX);
    let predicate_overflow = match mutant {
        Mutant::UnsignedOverflow => match operation {
            Operation::Add => u64::from(a_bits) + u64::from(b_bits) > u64::from(u32::MAX),
            Operation::Sub => a_bits < b_bits,
        },
        Mutant::MissingSubCarryIn => {
            debug_assert_eq!(operation, Operation::Sub);
            (a.is_negative() != b.is_negative()) && (a.is_negative() != raw.is_negative())
        }
        _ => exact_overflow,
    };
    let checked_ok = mutant == Mutant::CheckedAlwaysOk || !predicate_overflow;
    let checked_error = (!checked_ok).then_some(Q16Error::Overflow);
    let saturated_overflow = match mutant {
        Mutant::WrapSaturation => false,
        Mutant::SpuriousEndpointFlag => predicate_overflow || raw == i32::MIN || raw == i32::MAX,
        _ => predicate_overflow,
    };
    let saturated_raw = if mutant == Mutant::WrapSaturation || !predicate_overflow {
        raw
    } else {
        let clamp_negative = match mutant {
            Mutant::WrongClampDirection => !a.is_negative(),
            _ => a.is_negative(),
        };
        if clamp_negative {
            i32::MIN
        } else {
            i32::MAX
        }
    };
    PublicObservation {
        checked_ok,
        checked_raw: if checked_ok { raw } else { 0 },
        checked_error,
        saturated_raw,
        overflowed: saturated_overflow,
    }
}

fn public_observation(operation: Operation, a: i32, b: i32) -> PublicObservation {
    let left = Q16::from_raw(a);
    let right = Q16::from_raw(b);
    match operation {
        Operation::Add => {
            let checked = checked_add(left, right);
            let saturated = saturating_add(left, right);
            let (checked_ok, checked_raw, checked_error) = match checked {
                Ok(value) => (true, value.raw(), None),
                Err(Q16Error::Overflow) => (false, 0, Some(Q16Error::Overflow)),
                Err(Q16Error::DivisionByZero) => panic!("addition cannot return DivisionByZero"),
            };
            PublicObservation {
                checked_ok,
                checked_raw,
                checked_error,
                saturated_raw: saturated.value.raw(),
                overflowed: saturated.overflowed,
            }
        }
        Operation::Sub => {
            let checked = checked_sub(left, right);
            let saturated = saturating_sub(left, right);
            let (checked_ok, checked_raw, checked_error) = match checked {
                Ok(value) => (true, value.raw(), None),
                Err(Q16Error::Overflow) => (false, 0, Some(Q16Error::Overflow)),
                Err(Q16Error::DivisionByZero) => panic!("subtraction cannot return DivisionByZero"),
            };
            PublicObservation {
                checked_ok,
                checked_raw,
                checked_error,
                saturated_raw: saturated.value.raw(),
                overflowed: saturated.overflowed,
            }
        }
    }
}

fn assert_api_matches_oracle(operation: Operation, a: i32, b: i32) -> PublicObservation {
    let expected = scalar_oracle(operation, a, b);
    let actual = public_observation(operation, a, b);
    assert_eq!(actual, expected, "{}({a}, {b})", operation.name());
    actual
}

fn solve_with_budget(cnf: &Cnf, with_proof: bool) -> Solver {
    let mut solver = if with_proof {
        Solver::from_cnf_with_proof(cnf)
    } else {
        Solver::from_cnf(cnf)
    };
    solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
    solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
    solver
}

fn prove_unsat(miter: Miter, label: &str) {
    let formula = write_dimacs(&miter.cnf);
    let started = Instant::now();
    let mut solver = solve_with_budget(&miter.cnf, true);
    let verdict = solver.solve();
    let stats = solver.stats();
    eprintln!(
        "Q16-ADD-SUB-SAT {label} vars={} clauses={} verdict={verdict:?} conflicts={} work={} conflict_limit={} work_limit={} elapsed_ms={}",
        miter.cnf.num_vars,
        miter.cnf.clauses.len(),
        stats.conflicts,
        stats.work,
        SOLVER_CONFLICT_LIMIT,
        SOLVER_WORK_LIMIT,
        started.elapsed().as_millis()
    );
    assert_eq!(
        verdict,
        SolveResult::Unsat,
        "{label}: UNKNOWN is not a proof"
    );
    let proof = solver
        .proof()
        .expect("UNSAT must include DRAT proof")
        .to_drat_text();
    let checked = check_with_limits(
        &formula,
        &proof,
        DratLimits {
            max_work: DRAT_WORK_LIMIT,
            ..DratLimits::default()
        },
    )
    .unwrap_or_else(|error| panic!("{label}: DRAT rejected: {error:?}"));
    eprintln!(
        "Q16-ADD-SUB-DRAT {label} accepted=true proof_bytes={} lemmas={} rat_lemmas={} deletions={} work={} work_limit={DRAT_WORK_LIMIT}",
        proof.len(), checked.lemmas, checked.rat_lemmas, checked.deletions, checked.work
    );
}

fn solve_mutant(
    operation: Operation,
    mutant: Mutant,
    forced: Option<(u32, u32)>,
) -> (i32, i32, PublicObservation, PublicObservation) {
    let mut miter = encode_miter(operation, mutant);
    if let Some((left, right)) = forced {
        constrain_word(&mut miter.cnf, &miter.inputs.left, left);
        constrain_word(&mut miter.cnf, &miter.inputs.right, right);
    }
    let mut solver = solve_with_budget(&miter.cnf, false);
    let verdict = solver.solve();
    assert_eq!(
        verdict,
        SolveResult::Sat,
        "{} mutant {mutant:?} must have an observable witness",
        operation.name()
    );
    let model = solver.model();
    assert!(
        miter.cnf.is_satisfied_by(model),
        "solver model must satisfy the miter clauses"
    );
    let a = signed_word(model, &miter.inputs.left);
    let b = signed_word(model, &miter.inputs.right);
    let reference = observe_model(model, &miter.reference);
    let broken = observe_model(model, &miter.candidate);
    let actual = assert_api_matches_oracle(operation, a, b);
    assert_eq!(
        reference, actual,
        "independent exact circuit agrees with public API at SAT witness"
    );
    let broken_scalar = scalar_mutant(operation, mutant, a, b);
    assert_eq!(
        broken, broken_scalar,
        "encoded mutant must match its independently evaluated scalar predicate"
    );
    assert_ne!(
        broken_scalar, actual,
        "the broken predicate must differ from a healthy public result"
    );
    eprintln!(
        "Q16-ADD-SUB-MUTANT op={} mutant={mutant:?} a={a} b={b} checked_ok={} checked_raw={} saturated_raw={} overflowed={}",
        operation.name(), broken.checked_ok, broken.checked_raw, broken.saturated_raw, broken.overflowed
    );
    (a, b, reference, broken)
}

fn force_gate_inputs(cnf: &mut Cnf, vars: &[Var], values: &[bool]) {
    assert_eq!(vars.len(), values.len());
    for (var, value) in vars.iter().zip(values) {
        cnf.add_clause(&[var.lit(!*value)]);
    }
}

fn prove_forced_output_is_impossible(mut cnf: Cnf, output: Var, expected: bool, label: &str) {
    // Force the output opposite its independent truth-table value. The
    // resulting tiny formula must be UNSAT and its proof must pass DRAT.
    cnf.add_clause(&[output.lit(expected)]);
    let formula = write_dimacs(&cnf);
    let mut solver = solve_with_budget(&cnf, true);
    let verdict = solver.solve();
    assert_eq!(
        verdict,
        SolveResult::Unsat,
        "{label}: opposite output is impossible"
    );
    let proof = solver
        .proof()
        .expect("tiny UNSAT query has DRAT proof")
        .to_drat_text();
    check_with_limits(
        &formula,
        &proof,
        DratLimits {
            max_work: DRAT_WORK_LIMIT,
            ..DratLimits::default()
        },
    )
    .unwrap_or_else(|error| panic!("{label}: tiny DRAT proof rejected: {error:?}"));
}

#[test]
fn gate_and_arithmetic_cell_truth_tables_pin_literal_polarity() {
    // A tiny independent truth-table harness catches polarity errors before
    // any larger theorem is trusted. Every query also checks the model against
    // the complete CNF, not only the selected output wire.
    let mut circuit = Circuit::new();
    let a = circuit.fresh();
    let b = circuit.fresh();
    let c = circuit.fresh();
    let not_out = circuit.not(a.positive());
    let and_out = circuit.and(a.positive(), b.positive());
    let or_out = circuit.or(a.positive(), b.positive());
    let xor_out = circuit.xor(a.positive(), b.positive());
    let mux_out = circuit.mux(c.positive(), a.positive(), b.positive());
    let false_out = circuit.constant(false);
    let true_out = circuit.constant(true);
    let mut gate_contradictions = 0;
    for av in [false, true] {
        for bv in [false, true] {
            for cv in [false, true] {
                let mut query = circuit.cnf.clone();
                force_gate_inputs(&mut query, &[a, b, c], &[av, bv, cv]);
                let mut solver = solve_with_budget(&query, false);
                assert_eq!(solver.solve(), SolveResult::Sat);
                assert!(query.is_satisfied_by(solver.model()));
                let model = solver.model();
                assert_eq!(model[and_out.index()], av & bv);
                assert_eq!(model[or_out.index()], av | bv);
                assert_eq!(model[xor_out.index()], av ^ bv);
                assert_eq!(model[mux_out.index()], if cv { av } else { bv });
                assert_eq!(model[not_out.index()], !av);
                assert!(!model[false_out.index()]);
                assert!(model[true_out.index()]);
                for (output, expected, name) in [
                    (and_out, av & bv, "and"),
                    (or_out, av | bv, "or"),
                    (xor_out, av ^ bv, "xor"),
                    (mux_out, if cv { av } else { bv }, "mux"),
                    (not_out, !av, "not"),
                ] {
                    let mut opposite = circuit.cnf.clone();
                    force_gate_inputs(&mut opposite, &[a, b, c], &[av, bv, cv]);
                    prove_forced_output_is_impossible(
                        opposite,
                        output,
                        expected,
                        &format!("gate {name}({av},{bv},{cv})"),
                    );
                    gate_contradictions += 1;
                }
            }
        }
    }
    prove_forced_output_is_impossible(circuit.cnf.clone(), false_out, false, "constant false");
    prove_forced_output_is_impossible(circuit.cnf.clone(), true_out, true, "constant true");
    gate_contradictions += 2;

    let mut cells = Circuit::new();
    let x = cells.fresh();
    let y = cells.fresh();
    let carry_or_borrow = cells.fresh();
    let (sum, carry) = cells.full_add(x.positive(), y.positive(), carry_or_borrow.positive());
    let (difference, borrow) =
        cells.full_sub(x.positive(), y.positive(), carry_or_borrow.positive());
    let mut cell_contradictions = 0;
    for xv in [false, true] {
        for yv in [false, true] {
            for cv in [false, true] {
                let mut query = cells.cnf.clone();
                force_gate_inputs(&mut query, &[x, y, carry_or_borrow], &[xv, yv, cv]);
                let mut solver = solve_with_budget(&query, false);
                assert_eq!(solver.solve(), SolveResult::Sat);
                assert!(query.is_satisfied_by(solver.model()));
                let model = solver.model();
                assert_eq!(model[sum.index()], xv ^ yv ^ cv);
                let total = u8::from(xv) + u8::from(yv) + u8::from(cv);
                assert_eq!(model[carry.index()], total >= 2);
                let signed_difference = i8::from(xv) - i8::from(yv) - i8::from(cv);
                assert_eq!(
                    model[difference.index()],
                    signed_difference.rem_euclid(2) != 0
                );
                assert_eq!(model[borrow.index()], signed_difference < 0);
                for (output, expected, name) in [
                    (sum, xv ^ yv ^ cv, "full-add sum"),
                    (carry, total >= 2, "full-add carry"),
                    (
                        difference,
                        signed_difference.rem_euclid(2) != 0,
                        "full-sub difference",
                    ),
                    (borrow, signed_difference < 0, "full-sub borrow"),
                ] {
                    let mut opposite = cells.cnf.clone();
                    force_gate_inputs(&mut opposite, &[x, y, carry_or_borrow], &[xv, yv, cv]);
                    prove_forced_output_is_impossible(
                        opposite,
                        output,
                        expected,
                        &format!("cell {name}({xv},{yv},{cv})"),
                    );
                    cell_contradictions += 1;
                }
            }
        }
    }
    eprintln!(
        "Q16-ADD-SUB-TRUTH gates={} cells={} opposite_output_checks_drat_checked={}",
        7,
        2,
        gate_contradictions + cell_contradictions
    );
}

#[test]
fn full_i32_add_checked_and_saturating_outputs_are_proved_and_drat_checked() {
    prove_unsat(encode_miter(Operation::Add, Mutant::None), "add_full_i32");
}

#[test]
fn full_i32_sub_checked_and_saturating_outputs_are_proved_and_drat_checked() {
    prove_unsat(encode_miter(Operation::Sub, Mutant::None), "sub_full_i32");
}

#[test]
fn public_add_sub_boundaries_match_independent_i128_oracle() {
    for (operation, pairs) in [
        (
            Operation::Add,
            [
                (i32::MAX, 0),
                (i32::MIN, 0),
                (i32::MAX, 1),
                (i32::MIN, -1),
                (i32::MIN, i32::MAX),
                (i32::MAX, i32::MIN),
                (0, 0),
            ],
        ),
        (
            Operation::Sub,
            [
                (i32::MAX, 0),
                (i32::MIN, 0),
                (i32::MAX, -1),
                (i32::MIN, 1),
                (i32::MIN, i32::MIN),
                (i32::MAX, i32::MAX),
                (0, 0),
            ],
        ),
    ] {
        for (a, b) in pairs {
            assert_api_matches_oracle(operation, a, b);
        }
    }
}

#[test]
fn sat_mutants_have_model_checked_publicly_observable_witnesses() {
    let cases = [
        (Operation::Add, Mutant::CheckedAlwaysOk, None),
        (
            Operation::Add,
            Mutant::WrongClampDirection,
            Some((i32::MAX as u32, 1)),
        ),
        (
            Operation::Add,
            Mutant::WrongClampDirection,
            Some((i32::MIN as u32, u32::MAX)),
        ),
        (Operation::Add, Mutant::UnsignedOverflow, None),
        (Operation::Add, Mutant::WrapSaturation, None),
        (
            Operation::Add,
            Mutant::SpuriousEndpointFlag,
            Some((i32::MAX as u32, 0)),
        ),
        (Operation::Sub, Mutant::CheckedAlwaysOk, None),
        (
            Operation::Sub,
            Mutant::WrongClampDirection,
            Some((i32::MAX as u32, u32::MAX)),
        ),
        (
            Operation::Sub,
            Mutant::WrongClampDirection,
            Some((i32::MIN as u32, 1)),
        ),
        (Operation::Sub, Mutant::MissingSubCarryIn, None),
        (Operation::Sub, Mutant::UnsignedOverflow, None),
        (Operation::Sub, Mutant::WrapSaturation, None),
        (
            Operation::Sub,
            Mutant::SpuriousEndpointFlag,
            Some((i32::MIN as u32, 0)),
        ),
    ];
    for (operation, mutant, forced) in cases {
        solve_mutant(operation, mutant, forced);
    }
}
