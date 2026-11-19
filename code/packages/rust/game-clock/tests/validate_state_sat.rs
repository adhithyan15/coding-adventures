//! Hand-encoded bounded SAT checks for GAME10 `validate_state`.
//!
//! These are independent Boolean encodings, not circuits extracted from Rust
//! or Warrant. Every query fixes `fixed_dt_ns` and keeps `tick_count` and
//! `elapsed_sim_ns` at their full 64-bit widths; `accumulator_ns`, step cap,
//! and delta cap remain symbolic. Four ignored counters are intentionally not
//! represented. These checks do not prove source-to-Warrant equivalence.

use cdcl_sat::{write_dimacs, Cnf, Lit, SolveResult, Solver, Var};
use drat_check::{check_with_limits, Limits as DratLimits};
use game_clock::{
    validate_state, ClockError, GameClockState, HARD_MAX_DELTA_NS, HARD_MAX_FRAME_STEPS,
};
use std::time::Instant;

#[path = "support/warrant_state.rs"]
mod warrant_state;

const U64_BITS: usize = 64;
const WIDE_BITS: usize = 128;
const SOLVER_CONFLICT_LIMIT: u64 = 10_000_000;
const SOLVER_WORK_LIMIT: u64 = 1_000_000_000;
const DRAT_WORK_LIMIT: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutant {
    None,
    SuppressOverflow,
    IgnoreElapsedMismatch,
    AlwaysAccept,
    AlwaysError,
    OverflowBeforeInvariant,
}

#[derive(Clone, Copy)]
struct Inputs {
    tick: [Var; U64_BITS],
    elapsed: [Var; U64_BITS],
    accumulator: [Var; U64_BITS],
    steps: [Var; 32],
    max_delta: [Var; U64_BITS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddMode {
    PropagateGenerate,
    Majority,
}

#[derive(Clone, Debug)]
struct CellReceipt {
    mode: AddMode,
    a: Var,
    b: Var,
    cin: Var,
    sum: Var,
    cout: Var,
    allocated: Vec<Var>,
    clause_start: usize,
    clause_end: usize,
}

#[derive(Clone, Debug)]
struct AdditionReceipt {
    mode: AddMode,
    left: Vec<Var>,
    right: Vec<Var>,
    carry_zero: Var,
    sum: Vec<Var>,
    cells: Vec<CellReceipt>,
}

#[derive(Clone, Debug)]
struct ProductStageReceipt {
    shift: usize,
    acc_before: Vec<Var>,
    term: Vec<Var>,
    addition_index: usize,
    acc_after: Vec<Var>,
}

#[derive(Clone, Debug)]
struct MultiplicationReceipt {
    dt: u64,
    mode: AddMode,
    tick: Vec<Var>,
    initial_acc: Vec<Var>,
    stages: Vec<ProductStageReceipt>,
    output: Vec<Var>,
}

struct Circuit {
    cnf: Cnf,
    cells: Vec<CellReceipt>,
    additions: Vec<AdditionReceipt>,
    multiplications: Vec<MultiplicationReceipt>,
}

impl Circuit {
    fn new() -> Self {
        Self {
            cnf: Cnf::new(0),
            cells: Vec::new(),
            additions: Vec::new(),
            multiplications: Vec::new(),
        }
    }

    fn fresh(&mut self) -> Var {
        let v = Var::from_index(self.cnf.num_vars);
        self.cnf.num_vars += 1;
        v
    }

    fn constant(&mut self, value: bool) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[out.lit(!value)]);
        out
    }

    fn input<const N: usize>(&mut self) -> [Var; N] {
        std::array::from_fn(|_| self.fresh())
    }

    fn and(&mut self, a: Lit, b: Lit) -> Var {
        let o = self.fresh();
        self.cnf.add_clause(&[!a, !b, o.positive()]);
        self.cnf.add_clause(&[a, o.negative()]);
        self.cnf.add_clause(&[b, o.negative()]);
        o
    }

    fn or(&mut self, a: Lit, b: Lit) -> Var {
        let o = self.fresh();
        self.cnf.add_clause(&[a, b, o.negative()]);
        self.cnf.add_clause(&[!a, o.positive()]);
        self.cnf.add_clause(&[!b, o.positive()]);
        o
    }

    fn xor(&mut self, a: Lit, b: Lit) -> Var {
        let o = self.fresh();
        self.cnf.add_clause(&[!a, !b, o.negative()]);
        self.cnf.add_clause(&[a, b, o.negative()]);
        self.cnf.add_clause(&[a, !b, o.positive()]);
        self.cnf.add_clause(&[!a, b, o.positive()]);
        o
    }

    fn mux(&mut self, select: Lit, yes: Lit, no: Lit) -> Var {
        let out = self.fresh();
        self.cnf.add_clause(&[!select, !yes, out.positive()]);
        self.cnf.add_clause(&[!select, yes, out.negative()]);
        self.cnf.add_clause(&[select, !no, out.positive()]);
        self.cnf.add_clause(&[select, no, out.negative()]);
        out
    }

    fn eq_const(&mut self, bits: &[Var], value: u128) -> Var {
        if bits.len() < 128 && value >= (1_u128 << bits.len()) {
            return self.constant(false);
        }
        let terms: Vec<_> = bits
            .iter()
            .enumerate()
            .map(|(i, b)| b.lit(((value >> i) & 1) == 0))
            .collect();
        self.all(&terms)
    }

    fn all(&mut self, terms: &[Lit]) -> Var {
        let out = self.fresh();
        for term in terms {
            self.cnf.add_clause(&[out.negative(), *term]);
        }
        let mut reverse = vec![out.positive()];
        reverse.extend(terms.iter().map(|term| !*term));
        self.cnf.add_clause(&reverse);
        out
    }

    fn any(&mut self, terms: &[Lit]) -> Var {
        let out = self.fresh();
        for term in terms {
            self.cnf.add_clause(&[!*term, out.positive()]);
        }
        let mut reverse = vec![out.negative()];
        reverse.extend_from_slice(terms);
        self.cnf.add_clause(&reverse);
        out
    }

    /// Unsigned `a >= constant`, compared from the most significant bit.
    fn ge_const(&mut self, bits: &[Var], value: u128) -> Var {
        if bits.len() < 128 && value >= (1_u128 << bits.len()) {
            return self.constant(false);
        }
        let mut equal_prefix = self.constant(true);
        let mut greater = self.constant(false);
        for i in (0..bits.len()).rev() {
            if (value >> i) & 1 == 0 {
                let term = self.and(equal_prefix.positive(), bits[i].positive());
                greater = self.or(greater.positive(), term.positive());
                equal_prefix = self.and(equal_prefix.positive(), bits[i].negative());
            } else {
                equal_prefix = self.and(equal_prefix.positive(), bits[i].positive());
            }
        }
        self.or(greater.positive(), equal_prefix.positive())
    }

    /// Independent first-differing-bit DNF for unsigned `bits >= value`.
    fn ge_const_first_difference(&mut self, bits: &[Var], value: u128) -> Var {
        if bits.len() < 128 && value >= (1_u128 << bits.len()) {
            return self.constant(false);
        }
        let mut terms = Vec::new();
        for i in (0..bits.len()).rev() {
            if (value >> i) & 1 == 0 {
                let mut prefix = Vec::new();
                for j in (i + 1..bits.len()).rev() {
                    prefix.push(bits[j].lit(((value >> j) & 1) == 0));
                }
                prefix.push(bits[i].positive());
                terms.push(self.all(&prefix).positive());
            }
        }
        let exact: Vec<_> = bits
            .iter()
            .enumerate()
            .map(|(i, bit)| bit.lit(((value >> i) & 1) == 0))
            .collect();
        terms.push(self.all(&exact).positive());
        self.any(&terms)
    }

