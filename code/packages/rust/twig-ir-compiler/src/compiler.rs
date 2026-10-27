//! Twig → InterpreterIR (IIR) compiler.
//!
//! This module turns a Twig **parse tree** — the generic
//! [`GrammarASTNode`] that `GrammarParser` builds from `twig.grammar` —
//! into an [`interpreter_ir::IIRModule`].  The lowering follows the design
//! laid out in `code/specs/TW00-twig-language.md` and mirrors the Python
//! reference implementation at `code/packages/python/twig/src/twig/compiler.py`.
//!
//! The tree is read through the borrowed views in `src/cst.rs`
//! (`ExprNode::kind` answers "what is this expression?" one level at a
//! time); no typed AST is built (LANG84 decision 6).  The tree must already
//! have passed [`crate::check_cst()`] — [`Compiler::compile`] runs it first.
//!
//! ## What gets emitted
//!
//! The output module always contains:
//!
//! 1. **One `IIRFunction` per `(define (name args...) body+)` form.**
//!    Parameters lower 1-to-1 to IIR params.  They stay `"any"` unless an
//!    explicit static annotation or a conservative LANG-FULL E4 call-site proof
//!    can stamp a concrete hint such as `"str"`; the function body is the
//!    lowered body expressions plus a final `ret`.
//! 2. **One `IIRFunction` per anonymous `(lambda ...)` expression.**
//!    Synthetic name (`__lambda_0`, `__lambda_1`, …); captured variables
//!    appear as the *leading* parameters, in the order produced by
//!    `free_vars` (`src/free_vars.rs`).
//! 3. **A synthesised `main` function.**  Holds, in source order:
//!    - top-level value defines (each emitted as
//!      `call_builtin "global_set" <name> <rhs>`),
//!    - bare top-level expressions.
//!
//!    The value of the *last* bare expression becomes `main`'s return.
//!    Programs with no bare expressions return `nil` via
//!    `call_builtin "make_nil"`.
//!
//! Twig remains dynamically typed, so functions keep `type_status = Untyped`
//! and dynamic paths still carry `type_hint = "any"`.  The LANG-FULL fast paths
//! stamp concrete hints only where source-local evidence makes the type
//! unambiguous.
//! The vm-core profiler will fill in observed types at runtime; the JIT can
//! specialise from those observations.
//!
//! ## Apply-site dispatch (compile-time)
//!
//! The compiler decides at compile time whether each `(fn args...)` is a
//! direct call, a builtin, or an indirect closure call:
//!
//! | Function position (first match wins)   | Emitted IIR                       |
//! |----------------------------------------|-----------------------------------|
//! | `and` / `or` (special forms)           | inline `jmp_if_false` chain       |
//! | Lexically bound name (param, `let`,    | `call_closure h, ...args`         |
//! | `let*`, lambda param, `match` binding) |                                   |
//! | Top-level user fn name / extern        | `call <name>, ...args`            |
//! | Builtin name (`+`, `cons`)             | `call_builtin <name>, ...args`    |
//! | Anything else (any other expression)   | `call_closure h, ...args` (LANG34)|
//!
//! Top-level recursion stays on the fast `call` path; only locals
//! holding closures pay the indirect cost.
//!
//! ### Lexical scope wins (#18435)
//!
//! The *order* of that table is the language's scoping rule, so it is
//! worth spelling out.  Twig is lexically scoped like Scheme: the
//! innermost binding of a name is the one a use refers to, whether the
//! use is in value position (`helper`) or in call position
//! (`(helper 1)`).  Until #18435 the call-position lookup asked "is this a
//! top-level function?" *before* "is this a local?", so
//!
//! ```text
//! (define (helper x) (* x 100))
//! (define (f helper) (helper 1))   ; helper is f's PARAMETER here
//! (f (lambda (x) (+ x 1)))
//! ```
//!
//! compiled `f`'s body to `call helper` — the global — and returned 100
//! instead of 2.  Every name lookup in this file now consults the current
//! lexical frame first:
//!
//! | Site                                  | Lexical check                         |
//! |---------------------------------------|---------------------------------------|
//! | `compile_apply` direct / builtin call | `ctx.is_bound` before `fn_globals` /  |
//! |                                       | `BUILTINS`                            |
//! | `compile_var_ref` (value position)    | `ctx.lookup` first (always was)       |
//! | lambda free-variable analysis         | a local that shares a global's name   |
//! |                                       | is captured, not skipped as global    |
//! | E4 string fast paths                  | a shadowed `string-append` etc. is    |
//! |                                       | not lowered to `str_concat` etc.      |
//! | `main` direct-call string evidence    | a shadowed call is not evidence about |
//! |                                       | the global it happens to share a name |
//! |                                       | with                                  |
//!
//! The shadowed global loses *everything* at that site — its direct
//! `call`, its statically-known return type and parameter types — because
//! the callee is simply a different value.  `and` / `or` are special
//! forms, not bindings, and are not shadowable.
//!
//! ## LANG34 closure opcodes
//!
//! Since LANG34 the compiler emits first-class closure opcodes instead of
//! routing through `call_builtin`:
//!
//! - Lambda allocation: `alloc_closure(Str(fn_name), cap0, cap1, …) : "closure"`
//!   The function name is an inline `Operand::Str` — no preceding `const`
//!   instruction is needed.  The old `string_arg` helper is retained for
//!   `global_set`/`global_get` / `make_symbol` uses.
//! - Indirect calls: `call_closure(handle, arg0, arg1, …) : "any"`
//!   Replaces the former `call_builtin "apply_closure" handle args…` form.
//!
//! ## Encoding string operands (for globals and symbols)
//!
//! `Operand::Str(literal)` is now the canonical way to embed a compile-time
//! string in an instruction.  The `string_arg` helper (which emits a `const`
//! with `Operand::Var(literal)`) is retained for operations that still need
//! the old convention: `global_set`, `global_get`, `make_symbol`.  See the
//! LANG32 spec for details on the `global_load`/`global_store` lowering pass.

use std::collections::{HashMap, HashSet};

use interpreter_ir::{
    function::{FunctionTypeStatus, IIRFunction},
    instr::{IIRInstr, Operand},
    module::IIRModule,
    module_exports::IIRExport,
    SourceLoc,
};
use lang_refined_types::{Kind, Predicate, RefinedType};

use parser::grammar_parser::GrammarASTNode;

use crate::check_cst::check_cst;
use crate::cst::{
    classify_annotation, Annotation, ApplyView, ExprKind, ExprNode, Exprs, FormKind, IfView,
    LambdaView, LetView, MatchView, Pat, ProgramView, RecordView, TyItem, UnionView,
};
use crate::errors::TwigCompileError;
use crate::free_vars::free_vars;

// ---------------------------------------------------------------------------
// LANG23 PR 23-E — TypeAnnotation → RefinedType conversion
// ---------------------------------------------------------------------------

/// Convert a `type_annotation` node into a [`RefinedType`] that the IIR
/// carries and the refinement checker reads.
///
/// This is the bridge between the syntactic form (what the Twig grammar
/// accepts) and the semantic form (what `lang-refinement-checker`
/// understands).  [`classify_annotation`] reads the node; this function
/// only maps the class.
///
/// # Mapping
///
/// | annotation (class)                 | `RefinedType`                                       |
/// |------------------------------------|-----------------------------------------------------|
/// | `int` (`UnrefinedInt`)             | `RefinedType::unrefined(Kind::Int)`                 |
/// | `any` (`Any`)                      | `RefinedType::unrefined(Kind::Any)`                 |
/// | `bool` (`UnrefinedBool`)           | `RefinedType::unrefined(Kind::Bool)`                |
/// | `(Int lo hi)` (`RangeInt`)         | `RefinedType::refined(Int, Range{lo,hi,excl_hi})`   |
/// | `(Member int (v…))` (`MembershipInt`) | `RefinedType::refined(Int, Membership{values})`  |
/// | anything else (`Opaque`)           | `RefinedType::unrefined(Kind::Any)`                 |
fn type_annotation_to_refined_type(ann: &GrammarASTNode) -> RefinedType {
    match classify_annotation(ann) {
        Annotation::UnrefinedInt => RefinedType::unrefined(Kind::Int),
        Annotation::Any => RefinedType::unrefined(Kind::Any),
        Annotation::UnrefinedBool => RefinedType::unrefined(Kind::Bool),
        Annotation::RangeInt { lo, hi } => RefinedType::refined(
            Kind::Int,
            Predicate::Range {
                lo: Some(lo),
                hi: Some(hi),
                inclusive_hi: false, // Twig v1 always uses exclusive upper bound
            },
        ),
        Annotation::MembershipInt(values) => {
            RefinedType::refined(Kind::Int, Predicate::Membership { values })
        }
        // TW05-A / LANG48: Opaque annotations (non-LANG23 TW05 type expressions)
        // are erased to `any` in TW05-A.  The TW05-B type checker will interpret
        // them; for now they're treated as unconstrained.
        Annotation::Opaque(_) => RefinedType::unrefined(Kind::Any),
    }
}

/// A concrete IIR hint for a parameter annotation, when the annotation is
/// a bare string-type name (`str`, `Str`, `string`, `String`).  Such names
/// are `Opaque` to the LANG23 classifier; only their spelling matters.
fn type_annotation_static_iir_hint(ann: &GrammarASTNode) -> Option<&'static str> {
    match classify_annotation(ann) {
        Annotation::Opaque(node) => match crate::cst::ty_item(node) {
            TyItem::Name("str" | "Str" | "string" | "String") => Some("str"),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticParamEvidence {
    Unknown,
    Str,
    Conflict,
}

impl StaticParamEvidence {
    fn observe(self, is_str: bool) -> Self {
        match (self, is_str) {
            (StaticParamEvidence::Unknown, true) | (StaticParamEvidence::Str, true) => {
                StaticParamEvidence::Str
            }
            _ => StaticParamEvidence::Conflict,
        }
    }
}

/// Where a direct call sits, for the string-parameter evidence pre-pass
/// ([`Compiler::collect_direct_string_call_evidence`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EvidenceMode {
    /// A top-level expression or value-`define` RHS in `main`: a call here
    /// may establish `Str` evidence.
    Main,
    /// A function body, lambda body or `match` arm: a call here may only
    /// turn evidence into `Conflict`, never establish `Str`.
    Elsewhere,
}

/// Maximum AST-nesting depth the compiler will descend.
///
/// Mirrors [`twig_parser::MAX_AST_DEPTH`].  The parser and
/// [`crate::check_cst::check_cst`] already cap nesting on the way in, so
/// well-behaved inputs never hit this; the extra check protects against
/// `compile_ast` being called with a hand-built tree, and against future grammar
/// changes that might decouple parse depth from compile depth (e.g. a
/// macro-expansion pass that synthesises deeper trees).
pub const MAX_COMPILE_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Builtins
// ---------------------------------------------------------------------------
//
// Names that resolve to host-side callables registered with vm-core's
// `BuiltinRegistry`.  Keep this list in sync with the Python
// `twig.compiler.BUILTINS` set — the surface contract for what counts
// as a builtin is identical across language frontends.
// ---------------------------------------------------------------------------

const BUILTINS: &[&str] = &[
    // Arithmetic / comparison (TW00 core)
    "+", "-", "*", "/", "=", "<", ">",
    // LANG52: extended comparisons and arithmetic
    "<=", ">=", "modulo", "remainder", "quotient",
    // Cons cells
    "cons", "car", "cdr",
    // Predicates (TW00 core)
    "null?", "pair?", "number?", "symbol?",
    // LANG52: boolean and type predicates
    "not", "boolean?",
    // LANG52: structural equality
    "equal?",
    // LANG52: list stdlib
    "list", "length", "append", "reverse", "list-ref", "assoc", "list?",
    // LANG52: symbol utilities
    "symbol-append",
    // LANG57: string ↔ number ↔ symbol conversions (already in lispy-runtime LANG47+)
    "number->string", "string->symbol", "symbol->string",
    // LANG58: string and character operations (lispy-runtime LANG47, wired here for TW05-E)
    "string-length", "string-ref", "substring", "string-append",
    "string->number", "string=?", "string<?", "string>?",
    "char->integer", "integer->char",
    "char-alphabetic?", "char-numeric?", "char-whitespace?",
    // I/O
    "print",
    // Host I/O (LANG52) — these dispatch via call_builtin to exec_host_call
    "host/write_string", "host/read_line", "host/read_file",
    // LANG55: higher-order list operations — dispatch via special-cased
    // exec_hof_* handlers in twig-vm that can recurse into `dispatch`.
    "map", "filter", "fold-left", "fold-right",
];

fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

/// Path-A increment 2: map an arithmetic / comparison builtin name to the
/// typed CIR mnemonic that the IIR-to-* backends accept directly.
///
/// Returns `Some(&'static str)` when the builtin has a typed-CIR analog;
/// `None` for builtins that have no direct typed equivalent yet
/// (`cons`, `length`, `host/*`, the higher-order list ops, etc.).
///
/// The caller's responsibility is to verify that the operand types are
/// concrete before substituting the typed mnemonic — see the
/// `compile_apply` arm for `is_builtin(&v.name)`.  This table is the
/// pure name → name lookup; type-check happens at the call site.
///
/// Mirrors the same pattern used by `nib-iir-compiler::cir_op_for`
/// (PR #3903) and `oct-iir-compiler::compile_binary`.
fn typed_arith_op_for(name: &str) -> Option<&'static str> {
    match name {
        // Arithmetic — all four basic operators map to typed CIR mnemonics.
        "+" => Some("add"),
        "-" => Some("sub"),
        "*" => Some("mul"),
        "/" => Some("div"),
        // Comparison — Twig uses Scheme-style names; CIR uses C-style.
        // `=` is Twig's equality (works on numbers, symbols, booleans,
        // strings, etc.).  For increment 2 we only fire when both operands
        // are `i64`, in which case `=` reduces to integer equality —
        // which `cmp_eq` (with type `i64`) is exactly.
        "="  => Some("cmp_eq"),
        "<"  => Some("cmp_lt"),
        ">"  => Some("cmp_gt"),
        "<=" => Some("cmp_le"),
        ">=" => Some("cmp_ge"),
        _    => None,
    }
}

// ---------------------------------------------------------------------------
// Per-function compilation context
// ---------------------------------------------------------------------------

// ## Registers and bindings (#18439)
//
// IIR registers are named, and a Twig local is a register.  The obvious
// scheme — "a local's register is its source name" — is wrong as soon as two
// bindings share a name, because the second binding writes over the first:
//
// ```text
// (define (f x) (begin (let ((x 5)) x) x))   ; must return the PARAMETER x
//
//   one register per NAME            one register per BINDING
//   ---------------------            ------------------------
//   mov x ← 5     ; param x is gone  mov __r1_x ← 5  ; param x untouched
//   ret x         ; → 5   (wrong)    ret x           ; → 1   (right)
// ```
//
// Worse than a wrong value: the compiler keeps a static type per register
// (`var_types`) and lowers to typed ops (`str_len`, `add`, …) that the native
// backends trust.  When two bindings of different kinds share a register, a
// typed op can be pointed at a value of the other kind — memory-unsafe on a
// backend that believes the hint.
//
// So every binding gets a register of its own, through two tables:
//
// | Table     | Maps                                    | Changes when            |
// |-----------|-----------------------------------------|-------------------------|
// | `scope`   | source name → stack of registers, the   | a binder opens (push)   |
// |           | innermost binding on top                | or closes (pop)         |
// | `claimed` | every register name this function uses  | a register is handed out|
//
// A read of a name is "the register on top of its stack" — nothing else.
// Parameters, `let`, `let*`, lambda parameters, closure captures and `match`
// bindings all push; `compile_var_ref`, call-position lookups, closure
// captures and the E4 type checks all read through [`FnCtx::lookup`].
//
// ### Naming: keep the source name when it is free
//
// The FIRST binding of a name in a function keeps the name itself as its
// register, so a program without repeated names compiles to the very same IR
// it always did (parameter lists, debugger variable names and tests that
// spell `x` stay valid).  Any LATER binding of a name already claimed in the
// function — nested OR sibling — gets `__r{n}_{name}`, with `n` from a
// per-function counter.  Siblings need it too: `(let ((x 1)) x)` evaluates to
// register `x`, and that value may still be waiting to be used when the next
// `(let ((x "s")) …)` binds:
//
// ```text
// (+ (let ((x 1)) x) (begin (let ((x "s")) x) 2))
//   mov x ← 1; …; str_const __r1_x "s"; …; add x, _n3   ; x is still 1
// ```
//
// `claimed` makes the names collision-free whatever the user writes: a
// binding takes `name` only if no register of that spelling exists yet, a
// rebinding skips any `__r{n}_{name}` already taken, and [`FnCtx::fresh_var`]
// skips a temporary name a user binding already holds (`(let ((_n1 "s")) …)`).
// Nothing therefore depends on Twig source being unable to spell `__r1_x`.
//
// ### E4 aliases are just stack entries
//
// A string-producing binding (`(let ((s (string-append a b))) …)`) does not
// copy into a register of its own — the code-gen backends have no generic
// `mov [str]` — so its stack entry is the PRODUCER's register.  A read, a
// closure capture and a type query of `s` all see that register, because
// they all go through `lookup`.  (The capture used to read register `s`,
// which nothing had written.)
//
// ### Why `var_types` needs no save / restore
//
// `var_types` is keyed by register.  Every binding register is written by
// exactly one instruction (its `mov` / `str_const`, or the call for a
// parameter), so the type recorded at that write is true for the register's
// whole life — including after its scope closes, when the register can still
// be a pending operand.  Only the compiler's join temporaries (`if` / `and` /
// `or` / `match` results) have several writers, and their type is the
// consensus of all of them — see "Join temporaries" below.  The previous
// design shared registers between
// bindings and so had to snapshot, restore and drop types at every scope
// boundary (with a per-register write counter); with one writer per register
// all of that disappears.

