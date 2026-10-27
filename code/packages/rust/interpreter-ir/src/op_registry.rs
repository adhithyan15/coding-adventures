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
//! `Refused` until PR1b lowers them. Later slices add families that every
//! backend lowers in the same PR: PR4's overflow-checked `i64_*_ovf` ops,
//! PR6c's non-trapping IEEE
//! `f64_add`/`f64_sub`/`f64_mul`/`f64_div`/`f64_neg` with `f64_signbit` (lowered
//! on the eight IEEE backends; BEAM refuses the arithmetic at compile time),
//! and PR7b's language-neutral float primitives `f64_class`, `f64_digits` and
//! `f64_exp10` (the shortest round-trip digits as integers, on all nine).
//!
//! PR7f adds the **dynamic-value primitives** (`dyn_box_int` … `dyn_symbol`,
//! family [`OpFamily::Dynamic`]): the tagged-word representation a dynamically
//! typed language's values share — an integer, nil, a symbol, a boolean or a
//! two-field heap pair in one 64-bit word. They were the Lisp-vocabulary
//! builtins `dyn_cons`, `dyn_car`, `dyn_cdr`, `dyn_pair_p`, `dyn_null_p`,
//! `make_nil` and `make_symbol`, unregistered, until PR7f renamed them. Only
//! the backends that link the tagged-word runtime lower them (native x86-64
//! and AArch64, LLVM); the others refuse them, because their value models are
//! structural (a managed object, an Erlang term, a `vm-core` heap cell).
//!
//! PR6a's float writers `write_f64_py` / `write_f64_js` were registered here
//! until LANG83 PR7e. They printed a float in Python's or JavaScript's layout,
//! so every backend carried two languages' formatting rules. Each compiler
//! now emits its own layout helper over the PR7b digits, and the writers are
//! gone (see the next section).
//!
//! # The language-neutrality rule (LANG83 PR7)
//!
//! The IIR is a language-neutral machine. **No registry op, builtin or trap
//! kind may name a source language** or carry one language's semantics.
//! [`language_name_violation`] enforces the *name* half: it splits a name on
//! `_`, `.`, `-` and camelCase boundaries and refuses any word that is a
//! language token, ignoring trailing digits
//! ([`LANGUAGE_TOKENS`]) or shaped like a language's exception class (a part
//! ending in `error` or `exception`, such as `ReferenceError`):
//!
//! ```text
//!   write_f64_py         → write | f64 | py            "py" is Python       ✗
//!   Trap.ReferenceError  → Trap | ReferenceError       an exception class   ✗
//!   f64_digits           → f64 | digits                                     ✓
//!   Trap.Unbound         → Trap | Unbound              a condition          ✓
//! ```
//!
//! A test runs it over every registry name and every trap kind; vm-core runs
//! it over its default builtins and `lang-aot`'s harnesses over theirs. A
//! compiler's own module-local helpers (`__py_write_float`) are not IIR
//! vocabulary, so the rule does not apply to them.
//! Instructions that are not registry entries
//! (control flow, calls, memory, heap, bitwise, …) are simply not gated by the
//! registry; their existing validators still apply.
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
    /// LANG83 PR7f: the dynamic-value representation — one tagged machine word
    /// that is an integer, nil, a symbol, a boolean or a two-field heap pair.
    /// Representation only: what a value *means* (which values are false, when
    /// two are equal) is a language's rule, written by its compiler.
    Dynamic,
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

/// Why a backend without the tagged-word runtime refuses the LANG83 PR7f
/// dynamic-value primitives. Shared so the refusal a user sees is identical
/// on every such backend.
///
/// The primitives are calls into the tagged-word runtime (`dynval_runtime.c`
/// on the native and LLVM columns). The managed backends, BEAM, `vm-core` and
/// the generic JIT hold dynamic values structurally instead (a GC object, an
/// Erlang term, a heap cell): `lang-aot` lowers a frontend's pair builtins to
/// `alloc` / `field_*` / `is_null` before those backends run, and none of them
/// lowers a `dyn_*` call.
pub const NO_TAGGED_WORD_RUNTIME: &str = "this backend holds dynamic values structurally and \
     does not link the tagged-word runtime the dyn_* primitives call (LANG83 PR7f)";

