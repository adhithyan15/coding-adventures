//! The **standard operation registry** (LANG83) — one list naming every
//! operation a frontend may emit and every backend must either lower or refuse.
//!
//! # Why a registry
//!
//! The dynamic-language frontends (JavaScript, Python, Ruby, Perl — LANG78) need
//! a much richer vocabulary than the scalar core the older frontends grew up on:
//! floor division, output without a newline, defined traps, overflow checks,
//! number formatting. The repository's direction is that **every one of those is
//! a first-class IIR operation that each backend lowers itself** — there is no
//! portable runtime, no shared library of IIR helper functions, no per-frontend
//! helper generation.
//!
//! That only works if "which ops exist" and "which backend lowers which op" are
//! written down in exactly one place each, and if a mismatch is a CI failure
//! rather than a surprise at run time:
//!
//! ```text
//!                    ┌──────────────────────────────┐
//!   frontend ──emit──▶ STANDARD_OPS (this module)    │  "what may be emitted"
//!                    └──────────────┬───────────────┘
//!                                   │ every entry must be declared by…
//!        ┌──────────┬───────────┬───┴──────┬──────────┬─────────┐
//!        ▼          ▼           ▼          ▼          ▼         ▼
//!   x86_64 OP_SUPPORT  llvm  …  wasm  …  beam  …   vm-core   jit-core
//!     Lowered | Refused("why")   (one table per backend crate)
//!                                   │
//!                                   ▼
//!   lang_aot::check_module_for(backend, module) — refuse at compile time
//! ```
//!
//! A refusal is a typed compile error. It is never a miscompile and never a
//! silent fallback.
//!
//! # Op versus Builtin
//!
//! An [`OpSpec`] is one of two [`OpKind`]s:
//!
//! | Kind | IIR shape | Name lives in |
//! |------|-----------|---------------|
//! | [`OpKind::Op`] | `IIRInstr { op: "i64_div_floor", .. }` | `instr.op` |
//! | [`OpKind::Builtin`] | `call_builtin "print_i64", v` | `srcs[0]` as `Operand::Var` |
//!
//! Arithmetic, comparison, conversion, string and `trap` operations are Ops: they
//! lower inline and analyses (`has_side_effects`, `may_trap`, dead-code
//! elimination) see them without decoding a call. Output operations are
//! Builtins: every backend already owns a builtin table and an output sink.
//! [`registry_name`] does the "which name does this instruction carry?"
//! extraction once, so no backend re-derives it.
//!
//! # The naming rule — type first, never a dispatch prefix
//!
//! The native pipeline reaches the machine backends **by name**, and those
//! backends dispatch on name **prefixes**: `x86_64-backend` treats any op that
//! `starts_with("div_")` as a typed hardware divide and reads the rest of the
//! name as its type suffix. So an op called `div_floor` would never produce an
//! "unknown op" error — it would silently compile to *truncating* division,
//! which is wrong for every negative quotient:
//!
//! ```text
//!   IIR  div_floor a, b          (meant: floor(-7 / 2) = -4)
//!   x86  starts_with("div_") ✓ → cqo; idiv → -3        ✗ silent miscompile
//!
//!   IIR  i64_div_floor a, b      (type-first)
//!   x86  no prefix matches     → "unsupported op"      ✓ loud refusal
//! ```
//!
//! New names are therefore **type-first** (`i64_div_floor`, `f64_sin`). The test
//! `names_avoid_dispatch_prefixes` rejects any registry name starting with one
//! of [`DISPATCH_PREFIXES`], except the six `cmp_*` comparison ops that *are*
//! the family the `cmp_` prefix dispatches ([`GRANDFATHERED_PREFIXED_NAMES`]).
//!
//! # What is (and is not) in the registry yet
//!
//! PR1a registers the operations the existing frontends already rely on across
//! backends — output, strings, integer arithmetic and comparison, `f64` math and
//! the integer↔real conversions — plus the four PR1b operations (`write_i64`,
//! `i64_div_floor`, `i64_mod_floor`, `trap`), which every backend declares
//! `Refused` until PR1b lowers them. Instructions that are not registry entries
//! (control flow, calls, memory, heap, bitwise, the `dyn_*` lisp builtins, …)
//! are simply not gated by the registry; their existing validators still apply.
//! See `code/specs/LANG83-iir-standard-operation-registry.md` for the full
//! vocabulary including the families planned for later.

