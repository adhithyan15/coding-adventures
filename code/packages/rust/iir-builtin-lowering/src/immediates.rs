//! # immediates — materialize immediate value operands into `const` temporaries.
//!
//! ## Why this pass exists
//!
//! An IIR source operand is *either* a variable name or an immediate literal —
//! `Operand`'s own doc says so, and the frontends take it at their word. COBOL's
//! `ADD 7 TO R` lowers to a single
//!
//! ```text
//!   add  _acc0, 7  → _acc0 : i64
//! ```
//!
//! with the `7` inline, because writing it that way is both shorter and exactly
//! what the statement means.
//!
//! The **native, LLVM, VM and JIT** backends honour that contract: each has a
//! path for "this operand is a literal, fold it into the instruction". The
//! **JVM, CLR and WASM** backends do not — they are stack machines (or, for CLR,
//! a stack machine addressed through locals) whose lowering assumes every value
//! operand names a slot it can `iload`/`ldloc`/`local.get`. Handed a literal they
//! refuse the whole module:
//!
//! ```text
//!   JVM   InvalidOperand { detail: "add expects Var operands, got immediate" }
//!   CLR   InvalidOperand { detail: "cmp_eq src[1] must be a variable, got Some(Int(1))" }
//!   WASM  InvalidOperand { detail: "expected Var at src[1], got Int(1)" }
//! ```
//!
//! That mismatch cost 24 matrix cells across three backends and one language,
//! and it will cost more as frontends multiply: every future frontend has to
//! learn, by breaking, which half of the contract is real.
//!
//! ## Why a shared pass rather than three backend fixes
//!
//! Teaching each of the three backends to fold literals would be the other way
//! to close it, and is arguably what the contract asks for. It is also the same
//! work three times, in three instruction encoders, for every opcode family —
//! and it leaves the next backend to make the same choice again.
//!
//! Normalizing instead is one pass, deterministic, and language-agnostic: it
//! rewrites
//!
//! ```text
//!   add  _acc0, 7  → _acc0        ⇒   const 7 → __imm0 : i64
//!                                     add   _acc0, __imm0 → _acc0
//! ```
//!
//! so a stack backend sees only the shape it already handles, and no frontend
//! has to know which backend it is aimed at. `const` with a literal source is
//! the one form every backend lowers, because it is how a literal enters the
//! program at all.
//!
//! It runs **only on the JVM / CLR / WASM pipelines**, exactly like
//! `lower_box_unbox_to_runtime_calls` runs only on native/LLVM. The backends
//! that implement the full contract keep folding literals into instructions and
//! emit the tighter code.
//!
//! ## What counts as a value operand
//!
//! Only operands that name a *runtime value*. An immediate that is part of an
//! instruction's addressing — `field_load`'s field index, `jmp`'s target label,
//! `call_builtin`'s builtin name — is not a value and must stay inline, or the
//! instruction stops meaning what it meant. So the pass is opt-in per opcode
//! family rather than a blanket rewrite of every `Operand::Int` it can find:
//!
//! | family | value operands |
//! |--------|----------------|
//! | `is_arithmetic` — `add`/`sub`/`mul`/`div`/`mod` | `srcs[0]`, `srcs[1]` |
//! | `is_arithmetic` — `neg` | `srcs[0]` |
//! | `is_bitwise` — `and`/`or`/`xor`/`shl`/`shr` | `srcs[0]`, `srcs[1]` |
//! | `is_bitwise` — `not` | `srcs[0]` |
//! | `is_cmp` — every `cmp_*` | `srcs[0]`, `srcs[1]` |
//! | `mov` | `srcs[0]` |
//!
//! Anything else is left exactly as it was.
//!
//! ## The type the temporary gets
//!
//! For a **binary** op, not the instruction's `type_hint`: a comparison's hint
//! is `bool`, the *result* type, while the operands are the values being
//! compared. Emitting `const 1 : bool` for `cmp_eq x, 1` hands the backend a
//! boolean where it expects an integer — the same operand-width-versus-result-
//! width confusion that made BASIC's comparisons lower to `icmp i1` on LLVM. The
//! sibling operand's producer type is used instead, since it carries the width
//! the surrounding code agreed on.
//!
//! For a **unary** op (`mov`, `neg`, `not`) there is no sibling, and there the
//! `type_hint` *is* the operand type, so it is used. Defaulting to `i64` instead
//! would put a two-slot `long` local under an `i32`-model function's `INEG`, and
//! the JVM verifier rejects the class.
//!
//! Either way the candidate is adopted **only if the literal is representable in
//! it**. Every backend's `const` lowering narrows with a lossy `as` cast rather
//! than a checked conversion, so claiming `i32` for `5_000_000_000` would not
//! fail — it would quietly emit `705032704`. Introducing this pass must not
//! trade a loud refusal for a wrong answer, so an unrepresentable literal falls
//! back to its own kind (`Int → i64`, `Float → f64`, `Bool → bool`).

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::opcodes::{is_arithmetic, is_bitwise, is_cmp};
use interpreter_ir::source_loc::SourceLoc;
use interpreter_ir::IIRModule;
use std::collections::{HashMap, HashSet};