// ## Join temporaries (#18448)
//
// `if`, `and`, `or` and `match` are *joins*: several control-flow paths
// each produce a value, and all of them must end up in one register the
// rest of the program reads.  The compiler allocates a fresh temporary for
// that and has each path `mov` its value into it:
//
// ```text
// (or (< 1 2) 5)         _r1 = cmp_lt 1, 2       [bool]
//                        jmp_if_false _r1, falsy
//                        mov _or2 ← _r1          [bool]   ; writer 1
//                        jmp end
//                 falsy: _n3 = const 5           [i64]
//                        mov _or2 ← _n3          [i64]    ; writer 2
//                   end: …
// ```
//
// What type does `_or2` have?  Only one that is true on *every* path,
// because the code that reads it cannot know which path ran.  So the
// type is the writers' **consensus**:
//
// | writers' types       | `_or2`'s type |
// |----------------------|---------------|
// | all `i64`            | `i64`         |
// | `bool` and `i64`     | none (`any`)  |
// | `str` and `any`      | none (`any`)  |
//
// "Type" here is a promise the native backends act on without checking: an
// `add [i64]` reads raw machine words, a `str_len` follows a pointer.  When
// `_or2` above was typed `i64` (the type of whichever writer happened to be
// compiled last), `(+ (or (< 1 2) 5) 1)` became an `add [i64]` on `#t`,
// and `(let ((y (or 5 "s"))) (string-length y))` became a `str_len` that
// reads the bytes of `5` as a string.
//
// Every writer counts, including the ones with no source-level expression:
//
// * `and`'s short-circuit path writes `#f` (`bool`), so `(and c 5)` is
//   untyped — it can be `#f` or `5` — while `(and (< a b) (< b c))` stays
//   `bool`;
// * `or`'s truthy path writes the test value itself;
// * `match` first writes `nil` (`ref<LispyPair>`), the result when no arm
//   matches, so a `match` is typed only if every arm is `nil`-typed too.
//
// [`JoinType`] folds the writers' types as they are emitted, and
// [`FnCtx::settle_join`] records the result (or clears it) once the last
// writer is in.  Nothing reads a join temporary's type before then: the
// paths are compiled before the join's value is used.

/// The consensus type of one join temporary's writers so far (see "Join
/// temporaries" above).
///
/// ```text
/// writers:   (none)   i64    i64,i64    i64,bool    i64,any
/// consensus:  any     i64      i64        any         any
/// ```
#[derive(Debug, Default)]
struct JoinType {
    /// `None` until the first writer; then the type every writer so far
    /// agrees on, or `"any"` once two disagree (or one is `any`).
    agreed: Option<String>,
}

impl JoinType {
    /// Count one more writer of type `ty`.
    fn add(&mut self, ty: &str) {
        self.agreed = Some(match self.agreed.take() {
            None => ty.to_string(),
            Some(prev) if prev == ty => prev,
            Some(_) => "any".to_string(),
        });
    }

    /// The type that holds on every path: `"any"` unless all writers agree
    /// on a concrete type.
    fn consensus(&self) -> &str {
        self.agreed.as_deref().unwrap_or("any")
    }
}

/// Mutable state while lowering one [`IIRFunction`].
///
/// `instrs` accumulates the body in emission order; `scope` maps each
/// name bound at this function level to its registers (see "Registers and
/// bindings" above) so [`Compiler::compile_var_ref`] can distinguish locals
/// from globals; the counters generate fresh register and label names that
/// won't collide.
struct FnCtx {
    instrs: Vec<IIRInstr>,
    /// Per-instruction source positions, kept in **lockstep** with
    /// [`Self::instrs`] (`source_map[i]` = position of `instrs[i]`).
    /// See [`interpreter_ir::SourceLoc`] for indexing conventions.
    source_map: Vec<SourceLoc>,
    /// Source name → registers of its live bindings, innermost last.  A name
    /// with no entry is not lexically bound here (it may be a global).
    scope: HashMap<String, Vec<String>>,
    /// Every register name handed out in this function: parameters, binding
    /// registers and temporaries.  Keeps all three collision-free.
    claimed: HashSet<String>,
    var_counter: usize,
    /// Numbers the `__r{n}_{name}` registers of repeated bindings.  Separate
    /// from `var_counter` so a rebinding does not shift temporary names.
    rebind_counter: usize,
    label_counter: usize,
    /// Current AST-nesting depth.  Incremented on every entry to
    /// `compile_expr` and checked against [`MAX_COMPILE_DEPTH`] to
    /// guard against stack-overflow on adversarial input.
    depth: usize,
    /// LANG-Twig increment-1 local type inference.
    ///
    /// Tracks the *statically-known* type of each destination variable
    /// produced by the current function.  Populated only for sites where
    /// the type is unambiguous from the source code alone (integer
    /// literals → `"i64"`, boolean literals → `"bool"`, string literals
    /// → `"str"`).  Dynamic / call_builtin destinations are intentionally
    /// **not** recorded — the absence of an entry means "the type is
    /// genuinely `any` at static-analysis time".
    ///
    /// Used by `ret` emission sites to upgrade their `type_hint` from
    /// the legacy `"any"` to the precise type, which lets the IIR-to-*
    /// backends (wasm/jvm/clr/beam) actually accept the result.
    /// See [`crate::lib`]'s "Twig path A" notes for the larger story.
    ///
    /// Keyed by REGISTER, never by source name: see "Why `var_types` needs
    /// no save / restore" above.
    var_types: HashMap<String, String>,
}

impl FnCtx {
    fn new() -> Self {
        FnCtx {
            instrs: Vec::new(),
            source_map: Vec::new(),
            scope: HashMap::new(),
            claimed: HashSet::new(),
            var_counter: 0,
            rebind_counter: 0,
            label_counter: 0,
            depth: 0,
            var_types: HashMap::new(),
        }
    }