    /// Ripple-add two same-width unsigned words. Carry-out is returned as bit N.
    fn add(&mut self, a: &[Var], b: &[Var]) -> Vec<Var> {
        assert_eq!(a.len(), b.len());
        let first_cell = self.cells.len();
        let mut carry = self.constant(false);
        let carry_zero = carry;
        let mut sum = Vec::with_capacity(a.len() + 1);
        for i in 0..a.len() {
            let (bit_sum, carry_out) = self.full_adder_propagate(a[i], b[i], carry);
            sum.push(bit_sum);
            carry = carry_out;
        }
        sum.push(carry);
        self.additions.push(AdditionReceipt {
            mode: AddMode::PropagateGenerate,
            left: a.to_vec(),
            right: b.to_vec(),
            carry_zero,
            sum: sum.clone(),
            cells: self.cells[first_cell..].to_vec(),
        });
        sum
    }

    /// Separate constant-product encoding with majority carry. This does not
    /// reuse `add`: sum is XOR, carry is `(a&b)|(a&carry)|(b&carry)`.
    fn add_majority(&mut self, a: &[Var], b: &[Var]) -> Vec<Var> {
        assert_eq!(a.len(), b.len());
        let first_cell = self.cells.len();
        let mut carry = self.constant(false);
        let carry_zero = carry;
        let mut sum = Vec::with_capacity(a.len() + 1);
        for i in 0..a.len() {
            let (bit_sum, carry_out) = self.full_adder_majority(a[i], b[i], carry);
            sum.push(bit_sum);
            carry = carry_out;
        }
        sum.push(carry);
        self.additions.push(AdditionReceipt {
            mode: AddMode::Majority,
            left: a.to_vec(),
            right: b.to_vec(),
            carry_zero,
            sum: sum.clone(),
            cells: self.cells[first_cell..].to_vec(),
        });
        sum
    }

    /// Exact ripple cell used by the propagate/generate multiplier circuit.
    fn full_adder_propagate(&mut self, a: Var, b: Var, carry: Var) -> (Var, Var) {
        let var_start = self.cnf.num_vars;
        let clause_start = self.cnf.clauses.len();
        let propagate = self.xor(a.positive(), b.positive());
        let sum = self.xor(propagate.positive(), carry.positive());
        let generate = self.and(a.positive(), b.positive());
        let propagated_carry = self.and(propagate.positive(), carry.positive());
        let carry_out = self.or(generate.positive(), propagated_carry.positive());
        self.cells.push(CellReceipt {
            mode: AddMode::PropagateGenerate,
            a,
            b,
            cin: carry,
            sum,
            cout: carry_out,
            allocated: (var_start..self.cnf.num_vars)
                .map(Var::from_index)
                .collect(),
            clause_start,
            clause_end: self.cnf.clauses.len(),
        });
        (sum, carry_out)
    }

    /// Exact ripple cell used by the majority-carry multiplier circuit.
    fn full_adder_majority(&mut self, a: Var, b: Var, carry: Var) -> (Var, Var) {
        let var_start = self.cnf.num_vars;
        let clause_start = self.cnf.clauses.len();
        let ab = self.xor(a.positive(), b.positive());
        let sum = self.xor(ab.positive(), carry.positive());
        let ab_carry = self.and(a.positive(), b.positive());
        let ac = self.and(a.positive(), carry.positive());
        let bc = self.and(b.positive(), carry.positive());
        let first = self.or(ab_carry.positive(), ac.positive());
        let carry_out = self.or(first.positive(), bc.positive());
        self.cells.push(CellReceipt {
            mode: AddMode::Majority,
            a,
            b,
            cin: carry,
            sum,
            cout: carry_out,
            allocated: (var_start..self.cnf.num_vars)
                .map(Var::from_index)
                .collect(),
            clause_start,
            clause_end: self.cnf.clauses.len(),
        });
        (sum, carry_out)
    }

    /// Zero extend `tick << shift` to the exact 128-bit multiplier operand.
    fn shifted_term(&mut self, tick: &[Var; 64], shift: usize) -> [Var; 128] {
        std::array::from_fn(|i| {
            if i >= shift && i - shift < 64 {
                tick[i - shift]
            } else {
                self.constant(false)
            }
        })
    }

    fn multiply_constant_majority(&mut self, tick: &[Var; 64], dt: u64) -> [Var; 128] {
        let mut acc: Vec<Var> = (0..WIDE_BITS).map(|_| self.constant(false)).collect();
        let initial_acc = acc.clone();
        let mut stages = Vec::new();
        let mut partial_factor = 0_u128;
        for shift in 0..64 {
            if (dt >> shift) & 1 == 0 {
                continue;
            }
            partial_factor = partial_factor
                .checked_add(1_u128 << shift)
                .expect("partial constant factor fits u128");
            let _largest_partial_product = (u64::MAX as u128)
                .checked_mul(partial_factor)
                .expect("partial product has no 128-bit overflow");
            let term = self.shifted_term(tick, shift);
            let acc_before = acc.clone();
            let addition_index = self.additions.len();
            let added = self.add_majority(&acc, &term);
            acc = added[..WIDE_BITS].to_vec();
            stages.push(ProductStageReceipt {
                shift,
                acc_before,
                term: term.to_vec(),
                addition_index,
                acc_after: acc.clone(),
            });
        }
        assert_eq!(
            partial_factor, dt as u128,
            "all selected shift terms are present"
        );
        let output = acc.clone();
        self.multiplications.push(MultiplicationReceipt {
            dt,
            mode: AddMode::Majority,
            tick: tick.to_vec(),
            initial_acc,
            stages,
            output,
        });
        acc.try_into().expect("fixed width")
    }

    /// Schoolbook multiplication by a fixed constant, accumulated into 128 bits.
    fn multiply_constant_shift_add(&mut self, tick: &[Var; 64], dt: u64) -> [Var; 128] {
        let mut acc: Vec<Var> = (0..WIDE_BITS).map(|_| self.constant(false)).collect();
        let initial_acc = acc.clone();
        let mut stages = Vec::new();
        let mut partial_factor = 0_u128;
        for shift in 0..64 {
            if (dt >> shift) & 1 == 0 {
                continue;
            }
            partial_factor = partial_factor
                .checked_add(1_u128 << shift)
                .expect("partial constant factor fits u128");
            let _largest_partial_product = (u64::MAX as u128)
                .checked_mul(partial_factor)
                .expect("partial product has no 128-bit overflow");
            let term = self.shifted_term(tick, shift);
            let acc_before = acc.clone();
            let addition_index = self.additions.len();
            let added = self.add(&acc, &term);
            acc = added[..WIDE_BITS].to_vec();
            stages.push(ProductStageReceipt {
                shift,
                acc_before,
                term: term.to_vec(),
                addition_index,
                acc_after: acc.clone(),
            });
        }
        assert_eq!(
            partial_factor, dt as u128,
            "all selected shift terms are present"
        );
        let output = acc.clone();
        self.multiplications.push(MultiplicationReceipt {
            dt,
            mode: AddMode::PropagateGenerate,
            tick: tick.to_vec(),
            initial_acc,
            stages,
            output,
        });
        acc.try_into().expect("fixed width")
    }

    /// Independently encoded threshold decomposition: overflow iff tick exceeds
    /// floor(MAX/dt), while low product bits are a sum of constant-selected shifts.
    fn threshold_product(&mut self, tick: &[Var; 64], dt: u64) -> ([Var; 64], Var) {
        let threshold = u64::MAX / dt;
        let overflow = self.ge_const(tick, (threshold as u128) + 1);
        let wide = self.multiply_constant_majority(tick, dt);
        let low = std::array::from_fn(|i| wide[i]);
        (low, overflow)
    }

    fn mismatch(&mut self, a: &[Var], b: &[Var]) -> Var {
        let mut bits = Vec::with_capacity(a.len());
        for (x, y) in a.iter().zip(b) {
            let eq = self.xor(x.positive(), y.positive());
            bits.push(eq.positive());
        }
        self.any(&bits)
    }