/// Which source indices of `op` name a runtime value (see the table above).
/// `None` for an opcode this pass does not touch.
fn value_operand_indices(op: &str) -> Option<&'static [usize]> {
    match op {
        // Unary: only `srcs[0]` is a value.
        "neg" | "not" | "mov" => Some(&[0]),
        // Binary arithmetic / bitwise / comparison.
        _ if is_arithmetic(op) || is_bitwise(op) || is_cmp(op) => Some(&[0, 1]),
        _ => None,
    }
}

/// Can `lit` be represented exactly in the machine type `ty`?
///
/// Adopting a narrower type than the literal needs is not a rounding detail —
/// every backend's `const` lowering narrows with a lossy `as` cast rather than a
/// checked conversion (`*v as i32` on the JVM, WASM and binary-CIL paths). So
/// claiming `i32` for `5_000_000_000` does not fail, it silently emits
/// `705032704` and the program computes a different answer than its source says.
/// Before this pass existed, that instruction shape was rejected outright, which
/// at least was loud; introducing the pass must not trade a loud refusal for a
/// quiet wrong answer.
fn fits(lit: &Operand, ty: &str) -> bool {
    match (lit, ty) {
        (Operand::Int(v), "i8") => i8::try_from(*v).is_ok(),
        (Operand::Int(v), "i16") => i16::try_from(*v).is_ok(),
        (Operand::Int(v), "i32") => i32::try_from(*v).is_ok(),
        (Operand::Int(_), "i64") => true,
        // An `f32` round-trip is exact only when the value survives the narrowing.
        (Operand::Float(v), "f32") => *v as f32 as f64 == *v,
        (Operand::Float(_), "f64") => true,
        (Operand::Bool(_), "bool") => true,
        // Anything else — an integer claiming a float type, a bool claiming a
        // width, a literal against `any`/`ref<…>` — is not a representable match.
        _ => false,
    }
}

/// The literal's own kind, which is always a faithful home for it.
fn own_type(lit: &Operand) -> &'static str {
    match lit {
        Operand::Float(_) => "f64",
        Operand::Bool(_) => "bool",
        _ => "i64",
    }
}

/// The type hint a materialized literal should carry.
///
/// The rule differs by arity, because the two cases have different reliable
/// sources:
///
/// * **Binary** (`add`, `cmp_eq`, …) — prefer the *sibling operand's* producer
///   type, which carries the width the surrounding code agreed on. Never the
///   instruction's `type_hint`: a comparison's hint is `bool`, describing its
///   result while the operands are what is being compared, so `const 1 : bool`
///   for `cmp_eq x, 1` hands the backend a boolean where it wants an integer.
/// * **Unary** (`mov`, `neg`, `not`) — there is no sibling, and here the
///   instruction's `type_hint` *is* the operand type, so use it. Falling back to
///   `i64` instead would put a two-slot `long` under an `i32`-model function's
///   `INEG`, and the JVM verifier rejects the class.
///
/// Either way the candidate is adopted only if the literal is representable in
/// it (see `fits`); otherwise the literal's own kind wins.
fn literal_type(
    lit: &Operand,
    candidate: Option<&str>,
    types: &HashMap<String, String>,
    sibling: Option<&Operand>,
) -> String {
    // A sibling variable's producer type, when there is one.
    let from_sibling = sibling.and_then(|s| match s {
        Operand::Var(name) => types.get(name.as_str()).map(String::as_str),
        _ => None,
    });
    for ty in [from_sibling, candidate].into_iter().flatten() {
        if fits(lit, ty) {
            return ty.to_string();
        }
    }
    own_type(lit).to_string()
}

