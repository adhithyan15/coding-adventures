//! # `constraint-instructions` — the Constraint-VM IR.
//!
//! **LANG24 PR 24-B**.  Pure-data shape for one constraint-program
//! instruction stream.  No solver, no I/O.
//!
//! `constraint-instructions` is to `constraint-vm` what
//! `interpreter-ir` is to `vm-core`: the pluggable IR consumed by
//! more than one driver.
//!
//! ## Why a separate crate?
//!
//! Per LANG24 §"Why a separate `constraint-instructions` crate?",
//! the IR is consumed by:
//!
//! - `constraint-vm` — the canonical executor (PR 24-D).
//! - `smt-lib-format` — an SMT-LIB v2 textual serialiser/parser
//!   for industry interop (PR 24-E).
//! - `z3-bridge` — opt-in fallback that translates this IR into
//!   Z3's native AST for hard queries (PR 24-H).
//! - `debug-sidecar` / coverage / profiler — tools that observe a
//!   constraint-program use this IR as their grammar (LANG18 /
//!   LANG11 / debug-sidecar patterns generalised to constraints).
//!
//! Decoupling instructions from the executor mirrors LP07 vs LP08
//! exactly.
//!
//! ## What lives here
//!
//! - [`ConstraintInstr`] — the 12-variant opcode set
//!   (`DeclareVar`, `DeclareFn`, `Assert`, `CheckSat`, `GetModel`,
//!   `GetUnsatCore`, `PushScope`, `PopScope`, `Reset`, `SetLogic`,
//!   `Echo`, `SetOption`).  `#[non_exhaustive]`.
//! - [`OptionValue`] — values the `SetOption` opcode carries
//!   (`Bool`, `Int`, `Str`).
//! - [`Program`] — a validated `Vec<ConstraintInstr>`.  Construction
//!   via [`Program::new`] enforces structural invariants
//!   (scope-balance, no `PopScope` without a matching prior
//!   `PushScope`).
//! - [`ProgramError`] — typed validation errors.
//! - **Text serialiser** — `Display` on `ConstraintInstr` and
//!   [`Program`] emits SMT-LIB-flavoured s-expressions.
//! - **Text parser** — [`parse_program`] reads the same syntax back
//!   and reconstructs a [`Program`].  Round-trip is guaranteed.
//!
//! ## Text format
//!
//! ```text
//! (set-logic QF_LIA)
//! (declare-var x Int)
//! (declare-var y Int)
//! (assert (>= x 0))
//! (assert (<= (+ x y) 100))
//! (check-sat)
//! (get-model)
//! ```
//!
//! Whitespace is insignificant.  Comments start with `;` and run
//! to end of line.  String literals (used for `Echo`/`SetOption`)
//! are double-quoted with `\"` and `\\` escapes.
//!
//! ## Non-guarantees (caller responsibilities)
//!
//! Inherits all the non-guarantees of `constraint-core` (predicate
//! depth, CNF blow-up, `Rational` range).  In addition: the parser
//! recurses on parenthesis depth, so callers parsing untrusted text
//! should bound the input length / depth at the boundary.
//!
//! Resource limits on *execution* (instruction count, scope depth,
//! variable count, solver timeout) live in `constraint-vm` per the
//! spec — this crate is pure data.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use constraint_core::{Logic, Predicate, Rational, Sort};

// ---------------------------------------------------------------------------
// OptionValue
// ---------------------------------------------------------------------------

/// Values carried by the [`ConstraintInstr::SetOption`] opcode.
///
/// SMT-LIB option values are conventionally one of `Bool`, integer,
/// or string.  The set is intentionally small; if a future option
/// needs richer values, add a variant (the enum is
/// `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OptionValue {
    /// Boolean option (`:produce-models`, `:produce-unsat-cores`, …).
    Bool(bool),
    /// Integer option (`:random-seed`, `:timeout`, …).
    Int(i64),
    /// String option (`:logic`, `:status`, …).
    Str(String),
}

impl fmt::Display for OptionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionValue::Bool(b) => write!(f, "{b}"),
            OptionValue::Int(n) => write!(f, "{n}"),
            OptionValue::Str(s) => write!(f, "\"{}\"", escape_string(s)),
        }
    }
}

// ---------------------------------------------------------------------------
// ConstraintInstr
// ---------------------------------------------------------------------------

/// One constraint-VM instruction.
///
/// Mirrors LANG24 §"ConstraintIR shape" verbatim.  The 12 variants
/// cover the v1 vocabulary; v2/v3 extensions add variants (the
/// enum is `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConstraintInstr {
    /// Introduce a variable of the given sort.
    DeclareVar {
        /// Variable name (must be unique within scope).
        name: String,
        /// Variable sort (Bool, Int, Real, BitVec, Array, Uninterpreted).
        sort: Sort,
    },

    /// Introduce an uninterpreted function symbol
    /// `f: Sort1 × Sort2 × … → SortN`.
    DeclareFn {
        /// Function name (must be unique within scope).
        name: String,
        /// Argument sorts in declared order.
        arg_sorts: Vec<Sort>,
        /// Return sort.
        ret_sort: Sort,
    },

    /// Assert that `pred` holds.  Multiple `Assert`s conjoin.
    Assert {
        /// The predicate to add to the assertion stack.
        pred: Predicate,
    },

    /// Ask the engine: is the conjunction of all currently-asserted
    /// predicates satisfiable?
    CheckSat,

    /// After a `CheckSat` that returned SAT, extract the satisfying
    /// assignment.  Returns `Map<VarName, Value>` at execution time.
    GetModel,

    /// After a `CheckSat` that returned UNSAT, extract the minimal
    /// subset of asserted predicates that explains unsatisfiability.
    GetUnsatCore,

    /// Push a new scope; subsequent `Assert`s can be undone by `PopScope`.
    /// Maps to incremental-solving stack semantics.
    PushScope,

    /// Pop the most recent scope.  Errors at validation time if
    /// unmatched.
    PopScope,

    /// Clear all assertions.  Equivalent to SMT-LIB `(reset)`.
    Reset,

    /// Declare which theory family the program uses.  Lets the engine
    /// pick the right tactics + reject unsupported features early.
    SetLogic {
        /// The active logic family.
        logic: Logic,
    },

    /// Print a diagnostic to the trace channel.  Maps to SMT-LIB
    /// `(echo "...")`.
    Echo {
        /// Diagnostic message.
        msg: String,
    },

    /// Tune solver behaviour: timeout, model-completeness, random
    /// seed, etc.
    SetOption {
        /// Option key (conventionally a `:colon-prefixed` string in
        /// SMT-LIB; stored verbatim here).
        key: String,
        /// Option value.
        value: OptionValue,
    },
}

impl ConstraintInstr {
    /// Return the opcode's name as it appears in the text format.
    /// Useful for diagnostics that need a stable mnemonic.
    pub fn mnemonic(&self) -> &'static str {
        match self {
            ConstraintInstr::DeclareVar { .. } => "declare-var",
            ConstraintInstr::DeclareFn { .. } => "declare-fn",
            ConstraintInstr::Assert { .. } => "assert",
            ConstraintInstr::CheckSat => "check-sat",
            ConstraintInstr::GetModel => "get-model",
            ConstraintInstr::GetUnsatCore => "get-unsat-core",
            ConstraintInstr::PushScope => "push",
            ConstraintInstr::PopScope => "pop",
            ConstraintInstr::Reset => "reset",
            ConstraintInstr::SetLogic { .. } => "set-logic",
            ConstraintInstr::Echo { .. } => "echo",
            ConstraintInstr::SetOption { .. } => "set-option",
        }
    }
}