    fn validator_code(
        &mut self,
        dt: u64,
        input: &Inputs,
        wide_multiplier: bool,
        mutant: Mutant,
    ) -> [Var; 2] {
        if mutant == Mutant::AlwaysAccept {
            return [self.constant(false), self.constant(false)];
        }
        if mutant == Mutant::AlwaysError {
            return [self.constant(false), self.constant(true)];
        }
        let (product, mut overflow): ([Var; 64], Var) = if dt == 0 {
            (
                std::array::from_fn(|i| {
                    let _ = i;
                    self.constant(false)
                }),
                self.constant(false),
            )
        } else if wide_multiplier {
            let wide = self.multiply_constant_shift_add(&input.tick, dt);
            let high = self.any(&wide[64..].iter().map(|v| v.positive()).collect::<Vec<_>>());
            (std::array::from_fn(|i| wide[i]), high)
        } else {
            self.threshold_product(&input.tick, dt)
        };
        if mutant == Mutant::SuppressOverflow {
            overflow = self.constant(false);
        }
        self.result_from_cutpoints_predicate(dt, input, &product, overflow, mutant)
    }

    /// Predicate-shaped four-result circuit over checked arithmetic cutpoints.
    fn result_from_cutpoints_predicate(
        &mut self,
        dt: u64,
        input: &Inputs,
        product: &[Var; 64],
        overflow: Var,
        mutant: Mutant,
    ) -> [Var; 2] {
        if dt == 0 {
            return [self.constant(true), self.constant(false)];
        }
        let dt_bad = self.constant(dt > HARD_MAX_DELTA_NS);
        let steps_zero = self.eq_const(&input.steps, 0);
        let steps_large = self.ge_const(&input.steps, (HARD_MAX_FRAME_STEPS + 1) as u128);
        let steps_bad = self.or(steps_zero.positive(), steps_large.positive());
        let delta_below = if dt > HARD_MAX_DELTA_NS {
            self.constant(false)
        } else {
            self.less_than_const(&input.max_delta, dt as u128)
        };
        let delta_above = self.ge_const(&input.max_delta, HARD_MAX_DELTA_NS as u128 + 1);
        let delta_bad = self.or(delta_below.positive(), delta_above.positive());
        let accumulator_bad = if dt > HARD_MAX_DELTA_NS {
            self.constant(false)
        } else {
            self.ge_const(&input.accumulator, dt as u128)
        };
        let corrupt = self.any(&[
            dt_bad.positive(),
            steps_bad.positive(),
            delta_bad.positive(),
            accumulator_bad.positive(),
        ]);
        let mismatch = self.mismatch(&input.elapsed, product);
        let valid_overflow = if mutant == Mutant::OverflowBeforeInvariant {
            overflow
        } else {
            self.and(overflow.positive(), corrupt.negative())
        };
        let mismatch = if mutant == Mutant::IgnoreElapsedMismatch {
            self.constant(false)
        } else {
            mismatch
        };
        let valid_mismatch = self.and(mismatch.positive(), corrupt.negative());
        let low = valid_overflow;
        let high = self.any(&[
            corrupt.positive(),
            valid_mismatch.positive(),
            valid_overflow.positive(),
        ]);
        [low, high]
    }

    /// Priority-shaped result circuit over the same cutpoint interface.
    fn result_from_cutpoints_priority(
        &mut self,
        dt: u64,
        input: &Inputs,
        product: &[Var; 64],
        overflow: Var,
    ) -> [Var; 2] {
        if dt == 0 {
            return [self.constant(true), self.constant(false)];
        }
        let dt_bad = self.constant(dt > HARD_MAX_DELTA_NS);
        let steps_zero = self.eq_const(&input.steps, 0);
        let steps_large = self.ge_const(&input.steps, (HARD_MAX_FRAME_STEPS + 1) as u128);
        let steps_bad = self.or(steps_zero.positive(), steps_large.positive());
        let delta_below = if dt > HARD_MAX_DELTA_NS {
            self.constant(false)
        } else {
            self.less_than_const(&input.max_delta, dt as u128)
        };
        let delta_above = self.ge_const(&input.max_delta, HARD_MAX_DELTA_NS as u128 + 1);
        let delta_bad = self.or(delta_below.positive(), delta_above.positive());
        let accumulator_bad = if dt > HARD_MAX_DELTA_NS {
            self.constant(false)
        } else {
            self.ge_const(&input.accumulator, dt as u128)
        };
        let corrupt = self.any(&[
            dt_bad.positive(),
            steps_bad.positive(),
            delta_bad.positive(),
            accumulator_bad.positive(),
        ]);
        let mismatch = self.mismatch(&input.elapsed, product);

        let mut low = self.constant(false);
        let mut high = self.constant(false);
        let yes = self.constant(true);
        let no = self.constant(false);
        low = self.mux(mismatch.positive(), no.positive(), low.positive());
        high = self.mux(mismatch.positive(), yes.positive(), high.positive());
        low = self.mux(overflow.positive(), yes.positive(), low.positive());
        high = self.mux(overflow.positive(), yes.positive(), high.positive());
        low = self.mux(corrupt.positive(), no.positive(), low.positive());
        high = self.mux(corrupt.positive(), yes.positive(), high.positive());
        [low, high]
    }

    /// Independent priority-tree output circuit. Branches are applied from
    /// last to first with muxes, so earlier public guards override later ones.
    fn validator_priority_code(
        &mut self,
        dt: u64,
        input: &Inputs,
        suppress_overflow_error: bool,
    ) -> [Var; 2] {
        let (product, mut overflow): ([Var; 64], Var) = if dt == 0 || dt > HARD_MAX_DELTA_NS {
            (
                std::array::from_fn(|_| self.constant(false)),
                self.constant(false),
            )
        } else {
            let wide = self.multiply_constant_shift_add(&input.tick, dt);
            let high = self.any(&wide[64..].iter().map(|v| v.positive()).collect::<Vec<_>>());
            (std::array::from_fn(|i| wide[i]), high)
        };
        if suppress_overflow_error {
            overflow = self.constant(false);
        }
        self.result_from_cutpoints_priority(dt, input, &product, overflow)
    }

    fn less_than_const(&mut self, bits: &[Var], value: u128) -> Var {
        if value == 0 {
            return self.constant(false);
        }
        let ge = self.ge_const(bits, value);
        let not_ge = self.fresh();
        self.cnf.add_clause(&[ge.positive(), not_ge.positive()]);
        self.cnf.add_clause(&[ge.negative(), not_ge.negative()]);
        not_ge
    }

    fn inputs(&mut self) -> Inputs {
        Inputs {
            tick: self.input(),
            elapsed: self.input(),
            accumulator: self.input(),
            steps: self.input(),
            max_delta: self.input(),
        }
    }
}

fn cell_template(mode: AddMode) -> (Cnf, CellReceipt, [Var; 3]) {
    let mut c = Circuit::new();
    let inputs: [Var; 3] = c.input();
    match mode {
        AddMode::PropagateGenerate => {
            c.full_adder_propagate(inputs[0], inputs[1], inputs[2]);
        }
        AddMode::Majority => {
            c.full_adder_majority(inputs[0], inputs[1], inputs[2]);
        }
    }
    (c.cnf, c.cells[0].clone(), inputs)
}

fn certified_cell_template(mode: AddMode) -> (Cnf, CellReceipt, [Var; 3]) {
    // The cell certificate test proves these actual helper clauses against an
    // independent eight-row arithmetic oracle before receipts use this template.
    cell_template(mode)
}

fn remap_cell_literal(
    canonical: Lit,
    actual: &CellReceipt,
    canonical_cell: &CellReceipt,
) -> Result<Lit, String> {
    let index = canonical.var().index();
    let mapped = if index < 3 {
        [actual.a, actual.b, actual.cin][index]
    } else {
        let canonical_internal = canonical_cell
            .allocated
            .get(index - 3)
            .ok_or_else(|| format!("canonical cell variable {index} is out of receipt"))?;
        let offset = canonical_cell
            .allocated
            .iter()
            .position(|var| var == canonical_internal)
            .expect("found canonical cell variable")
            .min(actual.allocated.len());
        *actual
            .allocated
            .get(offset)
            .ok_or_else(|| format!("actual cell has no internal variable at {offset}"))?
    };
    Ok(mapped.lit(canonical.is_negated()))
}

