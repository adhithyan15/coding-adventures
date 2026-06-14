//! `lower_builtins` — the main lowering pass.
//!
//! Iterates over every function in an `IIRModule`, and for each function
//! iterates over every instruction.  Instructions that match the lowering
//! table are rewritten in place; everything else is left untouched.
//!
//! ## Rewriting in place vs. building a new list
//!
//! We build a new instruction list per function rather than mutating the
//! existing one, because Rust's borrow checker prevents iterating and
//! mutating the same Vec simultaneously.  The cost is one extra allocation
//! per function — acceptable for a one-shot compilation pass.
//!
//! ## The "all-remaining `call_builtin`s are fine" rule
//!
//! After this pass, `call_builtin` instructions that survive are those whose
//! first operand is NOT in the lowering table (e.g. `make_nil`, `cons`,
//! `global_get`).  The backends skip these — they are dispatched at runtime
//! by the VM's builtin table, not by native code-gen.

use interpreter_ir::{IIRInstr, IIRModule, Operand};

// ---------------------------------------------------------------------------
// Lowering table
// ---------------------------------------------------------------------------
//
// A simple static table mapping builtin names to their IIR opcode equivalents.
// Binary operations have arity 2; unary operations have arity 1.

struct TableEntry {
    builtin: &'static str,
    iir_op: &'static str,
    arity: usize,
}