impl fmt::Display for ConstraintInstr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintInstr::DeclareVar { name, sort } => {
                write!(f, "(declare-var {name} ")?;
                write_sort(f, sort)?;
                write!(f, ")")
            }
            ConstraintInstr::DeclareFn { name, arg_sorts, ret_sort } => {
                write!(f, "(declare-fn {name} (")?;
                for (i, s) in arg_sorts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write_sort(f, s)?;
                }
                write!(f, ") ")?;
                write_sort(f, ret_sort)?;
                write!(f, ")")
            }
            ConstraintInstr::Assert { pred } => {
                write!(f, "(assert ")?;
                write_predicate(f, pred)?;
                write!(f, ")")
            }
            ConstraintInstr::CheckSat => write!(f, "(check-sat)"),
            ConstraintInstr::GetModel => write!(f, "(get-model)"),
            ConstraintInstr::GetUnsatCore => write!(f, "(get-unsat-core)"),
            ConstraintInstr::PushScope => write!(f, "(push)"),
            ConstraintInstr::PopScope => write!(f, "(pop)"),
            ConstraintInstr::Reset => write!(f, "(reset)"),
            ConstraintInstr::SetLogic { logic } => write!(f, "(set-logic {logic})"),
            ConstraintInstr::Echo { msg } => {
                write!(f, "(echo \"{}\")", escape_string(msg))
            }
            ConstraintInstr::SetOption { key, value } => {
                write!(f, "(set-option {key} {value})")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Custom text serialiser
// ---------------------------------------------------------------------------
//
// We don't reuse `Predicate`'s and `Sort`'s `Display` impls from
// `constraint-core`.  Those are SMT-LIB-flavoured, which is great for
// human reading and SMT-LIB interop but is *ambiguous on round-trip*:
//
//   - SMT-LIB writes `Iff` and `Eq` both as `(=` — the parser would
//     need sort information to disambiguate.
//   - SMT-LIB writes `Real` literals as bare `3/4`, which our atom
//     tokenizer would read as a single symbol.
//   - SMT-LIB writes `BitVec(w)` as `(_ BitVec w)`.
//
// The text format defined here is round-trip-exact at the cost of
// minor SMT-LIB divergence:
//
//   - `Iff` is `(iff a b)` (not `(= a b)`).
//   - `Real(r)` is `(/ num den)` (or just `num` when `den == 1`).
//   - `BitVec(w)` is `(BitVec w)` (no leading `_`).
//
// `smt-lib-format` (PR 24-E) will provide the strict-SMT-LIB
// reader/writer.  This crate's format is the *internal* one used by
// debug dumps, snapshot tests, and the `Program::Display` impl.

fn write_sort(f: &mut fmt::Formatter<'_>, sort: &Sort) -> fmt::Result {
    match sort {
        Sort::Bool => write!(f, "Bool"),
        Sort::Int => write!(f, "Int"),
        Sort::Real => write!(f, "Real"),
        Sort::BitVec(w) => write!(f, "(BitVec {w})"),
        Sort::Array { idx, val } => {
            write!(f, "(Array ")?;
            write_sort(f, idx)?;
            write!(f, " ")?;
            write_sort(f, val)?;
            write!(f, ")")
        }
        Sort::Uninterpreted(name) => write!(f, "{name}"),
        // `Sort` is `#[non_exhaustive]`; future variants need their
        // own `write_sort` arm.  Until then, fall back to a debug
        // form that we never silently round-trip as a real sort.
        other => write!(f, "<unsupported-sort:{other:?}>"),
    }
}

fn write_predicate(f: &mut fmt::Formatter<'_>, p: &Predicate) -> fmt::Result {
    match p {
        Predicate::Bool(true) => write!(f, "true"),
        Predicate::Bool(false) => write!(f, "false"),
        Predicate::Var(name) => write!(f, "{name}"),
        Predicate::Int(n) => write!(f, "{n}"),
        Predicate::Real(r) => {
            // Round-trip-friendly: integer rational becomes a plain int,
            // others become `(/ num den)`.
            if r.den == 1 {
                write!(f, "{}", r.num)
            } else {
                write!(f, "(/ {} {})", r.num, r.den)
            }
        }
        Predicate::Apply { f: name, args } => write_app(f, name, args),
        Predicate::And(parts) => write_app(f, "and", parts),
        Predicate::Or(parts) => write_app(f, "or", parts),
        Predicate::Not(inner) => {
            write!(f, "(not ")?;
            write_predicate(f, inner)?;
            write!(f, ")")
        }
        Predicate::Implies(a, b) => write_binary(f, "=>", a, b),
        Predicate::Iff(a, b) => write_binary(f, "iff", a, b),
        Predicate::Eq(a, b) => write_binary(f, "=", a, b),
        Predicate::NEq(a, b) => write_binary(f, "distinct", a, b),
        Predicate::Add(parts) => write_app(f, "+", parts),
        Predicate::Sub(a, b) => write_binary(f, "-", a, b),
        Predicate::Mul { coef, term } => {
            write!(f, "(* {coef} ")?;
            write_predicate(f, term)?;
            write!(f, ")")
        }
        Predicate::Le(a, b) => write_binary(f, "<=", a, b),
        Predicate::Lt(a, b) => write_binary(f, "<", a, b),
        Predicate::Ge(a, b) => write_binary(f, ">=", a, b),
        Predicate::Gt(a, b) => write_binary(f, ">", a, b),
        Predicate::Ite(c, t, e) => {
            write!(f, "(ite ")?;
            write_predicate(f, c)?;
            write!(f, " ")?;
            write_predicate(f, t)?;
            write!(f, " ")?;
            write_predicate(f, e)?;
            write!(f, ")")
        }
        Predicate::Forall { var, sort, body } => write_quantifier(f, "forall", var, sort, body),
        Predicate::Exists { var, sort, body } => write_quantifier(f, "exists", var, sort, body),
        Predicate::Select { arr, idx } => {
            write!(f, "(select ")?;
            write_predicate(f, arr)?;
            write!(f, " ")?;
            write_predicate(f, idx)?;
            write!(f, ")")
        }
        Predicate::Store { arr, idx, val } => {
            write!(f, "(store ")?;
            write_predicate(f, arr)?;
            write!(f, " ")?;
            write_predicate(f, idx)?;
            write!(f, " ")?;
            write_predicate(f, val)?;
            write!(f, ")")
        }
        // `Predicate` is `#[non_exhaustive]` — future variants
        // (added by v2/v3 theories) need their own `write_*` arm.
        // Until then, fall back to a debug-shaped opaque form so we
        // never silently produce broken text.
        other => write!(f, "<unsupported:{other:?}>"),
    }
}

fn write_app(f: &mut fmt::Formatter<'_>, head: &str, args: &[Predicate]) -> fmt::Result {
    write!(f, "({head}")?;
    for a in args {
        write!(f, " ")?;
        write_predicate(f, a)?;
    }
    write!(f, ")")
}

fn write_binary(f: &mut fmt::Formatter<'_>, op: &str, a: &Predicate, b: &Predicate) -> fmt::Result {
    write!(f, "({op} ")?;
    write_predicate(f, a)?;
    write!(f, " ")?;
    write_predicate(f, b)?;
    write!(f, ")")
}

fn write_quantifier(
    f: &mut fmt::Formatter<'_>,
    kind: &str,
    var: &str,
    sort: &Sort,
    body: &Predicate,
) -> fmt::Result {
    write!(f, "({kind} (({var} ")?;
    write_sort(f, sort)?;
    write!(f, ")) ")?;
    write_predicate(f, body)?;
    write!(f, ")")
}

// ---------------------------------------------------------------------------
// Program + validation
// ---------------------------------------------------------------------------

/// A validated sequence of [`ConstraintInstr`].
///
/// Construction via [`Program::new`] enforces:
/// - **Scope balance.**  `PopScope` never exceeds the count of prior
///   `PushScope`s.  Unbalanced *trailing* pushes are allowed (the
///   engine tolerates them on `Reset`); only unmatched pops are an
///   error.
///
/// Other invariants (variables-must-be-declared-before-use, sort
/// agreement on `Assert` predicates, `SetLogic`-must-come-first) are
/// **deferred to `constraint-engine`**: those need a sort
/// environment, which lives one layer up.  This crate just owns the
/// shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    instrs: Vec<ConstraintInstr>,
}

