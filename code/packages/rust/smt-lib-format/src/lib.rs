//! # `smt-lib-format` — strict SMT-LIB v2 reader/writer.
//!
//! **LANG24 PR 24-E**.  Industry-standard textual format over the
//! [`constraint_instructions::Program`] IR.  Lets the constraint-VM
//! interoperate with industry solvers (Z3, CVC5) and run the
//! standard SMT-LIB benchmark suite as part of CI.
//!
//! ## Position in the stack
//!
//! ```text
//! SMT-LIB v2 textual file (.smt2)
//!         │
//!         ▼
//!   smt-lib-format            ← this crate
//!         │
//!         ▼
//!   constraint-instructions::Program
//!         │
//!         ▼
//!   constraint-vm (PR 24-D) / Z3 bridge (PR 24-H) / debug-sidecar
//! ```
//!
//! ## Coverage (v1 scope)
//!
//! Commands handled by both reader and writer:
//!
//! | SMT-LIB v2 command           | constraint-instructions opcode |
//! |------------------------------|-------------------------------|
//! | `(set-logic L)`              | `SetLogic`                    |
//! | `(set-option :k v)`          | `SetOption`                   |
//! | `(declare-const x S)`        | `DeclareVar`                  |
//! | `(declare-fun x () S)`       | `DeclareVar`                  |
//! | `(declare-fun f (S₁ … Sₙ) S)`| `DeclareFn`                   |
//! | `(assert φ)`                 | `Assert`                      |
//! | `(check-sat)`                | `CheckSat`                    |
//! | `(get-model)`                | `GetModel`                    |
//! | `(get-unsat-core)`           | `GetUnsatCore`                |
//! | `(push)` / `(push N)`        | `PushScope` (N copies)        |
//! | `(pop)`  / `(pop N)`         | `PopScope`  (N copies)        |
//! | `(reset)`                    | `Reset`                       |
//! | `(echo "msg")`               | `Echo`                        |
//!
//! Predicate vocabulary handled (covers SAT + LIA + arrays +
//! quantifiers — the v1 LANG24 logics):
//!
//! | SMT-LIB form                | `Predicate` variant           |
//! |-----------------------------|-------------------------------|
//! | `true` / `false`            | `Bool`                        |
//! | `<int-literal>`             | `Int`                         |
//! | `(- n)` / `<dec-literal>`   | `Int`(negative) / `Real`      |
//! | `(/ n d)`                   | `Real(Rational)`              |
//! | `<symbol>`                  | `Var`                         |
//! | `(and …)` `(or …)` `(not …)`| `And` / `Or` / `Not`          |
//! | `(=> a b)`                  | `Implies`                     |
//! | `(= a b)`                   | `Eq`                          |
//! | `(distinct a b)`            | `NEq`                         |
//! | `(+ …)` / `(- a b)`         | `Add` / `Sub`                 |
//! | `(* k v)` (k integer, v term)| `Mul` (linear)               |
//! | `(<= a b)` / `(<)` / `(>=)` / `(>)`| `Le` / `Lt` / `Ge` / `Gt` |
//! | `(ite c t e)`               | `Ite`                         |
//! | `(forall ((x S)) φ)`        | `Forall` (one binder)         |
//! | `(exists ((x S)) φ)`        | `Exists` (one binder)         |
//! | `(select arr idx)`          | `Select`                      |
//! | `(store arr idx val)`       | `Store`                       |
//! | `(f a₁ … aₙ)`               | `Apply`                       |
//!
//! Sort vocabulary: `Bool`, `Int`, `Real`, `(_ BitVec w)`,
//! `(Array idx-sort val-sort)`, plus uninterpreted symbols.
//!
//! ## SMT-LIB ↔ constraint-instructions divergences
//!
//! Three intentional differences — see the constraint-instructions
//! README for *why* the internal IR diverges; this crate exists to
//! bridge them:
//!
//! - **`Iff`**: SMT-LIB writes `(= a b)` for both `Eq` and `Iff`
//!   (overloaded on `Bool` operands).  This crate's reader always
//!   parses `(=` as `Eq`; the writer always emits `(=` for both
//!   `Eq` and `Iff`.  Round-trip from `Iff` is therefore lossy
//!   (reads back as `Eq`).  Document; users wanting to preserve
//!   `Iff` should use `constraint-instructions`'s own text format.
//! - **`Real(num/den)`**: SMT-LIB has bare `n/d` decimal literals
//!   *and* `(/ n d)`; this crate writes the s-expression form for
//!   exact round-trip.  Bare decimals are also accepted on read.
//! - **`BitVec(w)`**: SMT-LIB uses the indexed-identifier form
//!   `(_ BitVec w)`; this crate handles it on both sides.
//!
//! ## Multi-binder quantifiers
//!
//! SMT-LIB allows `(forall ((x S₁) (y S₂)) φ)` (multi-binder).
//! `Predicate::Forall` / `Exists` only carry one binder, so the
//! reader **lowers** multi-binder quantifiers to nested
//! single-binder ones.  The writer emits one binder per quantifier
//! (the simpler form).
//!
//! ## Out of scope (v1)
//!
//! - `let`-bindings (will be expanded later, per LANG24 §"Out of
//!   scope").
//! - `define-fun`, `define-sort`, `declare-datatypes`.
//! - String theory.
//! - Floating-point theory.
//! - `set-info`, `get-info`, `get-assignment`, `get-proof`,
//!   `get-assertions`, `get-value`, `simplify`.
//! - `as` ascription on terms.
//!
//! These extensions land in subsequent v2/v3 PRs as their use-cases
//! arrive (LANG24 §"Theories supported (versioned scope)").
//!
//! ## Caller responsibilities
//!
//! Inherits all `constraint-instructions` non-guarantees (predicate
//! depth, parser/`Display` recursion).  In addition, the SMT-LIB
//! reader recurses on parenthesis depth without an explicit guard
//! and accepts arbitrarily long input — bound at the boundary when
//! ingesting untrusted SMT-LIB files.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::fmt;

use constraint_core::{Logic, Predicate, Rational, Sort};
use constraint_instructions::{ConstraintInstr, OptionValue, Program, ProgramError};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by [`read`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SmtLibError {
    /// Unexpected end of input partway through a form.
    UnexpectedEof,
    /// Saw an unmatched `)`.
    UnexpectedCloseParen {
        /// Byte offset of the offending paren.
        offset: usize,
    },
    /// A string literal was unterminated, malformed, or invalid UTF-8.
    BadString(String),
    /// An integer literal failed to parse.
    BadInt(String),
    /// A SMT-LIB command was given the wrong number / shape of arguments.
    BadCommand {
        /// Command name (e.g. `"declare-fun"`).
        command: String,
        /// Diagnostic detail.
        detail: String,
    },
    /// An unrecognised top-level command.
    UnknownCommand(String),
    /// A `Sort` token was unrecognised.
    BadSort(String),
    /// A `Logic` token was unrecognised.
    BadLogic(String),
    /// A predicate term was malformed.
    BadTerm(String),
    /// Validation failed after parsing.
    Program(ProgramError),
    /// Parenthesis nesting exceeded the configured depth cap.
    /// See [`DEFAULT_MAX_DEPTH`] / [`read_with_limit`].
    TooDeep {
        /// Depth that triggered the rejection.
        depth: usize,
        /// Configured cap.
        max: usize,
    },
}

impl fmt::Display for SmtLibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmtLibError::UnexpectedEof => write!(f, "unexpected end of input"),
            SmtLibError::UnexpectedCloseParen { offset } => {
                write!(f, "unexpected `)` at byte offset {offset}")
            }
            SmtLibError::BadString(s) => write!(f, "bad string literal: {s}"),
            SmtLibError::BadInt(s) => write!(f, "bad integer literal `{s}`"),
            SmtLibError::BadCommand { command, detail } => {
                write!(f, "bad arguments to `{command}`: {detail}")
            }
            SmtLibError::UnknownCommand(s) => write!(f, "unknown command `{s}`"),
            SmtLibError::BadSort(s) => write!(f, "unknown sort `{s}`"),
            SmtLibError::BadLogic(s) => write!(f, "unknown logic `{s}`"),
            SmtLibError::BadTerm(s) => write!(f, "bad term: {s}"),
            SmtLibError::Program(e) => write!(f, "{e}"),
            SmtLibError::TooDeep { depth, max } => {
                write!(f, "parenthesis nesting depth {depth} exceeds cap {max}")
            }
        }
    }
}

