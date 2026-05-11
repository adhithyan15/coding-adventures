//! # numeric — the numeric/comparison builtin lowering table and rewrite logic.
//!
//! This module implements Phase 1 of the LANG31 builtin lowering pass: it
//! rewrites `call_builtin` instructions for arithmetic and comparison
//! operations into the typed IIR opcodes that the `iir-to-*` backends can
//! lower directly to target bytecode.
//!
//! ## Why a separate module?
//!
//! The full lowering pass (Phase 1 + Phase 2) has two distinct concerns:
//!
//! - **Numeric/comparison** (this file): pure arithmetic — no heap allocation,
//!   simple arity check, straightforward 1-to-1 opcode replacement.
//! - **Heap** (`heap.rs`, Phase 2): cons/car/cdr/null? — generates sequences
//!   of `alloc`, `field_load`, `field_store`, `is_null` instructions.
//!
//! Splitting them keeps each file focused and makes it easy to enable Phase 2
//! behind a Cargo feature flag later.
//!
//! ## The lowering table
//!
//! Each entry in `NUMERIC_TABLE` maps a builtin name string to:
//! 1. the expected arity of the **argument** operands (i.e. `srcs[1..]`), and
//! 2. the replacement opcode string.
//!
//! ```text
//! call_builtin srcs layout:
//!   srcs[0] = Operand::Var("<builtin_name>")   ← the name, treated as a literal
//!   srcs[1..] = the actual arguments
//! ```
//!
//! After rewriting, `srcs[0]` is dropped and `srcs[1..]` becomes the new `srcs`.
//!
//! ## Instruction rewrite algorithm
//!
//! For each instruction `I` in every function `F` of the module:
//! ```text
//! if I.op != "call_builtin": skip
//! name_operand = I.srcs[0]            // must be Operand::Var("<name>")
//! args = I.srcs[1..]
//!
//! look up name in NUMERIC_TABLE:
//!   not found  → leave I unchanged (e.g. "cons", "make_closure")
//!   found      → validate arity, then rewrite in place:
//!     I.op       ← table.replacement_op
//!     I.srcs     ← args             (drop the name at index 0)
//!     I.may_alloc ← false           (numeric ops never allocate)
//!     I.type_hint  ← unchanged      (already set by iir-type-checker)
//!     I.dest       ← unchanged
//! ```
//!
//! Unknown builtins (e.g. `"cons"`, `"make_closure"`, `"global_set"`) are left
//! completely unchanged so that later passes or backends can handle them.

use crate::error::BuiltinLoweringError;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;

// ---------------------------------------------------------------------------
// Lowering table entry
// ---------------------------------------------------------------------------

/// One entry in the numeric builtin lowering table.
///
/// `arity` counts only the **argument** operands (`srcs[1..]` of the
/// `call_builtin` instruction).  The name operand at `srcs[0]` is not counted.
struct NumericEntry {
    /// How many argument operands the builtin expects.
    arity: usize,
    /// The IIR opcode that replaces `call_builtin` after rewriting.
    replacement_op: &'static str,
}

// ---------------------------------------------------------------------------
// NUMERIC_TABLE — the 18 numeric/comparison builtins
// ---------------------------------------------------------------------------