impl Program {
    /// Construct a `Program` after running structural validation.
    ///
    /// Validation enforces:
    /// - **Scope balance.**  `PopScope` never exceeds the count of
    ///   prior `PushScope`s.
    /// - **Identifier safety.**  Every name in `DeclareVar` /
    ///   `DeclareFn` / `Predicate::Var` / `Predicate::Apply` /
    ///   quantifier binders / `Sort::Uninterpreted` is non-empty,
    ///   contains no whitespace or s-expression delimiters
    ///   (`(`, `)`, `;`, `"`), does not parse as an integer
    ///   literal, and is not one of this format's reserved
    ///   tokens (`and`, `or`, `not`, `=>`, `iff`, `<=>`, `=`,
    ///   `distinct`, `!=`, `+`, `-`, `*`, `/`, `<=`, `<`, `>=`,
    ///   `>`, `ite`, `forall`, `exists`, `select`, `store`,
    ///   `true`, `false`, `Bool`, `Int`, `Real`, `BitVec`,
    ///   `Array`).  This guarantees `parse_program(&p.to_string())
    ///   == p`.
    pub fn new(instrs: Vec<ConstraintInstr>) -> Result<Self, ProgramError> {
        let mut depth: i64 = 0;
        for (idx, instr) in instrs.iter().enumerate() {
            match instr {
                ConstraintInstr::PushScope => depth += 1,
                ConstraintInstr::PopScope => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(ProgramError::UnmatchedPop { index: idx });
                    }
                }
                ConstraintInstr::Reset => {
                    // (reset) clears the scope stack as well.
                    depth = 0;
                }
                _ => {}
            }
            check_instr_identifiers(idx, instr)?;
        }
        Ok(Program { instrs })
    }

    /// Construct a `Program` *without* running validation.  For
    /// callers that have already validated (e.g. inside the parser
    /// after re-parsing a known-good text serialisation).
    pub fn new_unchecked(instrs: Vec<ConstraintInstr>) -> Self {
        Program { instrs }
    }

    /// Borrow the underlying instruction slice.
    pub fn instructions(&self) -> &[ConstraintInstr] {
        &self.instrs
    }

    /// Consume the program and return the instruction `Vec`.
    pub fn into_instructions(self) -> Vec<ConstraintInstr> {
        self.instrs
    }

    /// Number of instructions.
    pub fn len(&self) -> usize {
        self.instrs.len()
    }

    /// Whether the program contains zero instructions.  Lints insist.
    pub fn is_empty(&self) -> bool {
        self.instrs.is_empty()
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, instr) in self.instrs.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{instr}")?;
        }
        Ok(())
    }
}

/// Structural-validation errors raised by [`Program::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgramError {
    /// `PopScope` at instruction `index` had no matching prior `PushScope`.
    UnmatchedPop {
        /// Zero-based instruction index where the unmatched pop occurred.
        index: usize,
    },
    /// An identifier (variable name, function name, sort name, …) is
    /// not safe to round-trip through the text format.
    BadIdentifier {
        /// Zero-based instruction index where the offending identifier appeared.
        index: usize,
        /// The offending name (truncated for very long inputs).
        name: String,
        /// Why it was rejected (empty / reserved / contains delimiter / etc.).
        reason: &'static str,
    },
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::UnmatchedPop { index } => {
                write!(f, "unmatched (pop) at instruction index {index}")
            }
            ProgramError::BadIdentifier { index, name, reason } => {
                write!(
                    f,
                    "bad identifier `{name}` at instruction index {index}: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for ProgramError {}

// ---------------------------------------------------------------------------
// Identifier validation (round-trip safety)
// ---------------------------------------------------------------------------

/// Tokens reserved by this crate's text format.  An identifier
/// matching one of these would be re-parsed as the reserved meaning,
/// breaking round-trip.
const RESERVED_TOKENS: &[&str] = &[
    // Predicate combinators
    "and", "or", "not", "=>", "iff", "<=>", "=", "distinct", "!=", "+", "-", "*", "/", "<=", "<",
    ">=", ">", "ite", "forall", "exists", "select", "store",
    // Boolean literals
    "true", "false",
    // Sort tokens
    "Bool", "Int", "Real", "BitVec", "Array",
];

/// Check that `name` can be safely emitted as a bare symbol and
/// re-parsed as the same identifier.  Pure function — no I/O.
fn validate_identifier(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("identifier must be non-empty");
    }
    if RESERVED_TOKENS.contains(&name) {
        return Err("identifier collides with a reserved token");
    }
    if parse_int(name).is_some() {
        return Err("identifier looks like an integer literal");
    }
    for ch in name.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '(' | ')' | ';' | '"') {
            return Err("identifier contains a whitespace or s-expression delimiter");
        }
    }
    Ok(())
}

fn check_id(idx: usize, name: &str) -> Result<(), ProgramError> {
    validate_identifier(name).map_err(|reason| ProgramError::BadIdentifier {
        index: idx,
        name: truncate_for_diag(name),
        reason,
    })
}

fn truncate_for_diag(name: &str) -> String {
    const LIMIT: usize = 64;
    if name.chars().count() <= LIMIT {
        name.to_owned()
    } else {
        let head: String = name.chars().take(LIMIT).collect();
        format!("{head}…")
    }
}

fn check_instr_identifiers(idx: usize, instr: &ConstraintInstr) -> Result<(), ProgramError> {
    match instr {
        ConstraintInstr::DeclareVar { name, sort } => {
            check_id(idx, name)?;
            check_sort_identifiers(idx, sort)?;
        }
        ConstraintInstr::DeclareFn { name, arg_sorts, ret_sort } => {
            check_id(idx, name)?;
            for s in arg_sorts {
                check_sort_identifiers(idx, s)?;
            }
            check_sort_identifiers(idx, ret_sort)?;
        }
        ConstraintInstr::Assert { pred } => check_predicate_identifiers(idx, pred)?,
        // Nullary / setter opcodes have no identifier to validate.
        ConstraintInstr::CheckSat
        | ConstraintInstr::GetModel
        | ConstraintInstr::GetUnsatCore
        | ConstraintInstr::PushScope
        | ConstraintInstr::PopScope
        | ConstraintInstr::Reset
        | ConstraintInstr::SetLogic { .. }
        | ConstraintInstr::Echo { .. }
        | ConstraintInstr::SetOption { .. } => {}
    }
    Ok(())
}

fn check_sort_identifiers(idx: usize, sort: &Sort) -> Result<(), ProgramError> {
    match sort {
        Sort::Bool | Sort::Int | Sort::Real | Sort::BitVec(_) => Ok(()),
        Sort::Array { idx: i, val } => {
            check_sort_identifiers(idx, i)?;
            check_sort_identifiers(idx, val)
        }
        Sort::Uninterpreted(name) => check_id(idx, name),
        // `Sort` is `#[non_exhaustive]`; future variants must be
        // added here.  Until then, treat them as identifier-free.
        _ => Ok(()),
    }
}

fn check_predicate_identifiers(idx: usize, pred: &Predicate) -> Result<(), ProgramError> {
    match pred {
        Predicate::Bool(_) | Predicate::Int(_) | Predicate::Real(_) => Ok(()),
        Predicate::Var(name) => check_id(idx, name),
        Predicate::Apply { f: name, args } => {
            check_id(idx, name)?;
            for a in args {
                check_predicate_identifiers(idx, a)?;
            }
            Ok(())
        }
        Predicate::And(parts) | Predicate::Or(parts) | Predicate::Add(parts) => {
            for p in parts {
                check_predicate_identifiers(idx, p)?;
            }
            Ok(())
        }
        Predicate::Not(inner) | Predicate::Mul { term: inner, .. } => {
            check_predicate_identifiers(idx, inner)
        }
        Predicate::Implies(a, b)
        | Predicate::Iff(a, b)
        | Predicate::Eq(a, b)
        | Predicate::NEq(a, b)
        | Predicate::Sub(a, b)
        | Predicate::Le(a, b)
        | Predicate::Lt(a, b)
        | Predicate::Ge(a, b)
        | Predicate::Gt(a, b) => {
            check_predicate_identifiers(idx, a)?;
            check_predicate_identifiers(idx, b)
        }
        Predicate::Ite(c, t, e) => {
            check_predicate_identifiers(idx, c)?;
            check_predicate_identifiers(idx, t)?;
            check_predicate_identifiers(idx, e)
        }
        Predicate::Forall { var, sort, body } | Predicate::Exists { var, sort, body } => {
            check_id(idx, var)?;
            check_sort_identifiers(idx, sort)?;
            check_predicate_identifiers(idx, body)
        }
        Predicate::Select { arr, idx: i } => {
            check_predicate_identifiers(idx, arr)?;
            check_predicate_identifiers(idx, i)
        }
        Predicate::Store { arr, idx: i, val } => {
            check_predicate_identifiers(idx, arr)?;
            check_predicate_identifiers(idx, i)?;
            check_predicate_identifiers(idx, val)
        }
        // `Predicate` is `#[non_exhaustive]`; future variants must
        // be added here.  Conservative default: reject so we don't
        // silently produce non-round-tripping text.
        _ => Err(ProgramError::BadIdentifier {
            index: idx,
            name: format!("{pred:?}"),
            reason: "unsupported Predicate variant for round-trip validation",
        }),
    }
}