/// Is this operand an immediate literal (rather than a variable or a name)?
fn is_literal(op: &Operand) -> bool {
    matches!(op, Operand::Int(_) | Operand::Float(_) | Operand::Bool(_))
}

/// Map each destination (and parameter) to the type hint it was produced with.
fn producer_types(f: &IIRFunction) -> HashMap<String, String> {
    let mut m: HashMap<String, String> = HashMap::new();
    for (name, ty) in &f.params {
        m.insert(name.clone(), ty.clone());
    }
    for instr in &f.instructions {
        if let Some(dest) = &instr.dest {
            m.insert(dest.clone(), instr.type_hint.clone());
        }
    }
    m
}

/// Every name already in use in `f` — parameters, destinations, and operands.
/// A generated temporary must avoid all of them, not merely avoid its siblings.
fn names_in_use(f: &IIRFunction) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    for (name, _) in &f.params {
        seen.insert(name.clone());
    }
    for instr in &f.instructions {
        if let Some(d) = &instr.dest {
            seen.insert(d.clone());
        }
        for s in &instr.srcs {
            if let Operand::Var(v) = s {
                seen.insert(v.clone());
            }
        }
    }
    seen
}

/// Rewrite one function's immediate value operands into `const` temporaries.
pub fn materialize_immediate_operands_function(f: &mut IIRFunction) {
    let types = producer_types(f);
    // A monotonic counter keeps the generated names distinct from each other; the
    // in-use set keeps them distinct from names that were already here. The
    // counter alone is not enough — nothing stops a frontend from emitting a
    // variable called `__imm1_add`, and silently overwriting it would make every
    // later read of that variable return this literal, with no diagnostic
    // anywhere.
    let taken = names_in_use(f);
    // `source_map` is documented as indexed in lockstep with `instructions`.
    // Inserting instructions without extending it shifts every later entry, so
    // rebuild it alongside when the function carries one.
    let had_source_map = !f.source_map.is_empty();
    let old_map = std::mem::take(&mut f.source_map);
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len());
    let mut new_map = Vec::with_capacity(old_map.len());
    let mut counter = 0usize;

    for (idx, mut instr) in old.into_iter().enumerate() {
        // The source location of the instruction being rewritten, reused for the
        // `const`s materialized for it — they are that statement's operands, so
        // attributing them to it is the honest answer.
        let loc = old_map.get(idx).cloned();
        let Some(indices) = value_operand_indices(&instr.op) else {
            out.push(instr);
            if had_source_map {
                new_map.push(loc.unwrap_or(SourceLoc::SYNTHETIC));
            }
            continue;
        };
        for &i in indices {
            let Some(operand) = instr.srcs.get(i) else { continue };
            if !is_literal(operand) {
                continue;
            }
            let literal = operand.clone();
            // Binary ops infer width from the other operand; unary ops have no
            // sibling and take the instruction's own hint (see `literal_type`).
            let sibling = indices
                .iter()
                .find(|&&j| j != i)
                .and_then(|&j| instr.srcs.get(j))
                .cloned();
            let ty = literal_type(&literal, Some(instr.type_hint.as_str()), &types, sibling.as_ref());
            let tmp = loop {
                counter += 1;
                let candidate = format!("__imm{counter}_{}", instr.op);
                if !taken.contains(&candidate) {
                    break candidate;
                }
            };
            out.push(IIRInstr::new("const", Some(tmp.clone()), vec![literal], &ty));
            if had_source_map {
                new_map.push(loc.unwrap_or(SourceLoc::SYNTHETIC));
            }
            instr.srcs[i] = Operand::Var(tmp);
        }
        out.push(instr);
        if had_source_map {
            new_map.push(loc.unwrap_or(SourceLoc::SYNTHETIC));
        }
    }

    f.instructions = out;
    f.source_map = new_map;
}