/// Static lowering table for all arithmetic, comparison, and bitwise builtins.
///
/// Sorted by arity for readability (unary first, then binary).
///
/// The table is a flat `&[(&str, NumericEntry)]` — linear scan is fine
/// because there are only 18 entries and the scan happens once per
/// `call_builtin` instruction (not on every VM dispatch).
static NUMERIC_TABLE: &[(&str, NumericEntry)] = &[
    // ── Unary operations (arity 1) ─────────────────────────────────────────
    //
    // neg: arithmetic negation  — `(neg x)`  → `neg %x`
    // not: logical/bitwise NOT  — `(not x)`  → `not %x`
    ("neg", NumericEntry { arity: 1, replacement_op: "neg" }),
    ("not", NumericEntry { arity: 1, replacement_op: "not" }),

    // ── Binary arithmetic (arity 2) ────────────────────────────────────────
    ("+",   NumericEntry { arity: 2, replacement_op: "add" }),
    ("-",   NumericEntry { arity: 2, replacement_op: "sub" }),
    ("*",   NumericEntry { arity: 2, replacement_op: "mul" }),
    ("/",   NumericEntry { arity: 2, replacement_op: "div" }),
    ("%",   NumericEntry { arity: 2, replacement_op: "mod" }),

    // ── Binary comparison (arity 2) ────────────────────────────────────────
    //
    // These map Twig/Lisp comparison names to the backend's typed opcodes.
    // The `"="` Twig operator becomes `cmp_eq` (not `eq`) to avoid
    // collision with any hypothetical structural-equality opcode in the future.
    ("=",   NumericEntry { arity: 2, replacement_op: "cmp_eq" }),
    ("!=",  NumericEntry { arity: 2, replacement_op: "cmp_ne" }),
    ("<",   NumericEntry { arity: 2, replacement_op: "cmp_lt" }),
    ("<=",  NumericEntry { arity: 2, replacement_op: "cmp_le" }),
    (">",   NumericEntry { arity: 2, replacement_op: "cmp_gt" }),
    (">=",  NumericEntry { arity: 2, replacement_op: "cmp_ge" }),

    // ── Bitwise / logical binary (arity 2) ────────────────────────────────
    //
    // On integer operands: bitwise.  On bool operands: logical shorthand.
    ("and", NumericEntry { arity: 2, replacement_op: "and" }),
    ("or",  NumericEntry { arity: 2, replacement_op: "or"  }),
    ("shl", NumericEntry { arity: 2, replacement_op: "shl" }),
    ("shr", NumericEntry { arity: 2, replacement_op: "shr" }),
    ("xor", NumericEntry { arity: 2, replacement_op: "xor" }),
];

/// Look up a builtin name in the numeric table.
///
/// Returns `Some(&NumericEntry)` for known numeric builtins,
/// `None` for anything else (including Phase-2 heap builtins).
fn lookup(name: &str) -> Option<&'static NumericEntry> {
    NUMERIC_TABLE
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, e)| e)
}

// ---------------------------------------------------------------------------
// Instruction rewriter
// ---------------------------------------------------------------------------

/// Attempt to rewrite a single `call_builtin` instruction in place.
///
/// Returns:
/// - `Ok(true)`  — the instruction was rewritten (numeric builtin recognised).
/// - `Ok(false)` — the instruction was left unchanged (unknown builtin or
///                 not a `call_builtin` at all).
/// - `Err(_)`    — the instruction was a recognised numeric builtin but
///                 had invalid arity.
///
/// The `fn_name` parameter is used only for error messages.
pub fn try_lower_instr(
    instr: &mut IIRInstr,
    fn_name: &str,
) -> Result<bool, BuiltinLoweringError> {
    // Fast path: skip non-call_builtin instructions immediately.
    if instr.op != "call_builtin" {
        return Ok(false);
    }

    // srcs[0] must be the builtin name as a Var operand.
    // If the instruction is malformed (no srcs), we leave it unchanged —
    // the backend's validator will catch it with a better error.
    let builtin_name = match instr.srcs.first() {
        Some(Operand::Var(name)) => name.clone(),
        _ => return Ok(false),
    };

    // Look up the name in the numeric table.
    let entry = match lookup(&builtin_name) {
        None => return Ok(false),  // Unknown builtin — leave unchanged.
        Some(e) => e,
    };

    // The argument operands are srcs[1..].
    let arg_count = instr.srcs.len().saturating_sub(1);
    if arg_count != entry.arity {
        return Err(BuiltinLoweringError::WrongArity {
            builtin_name,
            function_name: fn_name.to_string(),
            expected: entry.arity,
            found: arg_count,
        });
    }

    // Check the type_hint.  If it is still "any" after iir-type-checker ran,
    // that means the pipeline ordering is wrong — emit a hard error so the
    // programmer can fix the pipeline rather than getting a cryptic backend
    // failure later.
    if instr.type_hint == "any" {
        return Err(BuiltinLoweringError::UntypedBuiltin {
            builtin_name,
            function_name: fn_name.to_string(),
        });
    }

    // ── Rewrite in place ──────────────────────────────────────────────────
    //
    // Replace the op mnemonic.
    instr.op = entry.replacement_op.to_string();

    // Drop srcs[0] (the builtin name) and keep only the argument operands.
    // We do this by draining the first element rather than reallocating.
    instr.srcs.remove(0);

    // Numeric operations never trigger a heap allocation.
    instr.may_alloc = false;

    // type_hint, dest, observed_slot, observed_type, observation_count,
    // deopt_anchor, and ic_slot are all preserved from the original instruction.

    Ok(true)
}

// ---------------------------------------------------------------------------
// Function-level lowering
// ---------------------------------------------------------------------------