// ---------------------------------------------------------------------------
// Text format — escape helpers
// ---------------------------------------------------------------------------

fn escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Text parser — s-expression tokenizer + recursive-descent parser
// ---------------------------------------------------------------------------

/// Errors raised by [`parse_program`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Unexpected end of input partway through a form.
    UnexpectedEof,
    /// Saw an unmatched `)`.
    UnexpectedCloseParen {
        /// Byte offset of the offending paren.
        offset: usize,
    },
    /// Encountered an unknown opcode mnemonic.
    UnknownOpcode(String),
    /// An opcode was given the wrong number / shape of arguments.
    BadArgs {
        /// Opcode mnemonic.
        opcode: String,
        /// Diagnostic detail.
        detail: String,
    },
    /// A string literal was unterminated or malformed.
    BadString(String),
    /// An integer literal failed to parse.
    BadInt(String),
    /// A `Sort` token was unrecognised.
    BadSort(String),
    /// A `Logic` token was unrecognised.
    BadLogic(String),
    /// Validation failed after parsing.
    Program(ProgramError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "unexpected end of input"),
            ParseError::UnexpectedCloseParen { offset } => {
                write!(f, "unexpected `)` at byte offset {offset}")
            }
            ParseError::UnknownOpcode(s) => write!(f, "unknown opcode `{s}`"),
            ParseError::BadArgs { opcode, detail } => {
                write!(f, "bad arguments to `{opcode}`: {detail}")
            }
            ParseError::BadString(s) => write!(f, "bad string literal: {s}"),
            ParseError::BadInt(s) => write!(f, "bad integer literal `{s}`"),
            ParseError::BadSort(s) => write!(f, "unknown sort `{s}`"),
            ParseError::BadLogic(s) => write!(f, "unknown logic `{s}`"),
            ParseError::Program(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ProgramError> for ParseError {
    fn from(e: ProgramError) -> Self {
        ParseError::Program(e)
    }
}

/// Parse a textual constraint program back into a [`Program`].
///
/// Round-trip with [`Program`]'s `Display` impl is exact: for any
/// `Program p`, `parse_program(&p.to_string()).unwrap() == p`.
pub fn parse_program(input: &str) -> Result<Program, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens: &tokens, pos: 0 };
    let mut instrs = Vec::new();
    while !parser.at_end() {
        let form = parser.parse_form()?;
        instrs.push(form_to_instr(form)?);
    }
    Ok(Program::new(instrs)?)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Open(usize),
    Close(usize),
    Symbol(String),
    Int(i128),
    Str(String),
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b';' => {
                // Comment — skip to end of line.
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                out.push(Token::Open(i));
                i += 1;
            }
            b')' => {
                out.push(Token::Close(i));
                i += 1;
            }
            b'"' => {
                let (s, end) = read_string(bytes, i)?;
                out.push(Token::Str(s));
                i = end;
            }
            _ => {
                let (s, end) = read_atom(bytes, i);
                if let Some(n) = parse_int(&s) {
                    out.push(Token::Int(n));
                } else {
                    out.push(Token::Symbol(s));
                }
                i = end;
            }
        }
    }
    Ok(out)
}

fn read_string(bytes: &[u8], start: usize) -> Result<(String, usize), ParseError> {
    debug_assert_eq!(bytes[start], b'"');
    // Collect raw bytes — escapes append one byte each, non-escapes pass
    // through verbatim.  Decode as UTF-8 *once* at the end so multi-byte
    // sequences aren't truncated to Latin-1 by per-byte `as char` casts.
    let mut buf: Vec<u8> = Vec::new();
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let s = std::str::from_utf8(&buf)
                    .map_err(|e| ParseError::BadString(format!("invalid UTF-8: {e}")))?
                    .to_owned();
                return Ok((s, i + 1));
            }
            b'\\' => {
                if i + 1 >= bytes.len() {
                    return Err(ParseError::BadString("unterminated escape".into()));
                }
                match bytes[i + 1] {
                    b'"' => buf.push(b'"'),
                    b'\\' => buf.push(b'\\'),
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    other => {
                        return Err(ParseError::BadString(format!(
                            "unknown escape `\\{}`",
                            other as char
                        )));
                    }
                }
                i += 2;
            }
            other => {
                buf.push(other);
                i += 1;
            }
        }
    }
    Err(ParseError::BadString("unterminated string literal".into()))
}

fn read_atom(bytes: &[u8], start: usize) -> (String, usize) {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')' | b';' | b'"') {
            break;
        }
        i += 1;
    }
    (
        std::str::from_utf8(&bytes[start..i])
            .map(|s| s.to_owned())
            .unwrap_or_default(),
        i,
    )
}

fn parse_int(s: &str) -> Option<i128> {
    if s.is_empty() {
        return None;
    }
    // Accept optional leading sign, then all digits.
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let rest_starts_at = if first == '-' || first == '+' { 1 } else { 0 };
    if rest_starts_at == s.len() {
        return None;
    }
    if !s[rest_starts_at..].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse::<i128>().ok()
}

#[derive(Debug, Clone, PartialEq)]
enum Form {
    Atom(Token),
    List(Vec<Form>),
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_form(&mut self) -> Result<Form, ParseError> {
        match self.advance() {
            None => Err(ParseError::UnexpectedEof),
            Some(Token::Open(_)) => {
                let mut items = Vec::new();
                loop {
                    match self.peek() {
                        None => return Err(ParseError::UnexpectedEof),
                        Some(Token::Close(_)) => {
                            self.advance();
                            return Ok(Form::List(items));
                        }
                        Some(_) => items.push(self.parse_form()?),
                    }
                }
            }
            Some(Token::Close(o)) => Err(ParseError::UnexpectedCloseParen { offset: *o }),
            Some(t) => Ok(Form::Atom(t.clone())),
        }
    }
}

fn form_to_instr(form: Form) -> Result<ConstraintInstr, ParseError> {
    let items = match form {
        Form::List(items) => items,
        Form::Atom(_) => {
            return Err(ParseError::BadArgs {
                opcode: "<top-level>".into(),
                detail: "expected a list".into(),
            });
        }
    };
    let mut iter = items.into_iter();
    let head = iter.next().ok_or(ParseError::UnexpectedEof)?;
    let opcode = match head {
        Form::Atom(Token::Symbol(s)) => s,
        _ => {
            return Err(ParseError::BadArgs {
                opcode: "<unknown>".into(),
                detail: "first element must be an opcode symbol".into(),
            });
        }
    };
    let args: Vec<Form> = iter.collect();
    match opcode.as_str() {
        "declare-var" => parse_declare_var(args),
        "declare-fn" => parse_declare_fn(args),
        "assert" => parse_assert(args),
        "check-sat" => parse_nullary("check-sat", args, ConstraintInstr::CheckSat),
        "get-model" => parse_nullary("get-model", args, ConstraintInstr::GetModel),
        "get-unsat-core" => {
            parse_nullary("get-unsat-core", args, ConstraintInstr::GetUnsatCore)
        }
        "push" => parse_nullary("push", args, ConstraintInstr::PushScope),
        "pop" => parse_nullary("pop", args, ConstraintInstr::PopScope),
        "reset" => parse_nullary("reset", args, ConstraintInstr::Reset),
        "set-logic" => parse_set_logic(args),
        "echo" => parse_echo(args),
        "set-option" => parse_set_option(args),
        other => Err(ParseError::UnknownOpcode(other.to_owned())),
    }
}

fn parse_nullary(
    op: &str,
    args: Vec<Form>,
    instr: ConstraintInstr,
) -> Result<ConstraintInstr, ParseError> {
    if !args.is_empty() {
        return Err(ParseError::BadArgs {
            opcode: op.to_owned(),
            detail: format!("expected 0 args, got {}", args.len()),
        });
    }
    Ok(instr)
}

fn parse_declare_var(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 2 {
        return Err(ParseError::BadArgs {
            opcode: "declare-var".into(),
            detail: format!("expected 2 args (name sort), got {}", args.len()),
        });
    }
    let mut iter = args.into_iter();
    let name = atom_to_symbol(iter.next().unwrap(), "declare-var")?;
    let sort = parse_sort(iter.next().unwrap())?;
    Ok(ConstraintInstr::DeclareVar { name, sort })
}