    /// A new compiler temporary, `_{prefix}{n}`.  In the usual case this is
    /// the next number; it only skips ahead when a user binding already
    /// holds that spelling (Twig can name a local `_n1`).
    fn fresh_var(&mut self, prefix: &str) -> String {
        loop {
            self.var_counter += 1;
            let v = format!("_{prefix}{}", self.var_counter);
            if self.claimed.insert(v.clone()) {
                return v;
            }
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("_{prefix}{}", self.label_counter)
    }

    /// Push an instruction + its source position in lockstep.  Every
    /// IR-emit site goes through this — the lockstep invariant
    /// (`source_map.len() == instrs.len()`) is maintained by
    /// construction.  See [`SourceLoc::SYNTHETIC`] for instructions
    /// the compiler synthesises with no real source counterpart.
    fn emit(&mut self, instr: IIRInstr, loc: SourceLoc) {
        self.instrs.push(instr);
        self.source_map.push(loc);
    }

    /// Record that `var` was statically inferred to have type `ty`.
    ///
    /// Only literal-defining sites (`IntLit`, `BoolLit`, `StrLit`) call
    /// this; dynamically-typed sites (call_builtin, ret of unknown
    /// register, etc.) leave the entry absent, signalling "type is
    /// genuinely `any` at static-analysis time".
    fn record_type(&mut self, var: &str, ty: &str) {
        self.var_types.insert(var.to_string(), ty.to_string());
    }

    fn emit_str_const(&mut self, literal: &str, loc: SourceLoc) -> String {
        let v = self.fresh_var("s");
        self.emit_str_const_to(&v, literal, loc);
        v
    }

    fn emit_str_const_to(&mut self, dest: &str, literal: &str, loc: SourceLoc) {
        self.emit(
            IIRInstr::new(
                "str_const",
                Some(dest.to_string()),
                vec![Operand::Str(literal.to_string())],
                "str",
            ),
            loc,
        );
        self.record_type(dest, "str");
    }

    /// Look up the inferred type of `var`, returning the matching
    /// `&'static str` if known.  Returns `"any"` when nothing was
    /// inferred — which is the legacy default and a valid type hint
    /// (just one that the IIR-to-* backends reject).
    fn type_of(&self, var: &str) -> &str {
        self.var_types.get(var).map(|s| s.as_str()).unwrap_or("any")
    }

    // ── Bindings (#18439) ────────────────────────────────────────────

    /// The register a read of `name` returns, if `name` is lexically bound.
    fn lookup(&self, name: &str) -> Option<&str> {
        self.scope.get(name).and_then(|regs| regs.last()).map(String::as_str)
    }

    /// Is `name` lexically bound here (and so NOT the global or builtin
    /// that may share its spelling — #18435)?
    fn is_bound(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Claim a register for a new binding of `name`: `name` itself if this
    /// function has no register of that spelling yet, else `__r{n}_{name}`.
    /// The binding is not visible until [`Self::push_binding`].
    fn binding_register(&mut self, name: &str) -> String {
        if self.claimed.insert(name.to_string()) {
            return name.to_string();
        }
        loop {
            self.rebind_counter += 1;
            let reg = format!("__r{}_{name}", self.rebind_counter);
            if self.claimed.insert(reg.clone()) {
                return reg;
            }
        }
    }

    /// Make `name` read register `reg` until the matching
    /// [`Self::pop_binding`].  `reg` is either a fresh
    /// [`Self::binding_register`] or, for an E4 string alias, the register
    /// that already holds the value.
    fn push_binding(&mut self, name: &str, reg: String) {
        self.scope.entry(name.to_string()).or_default().push(reg);
    }

    /// Close the innermost binding of `name`; the next outer one (if any)
    /// becomes visible again.  Its register, and the register's type, are
    /// untouched — they were never written by the inner binding.
    fn pop_binding(&mut self, name: &str) {
        if let Some(regs) = self.scope.get_mut(name) {
            regs.pop();
            if regs.is_empty() {
                self.scope.remove(name);
            }
        }
    }

    /// Bind a parameter (or closure capture): claim its register, make it
    /// visible for the rest of the function, and return the register to use
    /// in the `IIRFunction::params` list.
    fn bind_param(&mut self, name: &str) -> String {
        let reg = self.binding_register(name);
        self.push_binding(name, reg.clone());
        reg
    }

    /// Bind `name` to a copy of `src` in a register of its own (typed `mov`)
    /// and return that register.
    fn bind_move(&mut self, name: &str, src: &str, loc: SourceLoc) -> String {
        let reg = self.binding_register(name);
        self.emit_move(&reg, src, loc);
        self.push_binding(name, reg.clone());
        reg
    }

    /// Path-A increment 3: emit a typed `mov dst = src` instruction,
    /// propagating the source's inferred type to the destination.
    ///
    /// Replaces the legacy `call_builtin "_move" src` emission pattern
    /// that the IIR-to-* backend validators all reject (because
    /// `_move` is not in their `CALL_BUILTIN_SUPPORTED_NAMES` whitelist).
    /// The typed `mov` IR opcode is accepted by every backend
    /// (vm-core dispatch fix #3888, iir-to-beam mov lowering #3898,
    /// and the iir-to-wasm / iir-to-jvm / iir-to-cil backends all
    /// have native `"mov"` arms in their `lower.rs` match tables).
    ///
    /// The `type_hint` carried on the `mov` is the source variable's
    /// type — `"any"` for dynamically-typed sources, `"i64"` / `"bool"`
    /// for sources statically inferred by increments 1 and 2.
    ///
    /// When the source's type is known, the destination's type is
    /// recorded so downstream consumers (e.g. the outer `ret` in
    /// `main`) can propagate further.
    fn emit_move(&mut self, dst: &str, src: &str, loc: SourceLoc) {
        let ty = self.type_of(src).to_string();
        self.emit(
            IIRInstr::new(
                "mov",
                Some(dst.to_string()),
                vec![Operand::Var(src.to_string())],
                ty.as_str(),
            ),
            loc,
        );
        if ty != "any" {
            self.record_type(dst, ty.as_str());
        }
    }

    /// Emit `mov dst ← src` as one writer of the join temporary `dst`, and
    /// count its type in `join`.
    ///
    /// The `mov` carries the *source's* type, exactly as [`Self::emit_move`]
    /// does — that is what this path really moves.  What it does not do is
    /// record a type for `dst`: one path's type is not the join's.  Call
    /// [`Self::settle_join`] once every writer has been emitted.
    fn emit_join_move(&mut self, dst: &str, src: &str, loc: SourceLoc, join: &mut JoinType) {
        let ty = self.type_of(src).to_string();
        self.emit(
            IIRInstr::new(
                "mov",
                Some(dst.to_string()),
                vec![Operand::Var(src.to_string())],
                ty.as_str(),
            ),
            loc,
        );
        join.add(&ty);
    }

    /// Record the join temporary `dst`'s type now that all its writers are
    /// in: their consensus, or no type at all when they disagree.
    fn settle_join(&mut self, dst: &str, join: &JoinType) {
        match join.consensus() {
            "any" => {
                self.var_types.remove(dst);
            }
            ty => self.record_type(dst, ty),
        }
    }
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

/// Walks a Twig parse tree and accumulates [`IIRFunction`]s.
///
/// One instance compiles one program.  Pre-pass classifies top-level
/// defines into `fn_globals` / `value_globals`; the main pass walks
/// every form and emits IR.  Anonymous lambdas append to
/// [`Self::functions`] as they are encountered (depth-first).
pub struct Compiler {
    /// Names of top-level defines whose RHS is a `Lambda` — direct
    /// callables.
    fn_globals: HashSet<String>,
    /// Statically-known return types for top-level functions already lowered in
    /// source order. Direct calls that appear after such a definition can carry
    /// the concrete IIR type instead of falling back to `any`.
    fn_return_types: HashMap<String, String>,
    /// Statically-known parameter types for top-level functions already lowered
    /// in source order. Used only to keep later direct string arguments on the
    /// E4 `str_const`/string-expression path when a callee parameter is `str`.
    fn_param_types: HashMap<String, Vec<String>>,
    /// Conservative call-site-derived parameter hints for top-level functions.
    /// A hint appears only when `main`-level direct calls provide static E4
    /// string evidence and no conflicting evidence for that parameter.
    fn_inferred_param_types: HashMap<String, Vec<String>>,
    /// Names of top-level defines whose RHS is *not* a lambda — looked
    /// up through `global_get` at use sites.
    value_globals: HashSet<String>,
    /// TW2: value-globals that are captured by a lambda somewhere and so MUST
    /// stay on the host global table (`global_set` / `global_get`).  Computed
    /// once in the pre-pass by [`free_vars::lambda_captured_globals`].
    escaping_value_globals: HashSet<String>,
    /// TW2: a *non-escaping*, statically-typed value-global's main-`fn` register.
    /// Reads of these names return the register directly (a typed value the
    /// code-gen backends accept) instead of emitting a dynamic `global_get`.
    value_global_locals: HashMap<String, String>,
    /// TW2: value-globals read *before* their `define` (a top-level forward
    /// reference).  Such a name already emitted a `global_get`, so its later
    /// `define` must emit the matching `global_set` rather than the typed-local
    /// form — keeping behaviour byte-identical to the pre-TW2 dynamic path.
    forced_global_set: HashSet<String>,
    /// Cumulative function table.  Top-level fns are appended in
    /// source order; anonymous lambdas append as the compiler
    /// encounters them, with `main` appended last.
    functions: Vec<IIRFunction>,
    /// Counter for synthesising lambda names (`__lambda_0`,
    /// `__lambda_1`, …).
    lambda_counter: usize,
    /// TW05-A / LANG48: integer tag for each union variant constructor.
    ///
    /// Populated during the pre-pass when `(union …)` forms are
    /// encountered.  Consulted when lowering `(match …)` arms — a
    /// pattern whose name is in this map is a variant pattern;
    /// otherwise it is a bare-name binding.
    variant_tags: HashMap<String, usize>,
    /// True once [`Self::with_extern_fns`] registered at least one extern —
    /// i.e. this program is one module of a `twig-module-driver` tree, where
    /// other modules can call its functions (see
    /// [`Self::infer_main_direct_string_param_types`]).
    has_externs: bool,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            fn_globals: HashSet::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            fn_inferred_param_types: HashMap::new(),
            value_globals: HashSet::new(),
            escaping_value_globals: HashSet::new(),
            value_global_locals: HashMap::new(),
            forced_global_set: HashSet::new(),
            functions: Vec::new(),
            lambda_counter: 0,
            variant_tags: HashMap::new(),
            has_externs: false,
        }
    }

    /// Pre-populate `fn_globals` with extern function names from other modules.
    ///
    /// Called by `twig-module-driver` (LANG56) before compilation so that
    /// cross-module calls compile to `call` instructions rather than failing
    /// with "unbound name".  The linker resolves the actual call targets.
    ///
    /// # Example
    ///
    /// ```
    /// use twig_ir_compiler::Compiler;
    ///
    /// let c = Compiler::new().with_extern_fns(&["double", "triple"]);
    /// // Now the compiler will accept `(double x)` even without a local define.
    /// ```
    pub fn with_extern_fns(mut self, extern_fns: &[&str]) -> Self {
        for name in extern_fns {
            self.fn_globals.insert((*name).to_string());
        }
        self.has_externs |= !extern_fns.is_empty();
        self
    }

    // ------------------------------------------------------------------
    // Top-level driver
    // ------------------------------------------------------------------

    /// Compile a Twig parse tree into an [`IIRModule`].  Consumes `self`.
    ///
    /// `program` must be the `program` root returned by
    /// [`twig_parser::parse_to_ast`].  It is validated by
    /// [`crate::check_cst::check_cst`] first (a malformed tree is reported
    /// as `parse error: …`, exactly as the old typed-AST extraction
    /// reported it), then lowered by walking the tree directly.
    ///
    /// This is the lowering only — the optional TW05-B type-check gate
    /// lives in [`crate::compile_ast`] and its siblings.
    pub fn compile(mut self, program: &GrammarASTNode, module_name: &str) -> Result<IIRModule, TwigCompileError> {
        check_cst(program)?;
        let program = ProgramView(program);

        // ── Pre-pass: classify top-level defines + register typed forms ────
        //
        // Free-variable analysis at lambda sites needs to know which names are
        // globals (and therefore *not* free) before we walk any bodies, so we
        // do this in one pre-pass.
        //
        // TW05-A / LANG48 additions:
        // - `Form::RecordDef` generates constructor + accessors — all registered
        //   as `fn_globals` so direct calls reach the fast `call` path.
        // - `Form::UnionDef` generates constructors + predicates + accessors *and*
        //   populates `self.variant_tags` so `match` lowering can look up tags.
        for form in program.forms() {
            match form {
                FormKind::Define(def) => {
                    if def.lambda().is_some() {
                        self.fn_globals.insert(def.name().to_string());
                    } else {
                        self.value_globals.insert(def.name().to_string());
                    }
                }
                FormKind::RecordDef(rec) => {
                    // Constructor name (same as type name, e.g. "Span").
                    self.fn_globals.insert(rec.name().to_string());
                    // Accessor names: <lower(RecordName)>-<field-name>.
                    let prefix = rec.name().to_lowercase();
                    for f in rec.fields() {
                        self.fn_globals.insert(format!("{prefix}-{f}"));
                    }
                    // Type predicate: <lower(RecordName)>?
                    self.fn_globals.insert(format!("{prefix}?"));
                }
                FormKind::UnionDef(union) => {
                    for (idx, variant) in union.variants().enumerate() {
                        // Constructor name (same as variant name, e.g. "IntLit").
                        self.fn_globals.insert(variant.name().to_string());
                        // Predicate: <VariantName>?
                        self.fn_globals.insert(format!("{}?", variant.name()));
                        // Accessor names: <lower(VariantName)>-<field-name>.
                        let vprefix = variant.name().to_lowercase();
                        for f in variant.fields() {
                            self.fn_globals.insert(format!("{vprefix}-{f}"));
                        }
                        // Register the integer tag for match-arm lowering.
                        self.variant_tags.insert(variant.name().to_string(), idx);
                    }
                }
                FormKind::TypeAlias | FormKind::Expr(_) => {}
            }
        }

        // TW2: decide which value-globals may be lowered to typed `main`
        // locals.  A value-global captured by any lambda must stay on the host
        // global table (the closure compiles to a separate function); the rest
        // are read only from `main` and can live in a register.
        self.escaping_value_globals =
            crate::free_vars::lambda_captured_globals(program, &self.value_globals);

        self.infer_main_direct_string_param_types(program);

        // ── Main pass: lower every form ──────────────────────────────
        let mut main_ctx = FnCtx::new();
        let mut last_main_value: Option<String> = None;

        for form in program.forms() {
            match form {
                FormKind::Define(def) if def.lambda().is_some() => {
                    // (define (f ...) ...) or (define f (lambda ...))
                    let lam = def.lambda().expect("guarded by is_some() above");
                    self.compile_top_level_lambda(def.name(), lam)?;
                }
                FormKind::Define(def) => {
                    // (define x value-expr) — evaluate at top level.
                    let (line, column) = def.pos();
                    let loc = SourceLoc::new(line, column);
                    let name = def.name();
                    let value_expr = def.value().ok_or_else(|| TwigCompileError {
                        message: format!("define {name:?} has no value expression"),
                        line,
                        column,
                    })?;
                    if let ExprKind::StrLit(value) = value_expr.kind() {
                        if !self.escaping_value_globals.contains(name)
                            && !self.forced_global_set.contains(name)
                        {
                            let v = main_ctx.emit_str_const(value, loc);
                            self.value_global_locals.insert(name.to_string(), v);
                            last_main_value = None;
                            continue;
                        }
                    }

                    let v = self.compile_expr(value_expr, &mut main_ctx)?;

                    // TW2: when the value is statically typed (`i64` / `bool`
                    // / main-only E4 `str`)
                    // and the name is neither captured by a lambda nor already
                    // forward-referenced, keep it in the register `v` and skip
                    // the dynamic `global_set` entirely.  Reads in `main` then
                    // return that register (see `compile_var_ref`), so the whole
                    // of `main` stays typed and clears every backend validator.
                    let ty = main_ctx.type_of(&v);
                    let typed = matches!(ty, "i64" | "bool" | "str");
                    if typed
                        && !self.escaping_value_globals.contains(name)
                        && !self.forced_global_set.contains(name)
                    {
                        self.value_global_locals.insert(name.to_string(), v);
                    } else {
                        // Captured / dynamically-typed / forward-referenced:
                        // store on the host global table as before.
                        let name_reg = self.string_arg(&mut main_ctx, name, loc);
                        main_ctx.emit(IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![
                                Operand::Var("global_set".into()),
                                Operand::Var(name_reg),
                                Operand::Var(v),
                            ],
                            "void",
                        ), loc);
                    }
                    last_main_value = None;
                }
                FormKind::Expr(e) => {
                    last_main_value = Some(self.compile_expr(e, &mut main_ctx)?);
                }

                // ── TW05-A / LANG48 typed-syntax forms ────────────────
                //
                // `type_alias`: compile-time only; no IIR emitted.
                FormKind::TypeAlias => {}

                // `record_def`: erased to constructor + accessor IIR fns.
                FormKind::RecordDef(rec) => self.emit_record_def(rec)?,

                // `union_def`: erased to integer-tagged constructors + predicates + accessors.
                FormKind::UnionDef(union) => self.emit_union_def(union)?,
            }
        }

        // ── Synthesise `main` ────────────────────────────────────────
        if let Some(reg) = last_main_value {
            // Twig path-A increment 1: propagate the source var's inferred
            // type to the `ret` instruction.  `_n1` returned from `42`
            // becomes `ret _n1 [i64]` instead of `ret _n1 [any]`, which
            // every IIR-to-* backend validator now accepts.
            let ret_ty = main_ctx.type_of(&reg).to_string();
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(reg)],
                ret_ty.as_str(),
            ), SourceLoc::SYNTHETIC);
        } else {
            // No final value-producing expression → return nil.
            //
            // Path A increment 6a: emit `const 0 [ref<LispyPair>]`
            // instead of `call_builtin "make_nil" [any]`.  This matches
            // the Phase 2 heap-lowering convention already used by the
            // IIR-to-{wasm,jvm,clr,beam} backends — nil is the null
            // LispyPair reference (represented as 0).  The typed `const`
            // is accepted by every backend, so programs whose only
            // return path is the implicit nil now flow through every
            // backend without going through the `call_builtin` fallback.
            let nil_var = main_ctx.fresh_var("nil");
            main_ctx.emit(IIRInstr::new(
                "const",
                Some(nil_var.clone()),
                vec![Operand::Int(0)],
                "ref<LispyPair>",
            ), SourceLoc::SYNTHETIC);
            main_ctx.record_type(&nil_var, "ref<LispyPair>");
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(nil_var)],
                "ref<LispyPair>",
            ), SourceLoc::SYNTHETIC);
        }

        // Twig path-A increment 1: derive `main`'s `return_type` from
        // the trailing `ret` instruction's `type_hint`.  This lets the
        // IIR-to-* backends emit a typed return (`ret_i64`, `ret_bool`,
        // …) instead of falling back to the untyped fallback path.
        // Functions whose ret source is genuinely dynamic still carry
        // `"any"` and continue to flow through the existing dynamic path.
        let main_return_type = main_ctx
            .instrs
            .iter()
            .rev()
            .find(|i| i.op == "ret")
            .map(|i| i.type_hint.clone())
            .unwrap_or_else(|| "any".to_string());

        let main_fn = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: main_return_type,
            register_count: count_registers(&main_ctx.instrs),
            instructions: main_ctx.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: main_ctx.source_map,
            param_refinements: Vec::new(),
            return_refinement: None,
        };
        self.functions.push(main_fn);

        // ── LANG56: populate exports from module_info ─────────────────────────
        //
        // When the program carries a `(module name (export f1 f2 ...))` clause,
        // register each exported name as an `IIRExport` — but only for names
        // that were actually compiled as top-level functions.  Value-defines,
        // builtins, and undeclared names are silently skipped rather than
        // erroring; the type-checker enforces that exported names exist.
        //
        // Non-root modules (those without a top-level expression body) will
        // have their `entry_point` cleared by `compile_module_tree` in
        // `twig-module-driver`.  We always produce `entry_point = Some("main")`
        // here; the driver overwrites it for library modules.
        let exports: Vec<IIRExport> = program
            .module()
            .map(|mi| {
                mi.exports()
                    .filter(|name| self.fn_globals.contains(*name))
                    .map(IIRExport::new)
                    .collect()
            })
            .unwrap_or_default();

        Ok(IIRModule {
            name: module_name.to_string(),
            functions: self.functions,
            entry_point: Some("main".to_string()),
            language: "twig".to_string(),
            exports,
            imports: Vec::new(),
            release: None,
        })
    }

    fn infer_main_direct_string_param_types(&mut self, program: ProgramView<'_>) {
        // Module code gets no call-site inference at all.  `twig-module-driver`
        // registers EVERY module's function names — private ones included —
        // as externs of every other module, so no single module sees all the
        // call sites of its functions:
        //
        //   lib.tw   (module lib (export run))
        //            (define (strlen x) (string-length x))   ; private
        //            (define n (strlen "abc"))               ; only call here
        //   main.tw  (module main (import lib)) (strlen 5)   ; ← an integer
        //
        // Inferring `x : str` from lib alone would lower `strlen` to `str_len`
        // and then hand it 5.  A `(module …)` declaration, or any extern
        // registered by the driver, marks the program as module code; every
        // parameter then stays dynamic (`fn_inferred_param_types` is left
        // empty).  Explicit `(x : str)` annotations still apply.
        if self.has_externs || program.module().is_some() {
            return;
        }
        let mut evidence: HashMap<String, Vec<StaticParamEvidence>> = HashMap::new();
        let mut known_static_string_values: HashSet<String> = HashSet::new();
        let mut defined_value_globals: HashSet<String> = HashSet::new();
        let mut forward_referenced_values: HashSet<String> = HashSet::new();

        for form in program.forms() {
            if let FormKind::Define(def) = form {
                if let Some(lam) = def.lambda() {
                    evidence.insert(
                        def.name().to_string(),
                        vec![StaticParamEvidence::Unknown; lam.params().count()],
                    );
                }
            }
        }

        for form in program.forms() {
            match form {
                FormKind::Expr(expr) => {
                    self.record_main_forward_value_refs(
                        expr,
                        &defined_value_globals,
                        &mut forward_referenced_values,
                        &HashSet::new(),
                    );
                    self.collect_direct_string_call_evidence(
                        expr,
                        &mut evidence,
                        &known_static_string_values,
                        &HashSet::new(),
                        EvidenceMode::Main,
                    );
                }
                FormKind::Define(def) if def.lambda().is_some() => {
                    known_static_string_values.remove(def.name());
                    // A function body is not `main`: calls in it may only
                    // *weaken* evidence (see `EvidenceMode::Elsewhere`).
                    let lam = def.lambda().expect("guarded by is_some()");
                    self.collect_lambda_string_call_evidence(lam, &mut evidence, &HashSet::new());
                }
                FormKind::Define(def) => {
                    let Some(value) = def.value() else { continue };
                    self.record_main_forward_value_refs(
                        value,
                        &defined_value_globals,
                        &mut forward_referenced_values,
                        &HashSet::new(),
                    );
                    self.collect_direct_string_call_evidence(
                        value,
                        &mut evidence,
                        &known_static_string_values,
                        &HashSet::new(),
                        EvidenceMode::Main,
                    );
                    let name = def.name();
                    if !self.escaping_value_globals.contains(name)
                        && !forward_referenced_values.contains(name)
                        && Self::is_syntax_static_e4_string_expr(
                            value,
                            &known_static_string_values,
                            &HashSet::new(),
                        )
                    {
                        known_static_string_values.insert(name.to_string());
                    } else {
                        known_static_string_values.remove(name);
                    }
                    defined_value_globals.insert(name.to_string());
                }
                _ => {}
            }
        }

        self.fn_inferred_param_types = evidence
            .into_iter()
            .map(|(name, slots)| {
                let types = slots
                    .into_iter()
                    .map(|e| {
                        if e == StaticParamEvidence::Str {
                            "str"
                        } else {
                            "any"
                        }
                        .to_string()
                    })
                    .collect();
                (name, types)
            })
            .collect();
    }

    fn record_main_forward_value_refs(
        &self,
        expr: ExprNode<'_>,
        defined_value_globals: &HashSet<String>,
        forward_referenced_values: &mut HashSet<String>,
        shadowed: &HashSet<String>,
    ) {
        match expr.kind() {
            ExprKind::VarRef(name) => {
                if self.value_globals.contains(name)
                    && !defined_value_globals.contains(name)
                    && !shadowed.contains(name)
                {
                    forward_referenced_values.insert(name.to_string());
                }
            }
            ExprKind::If(i) => {
                self.record_main_forward_value_refs(i.cond, defined_value_globals, forward_referenced_values, shadowed);
                self.record_main_forward_value_refs(i.then_branch, defined_value_globals, forward_referenced_values, shadowed);
                self.record_main_forward_value_refs(i.else_branch, defined_value_globals, forward_referenced_values, shadowed);
            }
            ExprKind::Begin(exprs) => {
                for e in exprs.iter() {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, shadowed);
                }
            }
            ExprKind::Let(l) => {
                for (_, rhs) in l.bindings() {
                    self.record_main_forward_value_refs(rhs, defined_value_globals, forward_referenced_values, shadowed);
                }
                let mut body_shadowed = shadowed.clone();
                for (name, _) in l.bindings() {
                    body_shadowed.insert(name.to_string());
                }
                for e in l.body().iter() {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &body_shadowed);
                }
            }
            ExprKind::LetStar(l) => {
                let mut scoped_shadowed = shadowed.clone();
                for (name, rhs) in l.bindings() {
                    self.record_main_forward_value_refs(rhs, defined_value_globals, forward_referenced_values, &scoped_shadowed);
                    scoped_shadowed.insert(name.to_string());
                }
                for e in l.body().iter() {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &scoped_shadowed);
                }
            }
            ExprKind::Apply(apply) => {
                if let Some(f) = apply.fn_expr() {
                    self.record_main_forward_value_refs(f, defined_value_globals, forward_referenced_values, shadowed);
                }
                for arg in apply.args().iter() {
                    self.record_main_forward_value_refs(arg, defined_value_globals, forward_referenced_values, shadowed);
                }
            }
            ExprKind::Match(m) => {
                if let Some(s) = m.scrutinee() {
                    self.record_main_forward_value_refs(s, defined_value_globals, forward_referenced_values, shadowed);
                }
                for arm in m.arms() {
                    let mut arm_shadowed = shadowed.clone();
                    for name in arm.pat().bound_names() {
                        arm_shadowed.insert(name.to_string());
                    }
                    for e in arm.body().iter() {
                        self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &arm_shadowed);
                    }
                }
            }
            ExprKind::Lambda(_) => {}
            ExprKind::IntLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::NilLit
            | ExprKind::SymLit(_)
            | ExprKind::StrLit(_) => {}
        }
    }

    /// Walk one expression and record, for every *direct* call `(f a…)` to a
    /// top-level function `f`, whether each actual argument is a static E4
    /// string — the evidence that lets `f`'s unannotated parameters be typed
    /// `str`.
    ///
    /// ## Soundness: every route into `f` must be seen
    ///
    /// Typing a parameter `str` is a promise that *every* value reaching it
    /// is a string (the body then lowers to `str_len`, `str_concat`, …).  So
    /// the walk covers the whole program — `main`, every function body,
    /// every lambda and `match` arm — and two rules guard the routes a
    /// syntactic walk cannot follow:
    ///
    /// | Use of top-level `f`                              | Effect on `f`'s slots    |
    /// |---------------------------------------------------|--------------------------|
    /// | direct call in `main` (`EvidenceMode::Main`)      | `observe` each argument  |
    /// | direct call anywhere else (`Elsewhere`)           | non-string arg ⇒ Conflict|
    /// | wrong number of arguments                         | all ⇒ Conflict           |
    /// | `f` as a value: `(let ((g f)) …)`, `(define h f)`,| all ⇒ Conflict (escape:  |
    /// | `(map f xs)`, returning `f`, …                    | callers are unknowable)  |
    /// | program is module code (`(module …)` or externs)  | no inference at all      |
    ///
    /// Only `main` may *establish* `Str`; everywhere else can only weaken it.
    /// That keeps the old behaviour for the programs the fast path was built
    /// for while making every other route conservative.
    ///
    /// ## Shadowing (#18435)
    ///
    /// `shadowed` holds the names bound by enclosing parameters, `let` /
    /// `let*`, lambdas and `match` arms.  A call whose head is one of them
    /// calls that local closure, not the top-level function of the same
    /// name, and a shadowed `string-append` / `substring` inside an argument
    /// is no longer a string builtin.
    fn collect_direct_string_call_evidence(
        &self,
        expr: ExprNode<'_>,
        evidence: &mut HashMap<String, Vec<StaticParamEvidence>>,
        known_static_string_values: &HashSet<String>,
        shadowed: &HashSet<String>,
        mode: EvidenceMode,
    ) {
        match expr.kind() {
            ExprKind::If(i) => {
                self.collect_direct_string_call_evidence(i.cond, evidence, known_static_string_values, shadowed, mode);
                self.collect_direct_string_call_evidence(i.then_branch, evidence, known_static_string_values, shadowed, mode);
                self.collect_direct_string_call_evidence(i.else_branch, evidence, known_static_string_values, shadowed, mode);
            }
            ExprKind::Begin(exprs) => {
                for e in exprs.iter() {
                    self.collect_direct_string_call_evidence(e, evidence, known_static_string_values, shadowed, mode);
                }
            }
            ExprKind::Let(l) => {
                // RHSs see the OUTER scope (Scheme `let`); the body sees the
                // bindings, which shadow any global of the same name.
                for (_, rhs) in l.bindings() {
                    self.collect_direct_string_call_evidence(rhs, evidence, known_static_string_values, shadowed, mode);
                }
                let mut body_static_string_values = known_static_string_values.clone();
                let mut body_shadowed = shadowed.clone();
                for (name, _) in l.bindings() {
                    body_static_string_values.remove(name);
                    body_shadowed.insert(name.to_string());
                }
                for (name, rhs) in l.bindings() {
                    if Self::is_syntax_static_e4_string_expr(rhs, known_static_string_values, shadowed) {
                        body_static_string_values.insert(name.to_string());
                    }
                }
                for e in l.body().iter() {
                    self.collect_direct_string_call_evidence(e, evidence, &body_static_string_values, &body_shadowed, mode);
                }
            }
            ExprKind::LetStar(l) => {
                // Sequential: each RHS sees every earlier binding.
                let mut scoped_static_string_values = known_static_string_values.clone();
                let mut scoped_shadowed = shadowed.clone();
                for (name, rhs) in l.bindings() {
                    self.collect_direct_string_call_evidence(rhs, evidence, &scoped_static_string_values, &scoped_shadowed, mode);
                    if Self::is_syntax_static_e4_string_expr(rhs, &scoped_static_string_values, &scoped_shadowed) {
                        scoped_static_string_values.insert(name.to_string());
                    } else {
                        scoped_static_string_values.remove(name);
                    }
                    scoped_shadowed.insert(name.to_string());
                }
                for e in l.body().iter() {
                    self.collect_direct_string_call_evidence(e, evidence, &scoped_static_string_values, &scoped_shadowed, mode);
                }
            }
            ExprKind::Apply(apply) => {
                // Only an UNshadowed head is a direct call to the top-level
                // function `f`; a shadowed one is a closure call.
                if let Some(f) = apply.fn_name().filter(|f| !shadowed.contains(*f)) {
                    if let Some(slots) = evidence.get_mut(f) {
                        if apply.args().len() == slots.len() {
                            for (slot, arg) in slots.iter_mut().zip(apply.args().iter()) {
                                let is_str = Self::is_syntax_static_e4_string_expr(
                                    arg,
                                    known_static_string_values,
                                    shadowed,
                                );
                                *slot = match mode {
                                    EvidenceMode::Main => slot.observe(is_str),
                                    EvidenceMode::Elsewhere if is_str => *slot,
                                    EvidenceMode::Elsewhere => StaticParamEvidence::Conflict,
                                };
                            }
                        } else {
                            slots.fill(StaticParamEvidence::Conflict);
                        }
                    }
                }
                // A bare-name head is a call, not an escape — walk the head
                // only when it is a computed expression like `((g) 1)`.
                if apply.fn_name().is_none() {
                    if let Some(f) = apply.fn_expr() {
                        self.collect_direct_string_call_evidence(f, evidence, known_static_string_values, shadowed, mode);
                    }
                }
                for arg in apply.args().iter() {
                    self.collect_direct_string_call_evidence(arg, evidence, known_static_string_values, shadowed, mode);
                }
            }
            // `f` used as a value escapes: any closure call may reach it.
            ExprKind::VarRef(name) => {
                if !shadowed.contains(name) {
                    if let Some(slots) = evidence.get_mut(name) {
                        slots.fill(StaticParamEvidence::Conflict);
                    }
                }
            }
            // Lambda bodies and match arms run with dynamic closure / match
            // values, so they may only weaken evidence.
            ExprKind::Lambda(lam) => {
                self.collect_lambda_string_call_evidence(lam, evidence, shadowed);
            }
            ExprKind::Match(m) => {
                if let Some(scrutinee) = m.scrutinee() {
                    self.collect_direct_string_call_evidence(scrutinee, evidence, known_static_string_values, shadowed, mode);
                }
                for arm in m.arms() {
                    let mut arm_shadowed = shadowed.clone();
                    arm_shadowed.extend(arm.pat().bound_names().map(str::to_string));
                    for e in arm.body().iter() {
                        self.collect_direct_string_call_evidence(e, evidence, &HashSet::new(), &arm_shadowed, EvidenceMode::Elsewhere);
                    }
                }
            }
            ExprKind::IntLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::NilLit
            | ExprKind::SymLit(_)
            | ExprKind::StrLit(_) => {}
        }
    }

    /// [`Self::collect_direct_string_call_evidence`] over a lambda's body, in
    /// `Elsewhere` mode with its parameters added to `shadowed`.  Used for
    /// anonymous lambdas and for top-level function bodies alike.
    fn collect_lambda_string_call_evidence(
        &self,
        lam: LambdaView<'_>,
        evidence: &mut HashMap<String, Vec<StaticParamEvidence>>,
        shadowed: &HashSet<String>,
    ) {
        let mut body_shadowed = shadowed.clone();
        body_shadowed.extend(lam.params().map(|p| p.name.to_string()));
        for e in lam.body().iter() {
            self.collect_direct_string_call_evidence(
                e,
                evidence,
                &HashSet::new(),
                &body_shadowed,
                EvidenceMode::Elsewhere,
            );
        }
    }

    /// The `(f a…)` view of `expr` when it is an application whose function
    /// position is a bare name: `Some((f, args))`.
    fn named_apply(expr: ExprNode<'_>) -> Option<(&str, Exprs<'_>)> {
        match expr.kind() {
            ExprKind::Apply(apply) => Some((apply.fn_name()?, apply.args())),
            _ => None,
        }
    }

    /// [`Self::named_apply`], but only when `f` is **not** lexically bound
    /// — i.e. when `f` really names a builtin or top-level function.
    ///
    /// The E4 string fast paths recognise calls by name (`string-append`,
    /// `substring`, `string-length`, `+`).  If a parameter or `let` binding
    /// is called `string-append`, `(string-append a b)` calls *that* closure
    /// (#18435), and treating it as the builtin would compile a
    /// `str_concat` the program never asked for.  `is_bound` answers "is
    /// this name bound at this point?": [`FnCtx::is_bound`] during lowering,
    /// the pre-pass's `shadowed` set before it.
    fn unshadowed_apply<'a>(
        expr: ExprNode<'a>,
        is_bound: &dyn Fn(&str) -> bool,
    ) -> Option<(&'a str, Exprs<'a>)> {
        Self::named_apply(expr).filter(|(f, _)| !is_bound(f))
    }

    fn is_syntax_static_e4_string_expr(
        expr: ExprNode<'_>,
        known_static_string_values: &HashSet<String>,
        shadowed: &HashSet<String>,
    ) -> bool {
        match expr.kind() {
            ExprKind::StrLit(_) => true,
            ExprKind::VarRef(name) => known_static_string_values.contains(name),
            ExprKind::Apply(_) => {
                let Some((f, args)) = Self::unshadowed_apply(expr, &|n| shadowed.contains(n)) else {
                    return false;
                };
                let arg = |i: usize| args.get(i).expect("length checked first");
                match f {
                    "string-append" => {
                        args.len() == 2
                            && Self::is_syntax_static_e4_string_expr(arg(0), known_static_string_values, shadowed)
                            && Self::is_syntax_static_e4_string_expr(arg(1), known_static_string_values, shadowed)
                    }
                    "substring" => {
                        args.len() == 3
                            && Self::is_syntax_static_e4_string_expr(arg(0), known_static_string_values, shadowed)
                            && Self::is_syntax_static_e4_index_expr(arg(1), known_static_string_values, shadowed)
                            && Self::is_syntax_static_e4_index_expr(arg(2), known_static_string_values, shadowed)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn is_syntax_static_e4_index_expr(
        expr: ExprNode<'_>,
        known_static_string_values: &HashSet<String>,
        shadowed: &HashSet<String>,
    ) -> bool {
        match expr.kind() {
            ExprKind::IntLit(_) => true,
            ExprKind::Apply(_) => {
                let Some((f, args)) = Self::unshadowed_apply(expr, &|n| shadowed.contains(n)) else {
                    return false;
                };
                match f {
                    "string-length" => {
                        args.len() == 1
                            && Self::is_syntax_static_e4_string_expr(
                                args.get(0).expect("length checked first"),
                                known_static_string_values,
                                shadowed,
                            )
                    }
                    "+" | "-" | "*" | "/" => {
                        args.len() >= 2
                            && args
                                .iter()
                                .all(|arg| Self::is_syntax_static_e4_index_expr(arg, known_static_string_values, shadowed))
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn param_static_iir_hint(
        &self,
        ann: Option<&GrammarASTNode>,
        inferred_param_types: &[String],
        idx: usize,
    ) -> Option<&'static str> {
        if let Some(ann) = ann {
            return type_annotation_static_iir_hint(ann);
        }
        if inferred_param_types.get(idx).map(|ty| ty.as_str()) == Some("str") {
            Some("str")
        } else {
            None
        }
    }

    // ------------------------------------------------------------------
    // Top-level fn (define (name args...) body+)
    // ------------------------------------------------------------------

    fn compile_top_level_lambda(&mut self, name: &str, lam: LambdaView<'_>) -> Result<(), TwigCompileError> {
        let mut ctx = FnCtx::new();
        let (lam_line, lam_column) = lam.pos();
        let lam_loc = SourceLoc::new(lam_line, lam_column);
        let inferred_param_types = self
            .fn_inferred_param_types
            .get(name)
            .cloned()
            .unwrap_or_default();
        // Each parameter's register is its own name, unless the name repeats
        // in the list (`(define (f x x) …)`): then the later one is renamed
        // and, as in Scheme, the body sees the last.
        let mut param_regs: Vec<String> = Vec::new();
        for (idx, p) in lam.params().enumerate() {
            let reg = ctx.bind_param(p.name);
            if let Some(ty) = self.param_static_iir_hint(p.annotation, &inferred_param_types, idx)
            {
                ctx.record_type(&reg, ty);
            }
            param_regs.push(reg);
        }

        let mut last: Option<String> = None;
        for e in lam.body().iter() {
            last = Some(self.compile_expr(e, &mut ctx)?);
        }
        let last = last.ok_or_else(|| TwigCompileError {
            message: format!("function {name:?} has empty body"),
            line: lam_line,
            column: lam_column,
        })?;
        let return_type = ctx.type_of(&last).to_string();
        ctx.emit(
            IIRInstr::new("ret", None, vec![Operand::Var(last)], return_type.as_str()),
            lam_loc,
        );

        let params: Vec<(String, String)> = lam
            .params()
            .zip(param_regs)
            .enumerate()
            .map(|(idx, (p, reg))| {
                let ty = self
                    .param_static_iir_hint(p.annotation, &inferred_param_types, idx)
                    .unwrap_or("any");
                (reg, ty.to_string())
            })
            .collect();

        // LANG23 PR 23-E — lower each `type_annotation` node → RefinedType for
        // every param and for the return type.  Unannotated params stay `None`.
        let param_refinements: Vec<Option<RefinedType>> = lam
            .params()
            .map(|p| p.annotation.map(type_annotation_to_refined_type))
            .collect();

        let return_refinement: Option<RefinedType> =
            lam.return_annotation().map(type_annotation_to_refined_type);

        self.functions.push(IIRFunction {
            name: name.to_string(),
            params: params.clone(),
            return_type: return_type.clone(),
            register_count: count_registers(&ctx.instrs),
            instructions: ctx.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: ctx.source_map,
            param_refinements,
            return_refinement,
        });
        self.fn_return_types.insert(name.to_string(), return_type);
        self.fn_param_types.insert(
            name.to_string(),
            params.into_iter().map(|(_, ty)| ty).collect(),
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Anonymous lambda — closure construction
    // ------------------------------------------------------------------

    fn compile_anonymous_lambda(
        &mut self,
        lam: LambdaView<'_>,
        outer: &mut FnCtx,
    ) -> Result<String, TwigCompileError> {
        let (lam_line, lam_column) = lam.pos();
        let lam_loc = SourceLoc::new(lam_line, lam_column);
        // 1. Compute free variables using the union of all globals + builtins
        //    — minus every name the enclosing scope binds locally (#18435).
        //    In `(define (mk helper) (lambda (y) (helper y)))` the lambda's
        //    `helper` is mk's PARAMETER even though a top-level `helper`
        //    exists; left in `globals` it would be skipped as "not free",
        //    never captured, and the body would call the global instead.
        let mut globals: HashSet<String> = HashSet::new();
        globals.extend(self.fn_globals.iter().cloned());
        globals.extend(self.value_globals.iter().cloned());
        for b in BUILTINS {
            globals.insert((*b).to_string());
        }
        globals.retain(|g| !outer.is_bound(g));
        let captures = free_vars(lam, &globals);

        // Every capture must be currently bound in `outer`; otherwise the
        // user wrote a name that doesn't resolve to anything reachable
        // from the lambda site.
        for c in &captures {
            if !outer.is_bound(c) {
                return Err(TwigCompileError {
                    message: format!(
                        "unbound name {c:?} captured by lambda — \
                         did you forget a (define) or a (let ...) binding?"
                    ),
                    line: lam_line,
                    column: lam_column,
                });
            }
        }

        // 2. Build the inner function: captures ++ params for its parameter list.
        let fn_name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;

        // Captures come first, then the lambda's own parameters; both are
        // bindings of the NEW function, so they claim registers in `inner`.
        // A parameter that repeats a capture's name cannot happen (`free_vars`
        // excludes parameters), but a repeated parameter can, and is renamed.
        let mut inner = FnCtx::new();
        let mut params: Vec<(String, String)> = Vec::new();
        for c in &captures {
            params.push((inner.bind_param(c), "any".to_string()));
        }
        for p in lam.params() {
            params.push((inner.bind_param(p.name), "any".to_string()));
        }

        let mut last: Option<String> = None;
        for e in lam.body().iter() {
            last = Some(self.compile_expr(e, &mut inner)?);
        }
        let last = last.ok_or_else(|| TwigCompileError {
            message: "lambda has empty body".into(),
            line: lam_line,
            column: lam_column,
        })?;
        inner.emit(
            IIRInstr::new("ret", None, vec![Operand::Var(last)], "any"),
            lam_loc,
        );

        self.functions.push(IIRFunction {
            name: fn_name.clone(),
            params,
            return_type: "any".into(),
            register_count: count_registers(&inner.instrs),
            instructions: inner.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: inner.source_map,
            param_refinements: Vec::new(),
            return_refinement: None,
        });

        // 3. Emit `alloc_closure` at the call site (LANG34).
        //
        // The function name is now an inline `Operand::Str` — no preceding
        // `const` instruction is needed.  This is cleaner than the old
        // `string_arg` / `call_builtin "make_closure"` convention and
        // makes the callee name statically visible in the IR for analysis
        // passes and the iir-to-* backends.
        //
        // Each capture operand is the register the name READS in the outer
        // function — `lookup`, exactly as `compile_var_ref` would resolve
        // it — so a shadowed name passes its innermost binding and an E4
        // string alias passes the register that holds the string.
        let dest = outer.fresh_var("clos");
        let mut srcs: Vec<Operand> = vec![Operand::Str(fn_name.clone())];
        for c in &captures {
            let reg = outer.lookup(c).expect("every capture was checked to be bound above");
            srcs.push(Operand::Var(reg.to_string()));
        }
        outer.emit(
            IIRInstr::new("alloc_closure", Some(dest.clone()), srcs, "closure"),
            lam_loc,
        );
        Ok(dest)
    }

    // ------------------------------------------------------------------
    // Expression compilation
    // ------------------------------------------------------------------

    fn compile_expr(&mut self, expr: ExprNode<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        // Depth-bound the recursion at the single chokepoint.  Every
        // compound form lowers by recursing through compile_expr on
        // its children, so wrapping it here covers if / let / begin /
        // lambda / apply / quoted-symbol-construction in one place.
        ctx.depth += 1;
        if ctx.depth > MAX_COMPILE_DEPTH {
            let (line, column) = expr.pos();
            return Err(TwigCompileError {
                message: format!(
                    "AST nesting exceeds MAX_COMPILE_DEPTH ({MAX_COMPILE_DEPTH}) — \
                     refusing to recurse further to avoid stack overflow"
                ),
                line,
                column,
            });
        }
        let result = self.compile_expr_inner(expr, ctx);
        ctx.depth = ctx.depth.saturating_sub(1);
        result
    }

    fn compile_expr_inner(&mut self, expr: ExprNode<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let (line, column) = expr.pos();
        let loc = SourceLoc::new(line, column);
        match expr.kind() {
            ExprKind::IntLit(value) => {
                let v = ctx.fresh_var("n");
                // Twig path-A increment 1: integer literals lower to typed
                // `const_i64`-compatible IR by stamping `type_hint = "i64"`.
                // The IIR-to-* validators (wasm/jvm/clr/beam) reject `"any"`,
                // so this is the difference between "Twig compiles to a
                // .wasm" and "Twig fails validation at every backend".
                //
                // `var_types[v]` is recorded so downstream `ret` emission
                // can propagate the type instead of falling back to `"any"`.
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Int(value)],
                    "i64",
                ), loc);
                ctx.record_type(&v, "i64");
                Ok(v)
            }

            ExprKind::BoolLit(value) => {
                let v = ctx.fresh_var("b");
                // Twig path-A increment 1: boolean literals lower with
                // `type_hint = "bool"` (the IIR-to-* validators all accept
                // it; the JVM/CLR backends map it to `i32` 0/1).
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Bool(value)],
                    "bool",
                ), loc);
                ctx.record_type(&v, "bool");
                Ok(v)
            }

            ExprKind::NilLit => {
                // Path A increment 6a: emit `const 0 [ref<LispyPair>]`
                // instead of `call_builtin "make_nil" [any]`.  Every
                // IIR-to-* backend accepts the typed const, and twig-vm
                // dispatches it as a nil `LispyPair` reference.
                let v = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Int(0)],
                    "ref<LispyPair>",
                ), loc);
                ctx.record_type(&v, "ref<LispyPair>");
                Ok(v)
            }

            ExprKind::SymLit(name) => {
                // E6d-4 (symbols): a quote literal (`'a` / `(quote a)`) whose name
                // is known at compile time lowers to `const Var(name) : symbol` —
                // the SAME interned-const form McCarthy Lisp's `emit_symbol` emits,
                // rather than the runtime `make_symbol` string path (which needs
                // data-section string emission the code-gen backends lack). This
                // rides the existing `intern_symbols` / `intern_symbols_structural`
                // passes: each distinct name gets one module-wide id, so `equal?`
                // on symbols is bit-equality (`(equal? 'a 'a)` #t, `(equal? 'a 'b)`
                // #f) on all five code-gen backends — with no new value type. On
                // twig-vm the `const Var(name)` dispatch already interns the text to
                // a symbol, so the VM is unaffected. (Runtime symbol *creation* —
                // `string->symbol` over a runtime string — keeps `make_symbol`.)
                let v = ctx.fresh_var("sym");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Var(name.to_string())],
                    "symbol",
                ), loc);
                ctx.record_type(&v, "symbol");
                Ok(v)
            }

            // LANG51: double-quoted string literal — `"hello"`.
            //
            // Lower to a single `const(Operand::Str(value))` instruction with
            // type_hint `"str"`.  The VM's `exec_const` (twig-vm/src/dispatch.rs)
            // already handles `Operand::Str` by calling `alloc_string` and
            // wrapping the result as a heap `LispyValue` — no VM changes needed.
            //
            // We use `Operand::Str` (the LANG32-canonical compile-time string form)
            // rather than the older `string_arg` helper that emits `Operand::Var`
            // because that helper was designed for internal builtin-call scaffolding,
            // not for user-visible string data.  `Operand::Str` is cleaner and
            // propagates the `"str"` type_hint automatically through LANG50 inference.
            ExprKind::StrLit(value) => {
                let v = ctx.fresh_var("s");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Str(value.to_string())],
                    "str",
                ), loc);
                ctx.record_type(&v, "str");
                Ok(v)
            }

            ExprKind::VarRef(name) => self.compile_var_ref(name, (line, column), ctx),

            ExprKind::If(i) => self.compile_if(i, ctx),

            ExprKind::Begin(exprs) => {
                // `check_cst` guarantees at least one body expr.
                let mut last: Option<String> = None;
                for e in exprs.iter() {
                    last = Some(self.compile_expr(e, ctx)?);
                }
                Ok(last.expect("check_cst rejects empty (begin)"))
            }

            ExprKind::Let(l) => self.compile_let(l, ctx),

            // LANG52: sequential let* — compile via compile_let_star.
            ExprKind::LetStar(l) => self.compile_let_star(l, ctx),

            ExprKind::Lambda(l) => self.compile_anonymous_lambda(l, ctx),

            ExprKind::Apply(a) => self.compile_apply(a, ctx),

            // TW05-A / LANG48: match expression — lowered to if/let chain.
            ExprKind::Match(m) => self.compile_match(m, ctx),
        }
    }

    /// Lower a bare name reference found at source position `at`.
    fn compile_var_ref(
        &mut self,
        name: &str,
        at: (usize, usize),
        ctx: &mut FnCtx,
    ) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(at.0, at.1);
        // Locals (params, lets, lambda / match bindings) — return the
        // innermost binding's register directly; the next instruction that
        // reads it resolves through the register file (#18439).
        if let Some(reg) = ctx.lookup(name) {
            return Ok(reg.to_string());
        }

        // Top-level function — wrap in a 0-capture closure handle so
        // the value can be passed around or applied later.
        //
        // LANG34: emit `alloc_closure(Str(fn_name))` instead of the old
        // `string_arg + call_builtin "make_closure"` form.  A 0-capture
        // closure is perfectly valid; exec_alloc_closure handles it.
        if self.fn_globals.contains(name) {
            let dest = ctx.fresh_var("fnref");
            ctx.emit(IIRInstr::new(
                "alloc_closure",
                Some(dest.clone()),
                vec![Operand::Str(name.to_string())],
                "closure",
            ), loc);
            return Ok(dest);
        }

        // TW2: a non-escaping, statically-typed value-global lives in a `main`
        // register — return it directly (the value, with its inferred type, is
        // already live in `main_ctx`).  This is only ever reached from `main`,
        // because escaping names never land in `value_global_locals`.
        if let Some(reg) = self.value_global_locals.get(name) {
            return Ok(reg.clone());
        }

        // Top-level value — look up via the host global table.
        if self.value_globals.contains(name) {
            // A read that reaches here for a value-global that is *not* in
            // `value_global_locals` is either captured (kept on the table by
            // design) or a forward reference whose `define` has not run yet.
            // Flag it so that `define` emits the matching `global_set`.
            self.forced_global_set.insert(name.to_string());
            let name_reg = self.string_arg(ctx, name, loc);
            let dest = ctx.fresh_var("g");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![Operand::Var("global_get".into()), Operand::Var(name_reg)],
                "any",
            ), loc);
            return Ok(dest);
        }

        // Builtin — wrap in a 0-capture builtin-closure handle so users
        // can pass `+` etc. into higher-order positions.
        if is_builtin(name) {
            let name_reg = self.string_arg(ctx, name, loc);
            let dest = ctx.fresh_var("bref");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![
                    Operand::Var("make_builtin_closure".into()),
                    Operand::Var(name_reg),
                ],
                "any",
            ), loc);
            return Ok(dest);
        }

        Err(TwigCompileError {
            message: format!(
                "unbound name {:?} (no local, define, or builtin matches)",
                name
            ),
            line: at.0,
            column: at.1,
        })
    }

    fn compile_if(&mut self, expr: IfView<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos.0, expr.pos.1);
        let cond = self.compile_expr(expr.cond, ctx)?;
        let else_label = ctx.fresh_label("else");
        let end_label = ctx.fresh_label("endif");
        let result = ctx.fresh_var("ifv");

        ctx.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond), Operand::Var(else_label.clone())],
            "void",
        ), loc);

        // `result` is a join temporary with one writer per arm; its type is
        // the arms' consensus (see "Join temporaries").  If both arms produce
        // the same statically-known type, `result` ends up typed and
        // propagates through downstream `ret` instructions; programs like
        // `(if (< x 0) -1 1)` flow through every IIR-to-* backend.
        let mut join = JoinType::default();

        // Then branch — compile and copy into `result` via typed `mov`.
        let then_v = self.compile_expr(expr.then_branch, ctx)?;
        let then_loc = SourceLoc::new(expr.then_branch.pos().0, expr.then_branch.pos().1);
        ctx.emit_join_move(&result, &then_v, then_loc, &mut join);
        ctx.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ), loc);

        // Else branch — same shape.
        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(else_label)],
            "void",
        ), loc);
        let else_v = self.compile_expr(expr.else_branch, ctx)?;
        let else_loc = SourceLoc::new(expr.else_branch.pos().0, expr.else_branch.pos().1);
        ctx.emit_join_move(&result, &else_v, else_loc, &mut join);
        // Both writers are in: `result` is typed only if they agree.
        ctx.settle_join(&result, &join);

        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(end_label)],
            "void",
        ), loc);
        Ok(result)
    }

    fn compile_let(&mut self, expr: LetView<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos().0, expr.pos().1);
        enum BindingValue {
            Reg(String),
            StrLiteral(String, SourceLoc),
        }

        // Compile RHSs in the OUTER scope (Scheme `let`, not `let*`).
        let mut binding_values: Vec<(String, BindingValue)> = Vec::new();
        for (name, rhs) in expr.bindings() {
            let v = if let ExprKind::StrLit(value) = rhs.kind() {
                BindingValue::StrLiteral(value.to_string(), SourceLoc::new(rhs.pos().0, rhs.pos().1))
            } else {
                BindingValue::Reg(self.compile_expr(rhs, ctx)?)
            };
            binding_values.push((name.to_string(), v));
        }

        // Bind each name to a register of its own (#18439) — the name
        // itself the first time this function binds it, `__r{n}_{name}`
        // after that — and push it on the name's scope stack.
        // Path-A increment 4: typed `mov` propagates the RHS's
        // inferred type to the binding register, so subsequent expressions
        // that reference `name` see the concrete type. E4 string RHSs alias
        // their producer register instead, because the current code-gen
        // backends accept string producers but not generic string copies.
        //
        // A new register has no recorded type, so the binding carries only
        // what its own value supports (#18435) — nothing from an outer
        // binding of the same name.
        for (name, src) in &binding_values {
            match src {
                // The alias register already holds (and is typed as) the
                // string; the name simply reads it.
                BindingValue::Reg(src) if ctx.type_of(src) == "str" => {
                    ctx.push_binding(name, src.clone());
                }
                BindingValue::Reg(src) => {
                    ctx.bind_move(name, src, loc);
                }
                BindingValue::StrLiteral(value, value_loc) => {
                    let reg = ctx.binding_register(name);
                    ctx.emit_str_const_to(&reg, value, *value_loc);
                    ctx.push_binding(name, reg);
                }
            }
        }

        // Compile body — at least one expression (`check_cst`-enforced).
        let mut last: Option<String> = None;
        for e in expr.body().iter() {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("check_cst rejects empty let body");

        // Close the scope: pop each name's stack (reverse order, so a name
        // bound twice in one `let` unwinds correctly).  The registers and
        // their types stay as they are — `last` may be one of them, and no
        // other binding ever writes them.
        for (name, _) in binding_values.iter().rev() {
            ctx.pop_binding(name);
        }
        Ok(last)
    }

    /// LANG52: compile `(let* ((x e1) (y e2) ...) body+)`.
    ///
    /// Unlike `compile_let`, each binding's RHS is compiled AFTER the previous
    /// binding is in scope — so each name is visible to all subsequent
    /// RHSs.  This is Scheme `let*` semantics.
    ///
    /// ```text
    /// ; (let* ((a 1) (b (+ a 1)) (a (* b 10))) a)
    /// const  _n1  = 1               ; compile a=1 in outer scope
    /// mov    a    ← _n1             ; bind 'a' (now in scope)
    /// add    _r3  = a, _n2          ; compile b=(+ a 1) — 'a' is visible
    /// mov    b    ← _r3             ; bind 'b'
    /// mul    _r5  = b, _n4          ; the second 'a' is a NEW binding…
    /// mov    __r1_a ← _r5           ; …in a register of its own (#18439)
    /// ret    __r1_a
    /// ```
    fn compile_let_star(&mut self, expr: LetView<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos().0, expr.pos().1);
        let mut bound: Vec<&str> = Vec::new();

        for (name, rhs) in expr.bindings() {
            // Compile the RHS in the current scope (which already includes
            // all prior let* bindings).
            if let ExprKind::StrLit(value) = rhs.kind() {
                let reg = ctx.binding_register(name);
                ctx.emit_str_const_to(&reg, value, SourceLoc::new(rhs.pos().0, rhs.pos().1));
                ctx.push_binding(name, reg);
                bound.push(name);
                continue;
            }

            let v = self.compile_expr(rhs, ctx)?;

            // Bind the name BEFORE compiling the next binding.
            // Path-A increment 4: typed `mov` propagates scalar RHS types.
            // E4 string RHSs mirror compile_let by aliasing the producer
            // register instead of emitting backend-unsupported `mov [str]`.
            if ctx.type_of(&v) == "str" {
                ctx.push_binding(name, v);
            } else {
                ctx.bind_move(name, &v, loc);
            }
            bound.push(name);
        }

        // Compile body — `check_cst`-enforced at least one expression.
        let mut last: Option<String> = None;
        for e in expr.body().iter() {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("check_cst rejects empty let* body");

        // Close the scope in reverse order, so each name's outer binding (if
        // any) is visible again.
        for name in bound.into_iter().rev() {
            ctx.pop_binding(name);
        }
        Ok(last)
    }

    // ── LANG52: and / or compile-time special forms ───────────────────────────

    /// Compile `(and args…)` with short-circuit semantics.
    ///
    /// Expansion rules (PEG-style, applied left-to-right):
    ///   `(and)`         → emit `const #t`, return the register
    ///   `(and e)`       → compile e, return its register
    ///   `(and e1 e2 …)` → compile e1; if truthy, evaluate `(and e2 …)`; else `#f`
    ///
    /// The result register holds the value of the last evaluated sub-expression,
    /// or `#f` if any sub-expression was falsy.  This matches Scheme semantics.
    /// Its type is therefore the consensus of the last value's type and
    /// `bool` (see "Join temporaries"): `(and (< a b) (< b c))` is `bool`,
    /// `(and c 5)` is untyped.
    ///
    /// IIR pattern for one step (mirrors `compile_if`):
    ///   `jmp_if_false cond, else_label`
    ///   then-path: compile rest, _move into dest, jmp end
    ///   else-path: label else_label; const #f, _move into dest
    ///   label end_label
    ///
    /// ### Why a loop, not recursion
    ///
    /// The rules above nest: `(and e1 e2 e3)` is `e1 ? (e2 ? e3 : #f) : #f`.
    /// Lowering that by re-entering `compile_and` once per argument used one
    /// native stack frame per argument, and argument *count* is bounded by
    /// neither the parser's nesting limit nor `MAX_COMPILE_DEPTH` — a 40 KB
    /// `(and 1 1 … 1)` overflowed the stack.  So the nesting is unrolled by
    /// hand into two passes over one collected argument slice:
    ///
    /// ```text
    ///   pass 1 (k = 1 … n-1):  compile e_k, open step k  (jmp_if_false)
    ///   middle:                compile e_n  → v
    ///   pass 2 (k = n-1 … 1):  close step k (move dest_k ← inner, else/end
    ///                          labels), where inner = v for the innermost
    ///                          step and dest_{k+1} otherwise
    /// ```
    ///
    /// Emission and fresh-name order are exactly what the recursion produced,
    /// so the IIR is byte-identical; stack use is constant in the argument
    /// count and the arguments are walked once (linear, not quadratic).
    fn compile_and(&mut self, args: Exprs<'_>, ctx: &mut FnCtx, loc: SourceLoc) -> Result<String, TwigCompileError> {
        let args: Vec<ExprNode<'_>> = args.iter().collect();
        let Some((last, init)) = args.split_last() else {
            // (and) → #t
            let v = ctx.fresh_var("and");
            ctx.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Bool(true)], "any"), loc);
            return Ok(v);
        };

        // Pass 1: open one step per non-final argument.  Each step's `dest`
        // is a join temporary with two writers — the inner value and `#f` —
        // both emitted in pass 2.
        let mut open: Vec<(String, String, String)> = Vec::with_capacity(init.len());
        for &e in init {
            let cond = self.compile_expr(e, ctx)?;
            let dest = ctx.fresh_var("and");
            let else_label = ctx.fresh_label("and_else");
            let end_label  = ctx.fresh_label("and_end");
            // jmp_if_false cond → else_label
            ctx.emit(IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var(cond), Operand::Var(else_label.clone())],
                "void"), loc);
            open.push((dest, else_label, end_label));
        }

        // Middle: the final argument is the value of the innermost step
        // (and the whole result when there is only one argument).
        let mut inner = self.compile_expr(*last, ctx)?;

        // Pass 2: close the steps innermost-first.
        while let Some((dest, else_label, end_label)) = open.pop() {
            let mut join = JoinType::default();
            // Then path: copy the inner value to dest via typed `mov`,
            // jump to end.  Path-A increment 4.
            ctx.emit_join_move(&dest, &inner, loc, &mut join);
            ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

            // Else path: dest ← #f (typed `mov` from a `bool`-typed
            // temporary).  This writer counts toward the consensus too, so
            // `dest` is `bool` only when the inner value is `bool` as well:
            // `(and c 5)` can be `#f` or `5` and stays untyped.
            ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(else_label)], "void"), loc);
            let false_tmp = ctx.fresh_var("f");
            // The false literal site uses the same typed `const`
            // shape that a `#f` literal emits — see compile_expr
            // path-A increment 1.
            ctx.emit(IIRInstr::new("const", Some(false_tmp.clone()),
                vec![Operand::Bool(false)], "bool"), loc);
            ctx.record_type(&false_tmp, "bool");
            ctx.emit_join_move(&dest, &false_tmp, loc, &mut join);
            ctx.settle_join(&dest, &join);

            ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(end_label)], "void"), loc);
            inner = dest;
        }
        Ok(inner)
    }

    /// Compile `(or args…)` with short-circuit semantics.
    ///
    /// Expansion rules:
    ///   `(or)`          → emit `const #f`, return the register
    ///   `(or e)`        → compile e, return its register
    ///   `(or e1 e2 …)`  → evaluate e1; if truthy return it, else `(or e2 …)`
    ///
    /// The result register holds the first truthy value, or the value of the
    /// last sub-expression if all were falsy.  This matches Scheme semantics.
    /// Its type is the consensus of every argument's type (see "Join
    /// temporaries"): `(or 1 2)` is `i64`, `(or (< 1 2) 5)` is untyped.
    ///
    /// Lowered with the same two-pass loop as [`Self::compile_and`] (see
    /// "Why a loop, not recursion" there): each step's truthy path is
    /// complete before the next argument is compiled, so pass 2 only has to
    /// move the inner value into each `dest` and place the end labels.
    fn compile_or(&mut self, args: Exprs<'_>, ctx: &mut FnCtx, loc: SourceLoc) -> Result<String, TwigCompileError> {
        let args: Vec<ExprNode<'_>> = args.iter().collect();
        let Some((last, init)) = args.split_last() else {
            // (or) → #f
            let v = ctx.fresh_var("or");
            ctx.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Bool(false)], "any"), loc);
            return Ok(v);
        };

        // Pass 1: one step per non-final argument.  Each step's `dest` is a
        // join temporary with two writers: the truthy test value (pass 1)
        // and the rest of the `or` (pass 2).  `open` carries the step's
        // consensus so far from one pass to the other.
        let mut open: Vec<(String, String, JoinType)> = Vec::with_capacity(init.len());
        for &e in init {
            let cond = self.compile_expr(e, ctx)?;
            let dest = ctx.fresh_var("or");
            let falsy_label = ctx.fresh_label("or_falsy");
            let end_label   = ctx.fresh_label("or_end");

            // jmp_if_false cond → falsy_label (i.e. if cond is FALSE, skip)
            ctx.emit(IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var(cond.clone()), Operand::Var(falsy_label.clone())],
                "void"), loc);

            // Truthy path: dest ← cond via typed `mov`, jump to end.
            // Path-A increment 4.
            let mut join = JoinType::default();
            ctx.emit_join_move(&dest, &cond, loc, &mut join);
            ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

            // Falsy path: the rest of the `or` follows this label.
            ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(falsy_label)], "void"), loc);
            open.push((dest, end_label, join));
        }

        // Middle: the final argument.
        let mut inner = self.compile_expr(*last, ctx)?;

        // Pass 2: close the steps innermost-first — copy the falsy-path
        // value via typed `mov`, settle `dest`'s type (the inner step's is
        // settled already, so `inner` reads its consensus), then the
        // step's end label.
        while let Some((dest, end_label, mut join)) = open.pop() {
            ctx.emit_join_move(&dest, &inner, loc, &mut join);
            ctx.settle_join(&dest, &join);
            ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(end_label)], "void"), loc);
            inner = dest;
        }
        Ok(inner)
    }

    fn is_known_main_value_type(&self, name: &str, ctx: &FnCtx, ty: &str) -> bool {
        // A `let` binding of the same name hides the value-global (#18435):
        // its type is the local's, not the global's register's.
        !ctx.is_bound(name)
            && matches!(self.value_global_locals.get(name), Some(reg) if ctx.type_of(reg) == ty)
    }

    /// Does the lexical name `name` statically hold a `ty` value?
    ///
    /// The answer is the type of the register a read of `name` returns —
    /// its own binding register or, for an E4 alias, the producer's.  That
    /// register has exactly one writer (#18439), so its recorded type holds
    /// for as long as the name can be read:
    /// `(let ((y x)) (let ((x 5)) (string-length y)))` with `x` a `str`
    /// parameter reads register `x`, which the inner `x` (register
    /// `__r1_x`) never touches — a sound typed `str_len`.
    fn is_known_value_type(&self, name: &str, ctx: &FnCtx, ty: &str) -> bool {
        self.is_known_main_value_type(name, ctx, ty)
            || ctx.lookup(name).is_some_and(|reg| ctx.type_of(reg) == ty)
    }

    fn can_compile_e4_string_expr(&self, expr: ExprNode<'_>, ctx: &FnCtx) -> bool {
        match expr.kind() {
            ExprKind::StrLit(_) => true,
            ExprKind::VarRef(name) => self.is_known_value_type(name, ctx, "str"),
            ExprKind::Apply(_) => {
                if let Some((f, args)) = Self::unshadowed_apply(expr, &|n| ctx.is_bound(n)) {
                    let arg = |i: usize| args.get(i).expect("length checked first");
                    match f {
                        "string-append" => {
                            args.len() == 2
                                && self.can_compile_e4_string_expr(arg(0), ctx)
                                && self.can_compile_e4_string_expr(arg(1), ctx)
                        }
                        "substring" => {
                            args.len() == 3
                                && self.can_compile_e4_string_expr(arg(0), ctx)
                                && self.can_compile_e4_index_expr(arg(1), ctx)
                                && self.can_compile_e4_index_expr(arg(2), ctx)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn can_compile_e4_index_expr(&self, expr: ExprNode<'_>, ctx: &FnCtx) -> bool {
        match expr.kind() {
            ExprKind::IntLit(_) => true,
            ExprKind::VarRef(name) => {
                self.is_known_value_type(name, ctx, "i64")
                    || self.is_known_value_type(name, ctx, "i32")
            }
            ExprKind::Apply(_) => {
                let Some((f, args)) = Self::unshadowed_apply(expr, &|n| ctx.is_bound(n)) else {
                    return false;
                };
                match f {
                    "string-length" => {
                        args.len() == 1
                            && self.can_compile_e4_string_expr(
                                args.get(0).expect("length checked first"),
                                ctx,
                            )
                    }
                    "+" | "-" | "*" | "/" => {
                        args.len() >= 2
                            && args
                                .iter()
                                .all(|arg| self.can_compile_e4_index_expr(arg, ctx))
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn compile_e4_string_expr(&mut self, expr: ExprNode<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos().0, expr.pos().1);
        if let ExprKind::StrLit(value) = expr.kind() {
            return Ok(ctx.emit_str_const(value, loc));
        }
        self.compile_expr(expr, ctx)
    }

    fn try_compile_e4_string_builtin(
        &mut self,
        name: &str,
        args: Exprs<'_>,
        ctx: &mut FnCtx,
        loc: SourceLoc,
    ) -> Result<Option<String>, TwigCompileError> {
        // `args[i]` for the guards below; each guard checks the length first.
        let arg = |i: usize| args.get(i).expect("length checked first");
        match name {
            "string-length"
                if args.len() == 1 && self.can_compile_e4_string_expr(arg(0), ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(arg(0), ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_len",
                        Some(dest.clone()),
                        vec![Operand::Var(string_reg)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "string-append"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(arg(0), ctx)
                    && self.can_compile_e4_string_expr(arg(1), ctx) =>
            {
                let left = self.compile_e4_string_expr(arg(0), ctx)?;
                let right = self.compile_e4_string_expr(arg(1), ctx)?;
                let dest = ctx.fresh_var("s");
                ctx.emit(
                    IIRInstr::new(
                        "str_concat",
                        Some(dest.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "str",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "str");
                Ok(Some(dest))
            }
            "string=?"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(arg(0), ctx)
                    && self.can_compile_e4_string_expr(arg(1), ctx) =>
            {
                let left = self.compile_e4_string_expr(arg(0), ctx)?;
                let right = self.compile_e4_string_expr(arg(1), ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_eq",
                        Some(dest.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "string<?" | "string>?"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(arg(0), ctx)
                    && self.can_compile_e4_string_expr(arg(1), ctx) =>
            {
                let left = self.compile_e4_string_expr(arg(0), ctx)?;
                let right = self.compile_e4_string_expr(arg(1), ctx)?;
                let cmp = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_cmp",
                        Some(cmp.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&cmp, "i64");

                let zero = ctx.fresh_var("n");
                ctx.emit(
                    IIRInstr::new("const", Some(zero.clone()), vec![Operand::Int(0)], "i64"),
                    loc,
                );
                ctx.record_type(&zero, "i64");

                let dest = ctx.fresh_var("r");
                let op = if name == "string<?" { "cmp_lt" } else { "cmp_gt" };
                ctx.emit(
                    IIRInstr::new(
                        op,
                        Some(dest.clone()),
                        vec![Operand::Var(cmp), Operand::Var(zero)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "bool");
                Ok(Some(dest))
            }
            "string-ref"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(arg(0), ctx)
                    && self.can_compile_e4_index_expr(arg(1), ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(arg(0), ctx)?;
                let idx_reg = self.compile_expr(arg(1), ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_index",
                        Some(dest.clone()),
                        vec![Operand::Var(string_reg), Operand::Var(idx_reg)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "substring"
                if args.len() == 3
                    && self.can_compile_e4_string_expr(arg(0), ctx)
                    && self.can_compile_e4_index_expr(arg(1), ctx)
                    && self.can_compile_e4_index_expr(arg(2), ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(arg(0), ctx)?;
                let start_reg = self.compile_expr(arg(1), ctx)?;
                let end_reg = self.compile_expr(arg(2), ctx)?;
                let dest = ctx.fresh_var("s");
                ctx.emit(
                    IIRInstr::new(
                        "str_slice",
                        Some(dest.clone()),
                        vec![
                            Operand::Var(string_reg),
                            Operand::Var(start_reg),
                            Operand::Var(end_reg),
                        ],
                        "str",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "str");
                Ok(Some(dest))
            }
            _ => Ok(None),
        }
    }

    fn compile_apply(&mut self, expr: ApplyView<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos().0, expr.pos().1);
        let args = expr.args();
        // `args[i]` for the literal fast paths below; each checks the length first.
        let arg = |i: usize| args.get(i).expect("length checked first");
        // ── LANG52: `and` / `or` short-circuit special forms ─────────────────
        //
        // `and` and `or` require short-circuit evaluation — the second argument
        // must NOT be evaluated when the first is sufficient.  We intercept them
        // at the apply site and lower them inline to `if` chains.  They do NOT
        // appear in BUILTINS and never reach the runtime.
        //
        // | Expression      | Expansion                         |
        // |-----------------|-----------------------------------|
        // | (and)           | #t                                |
        // | (and e)         | e                                 |
        // | (and e1 e2 …)   | (if e1 (and e2 …) #f)            |
        // | (or)            | #f                                |
        // | (or e)          | e                                 |
        // | (or e1 e2 …)    | emit cond-reg; if cond-reg return |
        //
        // The nesting is unrolled into a loop over the arguments (see
        // `compile_and`), so a wide `(and …)`/`(or …)` uses constant native
        // stack: argument count is bounded by neither the parser's nesting
        // limit nor MAX_COMPILE_DEPTH.
        if let Some(fn_name) = expr.fn_name() {
            if fn_name == "and" {
                return self.compile_and(args, ctx, loc);
            }
            if fn_name == "or" {
                return self.compile_or(args, ctx, loc);
            }
        }

        // Direct call: fn is a VarRef whose name is a top-level
        // function or a builtin.  We materialise this decision at
        // compile time so the hot path stays a single `call`.
        //
        // Lexical scope first (#18435): a name bound by a parameter,
        // `let`/`let*`, lambda or `match` arm is NOT the global or builtin
        // that happens to share its spelling.  Filtering it out here sends
        // it down the indirect path below, where `compile_var_ref` resolves
        // it to the local's register and we emit `call_closure`.  The
        // global's return / parameter types are skipped with it — they
        // describe a different function.
        //
        //   (define (f helper) (helper 1))
        //     before: call helper, 1          ; the GLOBAL helper
        //     after:  call_closure helper, 1  ; f's PARAMETER
        let direct_fn_name = expr.fn_name().filter(|name| !ctx.is_bound(name));
        if let Some(fn_name) = direct_fn_name {
            if self.fn_globals.contains(fn_name) {
                let return_type = self
                    .fn_return_types
                    .get(fn_name)
                    .cloned()
                    .unwrap_or_else(|| "any".to_string());
                let param_types = self
                    .fn_param_types
                    .get(fn_name)
                    .cloned()
                    .unwrap_or_default();
                let mut srcs: Vec<Operand> = vec![Operand::Var(fn_name.to_string())];
                for (idx, a) in args.iter().enumerate() {
                    let r = if param_types.get(idx).map(|ty| ty.as_str()) == Some("str")
                        && self.can_compile_e4_string_expr(a, ctx)
                    {
                        self.compile_e4_string_expr(a, ctx)?
                    } else {
                        self.compile_expr(a, ctx)?
                    };
                    srcs.push(Operand::Var(r));
                }
                let dest = ctx.fresh_var("r");
                ctx.emit(IIRInstr::new(
                    "call",
                    Some(dest.clone()),
                    srcs,
                    return_type.as_str(),
                ), loc);
                if return_type != "any" {
                    ctx.record_type(&dest, return_type.as_str());
                }
                return Ok(dest);
            }

            if is_builtin(fn_name) {
                if let Some(result) =
                    self.try_compile_e4_string_builtin(fn_name, args, ctx, loc)?
                {
                    return Ok(result);
                }

                if fn_name == "string-length" && args.len() == 1 {
                    if let ExprKind::StrLit(value) = arg(0).kind() {
                        let string_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(string_reg.clone()),
                            vec![Operand::Str(value.to_string())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(string_reg.clone(), "str".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_len",
                            Some(dest.clone()),
                            vec![Operand::Var(string_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                    if let Some((append_fn, append_args)) =
                        Self::unshadowed_apply(arg(0), &|n| ctx.is_bound(n))
                    {
                        if append_fn == "string-append" && append_args.len() == 2 {
                            let append_arg = |i: usize| {
                                append_args.get(i).expect("length checked first").kind()
                            };
                            if let (ExprKind::StrLit(left), ExprKind::StrLit(right)) =
                                (append_arg(0), append_arg(1))
                            {
                                let left_reg = ctx.fresh_var("s");
                                ctx.emit(IIRInstr::new(
                                    "str_const",
                                    Some(left_reg.clone()),
                                    vec![Operand::Str(left.to_string())],
                                    "str",
                                ), loc);
                                ctx.var_types.insert(left_reg.clone(), "str".to_string());

                                let right_reg = ctx.fresh_var("s");
                                ctx.emit(IIRInstr::new(
                                    "str_const",
                                    Some(right_reg.clone()),
                                    vec![Operand::Str(right.to_string())],
                                    "str",
                                ), loc);
                                ctx.var_types.insert(right_reg.clone(), "str".to_string());

                                let concat_reg = ctx.fresh_var("s");
                                ctx.emit(IIRInstr::new(
                                    "str_concat",
                                    Some(concat_reg.clone()),
                                    vec![
                                        Operand::Var(left_reg),
                                        Operand::Var(right_reg),
                                    ],
                                    "str",
                                ), loc);
                                ctx.var_types.insert(concat_reg.clone(), "str".to_string());

                                let dest = ctx.fresh_var("r");
                                ctx.emit(IIRInstr::new(
                                    "str_len",
                                    Some(dest.clone()),
                                    vec![Operand::Var(concat_reg)],
                                    "i64",
                                ), loc);
                                ctx.var_types.insert(dest.clone(), "i64".to_string());
                                return Ok(dest);
                            }
                        }
                    }
                }
                if fn_name == "string=?" && args.len() == 2 {
                    if let (ExprKind::StrLit(left), ExprKind::StrLit(right)) =
                        (arg(0).kind(), arg(1).kind())
                    {
                        let left_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(left_reg.clone()),
                            vec![Operand::Str(left.to_string())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(left_reg.clone(), "str".to_string());

                        let right_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(right_reg.clone()),
                            vec![Operand::Str(right.to_string())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(right_reg.clone(), "str".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_eq",
                            Some(dest.clone()),
                            vec![Operand::Var(left_reg), Operand::Var(right_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                }
                if fn_name == "string-ref" && args.len() == 2 {
                    if let (ExprKind::StrLit(value), ExprKind::IntLit(idx)) =
                        (arg(0).kind(), arg(1).kind())
                    {
                        let string_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(string_reg.clone()),
                            vec![Operand::Str(value.to_string())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(string_reg.clone(), "str".to_string());

                        let idx_reg = ctx.fresh_var("i");
                        ctx.emit(IIRInstr::new(
                            "const",
                            Some(idx_reg.clone()),
                            vec![Operand::Int(idx)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(idx_reg.clone(), "i64".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_index",
                            Some(dest.clone()),
                            vec![Operand::Var(string_reg), Operand::Var(idx_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                }

                // Resolve every argument before deciding the lowering path.
                let arg_regs: Vec<String> = args
                    .iter()
                    .map(|a| self.compile_expr(a, ctx))
                    .collect::<Result<_, _>>()?;

                // ── Twig path-A increment 2: typed binary arithmetic / cmp ───
                //
                // When every resolved argument has a statically-known type
                // (recorded in `FnCtx::var_types` by literal-emission sites
                // and by previous typed builds-up such as earlier `add`
                // operations), and the builtin name is in the arithmetic /
                // comparison set, we can lower the call to a *typed CIR
                // mnemonic* (`add`, `cmp_lt`, …) that the IIR-to-* backends
                // accept directly — instead of the legacy
                // `call_builtin "<op>"` path which carries `type_hint = "any"`
                // and gets rejected at every backend validator.
                //
                // The pattern mirrors PR #3903's fix for Nib
                // (`compile_binary_chain` → typed CIR mnemonics).  See
                // `typed_arith_op_for` below for the symbol → mnemonic table.
                //
                // Scoped to binary forms (`(+ a b)`).  Variadic forms
                // (`(+ a b c)`), and any call where one arg has an unknown /
                // dynamic type, continue to use the `call_builtin "any"`
                // fallback — they remain rejected by the backend validators
                // until a later increment lowers variadic folds and / or
                // adds runtime type guards.
                if arg_regs.len() == 2 {
                    if let Some(typed_mnemonic) = typed_arith_op_for(fn_name) {
                        let lhs_ty = ctx.type_of(&arg_regs[0]).to_string();
                        let rhs_ty = ctx.type_of(&arg_regs[1]).to_string();
                        if lhs_ty == "i64" && rhs_ty == "i64" {
                            // Emit `<typed_mnemonic> dest = lhs, rhs [i64]`.
                            // Result type depends on the family:
                            //   add/sub/mul/div  → i64
                            //   cmp_* / =        → bool
                            let result_ty: &str =
                                if typed_mnemonic.starts_with("cmp_") {
                                    "bool"
                                } else {
                                    "i64"
                                };
                            let dest = ctx.fresh_var("r");
                            ctx.emit(IIRInstr::new(
                                typed_mnemonic,
                                Some(dest.clone()),
                                vec![
                                    Operand::Var(arg_regs[0].clone()),
                                    Operand::Var(arg_regs[1].clone()),
                                ],
                                result_ty,
                            ), loc);
                            ctx.record_type(&dest, result_ty);
                            return Ok(dest);
                        }
                    }
                }

                // ── Twig path-A increment 3 (TW1): typed *variadic* arithmetic ─
                //
                // Scheme arithmetic is variadic: `(+ a b c d)` means
                // `a + b + c + d`.  When every argument is statically `i64`
                // and the builtin is one of the four *arithmetic* operators,
                // we fold the call into a **left-associated chain of typed
                // binary CIR mnemonics** — the exact shape the `(+ a b)` case
                // above emits, repeated:
                //
                //     (+ a b c)   →   r1 = add a, b   [i64]
                //                     r2 = add r1, c  [i64]   ⇒ result r2
                //
                // Each step is a typed `add`/`sub`/`mul`/`div` the IIR-to-*
                // backends accept directly, so a variadic arithmetic call now
                // clears every backend validator instead of falling back to the
                // `call_builtin "<op>"` path (`type_hint = "any"`), which they
                // reject.  Comparisons are deliberately excluded: variadic
                // `(< a b c)` is a chained predicate (`a<b ∧ b<c`), not a fold,
                // so it stays on the dynamic path until a dedicated increment.
                // Unary / nullary forms (`(+)`, `(- a)`) likewise stay on the
                // fallback; this increment targets the `n ≥ 2` fold (the
                // `n == 2` arithmetic case is already handled above, so this
                // block effectively lights up `n ≥ 3`).
                if arg_regs.len() >= 2 {
                    if let Some(typed_mnemonic) = typed_arith_op_for(fn_name) {
                        if !typed_mnemonic.starts_with("cmp_")
                            && arg_regs.iter().all(|r| ctx.type_of(r) == "i64")
                        {
                            let mut acc = arg_regs[0].clone();
                            for rhs in &arg_regs[1..] {
                                let dest = ctx.fresh_var("r");
                                ctx.emit(IIRInstr::new(
                                    typed_mnemonic,
                                    Some(dest.clone()),
                                    vec![Operand::Var(acc), Operand::Var(rhs.clone())],
                                    "i64",
                                ), loc);
                                ctx.record_type(&dest, "i64");
                                acc = dest;
                            }
                            return Ok(acc);
                        }
                    }
                }

                // Fallback: legacy dynamic `call_builtin` path.
                let mut srcs: Vec<Operand> =
                    vec![Operand::Var(fn_name.to_string())];
                for r in arg_regs {
                    srcs.push(Operand::Var(r));
                }
                let dest = ctx.fresh_var("r");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    srcs,
                    "any",
                ), loc);
                return Ok(dest);
            }
        }

        // Indirect: compile the fn expression to a closure handle, then
        // invoke via `call_closure` (LANG34).
        //
        // Before LANG34 this emitted `call_builtin "apply_closure" handle args…`.
        // Now: the handle is srcs[0] directly — no leading name-string operand.
        let fn_expr = expr.fn_expr().ok_or_else(|| TwigCompileError {
            message: "empty application '()' — use 'nil' for the empty list".into(),
            line: expr.pos().0,
            column: expr.pos().1,
        })?;
        let fn_handle = self.compile_expr(fn_expr, ctx)?;
        let mut srcs: Vec<Operand> = vec![Operand::Var(fn_handle)];
        for a in args.iter() {
            let r = self.compile_expr(a, ctx)?;
            srcs.push(Operand::Var(r));
        }
        let dest = ctx.fresh_var("r");
        ctx.emit(IIRInstr::new(
            "call_closure",
            Some(dest.clone()),
            srcs,
            "any",
        ), loc);
        Ok(dest)
    }

    // ------------------------------------------------------------------
    // TW05-A / LANG48 — match lowering
    // ------------------------------------------------------------------

    /// Lower a `(match scrutinee arm+)` expression to an if/let chain.
    ///
    /// Evaluation strategy:
    /// 1. Evaluate the scrutinee exactly once into `#matched_N`.
    /// 2. For each arm in order:
    ///    - **Variant arm** `(VarName b1 … bn) body+`:
    ///      Emit an `if` that tests `(= (car #matched_N) tag)`.
    ///      On true: bind fields via `(car (cdr …))` chains and evaluate body.
    ///    - **Binding arm** `name body+`:
    ///      Emit a `let`-style binding of `#matched_N` to `name`, evaluate body.
    ///    - **Wildcard arm** `_ body+`:
    ///      Evaluate body directly with no extra binding.
    /// 3. Fallthrough (no arm matched) → `nil`.
    ///
    /// Arms are chained in the else-branch of each if.  The innermost else
    /// produces a `make_nil` call so behaviour is deterministic when no arm
    /// matches (forward-compatible with exhaustiveness checking in TW05-B).
    fn compile_match(&mut self, m: MatchView<'_>, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(m.pos().0, m.pos().1);

        // Evaluate the scrutinee once.
        // Path-A increment 5: convert all `call_builtin "_move"` sites
        // in compile_match to typed `mov` via `FnCtx::emit_move`, which
        // unblocks the backend validators (they reject `call_builtin
        // "_move"` but accept `mov`).
        //
        // The `match` result is a join temporary (see "Join temporaries"):
        // its writers are the `nil` initialiser and every arm, and its type
        // is their consensus — so a `match` is typed only when every arm
        // is `ref<LispyPair>` like the `nil`.
        let scrutinee = m.scrutinee().ok_or_else(|| TwigCompileError {
            message: "(match ...) missing scrutinee expression".into(),
            line: m.pos().0,
            column: m.pos().1,
        })?;
        let scrutinee_reg = self.compile_expr(scrutinee, ctx)?;
        // Bind to a fresh stable register so arms can reference it freely.
        let matched = ctx.fresh_var("matched");
        ctx.emit_move(&matched, &scrutinee_reg, loc);

        // Result register — each arm writes its value here.
        let result = ctx.fresh_var("match_result");
        // Initialise to nil (fallthrough value).
        //
        // Path A increment 6a: emit typed `const 0 [ref<LispyPair>]`
        // instead of `call_builtin "make_nil" [any]`.  This is the first
        // writer of `result`; it counts toward the consensus even when a
        // catch-all arm makes it dead, because the code that reads
        // `result` cannot tell which writer ran.
        let mut join = JoinType::default();
        let nil_init = ctx.fresh_var("nil");
        ctx.emit(IIRInstr::new(
            "const",
            Some(nil_init.clone()),
            vec![Operand::Int(0)],
            "ref<LispyPair>",
        ), loc);
        ctx.record_type(&nil_init, "ref<LispyPair>");
        ctx.emit_join_move(&result, &nil_init, loc, &mut join);

        // Generate labels for the chain.
        // We build: if (test0) { arm0 } else { if (test1) { arm1 } else { … } }
        // via explicit label/jmp/jmpif instructions.
        let end_label = ctx.fresh_label("match_end");

        for arm in m.arms() {
            let arm_body = arm.body();
            let arm_loc = match arm_body.first() {
                None => loc,
                Some(first) => {
                    let p = first.pos();
                    SourceLoc::new(p.0, p.1)
                }
            };
            let pat = arm.pat();
            match pat {
                Pat::Variant { name, .. } => {
                    // Look up the integer tag.  Unknown names are treated as
                    // binding patterns (forward-compatible with future types).
                    let tag = match self.variant_tags.get(name) {
                        Some(&t) => t,
                        None => {
                            // Unknown constructor — treat as bare binding.
                            let arm_result = self.compile_match_binding_arm(
                                &matched, name, arm_body, ctx, arm_loc)?;
                            ctx.emit_join_move(&result, &arm_result, arm_loc, &mut join);
                            ctx.emit(IIRInstr::new(
                                "jmp",
                                None,
                                vec![Operand::Var(end_label.clone())],
                                "void",
                            ), arm_loc);
                            continue;
                        }
                    };

                    // Emit: tag_reg = (car matched)
                    //
                    // Path A increment 6c: typed `field_load[0]` instead of
                    // `call_builtin "car"`.  The Phase 2 heap-lowering
                    // convention is `field_load [ref<any>]` (the loaded
                    // field can hold any Lisp value).
                    let tag_reg = ctx.fresh_var("tag");
                    ctx.emit(IIRInstr::new(
                        "field_load",
                        Some(tag_reg.clone()),
                        vec![Operand::Var(matched.clone()), Operand::Int(0)],
                        "ref<any>",
                    ), arm_loc);
                    ctx.record_type(&tag_reg, "ref<any>");

                    // Emit: tag_int = integer constant for this variant.
                    //
                    // E6d-6: type this `i64`, NOT `"any"`. It is a raw
                    // compile-time integer, not a boxed lisp value. If it were
                    // `"any"`, `lower_dynamic_arith` would treat it as boxed
                    // (`is_boxed("any")`) and emit a bogus `unbox` on it — on WASM
                    // the `i31.get_s` of a raw `i64` traps `expected i32, got I64`.
                    // As `i64` it flows straight into the typed comparison; only
                    // the genuinely-boxed `tag_reg` (a `field_load` result) is
                    // unboxed. (This mirrors how a `list-ref` index literal is a
                    // raw `i64` const.)
                    let tag_int_reg = ctx.fresh_var("tag_val");
                    ctx.emit(IIRInstr::new(
                        "const",
                        Some(tag_int_reg.clone()),
                        vec![Operand::Int(tag as i64)],
                        "i64",
                    ), arm_loc);
                    ctx.record_type(&tag_int_reg, "i64");

                    // Emit: cond = (= tag_reg tag_int)
                    let cond_reg = ctx.fresh_var("tag_eq");
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(cond_reg.clone()),
                        vec![
                            Operand::Var("=".into()),
                            Operand::Var(tag_reg),
                            Operand::Var(tag_int_reg),
                        ],
                        "any",
                    ), arm_loc);

                    // jmp_if_false cond → skip_label
                    //
                    // If the tag comparison is false (tag ≠ expected variant),
                    // jump over the arm body to the next arm.  When the condition
                    // is true the arm body falls through immediately.
                    //
                    // This mirrors `compile_if` (same two-operand jmp_if_false
                    // pattern) and is the correct IIR opcode recognised by the VM.
                    // The old three-operand `jmpif` was never added to the VM
                    // dispatch and produced UnsupportedOpcode at runtime (LANG57).
                    let skip_label = ctx.fresh_label("match_skip");
                    ctx.emit(IIRInstr::new(
                        "jmp_if_false",
                        None,
                        vec![
                            Operand::Var(cond_reg),
                            Operand::Var(skip_label.clone()),
                        ],
                        "void",
                    ), arm_loc);
                    // arm body follows immediately (fall-through when cond is true)

                    // Bind fields: field_i = (car (cdr^(i+1) matched))
                    //
                    // Path A increment 6c: typed `field_load[1]` (cdr) and
                    // `field_load[0]` (car) instead of `call_builtin "cdr"`
                    // and `call_builtin "car"`.  Phase 2 convention.
                    let mut bound: Vec<&str> = Vec::new();
                    let mut cur_cdr = matched.clone();
                    for binding in pat.bound_names() {
                        // Advance one cdr step
                        let next_cdr = ctx.fresh_var("cdr");
                        ctx.emit(IIRInstr::new(
                            "field_load",
                            Some(next_cdr.clone()),
                            vec![Operand::Var(cur_cdr.clone()), Operand::Int(1)],
                            "ref<any>",
                        ), arm_loc);
                        ctx.record_type(&next_cdr, "ref<any>");
                        cur_cdr = next_cdr;

                        // Extract field: car of the cdr chain
                        let field_reg = ctx.fresh_var("field");
                        ctx.emit(IIRInstr::new(
                            "field_load",
                            Some(field_reg.clone()),
                            vec![Operand::Var(cur_cdr.clone()), Operand::Int(0)],
                            "ref<any>",
                        ), arm_loc);
                        ctx.record_type(&field_reg, "ref<any>");

                        // Bind the field to the name in a register of its
                        // own (#18439) via typed `mov`.
                        ctx.bind_move(binding, &field_reg, arm_loc);
                        bound.push(binding);
                    }

                    // Evaluate body
                    let mut body_result: Option<String> = None;
                    for e in arm_body.iter() {
                        body_result = Some(self.compile_expr(e, ctx)?);
                    }
                    let body_v = body_result.expect("arm body non-empty (check_cst-enforced)");

                    // Copy to result register via typed `mov`.
                    ctx.emit_join_move(&result, &body_v, arm_loc, &mut join);

                    // Unbind field names, so their outer bindings (if any)
                    // are visible again.
                    for name in bound.into_iter().rev() {
                        ctx.pop_binding(name);
                    }

                    // jmp to end
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);

                    // skip_label: next arm or fallthrough
                    ctx.emit(IIRInstr::new(
                        "label",
                        None,
                        vec![Operand::Var(skip_label)],
                        "void",
                    ), arm_loc);
                }

                Pat::Binding(name) => {
                    let arm_result = self.compile_match_binding_arm(
                        &matched, name, arm_body, ctx, arm_loc)?;
                    ctx.emit_join_move(&result, &arm_result, arm_loc, &mut join);
                    // Binding arm always matches → jump to end immediately.
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);
                }

                Pat::Wildcard => {
                    // No binding; evaluate body directly.
                    let mut body_result: Option<String> = None;
                    for e in arm_body.iter() {
                        body_result = Some(self.compile_expr(e, ctx)?);
                    }
                    let body_v = body_result.expect("arm body non-empty");
                    ctx.emit_join_move(&result, &body_v, arm_loc, &mut join);
                    // Wildcard always matches → jump to end.
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);
                }
            }
        }

        // Every writer of `result` is in: keep its type only if they agree.
        ctx.settle_join(&result, &join);

        // end_label: all paths converge here.
        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(end_label)],
            "void",
        ), loc);

        Ok(result)
    }

    /// Helper: bind `matched_reg` to `name` (in its own register), evaluate `body`,
    /// then remove the binding.  Returns the body's result register.
    fn compile_match_binding_arm(
        &mut self,
        matched_reg: &str,
        name: &str,
        body: Exprs<'_>,
        ctx: &mut FnCtx,
        loc: SourceLoc,
    ) -> Result<String, TwigCompileError> {
        // Path-A increment 5: typed `mov` for the binding-arm helper, into
        // a register of the binding's own (#18439).
        ctx.bind_move(name, matched_reg, loc);
        let mut last: Option<String> = None;
        for e in body.iter() {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("arm body non-empty (check_cst-enforced)");
        ctx.pop_binding(name);
        Ok(last)
    }

    // ------------------------------------------------------------------
    // TW05-A / LANG48 — record and union erasure
    // ------------------------------------------------------------------

    /// Erase a `(record Name field*)` declaration into IIR functions:
    ///
    /// - Constructor `Name(f0, f1, …, fn)` → `cons` chain ending in `nil`
    /// - Accessor `<lower(Name)>-<field>(r)` → `car` of the right `cdr` chain
    /// - Predicate `<lower(Name)>?(v)` → `(pair? v)`
    fn emit_record_def(&mut self, rec: RecordView<'_>) -> Result<(), TwigCompileError> {
        let loc = SourceLoc::SYNTHETIC;
        let prefix = rec.name().to_lowercase();

        // ── Constructor: Name(f0, f1, …, fn) ──────────────────────────
        // Builds (cons f0 (cons f1 (… (cons fn nil) …))).
        {
            let mut ctx = FnCtx::new();
            // Parameter registers (renamed only if a field name repeats).
            let params: Vec<String> = rec.fields().map(|f| ctx.bind_param(f)).collect();

            // Start from the nil end and fold right.
            //
            // Path A increment 6a + 6b: typed `const 0 [ref<LispyPair>]`
            // initial tail, and each cons cell lowers to typed
            // `alloc [ref<LispyPair>]` + 2× `field_store [void]`
            // (Phase 2 heap-lowering convention).
            let nil_r = ctx.fresh_var("nil");
            ctx.emit(IIRInstr::new(
                "const",
                Some(nil_r.clone()),
                vec![Operand::Int(0)],
                "ref<LispyPair>",
            ), loc);
            ctx.record_type(&nil_r, "ref<LispyPair>");

            // Path A increment 6b: emit typed `alloc` + 2× `field_store`
            // instead of `call_builtin "cons" [any]`.  The Phase 2 heap-
            // lowering convention is:
            //   alloc cell [ref<LispyPair>]
            //   field_store cell, 0, head [void]    -- car
            //   field_store cell, 1, tail [void]    -- cdr
            // Every IIR-to-* backend accepts this triple for ref<LispyPair>.
            let mut tail = nil_r;
            for field_name in params.iter().rev() {
                let cell = ctx.fresh_var("cell");
                let mut alloc = IIRInstr::new(
                    "alloc",
                    Some(cell.clone()),
                    vec![],
                    "ref<LispyPair>",
                );
                alloc.may_alloc = true;
                ctx.emit(alloc, loc);
                ctx.record_type(&cell, "ref<LispyPair>");
                // field_store cell, 0, head — car
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(cell.clone()),
                        Operand::Int(0),
                        Operand::Var(field_name.clone()),
                    ],
                    // type_hint "void" matches iir-builtin-lowering's
                    // Phase 2 convention and the BEAM validator.  WASM,
                    // JVM, CLR accept it via their GC-op handlers.
                    "void",
                ), loc);
                // field_store cell, 1, tail — cdr
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(cell.clone()),
                        Operand::Int(1),
                        Operand::Var(tail),
                    ],
                    "void",
                ), loc);
                tail = cell;
            }
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(tail)], "ref<LispyPair>"), loc);

            self.functions.push(IIRFunction {
                name: rec.name().to_string(),
                params: params.iter().map(|p| (p.clone(), "any".to_string())).collect(),
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }

        // ── Accessors: <prefix>-<field>(r) → car(cdr^i(r)) ───────────
        //
        // Path A increment 6c: typed `field_load[0]` / `field_load[1]`
        // instead of `call_builtin "car"` / `"cdr"`.  Phase 2 convention.
        for (i, field) in rec.fields().enumerate() {
            let mut ctx = FnCtx::new();
            ctx.bind_param("r"); // first binding in a fresh function: register `r`

            // Build the cdr chain: apply `cdr` i times.
            let mut cur = "r".to_string();
            for _ in 0..i {
                let next = ctx.fresh_var("cdr");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(next.clone()),
                    vec![Operand::Var(cur), Operand::Int(1)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&next, "ref<any>");
                cur = next;
            }
            // Then take car.
            let field_val = ctx.fresh_var("fv");
            ctx.emit(IIRInstr::new(
                "field_load",
                Some(field_val.clone()),
                vec![Operand::Var(cur), Operand::Int(0)],
                "ref<any>",
            ), loc);
            ctx.record_type(&field_val, "ref<any>");
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(field_val)], "ref<any>"), loc);

            self.functions.push(IIRFunction {
                name: format!("{prefix}-{field}"),
                params: vec![("r".to_string(), "any".to_string())],
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }

        // ── Predicate: <prefix>?(v) → (pair? v) ───────────────────────
        {
            let mut ctx = FnCtx::new();
            ctx.bind_param("v"); // first binding in a fresh function: register `v`
            let pred = ctx.fresh_var("pred");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(pred.clone()),
                vec![Operand::Var("pair?".into()), Operand::Var("v".to_string())],
                "any",
            ), loc);
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(pred)], "any"), loc);

            self.functions.push(IIRFunction {
                name: format!("{prefix}?"),
                params: vec![("v".to_string(), "any".to_string())],
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }
        Ok(())
    }

    /// Erase a `(union Name variant*)` declaration into IIR functions.
    ///
    /// Each variant at zero-based index `i` produces:
    /// - Constructor `VarName(f0, …, fn)` → `(cons i (cons f0 (… (cons fn nil) …)))`
    /// - Predicate `VarName?(v)` → `(= (car v) i)`
    /// - Accessor `<lower(VarName)>-<field>(v)` → `car(cdr^(k+1) v)` (k = field index)
    fn emit_union_def(&mut self, union: UnionView<'_>) -> Result<(), TwigCompileError> {
        let loc = SourceLoc::SYNTHETIC;

        for (tag, variant) in union.variants().enumerate() {
            let vprefix = variant.name().to_lowercase();

            // ── Constructor: VarName(f0, …, fn) ───────────────────────
            // → (cons tag (cons f0 (cons f1 (… (cons fn nil) …))))
            {
                let mut ctx = FnCtx::new();
                // Parameter registers (renamed only if a field name repeats).
                let params: Vec<String> =
                    variant.fields().map(|f| ctx.bind_param(f)).collect();

                // Start from nil, fold right over fields.
                //
                // Path A increment 6a: typed `const 0 [ref<LispyPair>]`
                // initial tail for the variant's cons chain.
                let nil_r = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(nil_r.clone()),
                    vec![Operand::Int(0)],
                    "ref<LispyPair>",
                ), loc);
                ctx.record_type(&nil_r, "ref<LispyPair>");

                // Path A increment 6b: typed `alloc` + 2× `field_store`
                // for each variant field's cons cell (see record-constructor
                // site above for the convention).
                let mut tail = nil_r;
                for field_name in params.iter().rev() {
                    // E6d-6b: the field must be stored as a *boxed* `DynValue`, not
                    // a raw word. `match`/accessors read the field back as `any`
                    // (the loaded value flows through `unbox`/dynamic ops), so the
                    // constructor boxes here. On the tagged backends (`any` = raw
                    // i64) this is the `n<<3` that makes `unbox` recover the value;
                    // on the structural backends `box` of an already-`anyref` field
                    // is the identity, so the round-trip is unchanged.
                    //
                    // VM-041 (BEAM05 follow-up): `box` is emitted BEFORE `alloc`
                    // here — not interleaved between them — so the resulting
                    // `alloc, field_store, field_store` triple is textually
                    // adjacent for `iir-to-beam`'s cons-cell fusion look-ahead
                    // (which only recognizes the three instructions when
                    // immediately adjacent; see `lower.rs`'s "alloc ref<LispyPair>
                    // -> put_list" arm). `box` doesn't read or write the `alloc`'d
                    // cell — it only reads `field_name`, which is independent of
                    // the fresh cell register — so hoisting it above the `alloc`
                    // changes no data dependency and mirrors this same function's
                    // tag/head cons cell below, which already computes `tag_boxed`
                    // before its `alloc_head`.
                    let field_boxed = ctx.fresh_var("fbox");
                    ctx.emit(IIRInstr::new(
                        "box",
                        Some(field_boxed.clone()),
                        vec![Operand::Var(field_name.clone())],
                        "ref<any>",
                    ), loc);
                    ctx.record_type(&field_boxed, "ref<any>");

                    let cell = ctx.fresh_var("cell");
                    let mut alloc = IIRInstr::new(
                        "alloc",
                        Some(cell.clone()),
                        vec![],
                        "ref<LispyPair>",
                    );
                    alloc.may_alloc = true;
                    ctx.emit(alloc, loc);
                    ctx.record_type(&cell, "ref<LispyPair>");
                    ctx.emit(IIRInstr::new(
                        "field_store",
                        None,
                        vec![
                            Operand::Var(cell.clone()),
                            Operand::Int(0),
                            Operand::Var(field_boxed),
                        ],
                        "void",
                    ), loc);
                    ctx.emit(IIRInstr::new(
                        "field_store",
                        None,
                        vec![
                            Operand::Var(cell.clone()),
                            Operand::Int(1),
                            Operand::Var(tail),
                        ],
                        "void",
                    ), loc);
                    tail = cell;
                }

                // Prepend the integer tag — typed `const Int(tag) [i64]`
                // then typed `alloc` + `field_store` cons cell.
                let tag_reg = ctx.fresh_var("tag");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(tag_reg.clone()),
                    vec![Operand::Int(tag as i64)],
                    "i64",
                ), loc);
                ctx.record_type(&tag_reg, "i64");
                // E6d-6b: box the tag for the same reason as the fields — `match`
                // reads the tag (`car`) back as a boxed `DynValue` and compares it
                // with the dynamic `=` (which unboxes). A raw tag word makes
                // `unbox(raw tag)` wrong on the tagged backends (`unbox(1)=0`, so
                // the second variant never matches).
                let tag_boxed = ctx.fresh_var("tbox");
                ctx.emit(IIRInstr::new(
                    "box",
                    Some(tag_boxed.clone()),
                    vec![Operand::Var(tag_reg)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&tag_boxed, "ref<any>");
                let head = ctx.fresh_var("head");
                let mut alloc_head = IIRInstr::new(
                    "alloc",
                    Some(head.clone()),
                    vec![],
                    "ref<LispyPair>",
                );
                alloc_head.may_alloc = true;
                ctx.emit(alloc_head, loc);
                ctx.record_type(&head, "ref<LispyPair>");
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(head.clone()),
                        Operand::Int(0),
                        Operand::Var(tag_boxed),
                    ],
                    "void",
                ), loc);
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(head.clone()),
                        Operand::Int(1),
                        Operand::Var(tail),
                    ],
                    "void",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(head)], "ref<LispyPair>"), loc);

                self.functions.push(IIRFunction {
                    name: variant.name().to_string(),
                    params: params.iter().map(|p| (p.clone(), "any".to_string())).collect(),
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }

            // ── Predicate: VarName?(v) → (= (car v) tag) ──────────────
            {
                let mut ctx = FnCtx::new();
                ctx.bind_param("v"); // first binding in a fresh function: register `v`

                // Path A increment 6c: typed `field_load[0]` instead of
                // `call_builtin "car"`.
                let car_v = ctx.fresh_var("hd");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(car_v.clone()),
                    vec![Operand::Var("v".to_string()), Operand::Int(0)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&car_v, "ref<any>");

                let tag_reg = ctx.fresh_var("tag");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(tag_reg.clone()),
                    vec![Operand::Int(tag as i64)],
                    "any",
                ), loc);

                let result = ctx.fresh_var("pred");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(result.clone()),
                    vec![
                        Operand::Var("=".into()),
                        Operand::Var(car_v),
                        Operand::Var(tag_reg),
                    ],
                    "any",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(result)], "any"), loc);

                self.functions.push(IIRFunction {
                    name: format!("{}?", variant.name()),
                    params: vec![("v".to_string(), "any".to_string())],
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }

            // ── Accessors: <vprefix>-<field>(v) → car(cdr^(k+1) v) ────
            // Field k is at position k+1 (after the tag at cdr^0/car).
            //
            // Path A increment 6c: typed `field_load[1]` (cdr) and
            // `field_load[0]` (car) instead of `call_builtin`.
            for (k, field) in variant.fields().enumerate() {
                let mut ctx = FnCtx::new();
                ctx.bind_param("v"); // first binding in a fresh function: register `v`

                // cdr^(k+1) v: skip the tag (1 cdr) then k more for field index.
                let mut cur = "v".to_string();
                for _ in 0..=(k) {
                    let next = ctx.fresh_var("cdr");
                    ctx.emit(IIRInstr::new(
                        "field_load",
                        Some(next.clone()),
                        vec![Operand::Var(cur), Operand::Int(1)],
                        "ref<any>",
                    ), loc);
                    ctx.record_type(&next, "ref<any>");
                    cur = next;
                }
                let fval = ctx.fresh_var("fv");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(fval.clone()),
                    vec![Operand::Var(cur), Operand::Int(0)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&fval, "ref<any>");
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(fval)], "ref<any>"), loc);

                self.functions.push(IIRFunction {
                    name: format!("{vprefix}-{field}"),
                    params: vec![("v".to_string(), "any".to_string())],
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // String-literal helper
    // ------------------------------------------------------------------

    /// Materialise a string literal into a fresh register and return
    /// the register's name.
    ///
    /// Used when a `call_builtin` needs a literal string as one of its
    /// runtime arguments — e.g. the `name` argument to `make_symbol`,
    /// `global_set`, `global_get`.  The string is emitted as a `const`
    /// instruction whose source is `Operand::Var(literal_text)`, which the
    /// twig-vm `const` handler interns as a symbol.
    ///
    /// **Note:** `alloc_closure` (LANG34) no longer uses `string_arg` — the
    /// function name is embedded inline as `Operand::Str` in the instruction
    /// itself.  `string_arg` is retained only for the operations above that
    /// still require the register-materialised convention.
    fn string_arg(&mut self, ctx: &mut FnCtx, literal: &str, loc: SourceLoc) -> String {
        let v = ctx.fresh_var("s");
        ctx.emit(IIRInstr::new(
            "const",
            Some(v.clone()),
            vec![Operand::Var(literal.to_string())],
            "any",
        ), loc);
        v
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler::new()
    }
}

// ---------------------------------------------------------------------------
// Register-count estimation
// ---------------------------------------------------------------------------
//
// `vm-core` allocates `register_count` slots per frame.  We count every
// distinct dest plus every Var operand that reads a name — the
// register file uses names as keys, so the count of distinct names is
// a tight upper bound.  +8 headroom matches the brainfuck-iir-compiler
// convention.

fn count_registers(instrs: &[IIRInstr]) -> usize {
    let mut names: HashSet<&str> = HashSet::new();
    for instr in instrs {
        if let Some(d) = instr.dest.as_deref() {
            names.insert(d);
        }
        for src in &instr.srcs {
            if let Operand::Var(s) = src {
                names.insert(s.as_str());
            }
        }
    }
    std::cmp::max(names.len() + 8, 16)
}