/// Lower all `call_builtin` instructions in `fn_` that correspond to numeric
/// builtins.  Unknown builtins are left unchanged.
///
/// Returns the list of errors encountered.  The pass is **best-effort**: it
/// processes every instruction and accumulates errors, then returns them all
/// together rather than bailing at the first one.  This gives better
/// diagnostics when multiple instructions are malformed.
pub fn lower_function(fn_: &mut IIRFunction) -> Vec<BuiltinLoweringError> {
    let fn_name = fn_.name.clone();
    let mut errors = Vec::new();

    for instr in &mut fn_.instructions {
        match try_lower_instr(instr, &fn_name) {
            Ok(_) => {}
            Err(e) => errors.push(e),
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Tests for the numeric module internals
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::{IIRInstr, Operand};

    /// Build a `call_builtin` instruction for a binary op.
    fn binary_call(name: &str, a: &str, b: &str, type_hint: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin",
            Some("%r0".into()),
            vec![
                Operand::Var(name.into()),
                Operand::Var(a.into()),
                Operand::Var(b.into()),
            ],
            type_hint,
        )
    }

    /// Build a `call_builtin` instruction for a unary op.
    fn unary_call(name: &str, a: &str, type_hint: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin",
            Some("%r0".into()),
            vec![Operand::Var(name.into()), Operand::Var(a.into())],
            type_hint,
        )
    }

    #[test]
    fn lookup_known_builtins() {
        for name in ["+", "-", "*", "/", "%", "neg", "=", "!=", "<", "<=", ">", ">=",
                     "and", "or", "not", "shl", "shr", "xor"] {
            assert!(lookup(name).is_some(), "missing builtin {name:?}");
        }
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup("cons").is_none());
        assert!(lookup("car").is_none());
        assert!(lookup("make_closure").is_none());
        assert!(lookup("global_set").is_none());
    }

    #[test]
    fn rewrite_add() {
        let mut instr = binary_call("+", "a", "b", "i64");
        assert_eq!(try_lower_instr(&mut instr, "f").unwrap(), true);
        assert_eq!(instr.op, "add");
        assert_eq!(instr.srcs.len(), 2);
        assert_eq!(instr.srcs[0], Operand::Var("a".into()));
        assert_eq!(instr.srcs[1], Operand::Var("b".into()));
        assert!(!instr.may_alloc);
    }

    #[test]
    fn rewrite_neg() {
        let mut instr = unary_call("neg", "x", "i64");
        assert_eq!(try_lower_instr(&mut instr, "f").unwrap(), true);
        assert_eq!(instr.op, "neg");
        assert_eq!(instr.srcs.len(), 1);
    }

    #[test]
    fn non_call_builtin_unchanged() {
        let mut instr = IIRInstr::new("add", Some("%r0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64");
        assert_eq!(try_lower_instr(&mut instr, "f").unwrap(), false);
        assert_eq!(instr.op, "add"); // unchanged
    }

    #[test]
    fn unknown_builtin_left_unchanged() {
        let mut instr = IIRInstr::new("call_builtin", Some("%r0".into()),
            vec![Operand::Var("cons".into()),
                 Operand::Var("h".into()),
                 Operand::Var("t".into())],
            "any");
        assert_eq!(try_lower_instr(&mut instr, "f").unwrap(), false);
        assert_eq!(instr.op, "call_builtin"); // still call_builtin
    }

    #[test]
    fn wrong_arity_returns_error() {
        // "+" expects 2 args but we give 3
        let mut instr = IIRInstr::new("call_builtin", Some("%r0".into()),
            vec![Operand::Var("+".into()),
                 Operand::Var("a".into()),
                 Operand::Var("b".into()),
                 Operand::Var("c".into())],
            "i64");
        let err = try_lower_instr(&mut instr, "myfn").unwrap_err();
        match err {
            BuiltinLoweringError::WrongArity { builtin_name, function_name, expected, found } => {
                assert_eq!(builtin_name, "+");
                assert_eq!(function_name, "myfn");
                assert_eq!(expected, 2);
                assert_eq!(found, 3);
            }
            _ => panic!("expected WrongArity"),
        }
    }

    #[test]
    fn untyped_builtin_returns_error() {
        let mut instr = binary_call("+", "a", "b", "any");
        let err = try_lower_instr(&mut instr, "g").unwrap_err();
        match err {
            BuiltinLoweringError::UntypedBuiltin { builtin_name, .. } => {
                assert_eq!(builtin_name, "+");
            }
            _ => panic!("expected UntypedBuiltin"),
        }
    }
}