impl std::error::Error for SmtLibError {}

impl From<ProgramError> for SmtLibError {
    fn from(e: ProgramError) -> Self {
        SmtLibError::Program(e)
    }
}

// ---------------------------------------------------------------------------
// Public API: read / write
// ---------------------------------------------------------------------------

/// Default maximum parenthesis-nesting depth for [`read`].  Above
/// this, the reader returns [`SmtLibError::TooDeep`] rather than
/// risking a stack overflow.
///
/// Real-world SMT-LIB benchmarks rarely exceed depth 50; the cap
/// is generous (1024) to accept Z3's regression suite while
/// rejecting adversarial `((((…))))` inputs.  Use
/// [`read_with_limit`] to override.
pub const DEFAULT_MAX_DEPTH: usize = 1024;

/// Read a strict SMT-LIB v2 program and return a validated
/// [`Program`].  Multi-binder quantifiers are lowered to nested
/// single-binder ones.  Multi-count `(push N)` / `(pop N)` are
/// expanded to N copies of `PushScope` / `PopScope`.
///
/// Parenthesis nesting is bounded at [`DEFAULT_MAX_DEPTH`].  Use
/// [`read_with_limit`] for a custom cap.
pub fn read(input: &str) -> Result<Program, SmtLibError> {
    read_with_limit(input, DEFAULT_MAX_DEPTH)
}

/// Like [`read`] but with a caller-supplied parenthesis-nesting
/// cap.  Pass [`usize::MAX`] to disable the cap (only safe for
/// trusted input).
pub fn read_with_limit(input: &str, max_depth: usize) -> Result<Program, SmtLibError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens: &tokens, pos: 0, depth: 0, max_depth };
    let mut instrs = Vec::new();
    while !parser.at_end() {
        let form = parser.parse_form()?;
        push_command(form, &mut instrs)?;
    }
    Ok(Program::new(instrs)?)
}

/// Write a [`Program`] in strict SMT-LIB v2 textual form.
///
/// The output is one command per line.  Round-trip with [`read`]
/// is exact for any program whose `Predicate`s avoid `Iff` (which
/// SMT-LIB overloads with `Eq` — see the crate-level docs).
pub fn write(program: &Program) -> String {
    let mut out = String::new();
    for (i, instr) in program.instructions().iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        write_instr(&mut out, instr);
    }
    out
}

fn write_instr(out: &mut String, instr: &ConstraintInstr) {
    match instr {
        ConstraintInstr::DeclareVar { name, sort } => {
            // SMT-LIB prefers `(declare-const name sort)`; the equivalent
            // long form is `(declare-fun name () sort)`.  Use the short
            // form on the writer side; the reader accepts both.
            out.push_str("(declare-const ");
            out.push_str(name);
            out.push(' ');
            write_sort(out, sort);
            out.push(')');
        }
        ConstraintInstr::DeclareFn { name, arg_sorts, ret_sort } => {
            out.push_str("(declare-fun ");
            out.push_str(name);
            out.push_str(" (");
            for (i, s) in arg_sorts.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_sort(out, s);
            }
            out.push_str(") ");
            write_sort(out, ret_sort);
            out.push(')');
        }
        ConstraintInstr::Assert { pred } => {
            out.push_str("(assert ");
            write_predicate(out, pred);
            out.push(')');
        }
        ConstraintInstr::CheckSat => out.push_str("(check-sat)"),
        ConstraintInstr::GetModel => out.push_str("(get-model)"),
        ConstraintInstr::GetUnsatCore => out.push_str("(get-unsat-core)"),
        ConstraintInstr::PushScope => out.push_str("(push 1)"),
        ConstraintInstr::PopScope => out.push_str("(pop 1)"),
        ConstraintInstr::Reset => out.push_str("(reset)"),
        ConstraintInstr::SetLogic { logic } => {
            out.push_str("(set-logic ");
            out.push_str(&format!("{logic}"));
            out.push(')');
        }
        ConstraintInstr::Echo { msg } => {
            out.push_str("(echo \"");
            out.push_str(&escape_string(msg));
            out.push_str("\")");
        }
        ConstraintInstr::SetOption { key, value } => {
            out.push_str("(set-option ");
            out.push_str(key);
            out.push(' ');
            write_option_value(out, value);
            out.push(')');
        }
        // `ConstraintInstr` is `#[non_exhaustive]`; future opcodes
        // need their own writer arm.  Until then, emit a
        // parser-illegal placeholder so we never silently produce
        // unreadable text.
        other => out.push_str(&format!("<unsupported-instr:{other:?}>")),
    }
}

fn write_option_value(out: &mut String, v: &OptionValue) {
    match v {
        OptionValue::Bool(true) => out.push_str("true"),
        OptionValue::Bool(false) => out.push_str("false"),
        OptionValue::Int(n) => out.push_str(&n.to_string()),
        OptionValue::Str(s) => {
            out.push('"');
            out.push_str(&escape_string(s));
            out.push('"');
        }
        // `OptionValue` is `#[non_exhaustive]`.
        other => out.push_str(&format!("<unsupported-option-value:{other:?}>")),
    }
}

fn write_sort(out: &mut String, sort: &Sort) {
    match sort {
        Sort::Bool => out.push_str("Bool"),
        Sort::Int => out.push_str("Int"),
        Sort::Real => out.push_str("Real"),
        Sort::BitVec(w) => out.push_str(&format!("(_ BitVec {w})")),
        Sort::Array { idx, val } => {
            out.push_str("(Array ");
            write_sort(out, idx);
            out.push(' ');
            write_sort(out, val);
            out.push(')');
        }
        Sort::Uninterpreted(name) => out.push_str(name),
        // `Sort` is `#[non_exhaustive]`; new variants need their
        // own arm.  Emit a parser-illegal placeholder so we never
        // silently round-trip something we can't read.
        other => out.push_str(&format!("<unsupported-sort:{other:?}>")),
    }
}