/// Module-level entry point: materialize immediate value operands in every
/// function, so the stack backends (JVM / CLR / WASM) see only variable
/// operands. A no-op for a module whose frontend already materialized them.
pub fn materialize_immediate_operands(module: &mut IIRModule) {
    for f in &mut module.functions {
        materialize_immediate_operands_function(f);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn func(instrs: Vec<IIRInstr>) -> IIRFunction {
        IIRFunction::new("main", vec![], "i64", instrs)
    }

    fn ops(f: &IIRFunction) -> Vec<String> {
        f.instructions.iter().map(|i| i.op.clone()).collect()
    }

    /// `IIRInstr` has no `PartialEq`, so compare a rendered form when a test
    /// needs to assert that nothing changed.
    fn render(f: &IIRFunction) -> Vec<String> {
        f.instructions
            .iter()
            .map(|i| format!("{} {:?} -> {:?} : {}", i.op, i.srcs, i.dest, i.type_hint))
            .collect()
    }

    /// `ADD 7 TO R` — the exact shape COBOL emits, and the one three backends
    /// rejected.
    #[test]
    fn binary_arithmetic_immediate_becomes_a_const_temp() {
        let mut f = func(vec![IIRInstr::new(
            "add",
            Some("acc".into()),
            vec![Operand::Var("acc".into()), Operand::Int(7)],
            "i64",
        )]);
        materialize_immediate_operands_function(&mut f);

        assert_eq!(ops(&f), vec!["const", "add"]);
        let konst = &f.instructions[0];
        assert_eq!(konst.srcs, vec![Operand::Int(7)]);
        assert_eq!(konst.type_hint, "i64");
        // The add now names the temporary, and its other operand is untouched.
        let add = &f.instructions[1];
        assert_eq!(add.srcs[0], Operand::Var("acc".into()));
        assert!(matches!(&add.srcs[1], Operand::Var(v) if v.starts_with("__imm")));
    }

    /// An immediate in the FIRST position works too — `MULTIPLY 3 BY R` lowers
    /// to `mul 3, itm_R`, which is where the CLR reported `mul src[0]`.
    #[test]
    fn immediate_in_first_position_is_materialized() {
        let mut f = func(vec![IIRInstr::new(
            "mul",
            Some("prod".into()),
            vec![Operand::Int(3), Operand::Var("r".into())],
            "i64",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(ops(&f), vec!["const", "mul"]);
        assert!(matches!(&f.instructions[1].srcs[0], Operand::Var(v) if v.starts_with("__imm")));
        assert_eq!(f.instructions[1].srcs[1], Operand::Var("r".into()));
    }

    /// Both operands literal → two temporaries, in operand order.
    #[test]
    fn two_immediates_get_two_distinct_temps() {
        let mut f = func(vec![IIRInstr::new(
            "add",
            Some("d".into()),
            vec![Operand::Int(2), Operand::Int(3)],
            "i64",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(ops(&f), vec!["const", "const", "add"]);
        let (a, b) = (&f.instructions[2].srcs[0], &f.instructions[2].srcs[1]);
        assert_ne!(a, b, "each literal gets its own temporary");
        assert_eq!(f.instructions[0].srcs, vec![Operand::Int(2)]);
        assert_eq!(f.instructions[1].srcs, vec![Operand::Int(3)]);
    }

    /// A comparison's temporary must carry the OPERAND type, never the `bool`
    /// result type — the operand-width-versus-result-width confusion that made
    /// BASIC's comparisons lower to `icmp i1`.
    #[test]
    fn comparison_temp_is_typed_by_the_operand_not_the_bool_result() {
        let mut f = func(vec![IIRInstr::new(
            "cmp_eq",
            Some("t".into()),
            vec![Operand::Var("n".into()), Operand::Int(1)],
            "bool",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(f.instructions[0].type_hint, "i64", "not `bool`: {:?}", f.instructions[0]);
    }

    /// A sibling with a concrete narrower type wins, so the literal matches the
    /// width the surrounding code agreed on.
    #[test]
    fn sibling_producer_type_sets_the_width() {
        let mut f = func(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("n".into()), Operand::Int(1)],
                "i32",
            ),
        ]);
        materialize_immediate_operands_function(&mut f);
        // instrs: const n, const __imm, add
        assert_eq!(f.instructions[1].type_hint, "i32");
    }

    /// Floats and bools keep their own kinds.
    #[test]
    fn float_and_bool_literals_get_their_own_types() {
        let mut f = func(vec![
            IIRInstr::new("add", Some("d".into()), vec![Operand::Var("x".into()), Operand::Float(1.5)], "f64"),
            IIRInstr::new("or", Some("e".into()), vec![Operand::Var("b".into()), Operand::Bool(true)], "bool"),
        ]);
        materialize_immediate_operands_function(&mut f);
        let consts: Vec<&IIRInstr> = f.instructions.iter().filter(|i| i.op == "const").collect();
        assert_eq!(consts[0].type_hint, "f64");
        assert_eq!(consts[1].type_hint, "bool");
    }

    /// A literal that does NOT fit the sibling's width must keep its own type.
    ///
    /// Every backend's `const` lowering narrows with a lossy `as` cast, not a
    /// checked conversion, so adopting `i32` for `5_000_000_000` would not fail —
    /// it would quietly emit `705032704`. Before this pass existed the shape was
    /// rejected outright; introducing it must not trade a loud refusal for a
    /// wrong answer. (Found in security review.)
    #[test]
    fn a_literal_too_wide_for_the_sibling_keeps_its_own_type() {
        let mut f = func(vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("n".into()), Operand::Int(5_000_000_000)],
                "i32",
            ),
        ]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(
            f.instructions[1].type_hint, "i64",
            "5e9 does not fit i32 — narrowing it would silently emit 705032704"
        );
    }

    /// An `f32` sibling is only adopted when the value survives the round trip.
    #[test]
    fn a_float_that_does_not_round_trip_to_f32_keeps_f64() {
        let mut f = func(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Float(1.0)], "f32"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                // Needs more mantissa than f32 has.
                vec![Operand::Var("x".into()), Operand::Float(0.123456789012345)],
                "f32",
            ),
        ]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(f.instructions[1].type_hint, "f64");
    }

    /// A unary op has no sibling, so it takes the instruction's own `type_hint`
    /// — which for `mov`/`neg`/`not` IS the operand type.
    ///
    /// Defaulting to `i64` instead put a two-slot `long` local under an
    /// `i32`-model function's `INEG`, and the JVM verifier rejects that class.
    /// (Found in security review.)
    #[test]
    fn unary_op_takes_its_own_type_hint_not_a_default_i64() {
        let mut f = func(vec![IIRInstr::new(
            "neg",
            Some("d".into()),
            vec![Operand::Int(5)],
            "i32",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(f.instructions[0].type_hint, "i32");
    }

    /// A generated name must not collide with one already in the function. The
    /// counter alone only keeps the temporaries distinct from each other.
    /// (Found in security review.)
    #[test]
    fn a_generated_temp_never_shadows_an_existing_name() {
        // A frontend variable that happens to match this pass's naming scheme.
        let mut f = func(vec![
            IIRInstr::new("const", Some("__imm1_add".into()), vec![Operand::Int(99)], "i64"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("__imm1_add".into()), Operand::Int(7)],
                "i64",
            ),
        ]);
        materialize_immediate_operands_function(&mut f);
        let generated = f.instructions[1].dest.clone().expect("a const was inserted");
        assert_ne!(generated, "__imm1_add", "must not clobber the frontend's variable");
        // …and the original variable still feeds the add.
        assert_eq!(f.instructions[2].srcs[0], Operand::Var("__imm1_add".into()));
    }

    /// `source_map` is documented as indexed in lockstep with `instructions`, so
    /// inserting instructions has to extend it. (Found in security review.)
    #[test]
    fn source_map_stays_in_lockstep_with_instructions() {
        use interpreter_ir::source_loc::SourceLoc;
        let mut f = func(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("a".into()), Operand::Int(7)],
                "i64",
            ),
        ]);
        f.source_map = vec![SourceLoc { line: 10, column: 1 }, SourceLoc { line: 11, column: 1 }];
        materialize_immediate_operands_function(&mut f);
        assert_eq!(
            f.source_map.len(),
            f.instructions.len(),
            "one location per instruction, including the materialized const"
        );
        // The materialized const is attributed to the statement it came from.
        let add_idx = f.instructions.iter().position(|i| i.op == "add").unwrap();
        assert_eq!(f.source_map[add_idx].line, 11);
        assert_eq!(f.source_map[add_idx - 1].line, 11, "the const belongs to the same statement");
    }

    /// A function with no `source_map` must not grow one.
    #[test]
    fn absent_source_map_is_left_absent() {
        let mut f = func(vec![IIRInstr::new(
            "add",
            Some("d".into()),
            vec![Operand::Var("x".into()), Operand::Int(7)],
            "i64",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert!(f.source_map.is_empty());
    }

    /// Addressing immediates are NOT values and must stay inline: a
    /// `field_load`'s index, a `jmp`'s label, a `call_builtin`'s builtin name.
    /// Rewriting those changes what the instruction means.
    #[test]
    fn addressing_immediates_are_left_alone() {
        let mut f = func(vec![
            IIRInstr::new(
                "field_load",
                Some("v".into()),
                vec![Operand::Var("p".into()), Operand::Int(0)],
                "ref<any>",
            ),
            IIRInstr::new("jmp", None, vec![Operand::Var("L".into())], "void"),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![Operand::Var("cons".into()), Operand::Int(1), Operand::Int(2)],
                "ref<any>",
            ),
            // `const`'s own source is a literal by definition.
            IIRInstr::new("const", Some("k".into()), vec![Operand::Int(9)], "i64"),
        ]);
        let before = render(&f);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(render(&f), before, "no addressing immediate may be rewritten");
    }

    /// A module whose frontend already materialized everything is untouched —
    /// the pass must be a no-op for the five languages that were already green.
    #[test]
    fn already_materialized_module_is_unchanged() {
        let mut f = func(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
        ]);
        let before = render(&f);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(render(&f), before);
    }

    /// Unary ops materialize only `srcs[0]`.
    #[test]
    fn unary_ops_materialize_their_single_operand() {
        let mut f = func(vec![IIRInstr::new(
            "neg",
            Some("d".into()),
            vec![Operand::Int(5)],
            "i64",
        )]);
        materialize_immediate_operands_function(&mut f);
        assert_eq!(ops(&f), vec!["const", "neg"]);
    }

    /// The module entry point walks every function.
    #[test]
    fn module_entry_point_rewrites_every_function() {
        let mut m = IIRModule::new("m", "cobol");
        m.functions = vec![
            func(vec![IIRInstr::new("add", Some("d".into()), vec![Operand::Var("x".into()), Operand::Int(1)], "i64")]),
            IIRFunction::new(
                "other",
                vec![],
                "i64",
                vec![IIRInstr::new("sub", Some("e".into()), vec![Operand::Var("y".into()), Operand::Int(2)], "i64")],
            ),
        ];
        materialize_immediate_operands(&mut m);
        for f in &m.functions {
            assert!(f.instructions.iter().any(|i| i.op == "const"), "{}", f.name);
            assert!(
                !f.instructions.iter().any(|i| {
                    value_operand_indices(&i.op)
                        .is_some_and(|ix| ix.iter().any(|&k| i.srcs.get(k).is_some_and(is_literal)))
                }),
                "no value operand may remain a literal in {}",
                f.name
            );
        }
    }
}