use crate::exception_kind::{
    CATCH_ANY, TRAP_BOUNDS, TRAP_CONV_RANGE, TRAP_DIV_ZERO, TRAP_OVERFLOW,
};
use crate::instr::{IIRInstr, Operand};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How an operation appears in IIR — see the module docs' "Op versus Builtin".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// An instruction mnemonic: `instr.op == name`.
    Op,
    /// A standard `call_builtin` name: `instr.op == "call_builtin"` and
    /// `srcs[0] == Operand::Var(name)`.
    Builtin,
}

/// The family an operation belongs to. Purely descriptive — it groups the
/// registry the way the LANG83 vocabulary table does, so a reader (or a
/// conformance report) can say "all output ops" without a name list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpFamily {
    /// Integer (and, for the classic ops, `f64`) arithmetic.
    Arithmetic,
    /// Comparisons producing a 0/1 or `bool` result.
    Comparison,
    /// `f64` math functions.
    Float,
    /// Integer↔real conversions.
    Conversion,
    /// Writes to the program's single ordered output stream.
    Output,
    /// LANG-FULL E4 byte strings.
    String,
    /// Program-terminating error operations.
    Error,
}

/// What a backend does with one registry operation.
///
/// Every backend crate publishes a table of these (`OP_SUPPORT`) and an
/// `op_support(name)` lookup over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpSupport {
    /// The backend lowers this operation to instructions it supports.
    Lowered,
    /// The backend cannot lower this operation; the string says why, and is
    /// quoted verbatim in the compile-time refusal a user sees.
    Refused(&'static str),
}

impl OpSupport {
    /// `true` for [`OpSupport::Lowered`].
    ///
    /// ```
    /// use interpreter_ir::op_registry::OpSupport;
    /// assert!(OpSupport::Lowered.is_lowered());
    /// assert!(!OpSupport::Refused("no").is_lowered());
    /// ```
    pub fn is_lowered(self) -> bool {
        matches!(self, OpSupport::Lowered)
    }
}

/// The reason every backend gives for the four PR1b operations until PR1b
/// lowers them. Shared so the refusal text a user sees is identical on every
/// backend.
pub const NOT_YET_LOWERED_PR1B: &str = "not yet lowered; LANG83 PR1b";

/// The `introduced_by` tag of the four operations PR1b will lower.
pub const LANG83_PR1B: &str = "LANG83 PR1b";

/// One registered operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpSpec {
    /// The operation's name (`instr.op` for an Op, the builtin name for a Builtin).
    pub name: &'static str,
    /// Op or Builtin.
    pub kind: OpKind,
    /// Descriptive family.
    pub family: OpFamily,
    /// Number of value operands. For a Builtin this excludes the leading name
    /// operand (`call_builtin "print_i64", v` has arity 1).
    pub arity: usize,
    /// Whether the operation writes a `dest`.
    pub dest: bool,
    /// One descriptive type per operand: `"int"` (any integer width), `"i64"`,
    /// `"f64"`, `"num"` (integer or `f64`), `"str"`, `"literal"` (an
    /// `Operand::Str` compile-time literal) or `"kind"` (an `Operand::Str` trap
    /// kind from the closed table in `exception_kind`).
    pub operand_types: &'static [&'static str],
    /// The trap kinds this operation may raise. [`CATCH_ANY`] means "whichever
    /// registered kind its operand names" (only `trap` itself).
    pub may_trap: &'static [&'static str],
    /// `true` if control never falls through this operation (like `ret`).
    pub terminator: bool,
    /// The spec slice that defined the operation.
    pub introduced_by: &'static str,
}

impl OpSpec {
    /// `true` if this entry is one of the operations PR1b will lower.
    pub fn is_pr1b(&self) -> bool {
        self.introduced_by == LANG83_PR1B
    }
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// Compact constructor so the table below reads as a table.
#[allow(clippy::too_many_arguments)]
const fn spec(
    name: &'static str,
    kind: OpKind,
    family: OpFamily,
    arity: usize,
    dest: bool,
    operand_types: &'static [&'static str],
    may_trap: &'static [&'static str],
    introduced_by: &'static str,
) -> OpSpec {
    OpSpec { name, kind, family, arity, dest, operand_types, may_trap, terminator: false, introduced_by }
}