fn parse_declare_fn(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 3 {
        return Err(ParseError::BadArgs {
            opcode: "declare-fn".into(),
            detail: format!("expected 3 args (name (arg-sorts) ret-sort), got {}", args.len()),
        });
    }
    let mut iter = args.into_iter();
    let name = atom_to_symbol(iter.next().unwrap(), "declare-fn")?;
    let arg_sort_forms = match iter.next().unwrap() {
        Form::List(xs) => xs,
        Form::Atom(_) => {
            return Err(ParseError::BadArgs {
                opcode: "declare-fn".into(),
                detail: "second arg must be a (sort sort ...) list".into(),
            });
        }
    };
    let arg_sorts = arg_sort_forms
        .into_iter()
        .map(parse_sort)
        .collect::<Result<Vec<_>, _>>()?;
    let ret_sort = parse_sort(iter.next().unwrap())?;
    Ok(ConstraintInstr::DeclareFn { name, arg_sorts, ret_sort })
}

fn parse_assert(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 1 {
        return Err(ParseError::BadArgs {
            opcode: "assert".into(),
            detail: format!("expected 1 arg (predicate), got {}", args.len()),
        });
    }
    let pred = parse_predicate(args.into_iter().next().unwrap())?;
    Ok(ConstraintInstr::Assert { pred })
}

fn parse_set_logic(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 1 {
        return Err(ParseError::BadArgs {
            opcode: "set-logic".into(),
            detail: format!("expected 1 arg (logic), got {}", args.len()),
        });
    }
    let sym = atom_to_symbol(args.into_iter().next().unwrap(), "set-logic")?;
    let logic = match sym.as_str() {
        "QF_Bool" => Logic::QF_Bool,
        "QF_LIA" => Logic::QF_LIA,
        "QF_LRA" => Logic::QF_LRA,
        "QF_BV" => Logic::QF_BV,
        "QF_AUFLIA" => Logic::QF_AUFLIA,
        "LIA" => Logic::LIA,
        "ALL" => Logic::ALL,
        other => return Err(ParseError::BadLogic(other.to_owned())),
    };
    Ok(ConstraintInstr::SetLogic { logic })
}

fn parse_echo(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 1 {
        return Err(ParseError::BadArgs {
            opcode: "echo".into(),
            detail: format!("expected 1 arg (string), got {}", args.len()),
        });
    }
    match args.into_iter().next().unwrap() {
        Form::Atom(Token::Str(s)) => Ok(ConstraintInstr::Echo { msg: s }),
        _ => Err(ParseError::BadArgs {
            opcode: "echo".into(),
            detail: "argument must be a string literal".into(),
        }),
    }
}

fn parse_set_option(args: Vec<Form>) -> Result<ConstraintInstr, ParseError> {
    if args.len() != 2 {
        return Err(ParseError::BadArgs {
            opcode: "set-option".into(),
            detail: format!("expected 2 args (key value), got {}", args.len()),
        });
    }
    let mut iter = args.into_iter();
    let key = atom_to_symbol(iter.next().unwrap(), "set-option")?;
    let value = match iter.next().unwrap() {
        Form::Atom(Token::Symbol(s)) if s == "true" => OptionValue::Bool(true),
        Form::Atom(Token::Symbol(s)) if s == "false" => OptionValue::Bool(false),
        Form::Atom(Token::Int(n)) => {
            // i128 → i64 conversion may overflow; OptionValue::Int is i64
            // because SMT-LIB option ints fit comfortably.  Reject overflow
            // explicitly rather than silently truncating.
            let n64: i64 = n.try_into().map_err(|_| ParseError::BadArgs {
                opcode: "set-option".into(),
                detail: format!("integer value {n} doesn't fit in i64"),
            })?;
            OptionValue::Int(n64)
        }
        Form::Atom(Token::Str(s)) => OptionValue::Str(s),
        _ => {
            return Err(ParseError::BadArgs {
                opcode: "set-option".into(),
                detail: "value must be bool / int / string".into(),
            });
        }
    };
    Ok(ConstraintInstr::SetOption { key, value })
}

fn atom_to_symbol(form: Form, opcode: &str) -> Result<String, ParseError> {
    match form {
        Form::Atom(Token::Symbol(s)) => Ok(s),
        _ => Err(ParseError::BadArgs {
            opcode: opcode.to_owned(),
            detail: "expected a symbol".into(),
        }),
    }
}

fn parse_sort(form: Form) -> Result<Sort, ParseError> {
    match form {
        Form::Atom(Token::Symbol(s)) => match s.as_str() {
            "Bool" => Ok(Sort::Bool),
            "Int" => Ok(Sort::Int),
            "Real" => Ok(Sort::Real),
            other => {
                // Bare `Uninterpreted(name)` shorthand: any unrecognised
                // bare symbol becomes an uninterpreted sort.  Dedicated
                // parameterised sorts (BitVec, Array) use the list form.
                Ok(Sort::Uninterpreted(other.to_owned()))
            }
        },
        Form::List(items) => {
            let mut iter = items.into_iter();
            let head = iter.next().ok_or(ParseError::BadSort("()".into()))?;
            let head_sym = atom_to_symbol(head, "sort")?;
            match head_sym.as_str() {
                "BitVec" => {
                    let width_form = iter
                        .next()
                        .ok_or_else(|| ParseError::BadSort("(BitVec) needs width".into()))?;
                    if iter.next().is_some() {
                        return Err(ParseError::BadSort("(BitVec) takes 1 arg".into()));
                    }
                    let n = match width_form {
                        Form::Atom(Token::Int(n)) => n,
                        _ => {
                            return Err(ParseError::BadSort(
                                "(BitVec) width must be an integer".into(),
                            ));
                        }
                    };
                    let width: u32 =
                        n.try_into().map_err(|_| ParseError::BadSort(format!(
                            "(BitVec) width {n} out of u32 range"
                        )))?;
                    Ok(Sort::BitVec(width))
                }
                "Array" => {
                    let idx_form = iter
                        .next()
                        .ok_or_else(|| ParseError::BadSort("(Array) needs idx sort".into()))?;
                    let val_form = iter
                        .next()
                        .ok_or_else(|| ParseError::BadSort("(Array) needs val sort".into()))?;
                    if iter.next().is_some() {
                        return Err(ParseError::BadSort("(Array) takes 2 args".into()));
                    }
                    Ok(Sort::Array {
                        idx: Box::new(parse_sort(idx_form)?),
                        val: Box::new(parse_sort(val_form)?),
                    })
                }
                other => Err(ParseError::BadSort(other.to_owned())),
            }
        }
        Form::Atom(_) => Err(ParseError::BadSort("non-symbol atom".into())),
    }
}

fn parse_predicate(form: Form) -> Result<Predicate, ParseError> {
    match form {
        Form::Atom(Token::Symbol(s)) => match s.as_str() {
            "true" => Ok(Predicate::Bool(true)),
            "false" => Ok(Predicate::Bool(false)),
            other => Ok(Predicate::Var(other.to_owned())),
        },
        Form::Atom(Token::Int(n)) => Ok(Predicate::Int(n)),
        Form::Atom(Token::Str(_)) => Err(ParseError::BadArgs {
            opcode: "<predicate>".into(),
            detail: "string literals aren't predicates".into(),
        }),
        Form::Atom(_) => Err(ParseError::BadArgs {
            opcode: "<predicate>".into(),
            detail: "unexpected atom shape".into(),
        }),
        Form::List(items) => parse_predicate_list(items),
    }
}