static LOWERING_TABLE: &[TableEntry] = &[
    // Unary
    TableEntry { builtin: "neg", iir_op: "neg", arity: 1 },
    TableEntry { builtin: "not", iir_op: "not", arity: 1 },
    // Binary arithmetic
    TableEntry { builtin: "+",   iir_op: "add", arity: 2 },
    TableEntry { builtin: "-",   iir_op: "sub", arity: 2 },
    TableEntry { builtin: "*",   iir_op: "mul", arity: 2 },
    TableEntry { builtin: "/",   iir_op: "div", arity: 2 },
    TableEntry { builtin: "%",   iir_op: "mod", arity: 2 },
    // Binary comparison
    TableEntry { builtin: "=",   iir_op: "cmp_eq", arity: 2 },
    TableEntry { builtin: "!=",  iir_op: "cmp_ne", arity: 2 },
    TableEntry { builtin: "<",   iir_op: "cmp_lt", arity: 2 },
    TableEntry { builtin: "<=",  iir_op: "cmp_le", arity: 2 },
    TableEntry { builtin: ">",   iir_op: "cmp_gt", arity: 2 },
    TableEntry { builtin: ">=",  iir_op: "cmp_ge", arity: 2 },
    // Binary bitwise/logical
    TableEntry { builtin: "and", iir_op: "and", arity: 2 },
    TableEntry { builtin: "or",  iir_op: "or",  arity: 2 },
    TableEntry { builtin: "shl", iir_op: "shl", arity: 2 },
    TableEntry { builtin: "shr", iir_op: "shr", arity: 2 },
    TableEntry { builtin: "xor", iir_op: "xor", arity: 2 },
];

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Lower `call_builtin` instructions to typed IIR ops, in place.
///
/// This function mutates `module` directly.  After this call:
///
/// - `call_builtin "+"` → `add` (with the same operands and dest)
/// - `call_builtin "not"` → `lnot`
/// - All other `call_builtin` instructions are left unchanged.
///
/// # Type hint handling
///
/// When the type_hint on the `call_builtin` instruction is `"any"`, the
/// lowered instruction also carries `"any"`.  The type-checker is meant to
/// run *before* this pass, but we don't hard-fail on `"any"` here — that
/// produces a more actionable error downstream (the backend rejects `"any"`
/// with an `UnsupportedType` / `UntypedInstruction` error that names the
/// specific instruction).
///
/// # Returns
///
/// This function is infallible by design.  Builtin calls that do not match
/// the table are left as `call_builtin`; backends that cannot handle them
/// will produce their own errors.
pub fn lower_builtins(module: &mut IIRModule) {
    for function in &mut module.functions {
        let old_instrs = std::mem::take(&mut function.instructions);
        let mut new_instrs = Vec::with_capacity(old_instrs.len());

        for instr in old_instrs {
            // Only process `call_builtin` instructions.
            // A `call_builtin` instruction's first source operand is a `Var`
            // holding the builtin name; the remaining srcs are the arguments.
            if instr.op != "call_builtin" {
                new_instrs.push(instr);
                continue;
            }

            // Extract the builtin name from the first source operand.
            // If the first src is not a Var (unusual but defensively handled),
            // leave the instruction untouched.
            let builtin_name = match instr.srcs.first() {
                Some(Operand::Var(name)) => name.clone(),
                _ => {
                    new_instrs.push(instr);
                    continue;
                }
            };

            // Look up the builtin in the lowering table.
            let entry = LOWERING_TABLE.iter().find(|e| e.builtin == builtin_name.as_str());

            match entry {
                None => {
                    // Not in the table — leave as call_builtin.
                    new_instrs.push(instr);
                }
                Some(entry) => {
                    // The argument operands are instr.srcs[1..] (skipping the
                    // builtin name at index 0).
                    let args: Vec<Operand> = instr.srcs[1..].to_vec();

                    // Build the replacement instruction with the same destination,
                    // the typed opcode, the argument operands, and the same
                    // type hint.
                    let new_instr = IIRInstr::new(
                        entry.iir_op,
                        instr.dest.clone(),
                        args,
                        &instr.type_hint,
                    );
                    new_instrs.push(new_instr);

                    // Note: we ignore the `arity` field here and trust the
                    // downstream backend to validate operand counts.  The error
                    // module is available for callers who want a stricter pass.
                    let _ = entry.arity;
                }
            }
        }

        function.instructions = new_instrs;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRModule};

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("test", "twig");
        m.functions.push(fn_);
        m
    }

    fn instrs(m: &IIRModule) -> &[IIRInstr] {
        &m.functions[0].instructions
    }

    #[test]
    fn plus_becomes_add() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("r".into()),
                vec![
                    Operand::Var("+".into()),
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ],
                "i64",
            ),
        ]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[2].op, "add");
    }

    #[test]
    fn minus_becomes_sub() {
        let mut m = make_module(vec![IIRInstr::new(
            "call_builtin",
            Some("r".into()),
            vec![
                Operand::Var("-".into()),
                Operand::Var("x".into()),
                Operand::Var("y".into()),
            ],
            "i64",
        )]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[0].op, "sub");
    }

    #[test]
    fn star_becomes_mul() {
        let mut m = make_module(vec![IIRInstr::new(
            "call_builtin",
            Some("r".into()),
            vec![
                Operand::Var("*".into()),
                Operand::Var("x".into()),
                Operand::Var("y".into()),
            ],
            "i64",
        )]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[0].op, "mul");
    }

    #[test]
    fn eq_becomes_cmp_eq() {
        let mut m = make_module(vec![IIRInstr::new(
            "call_builtin",
            Some("r".into()),
            vec![
                Operand::Var("=".into()),
                Operand::Var("x".into()),
                Operand::Var("y".into()),
            ],
            "bool",
        )]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[0].op, "cmp_eq");
    }

    #[test]
    fn unknown_builtin_left_unchanged() {
        let mut m = make_module(vec![IIRInstr::new(
            "call_builtin",
            Some("r".into()),
            vec![Operand::Var("cons".into()), Operand::Var("a".into())],
            "any",
        )]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[0].op, "call_builtin");
    }

    #[test]
    fn non_call_builtin_left_unchanged() {
        let mut m = make_module(vec![IIRInstr::new(
            "add",
            Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64",
        )]);
        lower_builtins(&mut m);
        assert_eq!(instrs(&m)[0].op, "add");
    }
}