/// Why the tagged-word backends refuse `dyn_symbol`: a symbol reaches them as
/// a `const Var(name) : symbol`, which `intern_symbols` turns into a finished
/// tagged immediate at compile time. No pass or runtime call lowers a symbol
/// built from a literal at run time.
pub const SYMBOLS_ARE_INTERNED_CONSTANTS: &str = "symbols reach this backend as interned \
     `const name : symbol` immediates; a dyn_symbol call is not lowered (LANG83 PR7f)";

/// The `introduced_by` tag of the four operations PR1b will lower.
pub const LANG83_PR1B: &str = "LANG83 PR1b";

/// The `introduced_by` tag of the overflow-checked arithmetic family (LANG83
/// PR4): `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`, `i64_neg_ovf`. Unlike
/// PR1b's, these were registered and lowered on every backend in one slice.
pub const LANG83_PR4: &str = "LANG83 PR4";

/// The `introduced_by` tag of the non-trapping IEEE-754 `f64` arithmetic
/// family and its sign-bit query (LANG83 PR6c): `f64_add`, `f64_sub`,
/// `f64_mul`, `f64_div`, `f64_neg`, `f64_signbit`. Registered in one slice and
/// lowered by every backend whose floats are IEEE binary64; BEAM, whose floats
/// are always finite, refuses the five arithmetic ops at compile time.
pub const LANG83_PR6C: &str = "LANG83 PR6c";

/// The `introduced_by` tag of the language-neutral float primitives (LANG83
/// PR7b): `f64_class`, `f64_digits`, `f64_exp10`. They expose the *digits*
/// every PR6a float writer already computes — the shortest round-trip decimal
/// significand and the exponent of its leading digit — as plain integers, so
/// a compiler can lay a float out in its own language's style with integer
/// ops and `putchar`, and no backend needs to know which language that is.
pub const LANG83_PR7B: &str = "LANG83 PR7b";