fn verify_cell_clause_instance(
    network: &Cnf,
    actual: &CellReceipt,
    template_cnf: &Cnf,
    template: &CellReceipt,
) -> Result<(), String> {
    if actual.mode != template.mode || actual.allocated.len() != template.allocated.len() {
        return Err("cell mode or allocated-variable count differs from certified template".into());
    }
    if actual.clause_end < actual.clause_start
        || actual.clause_end > network.clauses.len()
        || actual.clause_end - actual.clause_start != template_cnf.clauses.len()
    {
        return Err("actual cell clause range does not match certified template size".into());
    }
    let actual_clauses = &network.clauses[actual.clause_start..actual.clause_end];
    for (index, clause) in template_cnf.clauses.iter().enumerate() {
        let mapped = clause
            .iter()
            .map(|lit| remap_cell_literal(*lit, actual, template))
            .collect::<Result<Vec<_>, _>>()?;
        if actual_clauses[index] != mapped {
            return Err(format!(
                "actual cell clause {index} differs from certified template"
            ));
        }
    }
    let sum_index = template
        .sum
        .index()
        .checked_sub(3)
        .ok_or("sum aliases a cell input")?;
    let cout_index = template
        .cout
        .index()
        .checked_sub(3)
        .ok_or("carry aliases a cell input")?;
    if actual.allocated.get(sum_index) != Some(&actual.sum)
        || actual.allocated.get(cout_index) != Some(&actual.cout)
    {
        return Err("actual cell sum/carry output wires differ from certified template".into());
    }
    Ok(())
}

fn verify_addition_receipt(
    network: &Cnf,
    addition: &AdditionReceipt,
    template_cnf: &Cnf,
    template: &CellReceipt,
) -> Result<(), String> {
    if addition.left.len() != addition.right.len()
        || addition.sum.len() != addition.left.len() + 1
        || addition.cells.len() != addition.left.len()
        || addition.cells.iter().any(|cell| cell.mode != addition.mode)
    {
        return Err("addition width or cell mode receipt is inconsistent".into());
    }
    if !network
        .clauses
        .contains(&vec![addition.carry_zero.negative()])
    {
        return Err("ripple addition initial carry is not pinned false".into());
    }
    for bit in 0..addition.left.len() {
        let cell = &addition.cells[bit];
        let expected_cin = if bit == 0 {
            addition.carry_zero
        } else {
            addition.cells[bit - 1].cout
        };
        if cell.a != addition.left[bit]
            || cell.b != addition.right[bit]
            || cell.cin != expected_cin
            || cell.sum != addition.sum[bit]
        {
            return Err(format!(
                "ripple adder input/carry/sum wiring differs at bit {bit}"
            ));
        }
        verify_cell_clause_instance(network, cell, template_cnf, template)?;
    }
    if addition.sum.last() != addition.cells.last().map(|cell| &cell.cout) {
        return Err("ripple final carry output is not the last cell carry".into());
    }
    Ok(())
}

fn verify_multiplier_receipt(
    circuit: &Circuit,
    receipt: &MultiplicationReceipt,
    returned: &[Var; 128],
    additions: &[AdditionReceipt],
    template_cnf: &Cnf,
    template: &CellReceipt,
) -> Result<(), String> {
    if receipt.output != returned.to_vec()
        || receipt.initial_acc.len() != WIDE_BITS
        || receipt.output.len() != WIDE_BITS
    {
        return Err("multiplier returned-output or word-width receipt is inconsistent".into());
    }
    for bit in &receipt.initial_acc {
        if !circuit.cnf.clauses.contains(&vec![bit.negative()]) {
            return Err("multiplier initial accumulator is not pinned to zero".into());
        }
    }
    let expected_shifts: Vec<_> = (0..64)
        .filter(|shift| (receipt.dt >> shift) & 1 != 0)
        .collect();
    let actual_shifts: Vec<_> = receipt.stages.iter().map(|stage| stage.shift).collect();
    if actual_shifts != expected_shifts {
        return Err("multiplier stages do not exactly match set constant bits".into());
    }
    let mut acc = receipt.initial_acc.clone();
    let mut partial_factor = 0_u128;
    for stage in &receipt.stages {
        if stage.acc_before != acc || stage.term.len() != WIDE_BITS {
            return Err(format!(
                "stage {} accumulator input or term width is wrong",
                stage.shift
            ));
        }
        for output_bit in 0..WIDE_BITS {
            if output_bit >= stage.shift && output_bit - stage.shift < U64_BITS {
                if stage.term[output_bit] != receipt.tick[output_bit - stage.shift] {
                    return Err(format!(
                        "stage {} shifted input wiring wrong at bit {output_bit}",
                        stage.shift
                    ));
                }
            } else if !circuit
                .cnf
                .clauses
                .contains(&vec![stage.term[output_bit].negative()])
            {
                return Err(format!(
                    "stage {} zero padding wrong at bit {output_bit}",
                    stage.shift
                ));
            }
        }
        partial_factor = partial_factor
            .checked_add(1_u128 << stage.shift)
            .ok_or("partial multiplier factor overflow")?;
        let _max_stage_product = (u64::MAX as u128)
            .checked_mul(partial_factor)
            .ok_or("partial multiplier bound exceeds u128")?;
        let addition = additions
            .get(stage.addition_index)
            .ok_or_else(|| format!("stage {} refers to a missing addition", stage.shift))?;
        if addition.mode != receipt.mode
            || addition.left != stage.acc_before
            || addition.right != stage.term
            || addition.sum[..WIDE_BITS] != stage.acc_after
        {
            return Err(format!(
                "stage {} actual addition operands/result do not match receipt",
                stage.shift
            ));
        }
        verify_addition_receipt(&circuit.cnf, addition, template_cnf, template)?;
        acc = stage.acc_after.clone();
    }
    if partial_factor != receipt.dt as u128 || acc != receipt.output {
        return Err("multiplier final factor/output receipt mismatch".into());
    }
    Ok(())
}

fn encode_miter(dt: u64, mutant: Mutant) -> (Cnf, Inputs, [Var; 2], [Var; 2]) {
    let mut c = Circuit::new();
    let input = c.inputs();
    let left = c.validator_priority_code(dt, &input, false);
    let right = c.validator_code(dt, &input, false, mutant);
    let mismatch = c.mismatch(&left, &right);
    c.cnf.add_clause(&[mismatch.positive()]);
    (c.cnf, input, left, right)
}

/// Final result proof after two independently certified cutpoints:
/// low64(product) and product-overflow agree for the same full-width state.
/// The product and overflow wires remain symbolic here; their equality is a
/// separate lemma obligation discharged by the multiplier cell/wiring and
/// endpoint monotonicity checks below.
fn encode_cutpoint_result_miter(dt: u64) -> Cnf {
    let mut c = Circuit::new();
    let input = c.inputs();
    let product_left = c.input::<64>();
    let product_right = c.input::<64>();
    let overflow_left = c.fresh();
    let overflow_right = c.fresh();
    for (left, right) in product_left.iter().zip(&product_right) {
        let equal = c.xor(left.positive(), right.positive());
        c.cnf.add_clause(&[equal.negative()]);
    }
    let overflow_equal = c.xor(overflow_left.positive(), overflow_right.positive());
    c.cnf.add_clause(&[overflow_equal.negative()]);
    let reference = c.result_from_cutpoints_priority(dt, &input, &product_left, overflow_left);
    let candidate =
        c.result_from_cutpoints_predicate(dt, &input, &product_right, overflow_right, Mutant::None);
    let mismatch = c.mismatch(&reference, &candidate);
    c.cnf.add_clause(&[mismatch.positive()]);
    c.cnf
}

fn encode_full_width_comparator_miter(bound: u128) -> Cnf {
    let mut c = Circuit::new();
    let bits: [Var; 64] = c.input();
    let reference = c.ge_const(&bits, bound);
    let candidate = c.ge_const_first_difference(&bits, bound);
    let mismatch = c.xor(reference.positive(), candidate.positive());
    c.cnf.add_clause(&[mismatch.positive()]);
    c.cnf
}