fn write_predicate(out: &mut String, p: &Predicate) {
    match p {
        Predicate::Bool(true) => out.push_str("true"),
        Predicate::Bool(false) => out.push_str("false"),
        Predicate::Var(name) => out.push_str(name),
        Predicate::Int(n) => {
            // SMT-LIB integer literals: positive bare, negative as `(- N)`.
            if *n < 0 {
                out.push_str("(- ");
                // Negate carefully — i128::MIN is the only value whose
                // negation overflows, and Rational::new already rejects
                // i128::MIN, so a `Predicate::Int(i128::MIN)` reaching
                // here is a caller-side bug.  Defensive: format the
                // absolute via wrapping, which gives the correct text
                // for the only edge case that matters.
                let abs = n.unsigned_abs();
                out.push_str(&abs.to_string());
                out.push(')');
            } else {
                out.push_str(&n.to_string());
            }
        }
        Predicate::Real(r) => {
            // SMT-LIB rational literal: `(/ num den)` is unambiguous
            // and round-trips; bare `n/d` decimals are also legal but
            // we always emit the s-expression form.
            if r.den == 1 {
                if r.num < 0 {
                    out.push_str("(- ");
                    out.push_str(&r.num.unsigned_abs().to_string());
                    out.push_str(".0)");
                } else {
                    out.push_str(&format!("{}.0", r.num));
                }
            } else {
                out.push_str(&format!("(/ {} {})", r.num, r.den));
            }
        }
        Predicate::Apply { f: name, args } => write_app(out, name, args),
        Predicate::And(parts) => write_app(out, "and", parts),
        Predicate::Or(parts) => write_app(out, "or", parts),
        Predicate::Not(inner) => {
            out.push_str("(not ");
            write_predicate(out, inner);
            out.push(')');
        }
        Predicate::Implies(a, b) => write_binary(out, "=>", a, b),
        Predicate::Iff(a, b) => write_binary(out, "=", a, b),
        Predicate::Eq(a, b) => write_binary(out, "=", a, b),
        Predicate::NEq(a, b) => write_binary(out, "distinct", a, b),
        Predicate::Add(parts) => write_app(out, "+", parts),
        Predicate::Sub(a, b) => write_binary(out, "-", a, b),
        Predicate::Mul { coef, term } => {
            out.push_str("(* ");
            if *coef < 0 {
                out.push_str("(- ");
                out.push_str(&coef.unsigned_abs().to_string());
                out.push(')');
            } else {
                out.push_str(&coef.to_string());
            }
            out.push(' ');
            write_predicate(out, term);
            out.push(')');
        }
        Predicate::Le(a, b) => write_binary(out, "<=", a, b),
        Predicate::Lt(a, b) => write_binary(out, "<", a, b),
        Predicate::Ge(a, b) => write_binary(out, ">=", a, b),
        Predicate::Gt(a, b) => write_binary(out, ">", a, b),
        Predicate::Ite(c, t, e) => {
            out.push_str("(ite ");
            write_predicate(out, c);
            out.push(' ');
            write_predicate(out, t);
            out.push(' ');
            write_predicate(out, e);
            out.push(')');
        }
        Predicate::Forall { var, sort, body } => {
            write_quantifier(out, "forall", var, sort, body)
        }
        Predicate::Exists { var, sort, body } => {
            write_quantifier(out, "exists", var, sort, body)
        }
        Predicate::Select { arr, idx } => {
            out.push_str("(select ");
            write_predicate(out, arr);
            out.push(' ');
            write_predicate(out, idx);
            out.push(')');
        }
        Predicate::Store { arr, idx, val } => {
            out.push_str("(store ");
            write_predicate(out, arr);
            out.push(' ');
            write_predicate(out, idx);
            out.push(' ');
            write_predicate(out, val);
            out.push(')');
        }
        // `Predicate` is `#[non_exhaustive]`.
        other => out.push_str(&format!("<unsupported:{other:?}>")),
    }
}

fn write_app(out: &mut String, head: &str, args: &[Predicate]) {
    out.push('(');
    out.push_str(head);
    for a in args {
        out.push(' ');
        write_predicate(out, a);
    }
    out.push(')');
}

fn write_binary(out: &mut String, op: &str, a: &Predicate, b: &Predicate) {
    out.push('(');
    out.push_str(op);
    out.push(' ');
    write_predicate(out, a);
    out.push(' ');
    write_predicate(out, b);
    out.push(')');
}

fn write_quantifier(out: &mut String, kind: &str, var: &str, sort: &Sort, body: &Predicate) {
    out.push('(');
    out.push_str(kind);
    out.push_str(" ((");
    out.push_str(var);
    out.push(' ');
    write_sort(out, sort);
    out.push_str(")) ");
    write_predicate(out, body);
    out.push(')');
}

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
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Open(usize),
    Close(usize),
    Symbol(String),
    Int(i128),
    Decimal(String), // textual `n.d` form, parsed lazily for round-trip safety
    Str(String),
    Keyword(String), // SMT-LIB `:keyword` (used for SetOption keys)
}

fn tokenize(input: &str) -> Result<Vec<Token>, SmtLibError> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b';' => {
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
            b':' => {
                // SMT-LIB keyword: `:produce-models` etc.  Read like an
                // atom but tag as Keyword so we treat it specially.
                let (s, end) = read_atom(bytes, i)?;
                out.push(Token::Keyword(s));
                i = end;
            }
            _ => {
                let (s, end) = read_atom(bytes, i)?;
                if let Some(n) = parse_int(&s) {
                    out.push(Token::Int(n));
                } else if is_decimal_literal(&s) {
                    out.push(Token::Decimal(s));
                } else {
                    out.push(Token::Symbol(s));
                }
                i = end;
            }
        }
    }
    Ok(out)
}