/// The `introduced_by` tag of the dynamic-value primitives (LANG83 PR7f):
/// `dyn_box_int`, `dyn_unbox_int`, `dyn_tag`, `dyn_pair_new`, `dyn_pair_first`,
/// `dyn_pair_rest`, `dyn_is_pair`, `dyn_is_nil`, `dyn_nil` and `dyn_symbol`.
/// Before PR7f they were unregistered builtins with Lisp names (`dyn_cons`,
/// `dyn_car`, `dyn_cdr`, `dyn_pair_p`, `dyn_null_p`, `make_nil`, `make_symbol`).
pub const LANG83_PR7F: &str = "LANG83 PR7f";

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
    /// `"f64"`, `"num"` (integer or `f64`), `"str"`, `"dyn"` (a dynamic
    /// value: one tagged word, LANG83 PR7f), `"literal"` (an `Operand::Str`
    /// compile-time literal) or `"kind"` (an `Operand::Str` trap kind from the
    /// closed table in `exception_kind`).
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

    /// `true` if this entry is one of the LANG83 PR4 overflow-checked ops.
    pub fn is_pr4(&self) -> bool {
        self.introduced_by == LANG83_PR4
    }

    /// `true` if this entry is one of the LANG83 PR6c IEEE `f64` ops.
    pub fn is_pr6c(&self) -> bool {
        self.introduced_by == LANG83_PR6C
    }

    /// `true` if this entry is one of the LANG83 PR7b neutral float
    /// primitives (`f64_class`, `f64_digits`, `f64_exp10`).
    pub fn is_pr7b(&self) -> bool {
        self.introduced_by == LANG83_PR7B
    }

    /// `true` if this entry is one of the LANG83 PR7f dynamic-value
    /// primitives (`dyn_box_int` … `dyn_symbol`).
    pub fn is_pr7f(&self) -> bool {
        self.introduced_by == LANG83_PR7F
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

use OpFamily::{Arithmetic, Comparison, Conversion, Dynamic, Error, Float, Output, String as Str};
use OpKind::{Builtin, Op};

const NO_TRAP: &[&str] = &[];
const DIV_ZERO_ONLY: &[&str] = &[TRAP_DIV_ZERO];
const BOUNDS_ONLY: &[&str] = &[TRAP_BOUNDS];
const CONV_RANGE_ONLY: &[&str] = &[TRAP_CONV_RANGE];
const DIV_ZERO_OR_OVERFLOW: &[&str] = &[TRAP_DIV_ZERO, TRAP_OVERFLOW];
const OVERFLOW_ONLY: &[&str] = &[TRAP_OVERFLOW];
const ANY_REGISTERED_KIND: &[&str] = &[CATCH_ANY];

const BIN_NUM: &[&str] = &["num", "num"];
const UN_NUM: &[&str] = &["num"];
const UN_F64: &[&str] = &["f64"];
const BIN_F64: &[&str] = &["f64", "f64"];
const BIN_I64: &[&str] = &["i64", "i64"];
const UN_I64: &[&str] = &["i64"];
/// One dynamic value: a tagged word (see the PR7f block of [`STANDARD_OPS`]).
const UN_DYN: &[&str] = &["dyn"];

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
    // There is no float writer. LANG83 PR6a's `write_f64_py` / `write_f64_js`
    // (a float in Python's / JavaScript's layout) were removed in PR7e: a
    // compiler prints a float with its own module-local helper, built from the
    // PR7b digit ops below, integer arithmetic and `putchar`.
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
    // LANG83 PR4 — overflow-checked arithmetic. The exact result, or
    // `Trap.Overflow` when it does not fit in i64 (Python/Ruby integers promote
    // to a bignum on that trap; the frontend decides). Type-first so the
    // `add_`/`sub_`/`mul_`/`neg_` prefix arms of the native backends and the
    // generic JIT can never claim them as *wrapping* arithmetic.
    spec("i64_add_ovf", Op, Arithmetic, 2, true, BIN_I64, OVERFLOW_ONLY, LANG83_PR4),
    spec("i64_sub_ovf", Op, Arithmetic, 2, true, BIN_I64, OVERFLOW_ONLY, LANG83_PR4),
    spec("i64_mul_ovf", Op, Arithmetic, 2, true, BIN_I64, OVERFLOW_ONLY, LANG83_PR4),
    spec("i64_neg_ovf", Op, Arithmetic, 1, true, UN_I64, OVERFLOW_ONLY, LANG83_PR4),
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
    spec("f64_pow", Op, Float, 2, true, BIN_F64, NO_TRAP, "ALGOL AL-pow"),
    // LANG83 PR6c — IEEE-754 binary64 arithmetic with the edges IEEE defines
    // and nothing else: `1/0` is +Infinity, `0/0` is NaN, an overflow is
    // ±Infinity, the sign of a zero follows IEEE. They never trap, unlike the
    // classic `div` (whose zero-divisor edge is backend-dependent). Type-first
    // (`f64_add`, never `add_f64`): `add_f64` is what `jit_core::specialise`
    // names the CLASSIC add on f64, and these must stay distinct ops.
    spec("f64_add", Op, Float, 2, true, BIN_F64, NO_TRAP, LANG83_PR6C),
    spec("f64_sub", Op, Float, 2, true, BIN_F64, NO_TRAP, LANG83_PR6C),
    spec("f64_mul", Op, Float, 2, true, BIN_F64, NO_TRAP, LANG83_PR6C),
    spec("f64_div", Op, Float, 2, true, BIN_F64, NO_TRAP, LANG83_PR6C),
    // Flips the sign bit only: `f64_neg 0` is `-0` (where `0 - x` gives `0`).
    spec("f64_neg", Op, Float, 1, true, UN_F64, NO_TRAP, LANG83_PR6C),
    // The sign bit as an integer 0/1 — the only way to tell `-0` from `0`
    // without dividing by it (JavaScript's `console.log(-0)` prints `-0`).
    spec("f64_signbit", Op, Float, 1, true, UN_F64, NO_TRAP, LANG83_PR6C),
    // LANG83 PR7b — the language-neutral float primitives. Each takes one
    // `f64` and returns an INTEGER (like `f64_signbit`, a Float-family op
    // whose result is not a float). None traps, so all three are pure.
    //
    //   f64_class x    0 for ±0, 1 finite non-zero (subnormals too),
    //                  2 for ±Infinity, 3 for NaN
    //   f64_digits x   class 1: the shortest round-trip significand D of |x|
    //                  (shortest, then closest, ties to even — PR6a's rule),
    //                  no trailing zeros, 1 <= D < 10^17; otherwise 0
    //   f64_exp10 x    class 1: e10, the exponent of D's leading digit, so
    //                  |x| = D × 10^(e10 − n + 1) with n = digits of D;
    //                  otherwise 0
    //
    //   x        class  digits             exp10
    //   1.5      1      15                 0
    //   100.0    1      1                  2
    //   0.1      1      1                  -1
    //   5e-324   1      5                  -324
    //   1e21     1      1                  21
    //   -0.0     0      0                  0
    //   -inf     2      0                  0
    //   nan      3      0                  0
    //
    // With `f64_signbit` these identify every value a float writer prints;
    // where the point goes is the compiler's business (spec "PR7").
    spec("f64_class", Op, Float, 1, true, UN_F64, NO_TRAP, LANG83_PR7B),
    spec("f64_digits", Op, Float, 1, true, UN_F64, NO_TRAP, LANG83_PR7B),
    spec("f64_exp10", Op, Float, 1, true, UN_F64, NO_TRAP, LANG83_PR7B),
    // ── Conversions (LANG-FULL E8) ──────────────────────────────────────────
    spec("int_to_real", Op, Conversion, 1, true, &["int"], NO_TRAP, "LANG-FULL E8"),
    spec("real_to_int_trunc", Op, Conversion, 1, true, UN_F64, CONV_RANGE_ONLY, "LANG-FULL E8"),
    spec("real_to_int_floor", Op, Conversion, 1, true, UN_F64, CONV_RANGE_ONLY, "LANG-FULL E8"),
    // ── Dynamic values (LANG83 PR7f) ────────────────────────────────────────
    // The representation a dynamically typed language's values share: one
    // tagged 64-bit word (DVAL01). Its low three bits are the tag:
    //
    //   tag  value     encoding            dyn_tag
    //   000  integer   n << 3              0
    //   001  nil       the whole word 1    1
    //   010  symbol    (id << 32) | 010    2
    //   011  false     the whole word 3    3
    //   101  true      the whole word 5    5
    //   111  pair      pointer | 111       7
    //
    //   dyn_box_int n      an integer (|n| < 2^60) → its dynamic value
    //   dyn_unbox_int x    a dynamic integer → the integer
    //   dyn_tag x          x's tag number (table above), as an integer
    //   dyn_pair_new a b   a fresh, garbage-collected pair of a and b
    //   dyn_pair_first p   the pair's first field  (fails closed on a non-pair)
    //   dyn_pair_rest p    the pair's second field (fails closed on a non-pair)
    //   dyn_is_pair x      true if x is a pair, as a dynamic boolean
    //   dyn_is_nil x       true if x is nil, as a dynamic boolean
    //   dyn_nil            nil
    //   dyn_symbol "name"  the symbol the literal names (one value per name)
    //
    // Representation only. Which values count as false, when two values are
    // equal and what a program's result exits with are a language's rules;
    // LANG83 PR7 gives them to that language's compiler, never to an op. They
    // are Builtins because the tagged-word backends call a runtime function
    // for each (`__dyn_pair_new`, …), named by the `dyn_` prefix — the one
    // prefix arm that is meant to capture them (see
    // `GRANDFATHERED_PREFIXED_NAMES`).
    //
    // `dyn_pair_first`/`rest` of a non-pair stops the program; no
    // implementation reads memory through the word. The VM returns a
    // `TypeError`, `dynval-runtime`'s `extern "C"` functions panic, and the
    // C runtime the tagged-word backends link checks the tag and aborts with
    // "dyn_pair_first: argument with tag 0 (int) is not a pair" on stderr —
    // the tag, never the word, which could be a heap address (LANG83 PR7f
    // follow-up; it was a read through the word before, so Twig's
    // `(car 1000000)` read address 8000000). The traps list stays empty: a
    // type error is not one of the closed `Trap.*` kinds, so a program cannot
    // catch it — a frontend that wants a recoverable answer tests with
    // `dyn_is_pair` first. (One known exception: `lang_aot::jit_lisp`'s
    // McCarthy callbacks answer nil instead of failing.)
    spec("dyn_box_int", Builtin, Dynamic, 1, true, UN_I64, NO_TRAP, LANG83_PR7F),
    spec("dyn_unbox_int", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_tag", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_pair_new", Builtin, Dynamic, 2, true, &["dyn", "dyn"], NO_TRAP, LANG83_PR7F),
    spec("dyn_pair_first", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_pair_rest", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_is_pair", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_is_nil", Builtin, Dynamic, 1, true, UN_DYN, NO_TRAP, LANG83_PR7F),
    spec("dyn_nil", Builtin, Dynamic, 0, true, &[], NO_TRAP, LANG83_PR7F),
    spec("dyn_symbol", Builtin, Dynamic, 1, true, &["literal"], NO_TRAP, LANG83_PR7F),
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
/// comparisons are exactly the family `cmp_` dispatches, and the ten LANG83
/// PR7f dynamic-value primitives are exactly the family `dyn_` dispatches
/// (the native backends turn `call_builtin "dyn_<x>"` into a call to the
/// runtime's `__dyn_<x>`, and only for a name in their builtin table, so an
/// unregistered `dyn_` name is still refused there).
pub const GRANDFATHERED_PREFIXED_NAMES: &[&str] = &[
    "cmp_eq",
    "cmp_ne",
    "cmp_lt",
    "cmp_le",
    "cmp_gt",
    "cmp_ge",
    "dyn_box_int",
    "dyn_unbox_int",
    "dyn_tag",
    "dyn_pair_new",
    "dyn_pair_first",
    "dyn_pair_rest",
    "dyn_is_pair",
    "dyn_is_nil",
    "dyn_nil",
    "dyn_symbol",
];

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
/// source is `Operand::Var(name)`. A `call_builtin` of a non-registry name
/// (`getchar`, …) and every non-registry mnemonic return `None`.
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

/// The source-language tokens no IIR vocabulary name may contain (LANG83
/// PR7, "The principle"). A name is split into words (see
/// [`language_name_violation`]); a word equal to one of these, ignoring case
/// and trailing digits, names a language.
///
/// Languages the repository compiles, their common abbreviations, the
/// families the spec lists, and the runtimes a writer might name instead of
/// the language (`node`, `cpython`). It cannot be complete. It covers the
/// spelling a writer reaches for first (`_py`, `_js`, `_ts`, `_rb`, `_pl`)
/// and the languages with a frontend here whose names are not also ordinary
/// words: `Oct`, `Nib`, `REDUCE` and `Derive` are left out, because
/// `i64_to_oct` or `vec_reduce` would be neutral ops.
pub const LANGUAGE_TOKENS: &[&str] = &[
    "py", "python", "python2", "python3", "cpython", "js", "javascript", "ecma", "ecmascript",
    "node", "ts", "typescript", "rb", "ruby", "pl", "perl", "php", "lisp", "scheme", "clojure",
    "twig", "mccarthy", "basic", "algol", "cobol", "fortran", "flowmatic", "brainfuck", "macsyma",
    "maxima", "wolfram", "mathematica", "lua", "elixir", "erlang", "haskell", "ocaml", "swift",
    "csharp", "java", "kotlin", "scala", "golang", "rust", "smalltalk", "pascal",
];

/// The words of an IIR vocabulary name, for [`language_name_violation`]:
/// the parts between `_`, `.` and `-`, each followed by its camelCase words
/// when it has more than one.
///
/// | name | words |
/// |------|-------|
/// | `write_f64_py` | `write`, `f64`, `py` |
/// | `Trap.DivZero` | `Trap`, `DivZero`, `Div`, `Zero` |
/// | `toJSString` | `toJSString`, `to`, `JS`, `String` |
/// | `py-print` | `py`, `print` |
///
/// A camelCase boundary falls before an upper-case letter that follows a
/// lower-case letter or digit (`toJS` → `to` `JS`), or that follows an
/// upper-case letter and precedes a lower-case one (`JSString` → `JS`
/// `String`). The whole part is kept too, so `TypeScript` is caught as one
/// word although neither `Type` nor `Script` is a language.
fn name_words(name: &str) -> Vec<&str> {
    let mut words = Vec::new();
    for part in name.split(['_', '.', '-']).filter(|part| !part.is_empty()) {
        words.push(part);
        let bytes = part.as_bytes();
        let mut start = 0;
        let mut camel = Vec::new();
        for i in 1..bytes.len() {
            let (prev, cur) = (bytes[i - 1], bytes[i]);
            let next_lower = bytes.get(i + 1).is_some_and(u8::is_ascii_lowercase);
            let boundary = cur.is_ascii_uppercase()
                && (prev.is_ascii_lowercase() || prev.is_ascii_digit() || (prev.is_ascii_uppercase() && next_lower));
            if boundary {
                camel.push(&part[start..i]);
                start = i;
            }
        }
        if !camel.is_empty() {
            camel.push(&part[start..]);
            words.extend(camel);
        }
    }
    words
}

/// The language-neutrality violation for an IIR vocabulary name — a registry
/// op or builtin, a VM builtin, or a trap kind — or `None` if it names no
/// source language.
///
/// The name is split into words — on `_`, `.` and `-`, and at camelCase
/// boundaries (see `name_words`) — and each word is checked, ignoring case
/// and any trailing digits (`python3` → `python`, `js2` → `js`):
///
/// | Word | Verdict | Example |
/// |------|---------|---------|
/// | a [`LANGUAGE_TOKENS`] entry | names a language | `write_f64_py`, `Ruby.ZeroDivisionError`, `toJSString`, `py-print`, `rb2` |
/// | ends in `error` or `exception` | a language's exception class; a trap kind names a *condition* instead | `Trap.ReferenceError`, `NameError` |
/// | anything else | neutral | `f64_digits`, `Trap.Unbound`, `equal?` |
///
/// Splitting first matters: `print_str` contains no language although `str`
/// and `print` might look like source words, and `js` inside `adjust` is not
/// a word at all.
///
/// ```
/// use interpreter_ir::op_registry::language_name_violation;
/// assert!(language_name_violation("f64_digits").is_none());
/// assert!(language_name_violation("Trap.Unbound").is_none());
/// assert!(language_name_violation("write_f64_py").unwrap().contains("\"py\""));
/// assert!(language_name_violation("writeF64Js").unwrap().contains("\"Js\""));
/// assert!(language_name_violation("Trap.ReferenceError").unwrap().contains("exception class"));
/// ```
pub fn language_name_violation(name: &str) -> Option<String> {
    name_words(name).into_iter().find_map(|part| {
        let lower = part.to_ascii_lowercase();
        let stem = lower.trim_end_matches(|c: char| c.is_ascii_digit());
        if LANGUAGE_TOKENS.contains(&lower.as_str()) || LANGUAGE_TOKENS.contains(&stem) {
            Some(format!(
                "{name:?} contains {part:?}, a source language's name: the IIR is \
                 language-neutral (LANG83 PR7), so the compiler of that language must \
                 translate its semantics into neutral ops (or a module-local helper)"
            ))
        } else if lower.ends_with("error") || lower.ends_with("exception") {
            Some(format!(
                "{name:?} contains {part:?}, which reads as a language's exception class: \
                 a trap kind names a machine-level condition (LANG83 PR7), and a \
                 compiler maps its language's error onto one"
            ))
        } else {
            None
        }
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
    use crate::exception_kind::is_trap_kind;
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
        for ok in ["div", "mod", "or", "i64_div_floor", "i64_add_ovf", "i64_neg_ovf", "f64_sin", "str_len"] {
            assert!(name_rule_violation(ok).is_none(), "{ok} should be accepted");
        }
    }

    /// LANG83 PR7e: no registry op or builtin, and no trap kind, names a
    /// source language or a language's exception class.
    #[test]
    fn no_registry_name_or_trap_kind_names_a_language() {
        for name in STANDARD_OPS.iter().map(|s| s.name).chain(crate::exception_kind::ALL_TRAP_KINDS.iter().copied()) {
            if let Some(why) = language_name_violation(name) {
                panic!("{why}");
            }
        }
    }

    /// The rule catches the names PR7 retired, so none of them can come back,
    /// and they are really gone from the registry and the kind table.
    #[test]
    fn the_language_name_rule_rejects_the_retired_names() {
        for old in [
            "write_f64_py",
            "write_f64_js",
            "write_f64_rb",
            "f64_to_str_pl",
            "py_print_float",
            "py_name_error",
            "Trap.ReferenceError",
            "Ruby.ZeroDivisionError",
            "NameError",
            "lisp_truthy",
            "JS.TypeError",
        ] {
            assert!(language_name_violation(old).is_some(), "{old} names a language");
            assert!(lookup(old).is_none(), "{old} must not be registered");
            assert!(!is_trap_kind(old), "{old} must not be a trap kind");
        }
        // Case does not matter; empty parts (`__x`) are skipped.
        assert!(language_name_violation("Write_F64_PY").is_some());
        assert!(language_name_violation("__python_helper").is_some());
        // `-` separates words, camelCase boundaries do too, and a trailing
        // version number does not hide a language.
        for bad in [
            "py-print",
            "writeF64Py",
            "toJSString",
            "JSONtoRuby",
            "TypeScript.cast",
            "f64_to_str_js2",
            "ruby3_hash",
            "node_write",
            "ts_number",
            "php_echo",
            "ocaml_compare",
            "clojure_seq",
            "scala_match",
            "cpython_repr",
            "CPythonRepr",
        ] {
            assert!(language_name_violation(bad).is_some(), "{bad} names a language");
        }
        // Neutral names pass, including ones whose letters merely contain a
        // token (`adjust` holds "js", `plus` holds "pl"): only whole parts count.
        // Splitting at camelCase and digits must not invent a token either:
        // `f64` is `f`, `DivZero` is `Div` `Zero`, `nodes` and `tsc` are not
        // `node` and `ts`.
        for ok in [
            "f64_digits",
            "i64_div_floor",
            "putchar",
            "print_str",
            "Trap.Unbound",
            "Trap.DivZero",
            "Trap.ConvRange",
            "equal?",
            "adjust_plus",
            "+",
            "list-nodes",
            "toString",
            "u8_to_f64",
            "PutChar",
        ] {
            assert!(language_name_violation(ok).is_none(), "{ok} is neutral");
        }
    }

    #[test]
    fn grandfathered_names_are_registered_comparisons_or_dynamic_values() {
        for name in GRANDFATHERED_PREFIXED_NAMES {
            let spec = lookup(name).expect("grandfathered name must be registered");
            match spec.family {
                OpFamily::Comparison => assert!(name.starts_with("cmp_"), "{name}"),
                // The `dyn_` arm is a call into the tagged-word runtime, so the
                // family it captures must be Builtins.
                OpFamily::Dynamic => {
                    assert!(name.starts_with("dyn_"), "{name}");
                    assert_eq!(spec.kind, OpKind::Builtin, "{name}");
                }
                other => panic!("{name} is grandfathered but is in family {other:?}"),
            }
        }
        // Every dynamic-value primitive is grandfathered, and nothing else
        // with the prefix is: `dyn_new` (above) is still a violation.
        for spec in STANDARD_OPS.iter().filter(|s| s.family == OpFamily::Dynamic) {
            assert!(GRANDFATHERED_PREFIXED_NAMES.contains(&spec.name), "{}", spec.name);
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
                    *kind == CATCH_ANY || is_trap_kind(kind),
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
    fn pr4_set_is_exactly_the_overflow_family() {
        let mut pr4: Vec<&str> =
            STANDARD_OPS.iter().filter(|s| s.is_pr4()).map(|s| s.name).collect();
        pr4.sort_unstable();
        assert_eq!(pr4, ["i64_add_ovf", "i64_mul_ovf", "i64_neg_ovf", "i64_sub_ovf"]);
        for name in pr4 {
            let spec = lookup(name).unwrap();
            assert_eq!(spec.kind, OpKind::Op);
            assert!(spec.dest, "{name} writes its result");
            assert_eq!(spec.may_trap, [TRAP_OVERFLOW], "{name} traps only on overflow");
            assert!(spec.operand_types.iter().all(|t| *t == "i64"), "{name} is defined on i64");
        }
        assert_eq!(lookup("i64_neg_ovf").unwrap().arity, 1);
    }

    /// Every LANG83 operation that may trap must be kept by dead-code
    /// elimination: if it is not side-effecting, an unused result lets an
    /// optimiser delete the trap. Derived from the registry so a future
    /// trapping op cannot be added without its `has_side_effects` entry.
    #[test]
    fn pr6c_set_is_exactly_the_ieee_f64_family() {
        let mut pr6c: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr6c()).map(|s| s.name).collect();
        pr6c.sort_unstable();
        assert_eq!(pr6c, ["f64_add", "f64_div", "f64_mul", "f64_neg", "f64_signbit", "f64_sub"]);
        for name in pr6c {
            let spec = lookup(name).unwrap();
            assert_eq!(spec.kind, OpKind::Op, "{name}");
            assert!(spec.dest, "{name} writes a dest");
            assert!(spec.may_trap.is_empty(), "{name} never traps");
            assert!(spec.operand_types.iter().all(|t| *t == "f64"), "{name} takes f64 operands");
            let unary = matches!(name, "f64_neg" | "f64_signbit");
            assert_eq!(spec.arity, if unary { 1 } else { 2 }, "{name}");
        }
    }

    #[test]
    fn pr7b_set_is_exactly_the_neutral_float_primitives() {
        let mut pr7b: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr7b()).map(|s| s.name).collect();
        pr7b.sort_unstable();
        assert_eq!(pr7b, ["f64_class", "f64_digits", "f64_exp10"]);
        for name in pr7b {
            let spec = lookup(name).unwrap();
            // Ops, not Builtins: each reads one f64, writes an integer dest,
            // writes nothing else and never traps — so DCE may drop one.
            assert_eq!(spec.kind, OpKind::Op, "{name}");
            assert_eq!(spec.family, OpFamily::Float, "{name}");
            assert!(spec.dest, "{name} writes a dest");
            assert_eq!(spec.operand_types, ["f64"], "{name}");
            assert!(spec.may_trap.is_empty(), "{name} never traps");
            assert!(!crate::opcodes::has_side_effects(name), "{name} is pure");
        }
    }

    /// LANG83 PR7f: the dynamic-value primitives, under the neutral names
    /// that replaced the Lisp vocabulary (`dyn_cons`, `dyn_car`, `dyn_cdr`,
    /// `dyn_pair_p`, `dyn_null_p`, `make_nil`, `make_symbol`, none of which
    /// is registered).
    #[test]
    fn pr7f_set_is_exactly_the_dynamic_value_primitives() {
        let mut pr7f: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr7f()).map(|s| s.name).collect();
        pr7f.sort_unstable();
        assert_eq!(
            pr7f,
            [
                "dyn_box_int",
                "dyn_is_nil",
                "dyn_is_pair",
                "dyn_nil",
                "dyn_pair_first",
                "dyn_pair_new",
                "dyn_pair_rest",
                "dyn_symbol",
                "dyn_tag",
                "dyn_unbox_int",
            ]
        );
        for name in pr7f {
            let spec = lookup(name).unwrap();
            assert_eq!(spec.kind, OpKind::Builtin, "{name}");
            assert_eq!(spec.family, OpFamily::Dynamic, "{name}");
            assert!(spec.dest, "{name} writes a dest");
            assert!(spec.may_trap.is_empty(), "{name} names no trap kind");
            assert!(language_name_violation(name).is_none(), "{name}");
        }
        for old in ["dyn_cons", "dyn_car", "dyn_cdr", "dyn_pair_p", "dyn_null_p", "make_nil", "make_symbol"] {
            assert!(lookup(old).is_none(), "{old} is the pre-PR7f name");
        }
        // The language-semantic runtime ops are not representation, so they
        // are never registered (PR7f moves them into the compilers).
        for semantic in ["dyn_truthy", "dyn_equal", "dyn_not", "dyn_to_exit_code"] {
            assert!(lookup(semantic).is_none(), "{semantic} is a language's rule, not an op");
        }
        assert_eq!(lookup("dyn_nil").unwrap().arity, 0);
        assert_eq!(lookup("dyn_pair_new").unwrap().operand_types, ["dyn", "dyn"]);
        assert_eq!(lookup("dyn_box_int").unwrap().operand_types, ["i64"]);
        assert_eq!(lookup("dyn_symbol").unwrap().operand_types, ["literal"]);
    }

    #[test]
    fn every_trapping_lang83_op_has_side_effects() {
        for spec in STANDARD_OPS
            .iter()
            .filter(|s| s.kind == OpKind::Op && s.introduced_by.starts_with("LANG83") && !s.may_trap.is_empty())
        {
            assert!(crate::opcodes::has_side_effects(spec.name), "{} must survive DCE", spec.name);
            assert!(crate::opcodes::may_trap(spec.name), "{}", spec.name);
        }
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