use OpFamily::{Arithmetic, Comparison, Conversion, Error, Float, Output, String as Str};
use OpKind::{Builtin, Op};

const NO_TRAP: &[&str] = &[];
const DIV_ZERO_ONLY: &[&str] = &[TRAP_DIV_ZERO];
const BOUNDS_ONLY: &[&str] = &[TRAP_BOUNDS];
const CONV_RANGE_ONLY: &[&str] = &[TRAP_CONV_RANGE];
const DIV_ZERO_OR_OVERFLOW: &[&str] = &[TRAP_DIV_ZERO, TRAP_OVERFLOW];
const ANY_REGISTERED_KIND: &[&str] = &[CATCH_ANY];

const BIN_NUM: &[&str] = &["num", "num"];
const UN_NUM: &[&str] = &["num"];
const UN_F64: &[&str] = &["f64"];
const BIN_I64: &[&str] = &["i64", "i64"];

/// Every standard operation, grouped by family in vocabulary-table order.
///
/// Adding an entry here fails the `op_support` declaration test in every
/// backend crate until that backend lowers it or refuses it — which is the
/// point.
pub const STANDARD_OPS: &[OpSpec] = &[
    // ── Output: one ordered byte stream (LANG83 §"Ordered output") ──────────
    // `print_i64` writes the decimal value AND a trailing "\n" (BASIC, Oct,
    // ALGOL and Ruby `puts` depend on the newline); `write_i64` (PR1b) is the
    // newline-free sibling Perl `print` / Python `end=""` need.
    spec("print_i64", Builtin, Output, 1, false, &["int"], NO_TRAP, "LANG-MATRIX G2"),
    spec("putchar", Builtin, Output, 1, false, &["int"], NO_TRAP, "BF07"),
    spec("print_str", Op, Output, 1, false, &["str"], NO_TRAP, "LANG-FULL E4"),
    spec("write_i64", Builtin, Output, 1, false, &["int"], NO_TRAP, LANG83_PR1B),
    // ── Strings (LANG-FULL E4): immutable byte strings ──────────────────────
    spec("str_const", Op, Str, 1, true, &["literal"], NO_TRAP, "LANG-FULL E4"),
    spec("str_len", Op, Str, 1, true, &["str"], NO_TRAP, "LANG-FULL E4"),
    spec("str_index", Op, Str, 2, true, &["str", "int"], BOUNDS_ONLY, "LANG-FULL E4"),
    spec("str_slice", Op, Str, 3, true, &["str", "int", "int"], BOUNDS_ONLY, "LANG-FULL E4"),
    spec("str_concat", Op, Str, 2, true, &["str", "str"], NO_TRAP, "LANG-FULL E4"),
    spec("str_eq", Op, Str, 2, true, &["str", "str"], NO_TRAP, "LANG-FULL E4"),
    spec("str_cmp", Op, Str, 2, true, &["str", "str"], NO_TRAP, "LANG-FULL E4"),
    // ── Arithmetic ───────────────────────────────────────────────────────────
    // The classic ops wrap on integers and follow IEEE on f64. Their division
    // edges (zero divisor, MIN / -1) are backend-dependent today — LANG83's
    // "Legacy edge behaviour" table — which is why the defined-edge
    // `i64_div_floor`/`i64_mod_floor` are new ops rather than a change to `div`.
    spec("add", Op, Arithmetic, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("sub", Op, Arithmetic, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("mul", Op, Arithmetic, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("div", Op, Arithmetic, 2, true, BIN_NUM, DIV_ZERO_ONLY, "IIR core"),
    spec("mod", Op, Arithmetic, 2, true, BIN_NUM, DIV_ZERO_ONLY, "IIR core"),
    spec("neg", Op, Arithmetic, 1, true, UN_NUM, NO_TRAP, "IIR core"),
    spec("i64_div_floor", Op, Arithmetic, 2, true, BIN_I64, DIV_ZERO_OR_OVERFLOW, LANG83_PR1B),
    spec("i64_mod_floor", Op, Arithmetic, 2, true, BIN_I64, DIV_ZERO_ONLY, LANG83_PR1B),
    // ── Comparison ───────────────────────────────────────────────────────────
    spec("cmp_eq", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("cmp_ne", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("cmp_lt", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("cmp_le", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("cmp_gt", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    spec("cmp_ge", Op, Comparison, 2, true, BIN_NUM, NO_TRAP, "IIR core"),
    // ── Float math (ALGOL AL8 standard functions) ───────────────────────────
    spec("f64_sqrt", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_sin", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_cos", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_ln", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_exp", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_atan", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_tan", Op, Float, 1, true, UN_F64, NO_TRAP, "ALGOL AL8"),
    spec("f64_pow", Op, Float, 2, true, &["f64", "f64"], NO_TRAP, "ALGOL AL-pow"),
    // ── Conversions (LANG-FULL E8) ──────────────────────────────────────────
    spec("int_to_real", Op, Conversion, 1, true, &["int"], NO_TRAP, "LANG-FULL E8"),
    spec("real_to_int_trunc", Op, Conversion, 1, true, UN_F64, CONV_RANGE_ONLY, "LANG-FULL E8"),
    spec("real_to_int_floor", Op, Conversion, 1, true, UN_F64, CONV_RANGE_ONLY, "LANG-FULL E8"),
    // ── Errors ───────────────────────────────────────────────────────────────
    // `trap kind` terminates the program after flushing output. The only
    // terminator in the registry, so it is spelled out instead of `spec(..)`.
    OpSpec {
        name: "trap",
        kind: Op,
        family: Error,
        arity: 1,
        dest: false,
        operand_types: &["kind"],
        may_trap: ANY_REGISTERED_KIND,
        terminator: true,
        introduced_by: LANG83_PR1B,
    },
];

/// The name prefixes the native and JIT backends dispatch on (see the module
/// docs). A registry name starting with one of these would be silently
/// captured by an existing lowering arm.
pub const DISPATCH_PREFIXES: &[&str] = &[
    "add_", "sub_", "mul_", "div_", "mod_", "neg_", "not_", "and_", "or_", "xor_", "shl_",
    "shr_", "cmp_", "ret_", "mov_", "const_", "dyn_",
];

/// Registry names that start with a dispatch prefix *on purpose*: the six
/// comparisons are exactly the family `cmp_` dispatches.
pub const GRANDFATHERED_PREFIXED_NAMES: &[&str] =
    &["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"];

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

/// Look up a registry entry by name.
///
/// ```
/// use interpreter_ir::op_registry::{lookup, OpKind};
/// assert_eq!(lookup("print_i64").unwrap().kind, OpKind::Builtin);
/// assert_eq!(lookup("i64_div_floor").unwrap().kind, OpKind::Op);
/// assert!(lookup("jmp").is_none()); // control flow is not a registry op
/// ```
pub fn lookup(name: &str) -> Option<&'static OpSpec> {
    STANDARD_OPS.iter().find(|s| s.name == name)
}

/// The registry entry an instruction invokes, if any.
///
/// An Op matches on `instr.op`; a Builtin matches a `call_builtin` whose first
/// source is `Operand::Var(name)`. A `call_builtin` of a non-registry name (the
/// lisp `dyn_*` builtins, `getchar`, …) and every non-registry mnemonic return
/// `None`.
///
/// ```
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use interpreter_ir::op_registry::registry_entry;
///
/// let print = IIRInstr::new("call_builtin", None,
///     vec![Operand::Var("print_i64".into()), Operand::Var("x".into())], "void");
/// assert_eq!(registry_entry(&print).unwrap().name, "print_i64");
///
/// let floor = IIRInstr::new("i64_div_floor", Some("q".into()),
///     vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64");
/// assert_eq!(registry_entry(&floor).unwrap().name, "i64_div_floor");
///
/// let other = IIRInstr::new("call_builtin", None,
///     vec![Operand::Var("getchar".into())], "i64");
/// assert!(registry_entry(&other).is_none());
/// ```
pub fn registry_entry(instr: &IIRInstr) -> Option<&'static OpSpec> {
    if instr.op == "call_builtin" {
        let name = match instr.srcs.first() {
            Some(Operand::Var(name)) => name.as_str(),
            _ => return None,
        };
        return lookup(name).filter(|s| s.kind == OpKind::Builtin);
    }
    lookup(&instr.op).filter(|s| s.kind == OpKind::Op)
}

/// The registry name an instruction invokes, if any (see [`registry_entry`]).
pub fn registry_name(instr: &IIRInstr) -> Option<&'static str> {
    registry_entry(instr).map(|s| s.name)
}

/// `true` if `op` is a registry entry of kind [`OpKind::Op`] — used by
/// `opcodes::is_known_op` so every registered mnemonic is a known one.
pub fn is_standard_op_mnemonic(op: &str) -> bool {
    lookup(op).is_some_and(|s| s.kind == OpKind::Op)
}

/// The name-rule violation for `name`, or `None` if it is acceptable.
///
/// ```
/// use interpreter_ir::op_registry::name_rule_violation;
/// assert!(name_rule_violation("i64_div_floor").is_none());
/// assert!(name_rule_violation("cmp_lt").is_none()); // grandfathered family
/// assert!(name_rule_violation("div_floor").unwrap().contains("div_"));
/// ```
pub fn name_rule_violation(name: &str) -> Option<String> {
    if GRANDFATHERED_PREFIXED_NAMES.contains(&name) {
        return None;
    }
    DISPATCH_PREFIXES.iter().find(|p| name.starts_with(**p)).map(|p| {
        format!(
            "{name:?} starts with the dispatch prefix {p:?}: native/JIT backends would \
             capture it with an existing lowering arm instead of refusing it; \
             name it type-first (e.g. \"i64_{rest}\")",
            rest = name.trim_end_matches('_')
        )
    })
}

/// Check a backend's `OP_SUPPORT` table against the registry.
///
/// Every backend crate calls this from a unit test. It fails when a registry
/// entry is undeclared (the backend has not decided), declared twice
/// (ambiguous), or when the table names something that is not a registry entry
/// (a stale or misspelled row).
///
/// ```
/// use interpreter_ir::op_registry::{check_op_support_table, OpSupport, STANDARD_OPS};
///
/// let full: Vec<(&str, OpSupport)> =
///     STANDARD_OPS.iter().map(|s| (s.name, OpSupport::Lowered)).collect();
/// assert!(check_op_support_table(&full).is_ok());
///
/// let missing = &full[1..];
/// assert!(check_op_support_table(missing).unwrap_err().contains("undeclared"));
/// ```
pub fn check_op_support_table(table: &[(&str, OpSupport)]) -> Result<(), String> {
    let mut problems = Vec::new();
    for spec in STANDARD_OPS {
        match table.iter().filter(|(n, _)| *n == spec.name).count() {
            0 => problems.push(format!("undeclared standard op {:?}", spec.name)),
            1 => {}
            n => problems.push(format!("standard op {:?} declared {n} times", spec.name)),
        }
    }
    for (name, _) in table {
        if lookup(name).is_none() {
            problems.push(format!("{name:?} is declared but is not a standard op"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Look `name` up in a backend's `OP_SUPPORT` table.
///
/// The shared body of every backend's `op_support`. A name the table does not
/// declare is refused with a reason that says so; the per-crate declaration
/// test guarantees no registry entry ever reaches that arm.
pub fn support_from_table(table: &[(&str, OpSupport)], name: &str) -> OpSupport {
    table
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .unwrap_or(OpSupport::Refused("not declared in this backend's OP_SUPPORT table"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception_kind::is_registered_trap_kind;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique() {
        let mut seen = HashSet::new();
        for spec in STANDARD_OPS {
            assert!(seen.insert(spec.name), "duplicate registry name {:?}", spec.name);
        }
    }

    #[test]
    fn names_avoid_dispatch_prefixes() {
        for spec in STANDARD_OPS {
            if let Some(why) = name_rule_violation(spec.name) {
                panic!("{why}");
            }
        }
        // The hazard the rule exists for is actually detected.
        for bad in ["div_floor", "mod_floor", "add_ovf", "neg_ovf", "or_else", "dyn_new"] {
            assert!(name_rule_violation(bad).is_some(), "{bad} should violate the rule");
        }
        // Exactly-named classic ops are fine: the hazard is a *prefix*.
        for ok in ["div", "mod", "or", "i64_div_floor", "f64_sin", "str_len"] {
            assert!(name_rule_violation(ok).is_none(), "{ok} should be accepted");
        }
    }

    #[test]
    fn grandfathered_names_are_registered_comparisons() {
        for name in GRANDFATHERED_PREFIXED_NAMES {
            let spec = lookup(name).expect("grandfathered name must be registered");
            assert_eq!(spec.family, OpFamily::Comparison);
        }
    }

    #[test]
    fn builtins_never_shadow_an_iir_mnemonic() {
        // A Builtin named like a mnemonic would make `registry_entry` ambiguous
        // to a reader (and to any backend that switched on the name alone).
        for spec in STANDARD_OPS.iter().filter(|s| s.kind == OpKind::Builtin) {
            assert!(
                !crate::opcodes::is_known_op(spec.name),
                "builtin {:?} collides with an IIR mnemonic",
                spec.name
            );
        }
    }

    #[test]
    fn trap_kinds_are_registered() {
        for spec in STANDARD_OPS {
            for kind in spec.may_trap {
                assert!(
                    *kind == CATCH_ANY || is_registered_trap_kind(kind),
                    "{} lists unregistered trap kind {kind:?}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn terminators_have_no_dest_and_operand_types_match_arity() {
        for spec in STANDARD_OPS {
            assert_eq!(spec.operand_types.len(), spec.arity, "{}", spec.name);
            if spec.terminator {
                assert!(!spec.dest, "terminator {} must not write a dest", spec.name);
            }
        }
        assert!(lookup("trap").unwrap().terminator);
        assert_eq!(STANDARD_OPS.iter().filter(|s| s.terminator).count(), 1);
    }

    #[test]
    fn pr1b_set_is_exactly_the_four_new_ops() {
        let mut pr1b: Vec<&str> =
            STANDARD_OPS.iter().filter(|s| s.is_pr1b()).map(|s| s.name).collect();
        pr1b.sort_unstable();
        assert_eq!(pr1b, ["i64_div_floor", "i64_mod_floor", "trap", "write_i64"]);
    }

    #[test]
    fn registry_entry_distinguishes_op_and_builtin_shapes() {
        // An Op name used as a builtin is not a registry hit, and vice versa.
        let as_builtin = IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("str_len".into()), Operand::Var("s".into())],
            "i64",
        );
        assert!(registry_entry(&as_builtin).is_none());
        let as_op = IIRInstr::new("print_i64", None, vec![Operand::Var("x".into())], "void");
        assert!(registry_entry(&as_op).is_none());
        // A call_builtin whose name is not a Var is not a registry hit.
        let odd = IIRInstr::new("call_builtin", None, vec![Operand::Int(1)], "void");
        assert!(registry_entry(&odd).is_none());
        let no_srcs = IIRInstr::new("call_builtin", None, vec![], "void");
        assert!(registry_name(&no_srcs).is_none());
        assert!(is_standard_op_mnemonic("f64_sin"));
        assert!(!is_standard_op_mnemonic("print_i64"));
    }

    #[test]
    fn check_op_support_table_reports_every_problem() {
        let mut table: Vec<(&str, OpSupport)> =
            STANDARD_OPS.iter().map(|s| (s.name, OpSupport::Lowered)).collect();
        table.push(("add", OpSupport::Lowered));
        table.push(("not_an_op", OpSupport::Refused("x")));
        let err = check_op_support_table(&table).unwrap_err();
        assert!(err.contains("\"add\" declared 2 times"), "{err}");
        assert!(err.contains("\"not_an_op\" is declared but is not a standard op"), "{err}");
    }

    #[test]
    fn support_from_table_refuses_undeclared_names() {
        let table = [("add", OpSupport::Lowered)];
        assert_eq!(support_from_table(&table, "add"), OpSupport::Lowered);
        assert!(!support_from_table(&table, "sub").is_lowered());
    }
}