fn read_string(bytes: &[u8], start: usize) -> Result<(String, usize), SmtLibError> {
    debug_assert_eq!(bytes[start], b'"');
    // Collect raw bytes; decode UTF-8 once at end (avoids per-byte
    // truncation of multi-byte sequences).
    let mut buf: Vec<u8> = Vec::new();
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                // SMT-LIB v2.6 supports `""` as an embedded `"` (no
                // backslash escape).  Check for that here.
                if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                    buf.push(b'"');
                    i += 2;
                    continue;
                }
                let s = std::str::from_utf8(&buf)
                    .map_err(|e| SmtLibError::BadString(format!("invalid UTF-8: {e}")))?
                    .to_owned();
                return Ok((s, i + 1));
            }
            b'\\' => {
                if i + 1 >= bytes.len() {
                    return Err(SmtLibError::BadString("unterminated escape".into()));
                }
                match bytes[i + 1] {
                    b'"' => buf.push(b'"'),
                    b'\\' => buf.push(b'\\'),
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    other => {
                        return Err(SmtLibError::BadString(format!(
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
    Err(SmtLibError::BadString("unterminated string literal".into()))
}

fn read_atom(bytes: &[u8], start: usize) -> Result<(String, usize), SmtLibError> {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')' | b';' | b'"') {
            break;
        }
        i += 1;
    }
    let s = std::str::from_utf8(&bytes[start..i])
        .map_err(|e| SmtLibError::BadString(format!("invalid UTF-8 in atom: {e}")))?
        .to_owned();
    Ok((s, i))
}

fn parse_int(s: &str) -> Option<i128> {
    if s.is_empty() {
        return None;
    }
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

fn is_decimal_literal(s: &str) -> bool {
    // SMT-LIB v2 decimals: `<digits>.<digits>` (both sides required).
    let mut parts = s.splitn(2, '.');
    let l = parts.next().unwrap_or("");
    let r = parts.next().unwrap_or("");
    !l.is_empty()
        && !r.is_empty()
        && l.chars().all(|c| c.is_ascii_digit())
        && r.chars().all(|c| c.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Parser — recursive-descent over the token stream
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Form {
    Atom(Token),
    List(Vec<Form>),
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    max_depth: usize,
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

    fn parse_form(&mut self) -> Result<Form, SmtLibError> {
        match self.advance() {
            None => Err(SmtLibError::UnexpectedEof),
            Some(Token::Open(_)) => {
                self.depth = self.depth.saturating_add(1);
                if self.depth > self.max_depth {
                    return Err(SmtLibError::TooDeep {
                        depth: self.depth,
                        max: self.max_depth,
                    });
                }
                let mut items = Vec::new();
                loop {
                    match self.peek() {
                        None => return Err(SmtLibError::UnexpectedEof),
                        Some(Token::Close(_)) => {
                            self.advance();
                            self.depth -= 1;
                            return Ok(Form::List(items));
                        }
                        Some(_) => items.push(self.parse_form()?),
                    }
                }
            }
            Some(Token::Close(o)) => Err(SmtLibError::UnexpectedCloseParen { offset: *o }),
            Some(t) => Ok(Form::Atom(t.clone())),
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level command dispatch
// ---------------------------------------------------------------------------

fn push_command(form: Form, out: &mut Vec<ConstraintInstr>) -> Result<(), SmtLibError> {
    let items = match form {
        Form::List(items) => items,
        Form::Atom(_) => {
            return Err(SmtLibError::BadCommand {
                command: "<top-level>".into(),
                detail: "expected a list".into(),
            });
        }
    };
    let mut iter = items.into_iter();
    let head = iter.next().ok_or(SmtLibError::UnexpectedEof)?;
    let cmd = match head {
        Form::Atom(Token::Symbol(s)) => s,
        _ => {
            return Err(SmtLibError::BadCommand {
                command: "<unknown>".into(),
                detail: "first element must be a command symbol".into(),
            });
        }
    };
    let args: Vec<Form> = iter.collect();
    match cmd.as_str() {
        "set-logic" => out.push(parse_set_logic(args)?),
        "set-option" => out.push(parse_set_option(args)?),
        "declare-const" => out.push(parse_declare_const(args)?),
        "declare-fun" => out.push(parse_declare_fun(args)?),
        "assert" => out.push(parse_assert(args)?),
        "check-sat" => {
            expect_arity("check-sat", &args, 0)?;
            out.push(ConstraintInstr::CheckSat);
        }
        "get-model" => {
            expect_arity("get-model", &args, 0)?;
            out.push(ConstraintInstr::GetModel);
        }
        "get-unsat-core" => {
            expect_arity("get-unsat-core", &args, 0)?;
            out.push(ConstraintInstr::GetUnsatCore);
        }
        "push" => parse_push_pop(args, true, out)?,
        "pop" => parse_push_pop(args, false, out)?,
        "reset" => {
            expect_arity("reset", &args, 0)?;
            out.push(ConstraintInstr::Reset);
        }
        "echo" => out.push(parse_echo(args)?),
        // Common SMT-LIB commands we silently accept-and-skip for v1
        // (until later PRs add explicit support).  Keeping them as a
        // hard error makes parsing real-world benchmarks impossible.
        "set-info" | "exit" => { /* ignore */ }
        other => return Err(SmtLibError::UnknownCommand(other.to_owned())),
    }
    Ok(())
}

fn expect_arity(cmd: &str, args: &[Form], n: usize) -> Result<(), SmtLibError> {
    if args.len() != n {
        Err(SmtLibError::BadCommand {
            command: cmd.to_owned(),
            detail: format!("expected {} args, got {}", n, args.len()),
        })
    } else {
        Ok(())
    }
}

fn parse_set_logic(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("set-logic", &args, 1)?;
    let sym = atom_to_symbol(args.into_iter().next().unwrap(), "set-logic")?;
    let logic = match sym.as_str() {
        "QF_Bool" => Logic::QF_Bool,
        "QF_LIA" => Logic::QF_LIA,
        "QF_LRA" => Logic::QF_LRA,
        "QF_BV" => Logic::QF_BV,
        "QF_AUFLIA" => Logic::QF_AUFLIA,
        "LIA" => Logic::LIA,
        "ALL" => Logic::ALL,
        other => return Err(SmtLibError::BadLogic(other.to_owned())),
    };
    Ok(ConstraintInstr::SetLogic { logic })
}

fn parse_set_option(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("set-option", &args, 2)?;
    let mut iter = args.into_iter();
    let key_form = iter.next().unwrap();
    let key = match key_form {
        // SMT-LIB option keys are :colon-prefixed.
        Form::Atom(Token::Keyword(s)) => s,
        Form::Atom(Token::Symbol(s)) => s,
        _ => {
            return Err(SmtLibError::BadCommand {
                command: "set-option".into(),
                detail: "key must be a (keyword) symbol".into(),
            });
        }
    };
    let value = match iter.next().unwrap() {
        Form::Atom(Token::Symbol(s)) if s == "true" => OptionValue::Bool(true),
        Form::Atom(Token::Symbol(s)) if s == "false" => OptionValue::Bool(false),
        Form::Atom(Token::Int(n)) => {
            let n64: i64 = n.try_into().map_err(|_| SmtLibError::BadCommand {
                command: "set-option".into(),
                detail: format!("integer value {n} doesn't fit in i64"),
            })?;
            OptionValue::Int(n64)
        }
        Form::Atom(Token::Str(s)) => OptionValue::Str(s),
        _ => {
            return Err(SmtLibError::BadCommand {
                command: "set-option".into(),
                detail: "value must be bool / int / string".into(),
            });
        }
    };
    Ok(ConstraintInstr::SetOption { key, value })
}

fn parse_declare_const(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("declare-const", &args, 2)?;
    let mut iter = args.into_iter();
    let name = atom_to_symbol(iter.next().unwrap(), "declare-const")?;
    let sort = parse_sort(iter.next().unwrap())?;
    Ok(ConstraintInstr::DeclareVar { name, sort })
}

fn parse_declare_fun(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("declare-fun", &args, 3)?;
    let mut iter = args.into_iter();
    let name = atom_to_symbol(iter.next().unwrap(), "declare-fun")?;
    let arg_sort_forms = match iter.next().unwrap() {
        Form::List(xs) => xs,
        Form::Atom(_) => {
            return Err(SmtLibError::BadCommand {
                command: "declare-fun".into(),
                detail: "second arg must be a (sort sort ...) list".into(),
            });
        }
    };
    let arg_sorts = arg_sort_forms
        .into_iter()
        .map(parse_sort)
        .collect::<Result<Vec<_>, _>>()?;
    let ret_sort = parse_sort(iter.next().unwrap())?;
    if arg_sorts.is_empty() {
        // declare-fun with no args is the long form of declare-const.
        Ok(ConstraintInstr::DeclareVar { name, sort: ret_sort })
    } else {
        Ok(ConstraintInstr::DeclareFn { name, arg_sorts, ret_sort })
    }
}

fn parse_assert(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("assert", &args, 1)?;
    let pred = parse_term(args.into_iter().next().unwrap())?;
    Ok(ConstraintInstr::Assert { pred })
}

fn parse_push_pop(
    args: Vec<Form>,
    is_push: bool,
    out: &mut Vec<ConstraintInstr>,
) -> Result<(), SmtLibError> {
    let n: i128 = match args.len() {
        0 => 1,
        1 => match args.into_iter().next().unwrap() {
            Form::Atom(Token::Int(n)) => n,
            _ => {
                return Err(SmtLibError::BadCommand {
                    command: if is_push { "push".into() } else { "pop".into() },
                    detail: "argument must be an integer count".into(),
                });
            }
        },
        n => {
            return Err(SmtLibError::BadCommand {
                command: if is_push { "push".into() } else { "pop".into() },
                detail: format!("expected 0 or 1 args, got {n}"),
            });
        }
    };
    if !(0..=10_000).contains(&n) {
        // Defensive cap: SMT-LIB itself doesn't bound this, but a
        // hostile (push 999999999999) would explode the IR.  10k is
        // far above any realistic interactive use.
        return Err(SmtLibError::BadCommand {
            command: if is_push { "push".into() } else { "pop".into() },
            detail: format!("scope count {n} out of range [0, 10_000]"),
        });
    }
    let instr = if is_push { ConstraintInstr::PushScope } else { ConstraintInstr::PopScope };
    for _ in 0..(n as usize) {
        out.push(instr.clone());
    }
    Ok(())
}

fn parse_echo(args: Vec<Form>) -> Result<ConstraintInstr, SmtLibError> {
    expect_arity("echo", &args, 1)?;
    match args.into_iter().next().unwrap() {
        Form::Atom(Token::Str(s)) => Ok(ConstraintInstr::Echo { msg: s }),
        _ => Err(SmtLibError::BadCommand {
            command: "echo".into(),
            detail: "argument must be a string literal".into(),
        }),
    }
}

fn atom_to_symbol(form: Form, cmd: &str) -> Result<String, SmtLibError> {
    match form {
        Form::Atom(Token::Symbol(s)) => Ok(s),
        _ => Err(SmtLibError::BadCommand {
            command: cmd.to_owned(),
            detail: "expected a symbol".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Sort parsing
// ---------------------------------------------------------------------------

fn parse_sort(form: Form) -> Result<Sort, SmtLibError> {
    match form {
        Form::Atom(Token::Symbol(s)) => match s.as_str() {
            "Bool" => Ok(Sort::Bool),
            "Int" => Ok(Sort::Int),
            "Real" => Ok(Sort::Real),
            other => Ok(Sort::Uninterpreted(other.to_owned())),
        },
        Form::List(items) => {
            let mut iter = items.into_iter();
            let head = iter.next().ok_or_else(|| SmtLibError::BadSort("()".into()))?;
            let head_sym = atom_to_symbol(head, "sort")?;
            match head_sym.as_str() {
                // SMT-LIB indexed identifier: `(_ BitVec width)`.
                "_" => {
                    let kind_form = iter
                        .next()
                        .ok_or_else(|| SmtLibError::BadSort("(_) needs an identifier".into()))?;
                    let kind = atom_to_symbol(kind_form, "sort")?;
                    match kind.as_str() {
                        "BitVec" => {
                            let width_form = iter.next().ok_or_else(|| {
                                SmtLibError::BadSort("(_ BitVec) needs width".into())
                            })?;
                            if iter.next().is_some() {
                                return Err(SmtLibError::BadSort(
                                    "(_ BitVec) takes 1 arg".into(),
                                ));
                            }
                            let n = match width_form {
                                Form::Atom(Token::Int(n)) => n,
                                _ => {
                                    return Err(SmtLibError::BadSort(
                                        "(_ BitVec) width must be an integer".into(),
                                    ));
                                }
                            };
                            let width: u32 = n.try_into().map_err(|_| {
                                SmtLibError::BadSort(format!(
                                    "(_ BitVec) width {n} out of u32 range"
                                ))
                            })?;
                            Ok(Sort::BitVec(width))
                        }
                        other => Err(SmtLibError::BadSort(format!("(_ {other})"))),
                    }
                }
                "Array" => {
                    let idx_form = iter
                        .next()
                        .ok_or_else(|| SmtLibError::BadSort("(Array) needs idx sort".into()))?;
                    let val_form = iter
                        .next()
                        .ok_or_else(|| SmtLibError::BadSort("(Array) needs val sort".into()))?;
                    if iter.next().is_some() {
                        return Err(SmtLibError::BadSort("(Array) takes 2 args".into()));
                    }
                    Ok(Sort::Array {
                        idx: Box::new(parse_sort(idx_form)?),
                        val: Box::new(parse_sort(val_form)?),
                    })
                }
                other => Err(SmtLibError::BadSort(other.to_owned())),
            }
        }
        Form::Atom(_) => Err(SmtLibError::BadSort("non-symbol atom".into())),
    }
}

// ---------------------------------------------------------------------------
// Term parsing
// ---------------------------------------------------------------------------

fn parse_term(form: Form) -> Result<Predicate, SmtLibError> {
    match form {
        Form::Atom(Token::Symbol(s)) => match s.as_str() {
            "true" => Ok(Predicate::Bool(true)),
            "false" => Ok(Predicate::Bool(false)),
            other => Ok(Predicate::Var(other.to_owned())),
        },
        Form::Atom(Token::Int(n)) => Ok(Predicate::Int(n)),
        Form::Atom(Token::Decimal(s)) => parse_decimal(&s),
        Form::Atom(Token::Str(_)) => Err(SmtLibError::BadTerm("string literal in term".into())),
        Form::Atom(Token::Keyword(_)) => Err(SmtLibError::BadTerm(":keyword in term".into())),
        Form::Atom(Token::Open(_)) | Form::Atom(Token::Close(_)) => {
            // The token-form parser converts `Open` into `Form::List`
            // and never emits a bare `Close`; this branch is
            // structurally unreachable.  Defense in depth: return an
            // error rather than panicking if a refactor breaks the
            // invariant.
            Err(SmtLibError::BadTerm("unexpected paren token in atom position".into()))
        }
        Form::List(items) => parse_term_list(items),
    }
}

fn parse_decimal(s: &str) -> Result<Predicate, SmtLibError> {
    // `n.d` → Rational(n*10^d_len + d_val, 10^d_len).
    let mut parts = s.splitn(2, '.');
    let int_part = parts.next().ok_or_else(|| SmtLibError::BadInt(s.into()))?;
    let frac_part = parts.next().ok_or_else(|| SmtLibError::BadInt(s.into()))?;
    let int_val: i128 = int_part.parse().map_err(|_| SmtLibError::BadInt(s.into()))?;
    let frac_val: i128 = frac_part.parse().map_err(|_| SmtLibError::BadInt(s.into()))?;
    let frac_len = frac_part.len() as u32;
    // Build 10^frac_len defensively; cap at i128 range.
    let mut pow: i128 = 1;
    for _ in 0..frac_len {
        pow = pow
            .checked_mul(10)
            .ok_or_else(|| SmtLibError::BadInt(format!("decimal `{s}` overflows i128")))?;
    }
    let num = int_val
        .checked_mul(pow)
        .and_then(|x| x.checked_add(frac_val))
        .ok_or_else(|| SmtLibError::BadInt(format!("decimal `{s}` overflows i128")))?;
    // `Rational::new` rejects `i128::MIN`; surface that as `BadInt`
    // here rather than letting the panic escape.
    if num == i128::MIN {
        return Err(SmtLibError::BadInt(format!(
            "decimal `{s}` numerator equals i128::MIN (cannot be negated)"
        )));
    }
    Ok(Predicate::Real(Rational::new(num, pow)))
}

fn parse_term_list(items: Vec<Form>) -> Result<Predicate, SmtLibError> {
    let mut iter = items.into_iter();
    let head = iter.next().ok_or_else(|| SmtLibError::BadTerm("empty list".into()))?;
    // Special head: indexed identifier `(_ ...)` is a sort or a
    // theory-specific term; we don't handle theory-specific indexed
    // terms in v1.
    let op = match head {
        Form::Atom(Token::Symbol(s)) => s,
        _ => return Err(SmtLibError::BadTerm("non-symbol head".into())),
    };
    let args: Vec<Form> = iter.collect();
    let parse_args = |args: Vec<Form>| -> Result<Vec<Predicate>, SmtLibError> {
        args.into_iter().map(parse_term).collect()
    };

    fn binary(args: Vec<Predicate>, op: &str) -> Result<(Predicate, Predicate), SmtLibError> {
        if args.len() != 2 {
            return Err(SmtLibError::BadTerm(format!(
                "`{op}` expects 2 args, got {}",
                args.len()
            )));
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
                return Err(SmtLibError::BadTerm(format!(
                    "`not` expects 1 arg, got {}",
                    ps.len()
                )));
            }
            Ok(Predicate::Not(Box::new(ps.remove(0))))
        }
        "=>" => {
            let (a, b) = binary(parse_args(args)?, "=>")?;
            Ok(Predicate::Implies(Box::new(a), Box::new(b)))
        }
        "=" => {
            // SMT-LIB overloads `=` for both Eq and Iff — we always
            // produce Eq here.  Iff is preserved on the writer side
            // (as `=`), so `Iff` Programs round-trip lossily as
            // `Eq` per the crate-level docs.
            let (a, b) = binary(parse_args(args)?, "=")?;
            Ok(Predicate::Eq(Box::new(a), Box::new(b)))
        }
        "distinct" | "!=" => {
            let (a, b) = binary(parse_args(args)?, "distinct")?;
            Ok(Predicate::NEq(Box::new(a), Box::new(b)))
        }
        "+" => Ok(Predicate::Add(parse_args(args)?)),
        "-" => {
            // SMT-LIB unary minus: `(- n)` → -n.
            // Binary minus: `(- a b)` → a - b.
            let ps = parse_args(args)?;
            match ps.len() {
                1 => match ps.into_iter().next().unwrap() {
                    Predicate::Int(n) => {
                        // Reject `i128::MIN` — its negation overflows.
                        let neg = n.checked_neg().ok_or_else(|| {
                            SmtLibError::BadInt(format!(
                                "(- {n}) overflows i128"
                            ))
                        })?;
                        Ok(Predicate::Int(neg))
                    }
                    Predicate::Real(r) => {
                        // `r.num` came from `Rational::new` which already
                        // rejects `i128::MIN`, but be defensive.
                        let neg_num = r.num.checked_neg().ok_or_else(|| {
                            SmtLibError::BadInt(format!(
                                "(- {}/{}) overflows i128",
                                r.num, r.den
                            ))
                        })?;
                        Ok(Predicate::Real(Rational::new(neg_num, r.den)))
                    }
                    other => {
                        // (- term) on a non-literal → `0 - term`.
                        Ok(Predicate::Sub(
                            Box::new(Predicate::Int(0)),
                            Box::new(other),
                        ))
                    }
                },
                2 => {
                    let mut it = ps.into_iter();
                    let a = it.next().unwrap();
                    let b = it.next().unwrap();
                    Ok(Predicate::Sub(Box::new(a), Box::new(b)))
                }
                n => Err(SmtLibError::BadTerm(format!(
                    "`-` expects 1 or 2 args, got {n}"
                ))),
            }
        }
        "*" => {
            // Linear: `(* coef term)`.  coef must be an integer literal.
            if args.len() != 2 {
                return Err(SmtLibError::BadTerm(format!(
                    "`*` expects 2 args, got {}",
                    args.len()
                )));
            }
            let mut it = args.into_iter();
            let coef_form = it.next().unwrap();
            let term_form = it.next().unwrap();
            let coef = match coef_form {
                Form::Atom(Token::Int(n)) => n,
                Form::List(ref xs)
                    if xs.len() == 2
                        && matches!(&xs[0], Form::Atom(Token::Symbol(s)) if s == "-") =>
                {
                    // (* (- 5) x) → coef = -5.  Reject `i128::MIN`.
                    if let Form::Atom(Token::Int(n)) = &xs[1] {
                        n.checked_neg().ok_or_else(|| {
                            SmtLibError::BadTerm(format!(
                                "(* (- {n}) ...) coefficient overflows i128"
                            ))
                        })?
                    } else {
                        return Err(SmtLibError::BadTerm(
                            "first arg to `*` must be an integer literal (linear)".into(),
                        ));
                    }
                }
                _ => {
                    return Err(SmtLibError::BadTerm(
                        "first arg to `*` must be an integer literal (linear)".into(),
                    ));
                }
            };
            let term = parse_term(term_form)?;
            Ok(Predicate::Mul { coef, term: Box::new(term) })
        }
        "/" => {
            // Rational literal: `(/ num den)`.  Both must be int literals.
            if args.len() != 2 {
                return Err(SmtLibError::BadTerm(format!(
                    "`/` expects 2 args, got {}",
                    args.len()
                )));
            }
            let mut it = args.into_iter();
            let num = parse_term(it.next().unwrap())?;
            let den = parse_term(it.next().unwrap())?;
            match (num, den) {
                (Predicate::Int(n), Predicate::Int(d)) => {
                    // `Rational::new` panics on (a) zero denominator and
                    // (b) `i128::MIN` for either operand.  Surface both
                    // as `BadTerm` rather than letting the panic escape
                    // when the input is attacker-controlled.
                    if d == 0 {
                        return Err(SmtLibError::BadTerm(
                            "(/ n 0) — division by zero".into(),
                        ));
                    }
                    if n == i128::MIN || d == i128::MIN {
                        return Err(SmtLibError::BadTerm(
                            "(/ n d) — operand equals i128::MIN".into(),
                        ));
                    }
                    Ok(Predicate::Real(Rational::new(n, d)))
                }
                _ => Err(SmtLibError::BadTerm(
                    "`/` only accepts integer literals (rational literal)".into(),
                )),
            }
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
                return Err(SmtLibError::BadTerm(format!(
                    "`ite` expects 3 args, got {}",
                    ps.len()
                )));
            }
            let mut it = ps.into_iter();
            let c = it.next().unwrap();
            let t = it.next().unwrap();
            let e = it.next().unwrap();
            Ok(Predicate::Ite(Box::new(c), Box::new(t), Box::new(e)))
        }
        "forall" | "exists" => parse_quantifier(op == "forall", args),
        "select" => {
            let (a, i) = binary(parse_args(args)?, "select")?;
            Ok(Predicate::Select { arr: Box::new(a), idx: Box::new(i) })
        }
        "store" => {
            let ps = parse_args(args)?;
            if ps.len() != 3 {
                return Err(SmtLibError::BadTerm(format!(
                    "`store` expects 3 args, got {}",
                    ps.len()
                )));
            }
            let mut it = ps.into_iter();
            let arr = Box::new(it.next().unwrap());
            let idx = Box::new(it.next().unwrap());
            let val = Box::new(it.next().unwrap());
            Ok(Predicate::Store { arr, idx, val })
        }
        // Anything else: uninterpreted-function application.
        other => Ok(Predicate::Apply {
            f: other.to_owned(),
            args: parse_args(args)?,
        }),
    }
}

fn parse_quantifier(is_forall: bool, args: Vec<Form>) -> Result<Predicate, SmtLibError> {
    if args.len() != 2 {
        return Err(SmtLibError::BadTerm(format!(
            "quantifier expects 2 args (binders body), got {}",
            args.len()
        )));
    }
    let mut iter = args.into_iter();
    let binders_form = iter.next().unwrap();
    let body_form = iter.next().unwrap();
    let binders = match binders_form {
        Form::List(xs) => xs,
        _ => return Err(SmtLibError::BadTerm("quantifier binders must be a list".into())),
    };
    if binders.is_empty() {
        return Err(SmtLibError::BadTerm("quantifier needs at least one binder".into()));
    }
    let body = parse_term(body_form)?;
    // Lower multi-binder to nested single-binder quantifiers.
    let mut current = body;
    for binder in binders.into_iter().rev() {
        let pair = match binder {
            Form::List(xs) if xs.len() == 2 => xs,
            _ => return Err(SmtLibError::BadTerm("binder must be (name sort)".into())),
        };
        let mut bit = pair.into_iter();
        let var = atom_to_symbol(bit.next().unwrap(), "binder")?;
        let sort = parse_sort(bit.next().unwrap())?;
        current = if is_forall {
            Predicate::Forall { var, sort, body: Box::new(current) }
        } else {
            Predicate::Exists { var, sort, body: Box::new(current) }
        };
    }
    Ok(current)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_round_trip(p: &Program) {
        let text = write(p);
        let parsed = read(&text).expect("round-trip parse");
        assert_eq!(&parsed, p, "round-trip mismatch.\nText was:\n{text}");
    }

    fn assert(pred: Predicate) -> ConstraintInstr {
        ConstraintInstr::Assert { pred }
    }

    // ---------- Single-command round-trips ----------

    #[test]
    fn round_trip_check_sat() {
        let p = Program::new(vec![ConstraintInstr::CheckSat]).unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_get_model_unsat_core_reset() {
        let p = Program::new(vec![
            ConstraintInstr::GetModel,
            ConstraintInstr::GetUnsatCore,
            ConstraintInstr::Reset,
        ])
        .unwrap();
        assert_round_trip(&p);
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
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_declare_const_all_sorts() {
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
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_declare_fun() {
        let p = Program::new(vec![
            ConstraintInstr::DeclareFn {
                name: "len".into(),
                arg_sorts: vec![Sort::Int, Sort::Bool],
                ret_sort: Sort::Int,
            },
            // A nullary declare-fun is the long form of declare-const;
            // the writer always emits the short form, so round-trip
            // canonicalises here.
            ConstraintInstr::DeclareVar { name: "nullary".into(), sort: Sort::Bool },
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_push_pop() {
        let p = Program::new(vec![
            ConstraintInstr::PushScope,
            ConstraintInstr::PushScope,
            ConstraintInstr::PopScope,
            ConstraintInstr::PopScope,
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_echo_with_unicode() {
        let p = Program::new(vec![
            ConstraintInstr::Echo { msg: "hello".into() },
            ConstraintInstr::Echo { msg: "with \"quotes\" and \\ slash".into() },
            ConstraintInstr::Echo { msg: "héllo 世界 🦀".into() },
        ])
        .unwrap();
        assert_round_trip(&p);
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
        assert_round_trip(&p);
    }

    // ---------- Predicate round-trips ----------

    #[test]
    fn round_trip_assert_bool_lit() {
        let p = Program::new(vec![assert(Predicate::Bool(true)), assert(Predicate::Bool(false))])
            .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_assert_var_int() {
        let p = Program::new(vec![
            assert(Predicate::Var("x".into())),
            assert(Predicate::Int(42)),
            assert(Predicate::Int(-100)),
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_assert_real_literal() {
        let p = Program::new(vec![
            assert(Predicate::Real(Rational::new(3, 4))),
            assert(Predicate::Real(Rational::new(-5, 7))),
            // Integer rational round-trips as a decimal literal.
            assert(Predicate::Real(Rational::new(2, 1))),
            assert(Predicate::Real(Rational::new(-3, 1))),
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_assert_apply() {
        let p = Program::new(vec![assert(Predicate::Apply {
            f: "len".into(),
            args: vec![Predicate::Var("xs".into()), Predicate::Int(0)],
        })])
        .unwrap();
        assert_round_trip(&p);
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
            // NOTE: Iff round-trips as Eq (SMT-LIB overloads `=`).
            // That divergence is documented; not tested here to avoid
            // the asymmetry breaking the round-trip helper.
            assert(Predicate::Eq(Box::new(x), Box::new(y))),
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_assert_arithmetic_and_comparisons() {
        let x = Predicate::Var("x".into());
        let y = Predicate::Var("y".into());
        let p = Program::new(vec![
            assert(Predicate::Add(vec![x.clone(), y.clone(), Predicate::Int(3)])),
            assert(Predicate::Sub(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Mul { coef: 5, term: Box::new(x.clone()) }),
            assert(Predicate::Mul { coef: -3, term: Box::new(x.clone()) }),
            assert(Predicate::Eq(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::NEq(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Le(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Lt(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Ge(Box::new(x.clone()), Box::new(y.clone()))),
            assert(Predicate::Gt(Box::new(x), Box::new(y))),
        ])
        .unwrap();
        assert_round_trip(&p);
    }

    #[test]
    fn round_trip_assert_ite() {
        let p = Program::new(vec![assert(Predicate::Ite(
            Box::new(Predicate::Var("c".into())),
            Box::new(Predicate::Int(1)),
            Box::new(Predicate::Int(0)),
        ))])
        .unwrap();
        assert_round_trip(&p);
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
        assert_round_trip(&p);
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
        assert_round_trip(&p);
    }

    // ---------- Realistic programs ----------

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
        assert_round_trip(&p);
    }

    // ---------- Reader-only acceptance tests (real-world SMT-LIB) ----------

    #[test]
    fn read_real_world_smt2_with_set_info_and_exit() {
        // Mini benchmark in real SMT-LIB style — set-info and exit
        // should be ignored, declare-const accepted, push N expanded.
        let text = r#"
            ; Real benchmark from "the wild"
            (set-info :status sat)
            (set-info :smt-lib-version 2.6)
            (set-logic QF_LIA)
            (declare-const x Int)
            (declare-const y Int)
            (assert (>= x 0))
            (assert (>= y 0))
            (assert (= (+ x y) 10))
            (push 2)
            (assert (>= x 5))
            (check-sat)
            (get-model)
            (pop 2)
            (check-sat)
            (exit)
        "#;
        let p = read(text).unwrap();
        // 1 set-logic + 2 declare + 3 assert + 2 push + 1 assert + 1 check-sat
        // + 1 get-model + 2 pop + 1 check-sat = 14 instrs.
        assert_eq!(p.instructions().len(), 14);
        // First instruction is the set-logic (set-info dropped).
        assert!(matches!(
            p.instructions()[0],
            ConstraintInstr::SetLogic { logic: Logic::QF_LIA }
        ));
    }

    #[test]
    fn read_declare_fun_long_form_short_canonicalises() {
        let p = read("(declare-fun x () Int)").unwrap();
        assert_eq!(
            p.instructions(),
            &[ConstraintInstr::DeclareVar { name: "x".into(), sort: Sort::Int }]
        );
    }

    #[test]
    fn read_iff_overloads_to_eq() {
        // SMT-LIB has no `iff`; `=` does double duty for Eq + Iff.
        let p = read("(assert (= true false))").unwrap();
        assert!(matches!(
            p.instructions()[0],
            ConstraintInstr::Assert { pred: Predicate::Eq(_, _) }
        ));
    }

    #[test]
    fn read_unary_minus_on_int_literal() {
        let p = read("(assert (= x (- 7)))").unwrap();
        if let ConstraintInstr::Assert { pred: Predicate::Eq(_, b) } = &p.instructions()[0] {
            assert_eq!(**b, Predicate::Int(-7));
        } else {
            panic!("expected (assert (= x -7))");
        }
    }

    #[test]
    fn read_decimal_literal_to_rational() {
        let p = read("(assert (>= x 1.5))").unwrap();
        if let ConstraintInstr::Assert { pred: Predicate::Ge(_, b) } = &p.instructions()[0] {
            assert_eq!(**b, Predicate::Real(Rational::new(3, 2)));
        } else {
            panic!("expected (>= x 1.5)");
        }
    }

    #[test]
    fn read_multi_binder_quantifier_lowers_to_nested() {
        let text = "(assert (forall ((x Int) (y Int)) (>= (+ x y) 0)))";
        let p = read(text).unwrap();
        // Nested: forall x. forall y. (>= (+ x y) 0)
        if let ConstraintInstr::Assert {
            pred: Predicate::Forall { var, body, .. },
        } = &p.instructions()[0]
        {
            assert_eq!(var, "x");
            assert!(matches!(**body, Predicate::Forall { .. }));
        } else {
            panic!("expected nested forall");
        }
    }

    #[test]
    fn read_indexed_bitvec_sort() {
        let p = read("(declare-const v (_ BitVec 32))").unwrap();
        assert_eq!(
            p.instructions()[0],
            ConstraintInstr::DeclareVar { name: "v".into(), sort: Sort::BitVec(32) }
        );
    }

    #[test]
    fn read_array_sort() {
        let p = read("(declare-const a (Array Int Bool))").unwrap();
        if let ConstraintInstr::DeclareVar { sort, .. } = &p.instructions()[0] {
            assert_eq!(
                sort,
                &Sort::Array { idx: Box::new(Sort::Int), val: Box::new(Sort::Bool) }
            );
        } else {
            panic!("expected DeclareVar with Array sort");
        }
    }

    #[test]
    fn read_smt_lib_double_quote_escape() {
        // SMT-LIB v2.6 string literals: `""` is an embedded double-quote.
        let p = read(r#"(echo "hello ""world""")"#).unwrap();
        assert_eq!(
            p.instructions()[0],
            ConstraintInstr::Echo { msg: r#"hello "world""#.into() }
        );
    }

    #[test]
    fn read_set_option_with_keyword() {
        let p = read("(set-option :produce-models true)").unwrap();
        assert_eq!(
            p.instructions()[0],
            ConstraintInstr::SetOption {
                key: ":produce-models".into(),
                value: OptionValue::Bool(true),
            }
        );
    }

    // ---------- Error cases ----------

    #[test]
    fn unknown_command_errors() {
        let err = read("(do-magic)").unwrap_err();
        assert_eq!(err, SmtLibError::UnknownCommand("do-magic".into()));
    }

    #[test]
    fn unknown_logic_errors() {
        let err = read("(set-logic UNKNOWN)").unwrap_err();
        assert_eq!(err, SmtLibError::BadLogic("UNKNOWN".into()));
    }

    #[test]
    fn unmatched_close_paren_errors() {
        let err = read("(check-sat))").unwrap_err();
        assert!(matches!(err, SmtLibError::UnexpectedCloseParen { .. }));
    }

    #[test]
    fn unmatched_open_paren_errors() {
        let err = read("(check-sat").unwrap_err();
        assert_eq!(err, SmtLibError::UnexpectedEof);
    }

    #[test]
    fn unterminated_string_errors() {
        let err = read("(echo \"hello").unwrap_err();
        assert!(matches!(err, SmtLibError::BadString(_)));
    }

    #[test]
    fn invalid_utf8_in_string_errors() {
        let mut bytes: Vec<u8> = b"(echo \"".to_vec();
        bytes.push(0x80);
        bytes.extend_from_slice(b"\")");
        let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
        let err = read(s).unwrap_err();
        assert!(matches!(err, SmtLibError::BadString(ref m) if m.contains("UTF-8")));
    }

    #[test]
    fn validation_error_propagates() {
        let err = read("(pop)").unwrap_err();
        assert!(matches!(err, SmtLibError::Program(_)));
    }

    #[test]
    fn push_pop_count_overflow_errors() {
        let err = read("(push 99999)").unwrap_err();
        assert!(matches!(err, SmtLibError::BadCommand { command, .. } if command == "push"));
    }

    #[test]
    fn bad_decimal_overflow_errors() {
        // 39 nines after the decimal point — 10^39 doesn't fit in i128 (max ~10^38).
        let err = read("(assert (>= x 1.999999999999999999999999999999999999999))").unwrap_err();
        assert!(matches!(err, SmtLibError::BadInt(_)));
    }

    #[test]
    fn smt_lib_error_displays() {
        assert_eq!(SmtLibError::UnexpectedEof.to_string(), "unexpected end of input");
        assert_eq!(
            SmtLibError::UnknownCommand("foo".into()).to_string(),
            "unknown command `foo`"
        );
        assert_eq!(
            SmtLibError::TooDeep { depth: 1025, max: 1024 }.to_string(),
            "parenthesis nesting depth 1025 exceeds cap 1024"
        );
    }

    // ---------- Security-review hardening ----------

    #[test]
    fn unary_minus_on_i128_min_errors_not_panics() {
        // The inner `-N` parses as Token::Int(i128::MIN) directly (since
        // i128 can hold -2^127); the outer `(- ...)` then attempts to
        // negate that, which overflows.  Must surface as BadInt, not panic.
        let text = format!("(assert (= x (- {})))", i128::MIN);
        let err = read(&text).unwrap_err();
        assert!(matches!(err, SmtLibError::BadInt(_)), "expected BadInt, got {err:?}");
    }

    #[test]
    fn star_coef_negation_on_i128_min_errors_not_panics() {
        // (* (- N) x) where N == i128::MIN → coef = -N = overflow.
        let text = format!("(assert (= y (* (- {}) x)))", i128::MIN);
        let err = read(&text).unwrap_err();
        assert!(matches!(err, SmtLibError::BadTerm(_)), "expected BadTerm, got {err:?}");
    }

    #[test]
    fn rational_division_by_zero_errors_not_panics() {
        let err = read("(assert (>= x (/ 3 0)))").unwrap_err();
        assert!(matches!(err, SmtLibError::BadTerm(ref s) if s.contains("zero")));
    }

    #[test]
    fn rational_with_i128_min_operand_errors_not_panics() {
        let text = format!("(assert (>= x (/ {} 1)))", i128::MIN);
        let err = read(&text).unwrap_err();
        assert!(matches!(err, SmtLibError::BadTerm(_)));
    }

    #[test]
    fn parser_depth_cap_fires() {
        // (assert (and (and (and (and true)))))
        // Depth: assert=1, and=2, and=3, and=4, and=5.  Cap 4 → reject.
        let nested = "(assert (and (and (and (and true)))))";
        let err = read_with_limit(nested, 4).unwrap_err();
        assert!(matches!(err, SmtLibError::TooDeep { depth: 5, max: 4 }),
                "got {err:?}");
    }

    #[test]
    fn parser_depth_cap_default_accepts_normal_input() {
        // The default cap (1024) is generous; a 30-deep expression is
        // well within real-world SMT-LIB benchmark norms.
        let mut s = String::new();
        for _ in 0..30 {
            s.push('(');
            s.push_str("and ");
        }
        s.push_str("true");
        for _ in 0..30 {
            s.push(')');
        }
        let asserted = format!("(assert {s})");
        // Should parse without TooDeep.
        let result = read(&asserted);
        assert!(result.is_ok(),
                "expected parser to accept depth 30, got {result:?}");
    }

    #[test]
    fn read_with_limit_overrides_cap() {
        // (assert (and (and (and true))))
        // Depth: assert=1, and=2, and=3, and=4.
        let nested = "(assert (and (and (and true))))";
        assert!(read_with_limit(nested, 4).is_ok());
        assert!(matches!(read_with_limit(nested, 3).unwrap_err(), SmtLibError::TooDeep { .. }));
    }

    #[test]
    fn invalid_utf8_in_atom_errors() {
        // Token starting with a non-quote, non-paren byte that is invalid UTF-8.
        let mut bytes: Vec<u8> = b"(assert ".to_vec();
        bytes.push(0xff); // invalid UTF-8 lead byte
        bytes.extend_from_slice(b")");
        let s = unsafe { std::str::from_utf8_unchecked(&bytes) };
        let err = read(s).unwrap_err();
        assert!(matches!(err, SmtLibError::BadString(ref m) if m.contains("UTF-8")));
    }
}