fn check_unsat(cnf: &Cnf, label: &str) {
    let formula = write_dimacs(cnf);
    let started = Instant::now();
    let mut solver = Solver::from_cnf_with_proof(cnf);
    solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
    solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
    let verdict = solver.solve();
    let stats = solver.stats();
    eprintln!(
        "GBL003B-SAT-QUERY {label} vars={} clauses={} verdict={verdict:?} conflicts={} work={} conflicts_limit={} work_limit={} elapsed_ms={}",
        cnf.num_vars,
        cnf.clauses.len(),
        stats.conflicts,
        stats.work,
        SOLVER_CONFLICT_LIMIT,
        SOLVER_WORK_LIMIT,
        started.elapsed().as_millis()
    );
    assert_eq!(
        verdict,
        SolveResult::Unsat,
        "{label}: UNKNOWN/resource failure is not UNSAT"
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
    .unwrap_or_else(|e| panic!("{label}: DRAT rejected: {e:?}"));
    eprintln!(
        "GBL003B-DRAT {label} accepted=true proof_bytes={} lemmas={} rat_lemmas={} deletions={} work={} work_limit={DRAT_WORK_LIMIT}",
        proof.len(), checked.lemmas, checked.rat_lemmas, checked.deletions, checked.work
    );
}

fn forced_gate_value(cnf: &Cnf, input: &[Var], out: Var, row: usize) -> bool {
    let mut query = cnf.clone();
    for (bit, var) in input.iter().enumerate() {
        query.add_clause(&[var.lit(((row >> bit) & 1) == 0)]);
    }
    let mut solver = Solver::from_cnf(&query);
    solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
    solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
    assert_eq!(solver.solve(), SolveResult::Sat);
    assert!(query.is_satisfied_by(solver.model()));
    solver.model()[out.index()]
}

fn forced_word_value(cnf: &Cnf, input: &[Var], output: &[Var], value: u64) -> u128 {
    let mut query = cnf.clone();
    for (bit, var) in input.iter().enumerate() {
        query.add_clause(&[var.lit(((value >> bit) & 1) == 0)]);
    }
    let mut solver = Solver::from_cnf(&query);
    solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
    solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
    assert_eq!(solver.solve(), SolveResult::Sat);
    assert!(query.is_satisfied_by(solver.model()));
    output.iter().enumerate().fold(0_u128, |word, (bit, var)| {
        word | ((solver.model()[var.index()] as u128) << bit)
    })
}

/// Positive SAT checks against the actual two constant-product networks at the
/// representability endpoints. Q+1 is omitted only for dt=1 because it is not a
/// representable u64 tick.
fn verify_fixed_product_endpoints(dt: u64) {
    let q = u64::MAX / dt;
    let mut points = vec![0, 1, q, u64::MAX];
    if let Some(q1) = q.checked_add(1) {
        points.push(q1);
    }
    points.sort_unstable();
    points.dedup();

    let mut threshold = Circuit::new();
    let threshold_tick: [Var; 64] = threshold.input();
    let (threshold_low, threshold_overflow) = threshold.threshold_product(&threshold_tick, dt);
    let threshold_receipt = threshold.multiplications[0].clone();
    let (majority_template_cnf, majority_template_cell, _) =
        certified_cell_template(AddMode::Majority);
    let threshold_returned: [Var; 128] = threshold_receipt
        .output
        .clone()
        .try_into()
        .expect("threshold multiplier receipt has 128 output wires");
    verify_multiplier_receipt(
        &threshold,
        &threshold_receipt,
        &threshold_returned,
        &threshold.additions,
        &majority_template_cnf,
        &majority_template_cell,
    )
    .expect("actual majority-carry threshold network matches its certified wire transcript");
    assert_eq!(threshold_low.as_slice(), &threshold_returned[..64]);

    let mut wide = Circuit::new();
    let wide_tick: [Var; 64] = wide.input();
    let wide_product = wide.multiply_constant_shift_add(&wide_tick, dt);
    let wide_receipt = wide.multiplications[0].clone();
    let (pg_template_cnf, pg_template_cell, _) =
        certified_cell_template(AddMode::PropagateGenerate);
    verify_multiplier_receipt(
        &wide,
        &wide_receipt,
        &wide_product,
        &wide.additions,
        &pg_template_cnf,
        &pg_template_cell,
    )
    .expect("actual propagate-generate network matches its certified wire transcript");

    let mut missing_shift = threshold_receipt.clone();
    if !missing_shift.stages.is_empty() {
        missing_shift.stages.remove(0);
        assert!(
            verify_multiplier_receipt(
                &threshold,
                &missing_shift,
                &threshold_returned,
                &threshold.additions,
                &majority_template_cnf,
                &majority_template_cell,
            )
            .is_err(),
            "wire checker rejects an omitted shift stage"
        );
    }
    let mut broken_additions = threshold.additions.clone();
    broken_additions[0].cells[0].cin = broken_additions[0].cells[0].a;
    assert!(
        verify_multiplier_receipt(
            &threshold,
            &threshold_receipt,
            &threshold_returned,
            &broken_additions,
            &majority_template_cnf,
            &majority_template_cell,
        )
        .is_err(),
        "wire checker rejects a corrupted carry-chain receipt"
    );
    for &tick in &points {
        let product = (tick as u128) * (dt as u128);
        let overflow = product > u64::MAX as u128;
        let mut threshold_query = threshold.cnf.clone();
        threshold_query.add_clause(&[threshold_overflow.lit(!overflow)]);
        for (bit, var) in threshold_tick.iter().enumerate() {
            threshold_query.add_clause(&[var.lit(((tick >> bit) & 1) == 0)]);
        }
        let threshold_low_word = forced_word_value(&threshold_query, &[], &threshold_low, tick);
        let wide_word = forced_word_value(&wide.cnf, &wide_tick, &wide_product, tick);
        assert_eq!(threshold_low_word, product & u64::MAX as u128);
        assert_eq!(wide_word, product);
        assert_eq!(threshold_low_word, wide_word & u64::MAX as u128);
    }
    eprintln!(
        "GBL003B-SAT-ENDPOINTS dt={dt} q={q} points={points:?} fullwidth_tick64 threshold_and_wide=true"
    );
}

#[test]
fn boolean_gate_encodings_match_every_truth_table_row() {
    for width in 0..=2 {
        for row in 0..(1_usize << width) {
            let mut c = Circuit::new();
            let inputs: Vec<Var> = (0..width).map(|_| c.fresh()).collect();
            let terms: Vec<_> = inputs.iter().map(|v| v.positive()).collect();
            let all = c.all(&terms);
            let expected_all = width == 0 || (0..width).all(|i| (row >> i) & 1 != 0);
            assert_eq!(
                forced_gate_value(&c.cnf, &inputs, all, row),
                expected_all,
                "all width={width} row={row}"
            );

            let mut c = Circuit::new();
            let inputs: Vec<Var> = (0..width).map(|_| c.fresh()).collect();
            let terms: Vec<_> = inputs.iter().map(|v| v.positive()).collect();
            let any = c.any(&terms);
            let expected_any = (0..width).any(|i| (row >> i) & 1 != 0);
            assert_eq!(
                forced_gate_value(&c.cnf, &inputs, any, row),
                expected_any,
                "any width={width} row={row}"
            );
        }
    }
    for row in 0..4 {
        let mut c = Circuit::new();
        let a = c.fresh();
        let b = c.fresh();
        let xor = c.xor(a.positive(), b.positive());
        assert_eq!(
            forced_gate_value(&c.cnf, &[a, b], xor, row),
            (row & 1) != ((row >> 1) & 1)
        );
    }
    for row in 0..8 {
        let mut c = Circuit::new();
        let select = c.fresh();
        let yes = c.fresh();
        let no = c.fresh();
        let output = c.mux(select.positive(), yes.positive(), no.positive());
        let selected = if row & 1 != 0 {
            (row >> 1) & 1
        } else {
            (row >> 2) & 1
        };
        assert_eq!(
            forced_gate_value(&c.cnf, &[select, yes, no], output, row),
            selected != 0,
            "mux row={row}"
        );
    }
}

#[test]
fn constants_comparators_adders_and_fixed_products_match_concrete_arithmetic() {
    for value in [false, true] {
        let mut c = Circuit::new();
        let bit = c.constant(value);
        assert_eq!(forced_gate_value(&c.cnf, &[], bit, 0), value);
    }
    for value in 0..=16_u128 {
        let mut c = Circuit::new();
        let bits: [Var; 4] = c.input();
        let eq = c.eq_const(&bits, value);
        let ge = c.ge_const(&bits, value);
        for row in 0..16 {
            let got_eq = forced_gate_value(&c.cnf, &bits, eq, row);
            assert_eq!(got_eq, row as u128 == value, "eq row={row} value={value}");
            let got_ge = forced_gate_value(&c.cnf, &bits, ge, row);
            assert_eq!(got_ge, row as u128 >= value, "ge row={row} value={value}");
        }
    }
    for left in 0..16_u64 {
        for right in 0..16_u64 {
            let mut c = Circuit::new();
            let a: [Var; 4] = c.input();
            let b: [Var; 4] = c.input();
            let sum = c.add(&a, &b);
            let mut query = c.cnf.clone();
            for (i, var) in a.iter().enumerate() {
                query.add_clause(&[var.lit(((left >> i) & 1) == 0)]);
            }
            for (i, var) in b.iter().enumerate() {
                query.add_clause(&[var.lit(((right >> i) & 1) == 0)]);
            }
            let mut solver = Solver::from_cnf(&query);
            solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
            solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
            assert_eq!(solver.solve(), SolveResult::Sat);
            assert!(query.is_satisfied_by(solver.model()));
            let got = sum.iter().enumerate().fold(0_u64, |word, (i, v)| {
                word | ((solver.model()[v.index()] as u64) << i)
            });
            assert_eq!(got, left + right);
        }
    }
    for dt in [1_u64, 2, 3, 10_000_000_000] {
        let mut threshold = Circuit::new();
        let tick: [Var; 64] = threshold.input();
        let (threshold_low, threshold_overflow) = threshold.threshold_product(&tick, dt);
        let mut values = vec![0, 1, u64::MAX / dt, u64::MAX];
        if let Some(first_overflow) = (u64::MAX / dt).checked_add(1) {
            values.push(first_overflow);
        }
        values.sort_unstable();
        values.dedup();
        for value in values {
            let (cnf, outputs) = {
                let mut query = threshold.cnf.clone();
                let low = threshold_low;
                let overflow = threshold_overflow;
                query.add_clause(&[
                    overflow.lit(!((value as u128) * (dt as u128) > u64::MAX as u128))
                ]);
                for (bit, var) in tick.iter().enumerate() {
                    query.add_clause(&[var.lit(((value >> bit) & 1) == 0)]);
                }
                (query, low)
            };
            let expected = (value as u128) * (dt as u128);
            assert_eq!(
                forced_word_value(&cnf, &[], &outputs, value),
                expected & u64::MAX as u128
            );
        }

        let mut wide = Circuit::new();
        let tick: [Var; 64] = wide.input();
        let product = wide.multiply_constant_shift_add(&tick, dt);
        let mut values = vec![0, 1, u64::MAX / dt, u64::MAX];
        if let Some(first_overflow) = (u64::MAX / dt).checked_add(1) {
            values.push(first_overflow);
        }
        values.sort_unstable();
        values.dedup();
        for value in values {
            assert_eq!(
                forced_word_value(&wide.cnf, &tick, &product, value),
                (value as u128) * (dt as u128),
                "wide product dt={dt} tick={value}"
            );
        }
    }
}

#[test]
fn majority_and_propagate_generate_full_adder_cells_are_drat_equivalent() {
    let mut c = Circuit::new();
    let a = c.fresh();
    let b = c.fresh();
    let cin = c.fresh();
    let inputs = [a, b, cin];
    let (majority_sum, majority_carry) = c.full_adder_majority(a, b, cin);
    let (pg_sum, pg_carry) = c.full_adder_propagate(a, b, cin);
    let truth_sum = c.fresh();
    let truth_carry = c.fresh();
    for row in 0..8 {
        let values = [row & 1 != 0, row & 2 != 0, row & 4 != 0];
        let input_guard: Vec<_> = inputs
            .iter()
            .zip(values)
            .map(|(var, value)| var.lit(value))
            .collect();
        let total = values.iter().filter(|&&bit| bit).count();
        c.cnf.add_clause(&[
            input_guard[0],
            input_guard[1],
            input_guard[2],
            truth_sum.lit(total % 2 == 0),
        ]);
        c.cnf.add_clause(&[
            input_guard[0],
            input_guard[1],
            input_guard[2],
            truth_carry.lit(total < 2),
        ]);
    }
    let sum_diffs = [
        c.xor(majority_sum.positive(), truth_sum.positive()),
        c.xor(pg_sum.positive(), truth_sum.positive()),
    ];
    let carry_diffs = [
        c.xor(majority_carry.positive(), truth_carry.positive()),
        c.xor(pg_carry.positive(), truth_carry.positive()),
    ];
    let differences: Vec<_> = sum_diffs
        .iter()
        .chain(carry_diffs.iter())
        .map(|v| v.positive())
        .collect();
    let mismatch = c.any(&differences);
    c.cnf.add_clause(&[mismatch.positive()]);
    check_unsat(
        &c.cnf,
        "actual-full-adder-cells-vs-independent-eight-row-arithmetic",
    );
}

#[test]
fn every_selected_shift_term_matches_its_full_128_bit_wire_mapping() {
    for shift in 0..64 {
        let mut c = Circuit::new();
        let tick: [Var; 64] = c.input();
        let term = c.shifted_term(&tick, shift);
        for output_bit in 0..128 {
            if output_bit >= shift && output_bit - shift < 64 {
                assert_eq!(
                    term[output_bit],
                    tick[output_bit - shift],
                    "shift={shift} output_bit={output_bit} must alias source tick bit"
                );
            } else {
                assert!(
                    c.cnf
                        .clauses
                        .iter()
                        .any(|clause| clause == &[term[output_bit].negative()]),
                    "shift={shift} output_bit={output_bit} must be pinned zero"
                );
            }
        }
    }
}

fn public_result(
    dt: u64,
    tick: u64,
    elapsed: u64,
    accumulator: u64,
    steps: u32,
    max_delta: u64,
) -> Result<(), ClockError> {
    validate_state(&state_from_values(
        dt,
        tick,
        elapsed,
        accumulator,
        steps,
        max_delta,
    ))
}

fn state_from_values(
    dt: u64,
    tick: u64,
    elapsed: u64,
    accumulator: u64,
    steps: u32,
    max_delta: u64,
) -> GameClockState {
    GameClockState {
        tick_count: tick,
        elapsed_sim_ns: elapsed,
        accumulator_ns: accumulator,
        fixed_dt_ns: dt,
        max_frame_steps: steps,
        max_frame_delta_ns: max_delta,
        frame_count: u64::MAX,
        elapsed_wall_ns: 0,
        total_dropped_ns: 1,
        spiral_clamped_frames: 2,
    }
}

fn oracle_result(
    dt: u64,
    tick: u64,
    elapsed: u64,
    accumulator: u64,
    steps: u32,
    max_delta: u64,
) -> Result<(), ClockError> {
    if dt == 0 {
        return Err(ClockError::ZeroFixedTimestep);
    }
    if dt > HARD_MAX_DELTA_NS
        || steps == 0
        || steps > HARD_MAX_FRAME_STEPS
        || max_delta < dt
        || max_delta > HARD_MAX_DELTA_NS
        || accumulator >= dt
    {
        return Err(ClockError::CorruptedStateInvariant);
    }
    let product = (tick as u128) * (dt as u128);
    if product > u64::MAX as u128 {
        return Err(ClockError::TimestampOverflow);
    }
    if elapsed != product as u64 {
        return Err(ClockError::CorruptedStateInvariant);
    }
    Ok(())
}

/// Deliberately broken control: ignore product overflow and compare the wrapped
/// low 64 bits. It is evaluated on the decoded, unchanged SAT witness.
fn wrapping_product_mutant(
    dt: u64,
    tick: u64,
    elapsed: u64,
    accumulator: u64,
    steps: u32,
    max_delta: u64,
) -> Result<(), ClockError> {
    if dt == 0 {
        return Err(ClockError::ZeroFixedTimestep);
    }
    if dt > HARD_MAX_DELTA_NS
        || steps == 0
        || steps > HARD_MAX_FRAME_STEPS
        || max_delta < dt
        || max_delta > HARD_MAX_DELTA_NS
        || accumulator >= dt
    {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if elapsed != tick.wrapping_mul(dt) {
        return Err(ClockError::CorruptedStateInvariant);
    }
    Ok(())
}

fn mutant_result(
    mutant: Mutant,
    dt: u64,
    tick: u64,
    elapsed: u64,
    accumulator: u64,
    steps: u32,
    max_delta: u64,
) -> Result<(), ClockError> {
    match mutant {
        Mutant::None => oracle_result(dt, tick, elapsed, accumulator, steps, max_delta),
        Mutant::SuppressOverflow => {
            wrapping_product_mutant(dt, tick, elapsed, accumulator, steps, max_delta)
        }
        Mutant::IgnoreElapsedMismatch => {
            if dt == 0 {
                return Err(ClockError::ZeroFixedTimestep);
            }
            if dt > HARD_MAX_DELTA_NS
                || steps == 0
                || steps > HARD_MAX_FRAME_STEPS
                || max_delta < dt
                || max_delta > HARD_MAX_DELTA_NS
                || accumulator >= dt
            {
                return Err(ClockError::CorruptedStateInvariant);
            }
            if (tick as u128) * (dt as u128) > u64::MAX as u128 {
                return Err(ClockError::TimestampOverflow);
            }
            Ok(())
        }
        Mutant::AlwaysAccept => Ok(()),
        Mutant::AlwaysError => Err(ClockError::CorruptedStateInvariant),
        Mutant::OverflowBeforeInvariant => {
            if dt == 0 {
                Err(ClockError::ZeroFixedTimestep)
            } else if (tick as u128) * (dt as u128) > u64::MAX as u128 {
                Err(ClockError::TimestampOverflow)
            } else {
                oracle_result(dt, tick, elapsed, accumulator, steps, max_delta)
            }
        }
    }
}

fn pin_word(cnf: &mut Cnf, variables: &[Var], value: u64) {
    for (bit, var) in variables.iter().enumerate() {
        cnf.add_clause(&[var.lit(((value >> bit) & 1) == 0)]);
    }
}

fn model_u64(model: &[bool], bits: &[Var]) -> u64 {
    bits.iter().enumerate().fold(0_u64, |word, (bit, var)| {
        word | ((model[var.index()] as u64) << bit)
    })
}

fn model_u32(model: &[bool], bits: &[Var]) -> u32 {
    bits.iter().enumerate().fold(0_u32, |word, (bit, var)| {
        word | ((model[var.index()] as u32) << bit)
    })
}

fn result_code(model: &[bool], bits: &[Var; 2]) -> u8 {
    (model[bits[0].index()] as u8) | ((model[bits[1].index()] as u8) << 1)
}

fn code_result(code: u8) -> Result<(), ClockError> {
    match code {
        0 => Ok(()),
        1 => Err(ClockError::ZeroFixedTimestep),
        2 => Err(ClockError::CorruptedStateInvariant),
        3 => Err(ClockError::TimestampOverflow),
        _ => panic!("validator circuit emitted invalid two-bit result code {code}"),
    }
}

#[test]
fn sat_mutants_produce_unchanged_witnesses_replayed_by_rust_oracle_and_warrant() {
    let cases = [
        (Mutant::SuppressOverflow, u64::MAX / 2 + 1, 0, 0, 1, 2),
        (Mutant::IgnoreElapsedMismatch, 0, 1, 0, 1, 2),
        (Mutant::AlwaysAccept, 0, 0, 2, 1, 2),
        (Mutant::AlwaysError, 0, 0, 0, 1, 2),
        (Mutant::OverflowBeforeInvariant, u64::MAX, 0, 0, 0, 2),
    ];
    for (mutant_kind, tick, elapsed, accumulator, steps, max_delta) in cases {
        let dt = 2_u64;
        let (mut cnf, inputs, exact, mutant) = encode_miter(dt, mutant_kind);
        pin_word(&mut cnf, &inputs.tick, tick);
        pin_word(&mut cnf, &inputs.elapsed, elapsed);
        pin_word(&mut cnf, &inputs.accumulator, accumulator);
        pin_word(&mut cnf, &inputs.steps, steps as u64);
        pin_word(&mut cnf, &inputs.max_delta, max_delta);
        let mut solver = Solver::from_cnf(&cnf);
        solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
        solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
        assert_eq!(
            solver.solve(),
            SolveResult::Sat,
            "{mutant_kind:?} must have a witness"
        );
        let model = solver.model();
        assert!(
            cnf.is_satisfied_by(model),
            "SAT witness satisfies the exact miter CNF"
        );
        let exact_code = result_code(model, &exact);
        let mutant_code = result_code(model, &mutant);
        assert_ne!(
            exact_code, mutant_code,
            "{mutant_kind:?} changes the output code"
        );

        let decoded_tick = model_u64(model, &inputs.tick);
        let decoded_elapsed = model_u64(model, &inputs.elapsed);
        let decoded_accumulator = model_u64(model, &inputs.accumulator);
        let decoded_steps = model_u32(model, &inputs.steps);
        let decoded_max_delta = model_u64(model, &inputs.max_delta);
        let exact_public = public_result(
            dt,
            decoded_tick,
            decoded_elapsed,
            decoded_accumulator,
            decoded_steps,
            decoded_max_delta,
        );
        let independent = oracle_result(
            dt,
            decoded_tick,
            decoded_elapsed,
            decoded_accumulator,
            decoded_steps,
            decoded_max_delta,
        );
        let actual_mutant = mutant_result(
            mutant_kind,
            dt,
            decoded_tick,
            decoded_elapsed,
            decoded_accumulator,
            decoded_steps,
            decoded_max_delta,
        );
        assert_eq!(
            code_result(exact_code),
            exact_public,
            "{mutant_kind:?} exact output matches Rust"
        );
        assert_eq!(
            code_result(mutant_code),
            actual_mutant,
            "{mutant_kind:?} broken predicate matches candidate circuit"
        );
        assert_eq!(
            independent, exact_public,
            "u128 oracle agrees on unchanged decoded witness"
        );
        let warrant = warrant_state::evaluate_validator(state_from_values(
            dt,
            decoded_tick,
            decoded_elapsed,
            decoded_accumulator,
            decoded_steps,
            decoded_max_delta,
        ))
        .expect("unchanged decoded SAT witness evaluates in parsed Warrant model");
        assert_eq!(warrant, warrant_state::expected_value(exact_public));
        assert_ne!(
            actual_mutant, exact_public,
            "{mutant_kind:?} changes the actual exact result"
        );
        eprintln!(
            "GBL003B-SAT-MUTANT mutant={mutant_kind:?} dt={dt} tick={decoded_tick} elapsed={decoded_elapsed} acc={decoded_accumulator} steps={decoded_steps} max_delta={decoded_max_delta} exact={exact_public:?} oracle={independent:?} mutant_result={actual_mutant:?} conflicts={} work={}",
            solver.stats().conflicts,
            solver.stats().work
        );
    }
}

#[test]
fn fixed_parameter_result_miters_are_drat_checked() {
    // `dt = 1` is included for exact-result coverage; it cannot overflow.
    for dt in [1, 2, 3] {
        let (cnf, _, _, _) = encode_miter(dt, Mutant::None);
        check_unsat(&cnf, &format!("dt={dt}"));
        eprintln!("GBL003B-SAT dt={dt} widths=tick64,elapsed64,accumulator64,steps32,max_delta64 mismatch=UNSAT drat=checked hand_encoded=true");
    }
}

#[test]
fn compositional_full_width_result_miters_cover_all_required_valid_dt_values() {
    // Discharge the actual network lemma suite inside the composition rather
    // than relying on a separately selected test binary target.
    majority_and_propagate_generate_full_adder_cells_are_drat_equivalent();
    every_selected_shift_term_matches_its_full_128_bit_wire_mapping();
    for dt in [1_u64, 2, 3, 10_000_000_000] {
        let threshold = u64::MAX / dt;
        let at_threshold = (threshold as u128)
            .checked_mul(dt as u128)
            .expect("Q*dt is within u128");
        assert!(at_threshold <= u64::MAX as u128);
        let first_overflow = threshold.checked_add(1).map(|tick| {
            (tick as u128)
                .checked_mul(dt as u128)
                .expect("(Q+1)*dt is within u128")
        });
        if let Some(first_overflow) = first_overflow {
            assert!(first_overflow > u64::MAX as u128);
        } else {
            assert_eq!(dt, 1, "only dt=1 has no representable Q+1 tick");
            assert_eq!(at_threshold, u64::MAX as u128);
        }
        verify_fixed_product_endpoints(dt);
        let overflow_bound = (threshold as u128) + 1;
        let comparator = encode_full_width_comparator_miter(overflow_bound);
        check_unsat(&comparator, &format!("full64-threshold-comparator-dt={dt}"));

        // Arithmetic trust base: actual product networks are sums of the exact
        // shifted terms checked by `every_selected_shift_term...`; every actual
        // ripple cell is covered by the DRAT cell identity test. For each actual
        // stage the checked `MAX * partial_factor` bound prevents discarded
        // 128-bit carry. Therefore P(x)=dt*x is exact and monotone for all
        // 64-bit x; the endpoint inequalities prove high(P(x)) iff x>Q.
        let cutpoint_miter = encode_cutpoint_result_miter(dt);
        check_unsat(&cutpoint_miter, &format!("result-cutpoints-dt={dt}"));
        eprintln!(
            "GBL003B-SAT-COMPOSITION dt={dt} q={threshold} p_q={at_threshold} p_q1={first_overflow:?} full_width_tick64_elapsed64_accumulator64_steps32_delta64 result_miter=UNSAT cutpoints=product_low64,overflow DRAT=checked"
        );
    }
}

#[test]
fn fixed_invalid_timestep_controls_replay_public_api() {
    for (dt, expected) in [
        (0, ClockError::ZeroFixedTimestep),
        (10_000_000_001, ClockError::CorruptedStateInvariant),
    ] {
        let (cnf, inputs, exact, model) = encode_miter(dt, Mutant::None);
        check_unsat(&cnf, &format!("invalid-dt={dt}"));
        let dt_code = if dt == 0 { 1 } else { 2 };
        let mut query = cnf.clone();
        query.clauses.pop(); // remove the miter's final mismatch requirement for reachability
        query.add_clause(&[exact[0].lit(dt_code & 1 == 0)]);
        query.add_clause(&[exact[1].lit(dt_code & 2 == 0)]);
        let mut solver = Solver::from_cnf(&query);
        solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
        solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
        assert_eq!(
            solver.solve(),
            SolveResult::Sat,
            "fixed invalid-dt exact branch is reachable"
        );
        assert!(query.is_satisfied_by(solver.model()));
        assert_eq!(result_code(solver.model(), &exact), dt_code);
        assert_eq!(result_code(solver.model(), &model), dt_code);

        let assignment = solver.model();
        let tick = model_u64(assignment, &inputs.tick);
        let elapsed = model_u64(assignment, &inputs.elapsed);
        let accumulator = model_u64(assignment, &inputs.accumulator);
        let steps = model_u32(assignment, &inputs.steps);
        let max_delta = model_u64(assignment, &inputs.max_delta);
        let public = public_result(dt, tick, elapsed, accumulator, steps, max_delta);
        let oracle = oracle_result(dt, tick, elapsed, accumulator, steps, max_delta);
        assert_eq!(code_result(result_code(assignment, &exact)), public);
        assert_eq!(code_result(result_code(assignment, &model)), public);
        let warrant = warrant_state::evaluate_validator(state_from_values(
            dt,
            tick,
            elapsed,
            accumulator,
            steps,
            max_delta,
        ))
        .expect("decoded invalid-dt SAT assignment evaluates in Warrant");
        assert_eq!(warrant, warrant_state::expected_value(public));
        assert_eq!(public, Err(expected));
        assert_eq!(
            oracle, public,
            "independent oracle agrees on unchanged invalid-dt replay"
        );
    }
}

#[test]
fn sat_branch_vectors_match_rust_oracle_and_parsed_warrant_model() {
    let cases = [
        ("accepted", 2, 3, 6, 1, 8, 10, Ok(())),
        (
            "elapsed-mismatch",
            2,
            3,
            7,
            1,
            8,
            10,
            Err(ClockError::CorruptedStateInvariant),
        ),
        (
            "accumulator-invalid",
            2,
            3,
            6,
            2,
            8,
            10,
            Err(ClockError::CorruptedStateInvariant),
        ),
        (
            "overflow",
            2,
            u64::MAX / 2 + 1,
            0,
            0,
            1,
            2,
            Err(ClockError::TimestampOverflow),
        ),
        (
            "corrupt-before-overflow",
            2,
            u64::MAX / 2 + 1,
            0,
            0,
            0,
            2,
            Err(ClockError::CorruptedStateInvariant),
        ),
        (
            "zero-dt",
            0,
            u64::MAX,
            0,
            u64::MAX,
            0,
            0,
            Err(ClockError::ZeroFixedTimestep),
        ),
        (
            "dt-above-cap",
            10_000_000_001,
            0,
            0,
            0,
            0,
            0,
            Err(ClockError::CorruptedStateInvariant),
        ),
    ];
    for (label, dt, tick, elapsed, accumulator, steps, max_delta, expected) in cases {
        let (mut cnf, inputs, reference, candidate) = encode_miter(dt, Mutant::None);
        cnf.clauses.pop(); // Replace whole-domain disequality with this concrete row.
        pin_word(&mut cnf, &inputs.tick, tick);
        pin_word(&mut cnf, &inputs.elapsed, elapsed);
        pin_word(&mut cnf, &inputs.accumulator, accumulator);
        pin_word(&mut cnf, &inputs.steps, steps as u64);
        pin_word(&mut cnf, &inputs.max_delta, max_delta);
        let mut solver = Solver::from_cnf(&cnf);
        solver.set_conflict_budget(Some(SOLVER_CONFLICT_LIMIT));
        solver.set_work_budget(Some(SOLVER_WORK_LIMIT));
        assert_eq!(
            solver.solve(),
            SolveResult::Sat,
            "branch vector {label} is reachable"
        );
        let assignment = solver.model();
        assert!(cnf.is_satisfied_by(assignment));
        let public = public_result(dt, tick, elapsed, accumulator, steps, max_delta);
        let oracle = oracle_result(dt, tick, elapsed, accumulator, steps, max_delta);
        assert_eq!(public, expected, "public branch vector {label}");
        assert_eq!(oracle, public, "u128 oracle branch vector {label}");
        assert_eq!(code_result(result_code(assignment, &reference)), public);
        assert_eq!(code_result(result_code(assignment, &candidate)), public);
        let warrant = warrant_state::evaluate_validator(state_from_values(
            dt,
            tick,
            elapsed,
            accumulator,
            steps,
            max_delta,
        ))
        .expect("concrete SAT branch vector evaluates in parsed Warrant model");
        assert_eq!(warrant, warrant_state::expected_value(public));
        eprintln!("GBL003B-SAT-BRANCH case={label} dt={dt} public={public:?} oracle={oracle:?} warrant=equal sat_codes=equal");
    }
}