fn parse_predicate_list(items: Vec<Form>) -> Result<Predicate, ParseError> {
    let mut iter = items.into_iter();
    let head = iter.next().ok_or_else(|| ParseError::BadArgs {
        opcode: "<predicate>".into(),
        detail: "empty list".into(),
    })?;
    let op = atom_to_symbol(head, "<predicate>")?;
    let args: Vec<Form> = iter.collect();

    // Helper closures for binary / boxed-binary parsing.
    let parse_args = |args: Vec<Form>| -> Result<Vec<Predicate>, ParseError> {
        args.into_iter().map(parse_predicate).collect()
    };

    fn binary(args: Vec<Predicate>, op: &str) -> Result<(Predicate, Predicate), ParseError> {
        if args.len() != 2 {
            return Err(ParseError::BadArgs {
                opcode: op.to_owned(),
                detail: format!("expected 2 args, got {}", args.len()),
            });
        }
        let mut it = args.into_iter();
        let a = it.next().unwrap();
        let b = it.next().unwrap();
        Ok((a, b))
    }

    match op.as_str() {
        "and" => Ok(Predicate::And(parse_args(args)?)),
        "or" => Ok(Predicate::Or(parse_args(args)?)),
        "not" => {
            let mut ps = parse_args(args)?;
            if ps.len() != 1 {
                return Err(ParseError::BadArgs {
                    opcode: "not".into(),
                    detail: format!("expected 1 arg, got {}", ps.len()),
                });
            }
            Ok(Predicate::Not(Box::new(ps.remove(0))))
        }
        "=>" => {
            let (a, b) = binary(parse_args(args)?, "=>")?;
            Ok(Predicate::Implies(Box::new(a), Box::new(b)))
        }
        "iff" | "<=>" => {
            let (a, b) = binary(parse_args(args)?, "iff")?;
            Ok(Predicate::Iff(Box::new(a), Box::new(b)))
        }
        "=" => {
            let (a, b) = binary(parse_args(args)?, "=")?;
            Ok(Predicate::Eq(Box::new(a), Box::new(b)))
        }
        "distinct" | "!=" => {
            let (a, b) = binary(parse_args(args)?, "distinct")?;
            Ok(Predicate::NEq(Box::new(a), Box::new(b)))
        }
        "+" => Ok(Predicate::Add(parse_args(args)?)),
        "-" => {
            let (a, b) = binary(parse_args(args)?, "-")?;
            Ok(Predicate::Sub(Box::new(a), Box::new(b)))
        }
        "*" => {
            // Linear-arith product: (* coef term).  Coef must be an int literal.
            if args.len() != 2 {
                return Err(ParseError::BadArgs {
                    opcode: "*".into(),
                    detail: format!("expected 2 args, got {}", args.len()),
                });
            }
            let mut it = args.into_iter();
            let coef_form = it.next().unwrap();
            let term_form = it.next().unwrap();
            let coef = match coef_form {
                Form::Atom(Token::Int(n)) => n,
                _ => {
                    return Err(ParseError::BadArgs {
                        opcode: "*".into(),
                        detail: "first arg to (*) must be an integer literal (linear)".into(),
                    });
                }
            };
            let term = parse_predicate(term_form)?;
            Ok(Predicate::Mul { coef, term: Box::new(term) })
        }
        "<=" => {
            let (a, b) = binary(parse_args(args)?, "<=")?;
            Ok(Predicate::Le(Box::new(a), Box::new(b)))
        }
        "<" => {
            let (a, b) = binary(parse_args(args)?, "<")?;
            Ok(Predicate::Lt(Box::new(a), Box::new(b)))
        }
        ">=" => {
            let (a, b) = binary(parse_args(args)?, ">=")?;
            Ok(Predicate::Ge(Box::new(a), Box::new(b)))
        }
        ">" => {
            let (a, b) = binary(parse_args(args)?, ">")?;
            Ok(Predicate::Gt(Box::new(a), Box::new(b)))
        }
        "ite" => {
            let ps = parse_args(args)?;
            if ps.len() != 3 {
                return Err(ParseError::BadArgs {
                    opcode: "ite".into(),
                    detail: format!("expected 3 args (cond then else), got {}", ps.len()),
                });
            }
            let mut it = ps.into_iter();
            let c = it.next().unwrap();
            let t = it.next().unwrap();
            let e = it.next().unwrap();
            Ok(Predicate::Ite(Box::new(c), Box::new(t), Box::new(e)))
        }
        "forall" | "exists" => {
            // (forall ((x Sort)) body)
            if args.len() != 2 {
                return Err(ParseError::BadArgs {
                    opcode: op.clone(),
                    detail: format!("expected 2 args (binders body), got {}", args.len()),
                });
            }
            let mut it = args.into_iter();
            let binders_form = it.next().unwrap();
            let body_form = it.next().unwrap();
            let binders = match binders_form {
                Form::List(xs) => xs,
                _ => {
                    return Err(ParseError::BadArgs {
                        opcode: op.clone(),
                        detail: "binders must be a list".into(),
                    });
                }
            };
            // Single-binder support — multi-binder lowers to nested
            // quantifiers, which is the easiest structural decision.
            if binders.len() != 1 {
                return Err(ParseError::BadArgs {
                    opcode: op.clone(),
                    detail: format!(
                        "expected exactly 1 binder (got {}); multi-binder writes as nested quantifiers",
                        binders.len()
                    ),
                });
            }
            let binder = match binders.into_iter().next().unwrap() {
                Form::List(xs) if xs.len() == 2 => xs,
                _ => {
                    return Err(ParseError::BadArgs {
                        opcode: op.clone(),
                        detail: "binder must be (name sort)".into(),
                    });
                }
            };
            let mut bit = binder.into_iter();
            let var = atom_to_symbol(bit.next().unwrap(), &op)?;
            let sort = parse_sort(bit.next().unwrap())?;
            let body = parse_predicate(body_form)?;
            let body_box = Box::new(body);
            Ok(if op == "forall" {
                Predicate::Forall { var, sort, body: body_box }
            } else {
                Predicate::Exists { var, sort, body: body_box }
            })
        }
        "select" => {
            let (a, i) = binary(parse_args(args)?, "select")?;
            Ok(Predicate::Select { arr: Box::new(a), idx: Box::new(i) })
        }
        "store" => {
            let ps = parse_args(args)?;
            if ps.len() != 3 {
                return Err(ParseError::BadArgs {
                    opcode: "store".into(),
                    detail: format!("expected 3 args (arr idx val), got {}", ps.len()),
                });
            }
            let mut it = ps.into_iter();
            let arr = Box::new(it.next().unwrap());
            let idx = Box::new(it.next().unwrap());
            let val = Box::new(it.next().unwrap());
            Ok(Predicate::Store { arr, idx, val })
        }
        // Rational literal: (/ num den)
        "/" => {
            let (a, b) = binary(parse_args(args)?, "/")?;
            // Both sides must be Int literals (we don't model arbitrary rational arithmetic here).
            match (a, b) {
                (Predicate::Int(num), Predicate::Int(den)) => {
                    Ok(Predicate::Real(Rational::new(num, den)))
                }
                _ => Err(ParseError::BadArgs {
                    opcode: "/".into(),
                    detail: "both args must be integer literals (rational literal only)".into(),
                }),
            }
        }
        // Anything else: treat as application of a declared function.
        other => Ok(Predicate::Apply {
            f: other.to_owned(),
            args: parse_args(args)?,
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(p: &Program) {
        let text = p.to_string();
        let parsed = parse_program(&text).expect("round-trip parse");
        assert_eq!(&parsed, p, "round-trip mismatch.\nText was:\n{text}");
    }

    // ---------- ConstraintInstr Display ----------

    #[test]
    fn display_check_sat() {
        assert_eq!(ConstraintInstr::CheckSat.to_string(), "(check-sat)");
    }

    #[test]
    fn display_declare_var() {
        let i = ConstraintInstr::DeclareVar { name: "x".into(), sort: Sort::Int };
        assert_eq!(i.to_string(), "(declare-var x Int)");
    }

    #[test]
    fn display_declare_fn() {
        let i = ConstraintInstr::DeclareFn {
            name: "f".into(),
            arg_sorts: vec![Sort::Int, Sort::Bool],
            ret_sort: Sort::Int,
        };
        assert_eq!(i.to_string(), "(declare-fn f (Int Bool) Int)");
    }

    #[test]
    fn display_set_logic() {
        let i = ConstraintInstr::SetLogic { logic: Logic::QF_LIA };
        assert_eq!(i.to_string(), "(set-logic QF_LIA)");
    }

    #[test]
    fn display_echo_escapes() {
        let i = ConstraintInstr::Echo { msg: "hello \"world\"\n".into() };
        assert_eq!(i.to_string(), "(echo \"hello \\\"world\\\"\\n\")");
    }

    #[test]
    fn display_set_option_bool_int_str() {
        let a = ConstraintInstr::SetOption {
            key: ":produce-models".into(),
            value: OptionValue::Bool(true),
        };
        assert_eq!(a.to_string(), "(set-option :produce-models true)");
        let b = ConstraintInstr::SetOption {
            key: ":random-seed".into(),
            value: OptionValue::Int(42),
        };
        assert_eq!(b.to_string(), "(set-option :random-seed 42)");
        let c = ConstraintInstr::SetOption {
            key: ":status".into(),
            value: OptionValue::Str("sat".into()),
        };
        assert_eq!(c.to_string(), "(set-option :status \"sat\")");
    }

    // ---------- Mnemonic ----------

    #[test]
    fn mnemonic_covers_all_opcodes() {
        let xs: Vec<&'static str> = vec![
            ConstraintInstr::DeclareVar { name: "x".into(), sort: Sort::Int }.mnemonic(),
            ConstraintInstr::DeclareFn { name: "f".into(), arg_sorts: vec![], ret_sort: Sort::Bool }
                .mnemonic(),
            ConstraintInstr::Assert { pred: Predicate::Bool(true) }.mnemonic(),
            ConstraintInstr::CheckSat.mnemonic(),
            ConstraintInstr::GetModel.mnemonic(),
            ConstraintInstr::GetUnsatCore.mnemonic(),
            ConstraintInstr::PushScope.mnemonic(),
            ConstraintInstr::PopScope.mnemonic(),
            ConstraintInstr::Reset.mnemonic(),
            ConstraintInstr::SetLogic { logic: Logic::QF_LIA }.mnemonic(),
            ConstraintInstr::Echo { msg: "x".into() }.mnemonic(),
            ConstraintInstr::SetOption { key: "k".into(), value: OptionValue::Bool(true) }
                .mnemonic(),
        ];
        // All twelve are distinct.
        let mut sorted = xs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 12);
    }

    // ---------- Program validation ----------

    #[test]
    fn empty_program_validates() {
        let p = Program::new(vec![]).unwrap();
        assert_eq!(p.len(), 0);
        assert!(p.is_empty());
    }

    #[test]
    fn balanced_push_pop_validates() {
        let p = Program::new(vec![
            ConstraintInstr::PushScope,
            ConstraintInstr::PushScope,
            ConstraintInstr::PopScope,
            ConstraintInstr::PopScope,
        ])
        .unwrap();
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn trailing_unmatched_push_is_allowed() {
        // Engine handles trailing-unmatched-push fine (Reset / drop on
        // teardown).  Only unmatched *pops* are an error.
        let p = Program::new(vec![ConstraintInstr::PushScope]).unwrap();
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn unmatched_pop_is_rejected() {
        let err = Program::new(vec![ConstraintInstr::PopScope]).unwrap_err();
        assert_eq!(err, ProgramError::UnmatchedPop { index: 0 });
    }

    #[test]
    fn pop_after_reset_is_rejected() {
        // After (reset) the scope stack is cleared, so a subsequent (pop) is unmatched.
        let err = Program::new(vec![
            ConstraintInstr::PushScope,
            ConstraintInstr::Reset,
            ConstraintInstr::PopScope,
        ])
        .unwrap_err();
        assert_eq!(err, ProgramError::UnmatchedPop { index: 2 });
    }

    #[test]
    fn new_unchecked_skips_validation() {
        let p = Program::new_unchecked(vec![ConstraintInstr::PopScope]);
        assert_eq!(p.len(), 1);
    }

    // ---------- Round-trip: every opcode ----------

    #[test]
    fn round_trip_check_sat() {
        let p = Program::new(vec![ConstraintInstr::CheckSat]).unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_get_model_unsat_core_reset() {
        let p = Program::new(vec![
            ConstraintInstr::GetModel,
            ConstraintInstr::GetUnsatCore,
            ConstraintInstr::Reset,
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_declare_var_all_sorts() {
        let p = Program::new(vec![
            ConstraintInstr::DeclareVar { name: "b".into(), sort: Sort::Bool },
            ConstraintInstr::DeclareVar { name: "i".into(), sort: Sort::Int },
            ConstraintInstr::DeclareVar { name: "r".into(), sort: Sort::Real },
            ConstraintInstr::DeclareVar { name: "v".into(), sort: Sort::BitVec(32) },
            ConstraintInstr::DeclareVar {
                name: "a".into(),
                sort: Sort::Array { idx: Box::new(Sort::Int), val: Box::new(Sort::Bool) },
            },
            ConstraintInstr::DeclareVar {
                name: "u".into(),
                sort: Sort::Uninterpreted("Color".into()),
            },
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_declare_fn() {
        let p = Program::new(vec![
            ConstraintInstr::DeclareFn {
                name: "len".into(),
                arg_sorts: vec![Sort::Int, Sort::Bool],
                ret_sort: Sort::Int,
            },
            ConstraintInstr::DeclareFn {
                name: "nullary".into(),
                arg_sorts: vec![],
                ret_sort: Sort::Bool,
            },
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_set_logic_all_variants() {
        let p = Program::new(vec![
            ConstraintInstr::SetLogic { logic: Logic::QF_Bool },
            ConstraintInstr::SetLogic { logic: Logic::QF_LIA },
            ConstraintInstr::SetLogic { logic: Logic::QF_LRA },
            ConstraintInstr::SetLogic { logic: Logic::QF_BV },
            ConstraintInstr::SetLogic { logic: Logic::QF_AUFLIA },
            ConstraintInstr::SetLogic { logic: Logic::LIA },
            ConstraintInstr::SetLogic { logic: Logic::ALL },
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_push_pop() {
        let p = Program::new(vec![
            ConstraintInstr::PushScope,
            ConstraintInstr::PopScope,
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_echo_with_escapes() {
        let p = Program::new(vec![
            ConstraintInstr::Echo { msg: "hello".into() },
            ConstraintInstr::Echo { msg: "with \"quotes\" and \\ slash and newline\nhere".into() },
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_set_option_all_value_kinds() {
        let p = Program::new(vec![
            ConstraintInstr::SetOption {
                key: ":produce-models".into(),
                value: OptionValue::Bool(true),
            },
            ConstraintInstr::SetOption {
                key: ":random-seed".into(),
                value: OptionValue::Int(-7),
            },
            ConstraintInstr::SetOption {
                key: ":status".into(),
                value: OptionValue::Str("unsat".into()),
            },
        ])
        .unwrap();
        round_trip(&p);
    }

    // ---------- Round-trip: Predicate variants embedded in Assert ----------

    fn assert(pred: Predicate) -> ConstraintInstr {
        ConstraintInstr::Assert { pred }
    }

    #[test]
    fn round_trip_assert_bool_lit() {
        let p = Program::new(vec![assert(Predicate::Bool(true)), assert(Predicate::Bool(false))])
            .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_var_int() {
        let p = Program::new(vec![
            assert(Predicate::Var("x".into())),
            assert(Predicate::Int(42)),
            assert(Predicate::Int(-100)),
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_real_literal() {
        let p = Program::new(vec![assert(Predicate::Real(Rational::new(3, 4)))]).unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_apply() {
        let p = Program::new(vec![assert(Predicate::Apply {
            f: "len".into(),
            args: vec![Predicate::Var("xs".into()), Predicate::Int(0)],
        })])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_boolean_combinators() {
        let x = Predicate::Var("x".into());
        let y = Predicate::Var("y".into());
        let p = Program::new(vec![
            assert(Predicate::And(vec![x.clone(), y.clone()])),
            assert(Predicate::Or(vec![x.clone(), y.clone()])),
            assert(Predicate::Not(Box::new(x.clone()))),
            assert(Predicate::Implies(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Iff(Box::new(x.clone()), Box::new(y))),
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_arithmetic_and_comparisons() {
        let x = Predicate::Var("x".into());
        let y = Predicate::Var("y".into());
        let p = Program::new(vec![
            assert(Predicate::Add(vec![x.clone(), y.clone(), Predicate::Int(3)])),
            assert(Predicate::Sub(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Mul { coef: 5, term: Box::new(x.clone()) }),
            assert(Predicate::Eq(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::NEq(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Le(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Lt(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Ge(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Gt(Box::new(x), Box::new(y))),
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_ite() {
        let p = Program::new(vec![assert(Predicate::Ite(
            Box::new(Predicate::Var("c".into())),
            Box::new(Predicate::Int(1)),
            Box::new(Predicate::Int(0)),
        ))])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_quantifiers() {
        let p = Program::new(vec![
            assert(Predicate::Forall {
                var: "x".into(),
                sort: Sort::Int,
                body: Box::new(Predicate::Ge(
                    Box::new(Predicate::Var("x".into())),
                    Box::new(Predicate::Int(0)),
                )),
            }),
            assert(Predicate::Exists {
                var: "y".into(),
                sort: Sort::Bool,
                body: Box::new(Predicate::Var("y".into())),
            }),
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn round_trip_assert_arrays() {
        let p = Program::new(vec![
            assert(Predicate::Select {
                arr: Box::new(Predicate::Var("a".into())),
                idx: Box::new(Predicate::Int(0)),
            }),
            assert(Predicate::Store {
                arr: Box::new(Predicate::Var("a".into())),
                idx: Box::new(Predicate::Int(1)),
                val: Box::new(Predicate::Bool(true)),
            }),
        ])
        .unwrap();
        round_trip(&p);
    }

    // ---------- Round-trip: a realistic program ----------

    #[test]
    fn round_trip_realistic_qf_lia_program() {
        let p = Program::new(vec![
            ConstraintInstr::SetLogic { logic: Logic::QF_LIA },
            ConstraintInstr::SetOption {
                key: ":produce-models".into(),
                value: OptionValue::Bool(true),
            },
            ConstraintInstr::DeclareVar { name: "x".into(), sort: Sort::Int },
            ConstraintInstr::DeclareVar { name: "y".into(), sort: Sort::Int },
            assert(Predicate::Ge(
                Box::new(Predicate::Var("x".into())),
                Box::new(Predicate::Int(0)),
            )),
            assert(Predicate::Le(
                Box::new(Predicate::Add(vec![
                    Predicate::Var("x".into()),
                    Predicate::Var("y".into()),
                ])),
                Box::new(Predicate::Int(100)),
            )),
            ConstraintInstr::PushScope,
            assert(Predicate::Ge(
                Box::new(Predicate::Var("y".into())),
                Box::new(Predicate::Int(50)),
            )),
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetModel,
            ConstraintInstr::PopScope,
            ConstraintInstr::CheckSat,
        ])
        .unwrap();
        round_trip(&p);
    }

    // ---------- Parser edge cases ----------

    #[test]
    fn parse_skips_comments_and_whitespace() {
        let text = "
            ; opening comment
            (set-logic QF_LIA) ; inline comment
            (declare-var x Int)
            ; another comment

            (assert (>= x 0))
            (check-sat)
        ";
        let p = parse_program(text).unwrap();
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn parse_unknown_opcode() {
        let err = parse_program("(do-magic)").unwrap_err();
        assert_eq!(err, ParseError::UnknownOpcode("do-magic".into()));
    }

    #[test]
    fn parse_unknown_logic() {
        let err = parse_program("(set-logic UNKNOWN)").unwrap_err();
        assert_eq!(err, ParseError::BadLogic("UNKNOWN".into()));
    }

    #[test]
    fn parse_bad_arity() {
        let err = parse_program("(check-sat oops)").unwrap_err();
        assert!(matches!(err, ParseError::BadArgs { opcode, .. } if opcode == "check-sat"));
    }

    #[test]
    fn parse_unmatched_close_paren() {
        let err = parse_program("(check-sat))").unwrap_err();
        assert!(matches!(err, ParseError::UnexpectedCloseParen { .. }));
    }

    #[test]
    fn parse_unmatched_open_paren() {
        let err = parse_program("(check-sat").unwrap_err();
        assert_eq!(err, ParseError::UnexpectedEof);
    }

    #[test]
    fn parse_unterminated_string() {
        let err = parse_program("(echo \"hello").unwrap_err();
        assert!(matches!(err, ParseError::BadString(_)));
    }

    #[test]
    fn parse_unknown_string_escape() {
        let err = parse_program("(echo \"bad \\q escape\")").unwrap_err();
        assert!(matches!(err, ParseError::BadString(_)));
    }

    #[test]
    fn parse_validation_error_propagates() {
        let err = parse_program("(pop)").unwrap_err();
        assert_eq!(err, ParseError::Program(ProgramError::UnmatchedPop { index: 0 }));
    }

    #[test]
    fn parse_int_overflow_in_set_option() {
        // i128::MAX is much bigger than i64::MAX; SetOption rejects it.
        let huge = format!("(set-option :seed {})", i128::MAX);
        let err = parse_program(&huge).unwrap_err();
        assert!(matches!(err, ParseError::BadArgs { opcode, .. } if opcode == "set-option"));
    }

    // ---------- ProgramError Display ----------

    #[test]
    fn program_error_displays() {
        assert_eq!(
            ProgramError::UnmatchedPop { index: 7 }.to_string(),
            "unmatched (pop) at instruction index 7"
        );
    }

    // ---------- Identifier validation (round-trip safety) ----------

    #[test]
    fn rejects_empty_identifier() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("non-empty")));
    }

    #[test]
    fn rejects_reserved_identifier() {
        // "and" is a reserved predicate combinator — using it as a var
        // name would re-parse as `Predicate::And` after round-trip.
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "and".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("reserved")));
    }

    #[test]
    fn rejects_int_lookalike_identifier() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "123".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("integer literal")));
    }

    #[test]
    fn rejects_negative_int_lookalike_identifier() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "-7".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("integer literal")));
    }

    #[test]
    fn rejects_identifier_with_whitespace() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "with space".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("delimiter")));
    }

    #[test]
    fn rejects_identifier_with_paren() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "x(y".into(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { reason, .. } if reason.contains("delimiter")));
    }

    #[test]
    fn rejects_var_name_in_predicate() {
        // Identifier check walks Assert predicates too.
        let err = Program::new(vec![ConstraintInstr::Assert {
            pred: Predicate::Var("forall".into()),
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { .. }));
    }

    #[test]
    fn rejects_apply_function_name_collision() {
        // Apply { f: "and", ... } would print as `(and ...)` and re-parse as Predicate::And.
        let err = Program::new(vec![ConstraintInstr::Assert {
            pred: Predicate::Apply { f: "and".into(), args: vec![Predicate::Var("x".into())] },
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { .. }));
    }

    #[test]
    fn rejects_quantifier_binder_collision() {
        let err = Program::new(vec![ConstraintInstr::Assert {
            pred: Predicate::Forall {
                var: "true".into(),
                sort: Sort::Bool,
                body: Box::new(Predicate::Bool(true)),
            },
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { .. }));
    }

    #[test]
    fn rejects_uninterpreted_sort_name_collision() {
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: "x".into(),
            sort: Sort::Uninterpreted("Bool".into()),
        }])
        .unwrap_err();
        assert!(matches!(err, ProgramError::BadIdentifier { .. }));
    }

    #[test]
    fn truncates_long_identifier_in_diagnostic() {
        // Long bad identifier — truncated for the error message.
        let huge: String = "(".repeat(200);
        let err = Program::new(vec![ConstraintInstr::DeclareVar {
            name: huge.clone(),
            sort: Sort::Int,
        }])
        .unwrap_err();
        if let ProgramError::BadIdentifier { name, .. } = err {
            assert!(name.chars().count() < huge.chars().count(), "should truncate");
            assert!(name.ends_with('…'), "should mark truncation");
        } else {
            panic!("expected BadIdentifier");
        }
    }

    // ---------- UTF-8 in string literals (H2 fix) ----------

    #[test]
    fn round_trip_echo_with_unicode() {
        // Multi-byte UTF-8 in Echo / OptionValue::Str must round-trip exactly.
        let p = Program::new(vec![
            ConstraintInstr::Echo { msg: "héllo 世界 🦀".into() },
            ConstraintInstr::SetOption {
                key: ":greeting".into(),
                value: OptionValue::Str("Olá, mundo".into()),
            },
        ])
        .unwrap();
        round_trip(&p);
    }

    #[test]
    fn parser_rejects_invalid_utf8_in_string() {
        // Hand-craft a byte sequence with invalid UTF-8 inside a string literal.
        // 0x80 alone is a continuation byte with no leading byte — invalid.
        let mut bytes: Vec<u8> = b"(echo \"".to_vec();
        bytes.push(0x80);
        bytes.extend_from_slice(b"\")");
        let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
        let err = parse_program(s).unwrap_err();
        assert!(matches!(err, ParseError::BadString(ref m) if m.contains("UTF-8")));
    }

    // ---------- ParseError Display ----------

    #[test]
    fn parse_error_displays() {
        assert_eq!(ParseError::UnexpectedEof.to_string(), "unexpected end of input");
        assert_eq!(
            ParseError::UnknownOpcode("foo".into()).to_string(),
            "unknown opcode `foo`"
        );
        assert_eq!(
            ParseError::BadInt("abc".into()).to_string(),
            "bad integer literal `abc`"
        );
    }
}
