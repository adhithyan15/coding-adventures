//! LANG83 operation conformance — every standard operation, every backend.
//!
//! `code/specs/LANG83-iir-standard-operation-registry.md` makes every operation a
//! frontend may emit a first-class IIR op or standard builtin that **each backend
//! lowers itself** — or refuses at compile time. Each backend crate declares its
//! answer in an `op_support` table. This suite is where those declarations meet
//! reality:
//!
//! | Test | Claim it checks |
//! |------|-----------------|
//! | `every_registry_op_has_cases` | no registry op is without a conformance case |
//! | `every_lowered_op_runs_on_every_backend` | a `Lowered` declaration is true: the case compiles on every code-generation backend that declares it, and runs with the expected output on every executable column |
//! | `refused_ops_are_rejected_at_compile_time` | a `Refused` declaration is enforced: `check_module_for` *and* the backend's own compile entry point reject the case with a typed `UnsupportedOps` error |
//! | `print_i64_and_putchar_share_one_ordered_stream` | the LANG83 ordered-output contract for the one builtin pair no frontend emits together today |
//! | `pr1b_ops_are_lowered_on_every_backend` | LANG83 PR1b landed everywhere: no backend still refuses `write_i64`, `i64_div_floor`, `i64_mod_floor` or `trap` |
//! | `pr4_ops_are_lowered_on_every_backend` | LANG83 PR4 landed everywhere: every backend lowers `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf` and `i64_neg_ovf` |
//! | `pr7e_float_writers_are_gone_and_the_layouts_run_everywhere` | LANG83 PR7e: `write_f64_py` / `write_f64_js` are in no registry and no backend (each refuses a module that calls one), and the PR6a table, printed by the Python and JavaScript compilers' own layout helpers, runs on every executable column |
//! | `pr6c_ops_follow_the_ieee_contract` | LANG83 PR6c: the IEEE arithmetic is lowered everywhere but BEAM (which refuses it at compile time), `f64_signbit` on all nine, and the PR6c cases run on every executable column |
//! | `failed_narrowing_traps_conv_range_on_every_executable_column` | a `real_to_int_*` of NaN, ±∞, 2⁶³ or −1e30 stops with the typed `Trap.ConvRange` after earlier output on every executable column, and compiles for CLR and (finite rows) BEAM |
//! | `float_table_matches_host_python3_and_node` | the PR6a expected strings (the oracle the PR7e helper cases print) are what host `python3` (`repr`) and `node` (`String(x)`) print today, whenever they are installed |
//! | `harness_builtins_name_no_language` | LANG83 PR7e: no builtin a conformance runner registers names a source language |
//! | `pr7b_ops_are_lowered_on_every_backend` | LANG83 PR7b: `f64_class`, `f64_digits` and `f64_exp10` are lowered on all nine backends, the PR7b cases run on every executable column, and BEAM compiles the finite ones |
//! | `pr7f2_lisp_rules_are_module_code_over_the_primitives` | LANG83 PR7f-2: Lisp's truthiness, `not`, `equal?` and exit code, written as IIR by `iir_builtin_lowering::lisp_semantics` over the dynamic-value primitives, print what `dynval-runtime`'s Rust value model says on LLVM and native x86-64, and no tagged-word backend knows a language-semantic builtin |
//! | `pr7f_dynamic_values_run_on_the_tagged_word_backends` | LANG83 PR7f: the dynamic-value primitives are lowered exactly on the three tagged-word backends (native x86-64 and AArch64, LLVM) and refused, before any pass runs, everywhere else; the PR7f case runs on LLVM and native x86-64 whenever they are present |
//! | `float_digits_table_matches_host_python3` | the PR7b expected digits are host `python3`'s `repr` split at the digits, whenever it is installed |
//! | `pr5a_string_failures_are_registered_traps_on_every_column` | LANG83 PR5a: an out-of-range `str_index` / `str_slice` is `Trap.Bounds` and a `str_concat` past the module's string ceiling is `Trap.Overflow`, with the exact bytes written before the trap, in a foldable (literal) and an opaque (parameter) form, on every executable column; the 64-bit edges (2³², 2³² + 1, `i64::MIN`/`MAX`) included |
//! | `flush_before_trap_is_proven_on_every_executable_column` | a program that writes (unflushed) output and then traps keeps that output, and the trap kind is observed, on every column that can run it |
//!
//! # The case corpus is IIR, not source
//!
//! Each [`Case`] builds its `IIRModule` directly, so an operation can be tested the
//! moment it enters the registry — before any frontend emits it — and so the
//! corpus says exactly which operations a case exercises: the registry operations
//! a case *uses* are computed from its module (`registry_entry` over every
//! instruction), never hand-listed, and a case runs on a backend only if the
//! backend lowers all of them.
//!
//! # Columns and skip policy
//!
//! | Column | What runs here | Skips when |
//! |--------|----------------|------------|
//! | WASM | compile + execute (in-process `wasm-runtime`) | never |
//! | VM | execute (`vm-core`) | never |
//! | JIT | execute (`jit-core`), and every call-free function (`main`, or a layout helper's digit functions) must really be JIT-compiled | never (cases whose ops the JIT refuses are not JIT cases) |
//! | LLVM | compile + `clang` + execute | `clang` absent |
//! | native | compile + link + execute (Linux host) | not Linux |
//! | JVM | compile + `java` (verifier on) + execute | `java`/`javac` absent |
//! | CLR, BEAM | compile to the artifact the column runs | never — their execution stays in CI's `lang_matrix` columns (no local `dotnet`/`erl`) |
//!
//! The in-process columns follow `lang_matrix::proven_columns_do_not_silently_skip`:
//! they have no toolchain to be absent, so they never skip.
//!
//! # Outcomes
//!
//! [`Expect::StdoutThenTrap`] and [`RunResult::Trapped`]'s `stdout` exist so a
//! "print, then trap" case can assert both the output written before the trap and
//! the trap kind. Since LANG83 PR1b every column lowers `trap`, so those cases
//! run everywhere; each column reports the kind its own way (native/LLVM
//! `trap: <kind>` on stderr, JVM `RuntimeException: <kind>`, WASM the
//! `env.__trap` kind id, VM and JIT `VMError::Trap` — the JIT's error slot is
//! drained into that `Err`, and a message left in the slot fails the case as
//! fail-open), and every column must report it — `None` is a failure, not a skip.
//!
//! `StdoutThenTrap` / stdout-carrying `Trapped` live here, not in `lang_matrix`
//! yet: no language row emits a PR1b op. The first frontend that does (LANG83
//! PR2, Ruby) ports them there with its rows.

use std::process::Command;
use std::sync::{Arc, Mutex};

use interpreter_ir::op_registry::{language_name_violation, lookup, registry_entry, OpSupport, STANDARD_OPS};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lang_aot::{check_module_for, Backend, LangAotError, Language};

mod common;

// ===========================================================================
// Expected outcomes and observed results
// ===========================================================================

/// What a case must do when run.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// Complete with this stdout, compared after a lossy UTF-8 decode, a
    /// `trim()` and `\r\n` → `\n` (the cases that print lines).
    Stdout(&'static str),
    /// Write this stdout (compared like [`Expect::Stdout`]), then trap with
    /// this kind.
    StdoutThenTrap(&'static str, &'static str),
    /// LANG83 PR5a: complete with exactly these stdout bytes: no decode, no
    /// trim, no newline translation. A lossy decode reads a stray `0xA9` and a
    /// stray `0xFF` as the same U+FFFD, and `trim()` hides a missing or extra
    /// trailing newline; a byte-string case must see both.
    StdoutBytes(&'static [u8]),
    /// LANG83 PR5a: write exactly these bytes, then trap with this kind.
    StdoutBytesThenTrap(&'static [u8], &'static str),
}

impl Expect {
    /// The expected output, as text (lossy for a byte expectation).
    fn out_text(&self) -> std::borrow::Cow<'static, str> {
        match *self {
            Expect::Stdout(out) | Expect::StdoutThenTrap(out, _) => out.into(),
            Expect::StdoutBytes(out) | Expect::StdoutBytesThenTrap(out, _) => String::from_utf8_lossy(out),
        }
    }

    /// The trap kind the case must end with, if any.
    fn trap_kind(&self) -> Option<&'static str> {
        match *self {
            Expect::StdoutThenTrap(_, kind) | Expect::StdoutBytesThenTrap(_, kind) => Some(kind),
            Expect::Stdout(_) | Expect::StdoutBytes(_) => None,
        }
    }
}

/// What a column observed.
#[derive(Debug)]
enum RunResult {
    /// The program returned normally. `stdout` is the raw bytes it wrote.
    Completed { stdout: Vec<u8> },
    /// The program trapped. `stdout` is every byte written before the trap;
    /// `kind` is the reported trap kind when the column can recover it.
    Trapped { stdout: Vec<u8>, kind: Option<String> },
}

/// The text form [`Expect::Stdout`] compares: lossy, trimmed, LF newlines.
fn stdout_text(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout).trim().replace("\r\n", "\n")
}

fn assert_outcome(column: &str, case: &Case, result: RunResult) {
    match (case.expect, result) {
        (Expect::Stdout(want), RunResult::Completed { stdout }) => {
            assert_eq!(stdout_text(&stdout), want, "{column}: case `{}` stdout", case.name)
        }
        (Expect::StdoutThenTrap(want, kind), RunResult::Trapped { stdout, kind: got }) => {
            assert_eq!(stdout_text(&stdout), want, "{column}: case `{}` stdout before trap", case.name);
            assert_eq!(got.as_deref(), Some(kind), "{column}: case `{}` trap kind", case.name);
        }
        (Expect::StdoutBytes(want), RunResult::Completed { stdout }) => {
            assert_eq!(stdout, want, "{column}: case `{}` stdout bytes", case.name)
        }
        (Expect::StdoutBytesThenTrap(want, kind), RunResult::Trapped { stdout, kind: got }) => {
            assert_eq!(stdout, want, "{column}: case `{}` stdout bytes before trap", case.name);
            assert_eq!(got.as_deref(), Some(kind), "{column}: case `{}` trap kind", case.name);
        }
        (expect, other) => panic!("{column}: case `{}` expected {expect:?}, got {other:?}", case.name),
    }
}

// ===========================================================================
// The case corpus
// ===========================================================================

/// One conformance case: a named IIR program and its expected outcome.
struct Case {
    name: &'static str,
    build: fn() -> IIRModule,
    expect: Expect,
}

impl Case {
    fn module(&self) -> IIRModule {
        (self.build)()
    }

    /// Every registry operation the case's module uses — computed, not listed.
    fn registry_ops(&self) -> Vec<&'static str> {
        let module = self.module();
        let mut ops: Vec<&'static str> = module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .filter_map(|i| registry_entry(i).map(|s| s.name))
            .collect();
        ops.sort_unstable();
        ops.dedup();
        ops
    }

    /// `true` if `backend` lowers every registry operation the case uses,
    /// and can hold every constant the case writes.
    ///
    /// The second half is BEAM's alone: an Erlang float is always finite, so
    /// `validate_for_beam` refuses a `±Infinity`/`NaN` constant even though
    /// `const` itself is lowered. Before that check, BEAM compiled the
    /// float-table cases into modules its loader could not read.
    fn lowered_on(&self, backend: Backend) -> bool {
        self.registry_ops().iter().all(|op| backend.op_support(op).is_lowered())
            && !(backend == Backend::Beam && self.has_non_finite_float_const())
    }

    /// Whether any `const` in the case's module is a non-finite `f64`.
    fn has_non_finite_float_const(&self) -> bool {
        self.module().functions.iter().flat_map(|f| f.instructions.iter()).any(|i| {
            i.op == "const" && matches!(i.srcs.first(), Some(Operand::Float(x)) if !x.is_finite())
        })
    }
}

// --- tiny IIR builders, so each case reads as a program ----------------------

fn var(name: &str) -> Operand {
    Operand::Var(name.into())
}
fn int(d: &str, n: i64) -> IIRInstr {
    IIRInstr::new("const", Some(d.into()), vec![Operand::Int(n)], "i64")
}
fn real(d: &str, x: f64) -> IIRInstr {
    IIRInstr::new("const", Some(d.into()), vec![Operand::Float(x)], "f64")
}
fn text(d: &str, s: &str) -> IIRInstr {
    IIRInstr::new("str_const", Some(d.into()), vec![Operand::Str(s.into())], "str")
}
fn op(name: &str, d: &str, srcs: &[&str], ty: &str) -> IIRInstr {
    IIRInstr::new(name, Some(d.into()), srcs.iter().map(|s| var(s)).collect(), ty)
}
fn builtin(name: &str, arg: &str) -> IIRInstr {
    IIRInstr::new("call_builtin", None, vec![var(name), var(arg)], "void")
}
fn print_i64(v: &str) -> IIRInstr {
    builtin("print_i64", v)
}
fn print_str(v: &str) -> IIRInstr {
    IIRInstr::new("print_str", None, vec![var(v)], "void")
}
/// `d = call_builtin name args… : ty` — a builtin with a result (LANG83 PR7f's
/// dynamic-value primitives are all of this shape).
fn builtin_to(name: &str, d: &str, args: &[&str], ty: &str) -> IIRInstr {
    let mut srcs = vec![var(name)];
    srcs.extend(args.iter().map(|a| var(a)));
    IIRInstr::new("call_builtin", Some(d.into()), srcs, ty)
}

/// The dynamic-value type hint (LANG83 PR7f): a tagged word.
const DYN: &str = "ref<any>";

/// The values the PR7f-2 Lisp-rule case tests, in order: an integer 0 and 7,
/// nil, `#t`, `#f`, the pairs `(1 . 2)` (twice, separately allocated) and
/// `(1 . 3)`, the list `(1 2)` (twice), and the symbols `A` (twice) and `B`.
const LISP_RULE_VALUES: [&str; 13] = ["i0", "i7", "nil", "t", "f", "p12", "p12b", "p13", "l", "lb", "sa", "sa2", "sb"];
/// Truthiness and `not` are printed for these values …
const LISP_RULE_TESTED: [&str; 7] = ["i0", "i7", "nil", "t", "f", "p12", "sa"];
/// … the exit code for these (a pair's or symbol's code is its word, which
/// is not portable across runs) …
const LISP_RULE_EXITS: [&str; 5] = ["i7", "t", "f", "nil", "i0"];
/// … and `equal?` for these pairs of values.
const LISP_RULE_EQUAL: [(&str, &str); 12] = [
    ("p12", "p12b"),
    ("p12", "p13"),
    ("l", "lb"),
    ("i7", "i7"),
    ("i7", "i0"),
    ("p12", "i7"),
    ("nil", "nil"),
    ("l", "p12"),
    ("sa", "sa2"),
    ("sa", "sb"),
    ("t", "t"),
    ("t", "f"),
];

/// LANG83 PR7f-2: Lisp's four value rules as the Lisp lowering writes them —
/// truthiness and `not` inline, `equal?` and the exit code as module
/// functions — over values built with the dynamic-value primitives. The
/// module is complete before any pass: `lisp_semantics` expands and appends
/// exactly what `lower_dyn_repr` would.
fn lisp_rules_program() -> IIRModule {
    use iir_builtin_lowering::lisp_semantics::{self, EQUAL, EXIT_CODE, NOT, TRUTHY};
    let boxed = |d: &str, n: i64| vec![int(&format!("{d}.n"), n), builtin_to("dyn_box_int", d, &[&format!("{d}.n")], DYN)];
    let symbol = |d: &str, name: &str| IIRInstr::new("const", Some(d.into()), vec![var(name)], "symbol");
    let mut b = Vec::new();
    b.extend(boxed("i0", 0));
    b.extend(boxed("i7", 7));
    b.extend(boxed("one", 1));
    b.extend(boxed("two", 2));
    b.extend(boxed("three", 3));
    b.push(builtin_to("dyn_nil", "nil", &[], DYN));
    b.push(builtin_to("dyn_is_nil", "t", &["nil"], DYN));
    b.push(builtin_to("dyn_is_nil", "f", &["i0"], DYN));
    b.push(builtin_to("dyn_pair_new", "p12", &["one", "two"], DYN));
    b.push(builtin_to("dyn_pair_new", "p12b", &["one", "two"], DYN));
    b.push(builtin_to("dyn_pair_new", "p13", &["one", "three"], DYN));
    b.push(builtin_to("dyn_pair_new", "l.tail", &["two", "nil"], DYN));
    b.push(builtin_to("dyn_pair_new", "l", &["one", "l.tail"], DYN));
    b.push(builtin_to("dyn_pair_new", "lb.tail", &["two", "nil"], DYN));
    b.push(builtin_to("dyn_pair_new", "lb", &["one", "lb.tail"], DYN));
    b.push(symbol("sa", "A"));
    b.push(symbol("sa2", "A"));
    b.push(symbol("sb", "B"));
    for x in LISP_RULE_TESTED {
        b.push(lisp_semantics::call(&format!("{x}.truthy"), TRUTHY, &[x], "i64"));
        b.push(print_i64(&format!("{x}.truthy")));
        b.push(lisp_semantics::call(&format!("{x}.not"), NOT, &[x], DYN));
        b.push(print_i64(&format!("{x}.not")));
    }
    for x in LISP_RULE_EXITS {
        b.push(lisp_semantics::call(&format!("{x}.exit"), EXIT_CODE, &[x], "i64"));
        b.push(print_i64(&format!("{x}.exit")));
    }
    for (k, (x, y)) in LISP_RULE_EQUAL.iter().enumerate() {
        b.push(lisp_semantics::call(&format!("eq{k}"), EQUAL, &[x, y], DYN));
        b.push(print_i64(&format!("eq{k}")));
    }
    let mut module = program(b);
    for f in &mut module.functions {
        lisp_semantics::expand_inline_rules(f);
    }
    lisp_semantics::add_used_helpers(&mut module).expect("the case defines no helper of its own");
    module
}

/// LANG83 PR7f: every dynamic-value primitive but `dyn_symbol`, each result
/// printed as an integer. Atoms are boxed explicitly with `dyn_box_int`, so
/// the tagged-word lowering passes have no atom of their own to box.
///
/// | line | expression                    | value |
/// |------|-------------------------------|-------|
/// | 1    | unbox (first (pair 41 -7))    | 41    |
/// | 2    | unbox (rest (pair 41 -7))     | -7    |
/// | 3    | tag (pair …)                  | 7     |
/// | 4    | tag (box 41)                  | 0     |
/// | 5    | tag nil                       | 1     |
/// | 6    | tag (is_nil nil)              | 5 (true)  |
/// | 7    | tag (is_nil (pair …))         | 3 (false) |
/// | 8    | tag (is_pair (pair …))        | 5 (true)  |
/// | 9    | tag (is_pair (box 41))        | 3 (false) |
/// | 10   | unbox (first (rest (pair 1 (pair 2 nil)))) | 2 |
fn dynamic_value_program() -> IIRModule {
    program(vec![
        int("a", 41),
        builtin_to("dyn_box_int", "ba", &["a"], DYN),
        int("b", -7),
        builtin_to("dyn_box_int", "bb", &["b"], DYN),
        builtin_to("dyn_pair_new", "p", &["ba", "bb"], DYN),
        builtin_to("dyn_pair_first", "f", &["p"], DYN),
        builtin_to("dyn_unbox_int", "uf", &["f"], "i64"),
        print_i64("uf"),
        builtin_to("dyn_pair_rest", "r", &["p"], DYN),
        builtin_to("dyn_unbox_int", "ur", &["r"], "i64"),
        print_i64("ur"),
        builtin_to("dyn_tag", "t_pair", &["p"], "i64"),
        print_i64("t_pair"),
        builtin_to("dyn_tag", "t_int", &["ba"], "i64"),
        print_i64("t_int"),
        builtin_to("dyn_nil", "n", &[], DYN),
        builtin_to("dyn_tag", "t_nil", &["n"], "i64"),
        print_i64("t_nil"),
        builtin_to("dyn_is_nil", "nil_is_nil", &["n"], DYN),
        builtin_to("dyn_tag", "t1", &["nil_is_nil"], "i64"),
        print_i64("t1"),
        builtin_to("dyn_is_nil", "pair_is_nil", &["p"], DYN),
        builtin_to("dyn_tag", "t2", &["pair_is_nil"], "i64"),
        print_i64("t2"),
        builtin_to("dyn_is_pair", "pair_is_pair", &["p"], DYN),
        builtin_to("dyn_tag", "t3", &["pair_is_pair"], "i64"),
        print_i64("t3"),
        builtin_to("dyn_is_pair", "int_is_pair", &["ba"], DYN),
        builtin_to("dyn_tag", "t4", &["int_is_pair"], "i64"),
        print_i64("t4"),
        int("one", 1),
        builtin_to("dyn_box_int", "b1", &["one"], DYN),
        int("two", 2),
        builtin_to("dyn_box_int", "b2", &["two"], DYN),
        builtin_to("dyn_pair_new", "inner", &["b2", "n"], DYN),
        builtin_to("dyn_pair_new", "outer", &["b1", "inner"], DYN),
        builtin_to("dyn_pair_rest", "o_rest", &["outer"], DYN),
        builtin_to("dyn_pair_first", "o_second", &["o_rest"], DYN),
        builtin_to("dyn_unbox_int", "second", &["o_second"], "i64"),
        print_i64("second"),
    ])
}

/// `main() -> i64 { <body>; return 0 }` — exit code 0, observable via stdout.
fn program(mut body: Vec<IIRInstr>) -> IIRModule {
    body.push(int("__exit", 0));
    body.push(IIRInstr::new("ret", None, vec![var("__exit")], "i64"));
    let mut module = IIRModule::new("opconf", "iir");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
    module
}

/// Two-operand integer op `name(a, b)`, printed.
fn int_binop(name: &str, a: i64, b: i64) -> IIRModule {
    program(vec![int("a", a), int("b", b), op(name, "r", &["a", "b"], "i64"), print_i64("r")])
}

/// `name(a, b)` for every pair, each result printed with `print_i64`.
fn floor_table(name: &str, pairs: &[(i64, i64)]) -> IIRModule {
    let mut body = Vec::new();
    for (i, (a, b)) in pairs.iter().enumerate() {
        let (x, y, r) = (format!("a{i}"), format!("b{i}"), format!("r{i}"));
        body.push(int(&x, *a));
        body.push(int(&y, *b));
        body.push(op(name, &r, &[&x, &y], "i64"));
        body.push(print_i64(&r));
    }
    program(body)
}

/// Print `before`, then evaluate `name(a, b)` — which must trap — then print
/// its result (never reached).
fn floor_then_trap(name: &str, before: i64, a: i64, b: i64) -> IIRModule {
    program(vec![
        int("p", before),
        print_i64("p"),
        int("a", a),
        int("b", b),
        op(name, "r", &["a", "b"], "i64"),
        print_i64("r"),
    ])
}

/// `name(a, b)` (or `name(a)` for the unary `i64_neg_ovf`) for every pair, each
/// result printed with `print_i64`.
fn checked_table(name: &str, pairs: &[(i64, i64)]) -> IIRModule {
    let mut body = Vec::new();
    for (i, (a, b)) in pairs.iter().enumerate() {
        let (x, y, r) = (format!("a{i}"), format!("b{i}"), format!("r{i}"));
        body.push(int(&x, *a));
        body.push(int(&y, *b));
        let srcs: Vec<&str> = if name == "i64_neg_ovf" { vec![&x] } else { vec![&x, &y] };
        body.push(op(name, &r, &srcs, "i64"));
        body.push(print_i64(&r));
    }
    program(body)
}

/// Print `before`, then evaluate the checked `name(a, b)` (or `name(a)`) — which
/// must trap `Trap.Overflow` — then print its result (never reached).
fn checked_then_trap(name: &str, before: i64, a: i64, b: i64) -> IIRModule {
    let srcs: &[&str] = if name == "i64_neg_ovf" { &["a"] } else { &["a", "b"] };
    program(vec![
        int("p", before),
        print_i64("p"),
        int("a", a),
        int("b", b),
        op(name, "r", srcs, "i64"),
        print_i64("r"),
    ])
}


// --- LANG83 PR5a: strings fail closed ----------------------------------------
//
// Every string case runs in two forms (spec "PR5 — runtime strings",
// "Conformance cases"):
//
// | form    | where the operands come from                       | what it proves                 |
// |---------|----------------------------------------------------|--------------------------------|
// | literal | `str_const` / `const` in `main`                    | a backend that *folds* the op still traps (never a value, never a compile error) |
// | opaque  | parameters of a call-free function `f`, called by `main` | the run-time path traps; nothing can fold |
//
// The out-of-range values include the ones only a full 64-bit check sees:
// `2³² + 1` is 1 after a 32-bit narrowing, and `2³²` is 0. JVM and CLR
// narrowed first before PR5a, so `str_index "ABC", 2³² + 1` read `'B'`.
//
// Each trap case prints `B:` first (with `print_str`, no newline), so the
// expectation also proves the column flushed it before trapping. The cases
// compare bytes exactly (`Expect::StdoutBytes*`).

/// The string every PR5a bounds case reads.
const PR5A_SRC: &str = "ABC";

/// `f(params) -> i64 { body; ret <ret> }` plus `main` that sets up `args`,
/// prints `B:`, calls `f`, and writes `f`'s result. `f` makes no call, so the
/// JIT would compile it (it refuses strings until PR5d).
fn pr5a_opaque(params: &[(&str, &str)], args: Vec<IIRInstr>, body: Vec<IIRInstr>, ret: &str) -> IIRModule {
    let mut f_body = body;
    f_body.push(IIRInstr::new("ret", None, vec![var(ret)], "i64"));
    let mut main = args;
    main.push(text("pr5a.before", "B:"));
    main.push(print_str("pr5a.before"));
    let mut call = vec![var("f")];
    call.extend(params.iter().map(|(n, _)| var(&format!("arg.{n}"))));
    main.push(IIRInstr::new("call", Some("pr5a.r".into()), call, "i64"));
    main.push(builtin("write_i64", "pr5a.r"));
    let mut module = program(main);
    module.add_or_replace(IIRFunction::new(
        "f",
        params.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(),
        "i64",
        f_body,
    ));
    module
}

/// The same program with every operand a literal in `main`, so a backend may
/// fold the op.
fn pr5a_literal(args: Vec<IIRInstr>, body: Vec<IIRInstr>, ret: &str) -> IIRModule {
    let mut main = vec![text("pr5a.before", "B:"), print_str("pr5a.before")];
    main.extend(args);
    main.extend(body);
    main.push(builtin("write_i64", ret));
    program(main)
}

/// `str_index "ABC", I`, literal (`OPAQUE = false`) or opaque.
fn pr5a_index<const I: i64, const OPAQUE: bool>() -> IIRModule {
    let body = vec![op("str_index", "b", &["s", "i"], "i64")];
    if OPAQUE {
        pr5a_opaque(&[("s", "str"), ("i", "i64")], vec![text("arg.s", PR5A_SRC), int("arg.i", I)], body, "b")
    } else {
        pr5a_literal(vec![text("s", PR5A_SRC), int("i", I)], body, "b")
    }
}

/// `str_len (str_slice "ABC", A, B)`, literal or opaque.
fn pr5a_slice<const A: i64, const B: i64, const OPAQUE: bool>() -> IIRModule {
    let body = vec![op("str_slice", "t", &["s", "a", "b"], "str"), op("str_len", "n", &["t"], "i64")];
    if OPAQUE {
        pr5a_opaque(
            &[("s", "str"), ("a", "i64"), ("b", "i64")],
            vec![text("arg.s", PR5A_SRC), int("arg.a", A), int("arg.b", B)],
            body,
            "n",
        )
    } else {
        pr5a_literal(vec![text("s", PR5A_SRC), int("a", A), int("b", B)], body, "n")
    }
}

/// An out-of-range `str_index` whose result nobody reads: dead-code
/// elimination may not delete it, because its trap is observable. Prints `B:`
/// (and, if the read were wrongly removed, `ok`).
fn pr5a_unused_index<const OPAQUE: bool>() -> IIRModule {
    let body = vec![op("str_index", "unused", &["s", "i"], "i64"), int("zero", 0)];
    let mut m = if OPAQUE {
        pr5a_opaque(&[("s", "str"), ("i", "i64")], vec![text("arg.s", PR5A_SRC), int("arg.i", 7)], body, "zero")
    } else {
        pr5a_literal(vec![text("s", PR5A_SRC), int("i", 7)], body, "zero")
    };
    m.functions.iter_mut().find(|f| f.name == "main").unwrap().instructions.retain(|i| {
        !(i.op == "call_builtin" && i.srcs.get(1) == Some(&var(if OPAQUE { "pr5a.r" } else { "zero" })))
    });
    m
}

/// Under a module ceiling of 16: `d = s ++ s` (16 bytes, fits), write its
/// length, then `d ++ "x"` (17 bytes) must trap `Trap.Overflow`.
fn pr5a_ceiling<const OPAQUE: bool>() -> IIRModule {
    let body = vec![
        op("str_concat", "d", &["s", "s"], "str"),
        op("str_len", "n", &["d"], "i64"),
        builtin("write_i64", "n"),
        op("str_concat", "e", &["d", "x"], "str"),
        op("str_len", "m", &["e"], "i64"),
    ];
    let mut m = if OPAQUE {
        pr5a_opaque(
            &[("s", "str"), ("x", "str")],
            vec![text("arg.s", "01234567"), text("arg.x", "x")],
            body,
            "m",
        )
    } else {
        pr5a_literal(vec![text("s", "01234567"), text("x", "x")], body, "m")
    };
    m.str_max_bytes = Some(16);
    m
}

/// Aliasing: a string op whose destination is one of its own operands.
/// `s = s ++ s` ("abab"), then `s = str_slice s, 1, len s - 1` ("ba"), print.
fn pr5a_aliasing<const OPAQUE: bool>() -> IIRModule {
    let body = vec![
        op("str_concat", "s", &["s", "s"], "str"),
        op("str_len", "n", &["s"], "i64"),
        int("one", 1),
        op("sub", "j", &["n", "one"], "i64"),
        op("str_slice", "s", &["s", "one", "j"], "str"),
        print_str("s"),
        op("str_len", "k", &["s"], "i64"),
    ];
    if OPAQUE {
        pr5a_opaque(&[("s", "str")], vec![text("arg.s", "ab")], body, "k")
    } else {
        pr5a_literal(vec![text("s", "ab")], body, "k")
    }
}

/// The opaque form of the six E4 string-op cases: each op reads parameters
/// of `f`, and `f` returns the integer the literal case prints.
/// One opaque PR5a program: `f`'s parameters, `main`'s argument setup,
/// `f`'s body, and the register `f` returns.
type OpaqueParts = (&'static [(&'static str, &'static str)], Vec<IIRInstr>, Vec<IIRInstr>, &'static str);

fn pr5a_opaque_e4<const WHICH: u8>() -> IIRModule {
    let (params, args, body, ret): OpaqueParts = match WHICH {
        0 => (&[("s", "str")], vec![text("arg.s", "hello")], vec![op("str_len", "n", &["s"], "i64")], "n"),
        1 => (
            &[("s", "str"), ("i", "i64")],
            vec![text("arg.s", "HELLO"), int("arg.i", 1)],
            vec![op("str_index", "c", &["s", "i"], "i64")],
            "c",
        ),
        2 => (
            &[("s", "str"), ("a", "i64"), ("b", "i64")],
            vec![text("arg.s", "HELLO"), int("arg.a", 1), int("arg.b", 3)],
            vec![op("str_slice", "t", &["s", "a", "b"], "str"), print_str("t"), op("str_len", "n", &["t"], "i64")],
            "n",
        ),
        3 => (
            &[("a", "str"), ("b", "str")],
            vec![text("arg.a", "AB"), text("arg.b", "CD")],
            vec![op("str_concat", "t", &["a", "b"], "str"), print_str("t"), op("str_len", "n", &["t"], "i64")],
            "n",
        ),
        4 => (
            &[("a", "str"), ("b", "str")],
            vec![text("arg.a", "xy"), text("arg.b", "xy")],
            vec![op("str_eq", "r", &["a", "b"], "i64")],
            "r",
        ),
        _ => (
            &[("a", "str"), ("b", "str")],
            vec![text("arg.a", "a"), text("arg.b", "b")],
            vec![op("str_cmp", "r", &["a", "b"], "i64")],
            "r",
        ),
    };
    pr5a_opaque(params, args, body, ret)
}

/// The PR5a cases: every bounds edge in both forms, the unused read, the
/// ceiling, aliasing, and the opaque form of each E4 string case. Each name
/// ends in `(PR5a)`, which is how the gate finds them.
fn pr5a_cases() -> Vec<Case> {
    const BOUNDS: &str = "Trap.Bounds";
    const OVERFLOW: &str = "Trap.Overflow";
    const TWO_32: i64 = 1 << 32;
    vec![
        Case { name: "str_index at -1 traps Trap.Bounds, literal (PR5a)", build: pr5a_index::<{ -1 }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at -1 traps Trap.Bounds, opaque (PR5a)", build: pr5a_index::<{ -1 }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at len traps Trap.Bounds, literal (PR5a)", build: pr5a_index::<3, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at len traps Trap.Bounds, opaque (PR5a)", build: pr5a_index::<3, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at i64::MAX traps Trap.Bounds, literal (PR5a)", build: pr5a_index::<{ i64::MAX }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at i64::MAX traps Trap.Bounds, opaque (PR5a)", build: pr5a_index::<{ i64::MAX }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at i64::MIN traps Trap.Bounds, literal (PR5a)", build: pr5a_index::<{ i64::MIN }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at i64::MIN traps Trap.Bounds, opaque (PR5a)", build: pr5a_index::<{ i64::MIN }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at 2^32 + 1 traps Trap.Bounds, literal (PR5a)", build: pr5a_index::<{ TWO_32 + 1 }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at 2^32 + 1 traps Trap.Bounds, opaque (PR5a)", build: pr5a_index::<{ TWO_32 + 1 }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_index at 0 and len - 1 reads, opaque (PR5a)", build: pr5a_index::<2, true>, expect: Expect::StdoutBytes(b"B:67") },
        Case { name: "str_slice with a < 0 traps Trap.Bounds, literal (PR5a)", build: pr5a_slice::<{ -1 }, 1, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with a < 0 traps Trap.Bounds, opaque (PR5a)", build: pr5a_slice::<{ -1 }, 1, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b < a traps Trap.Bounds, literal (PR5a)", build: pr5a_slice::<2, 1, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b < a traps Trap.Bounds, opaque (PR5a)", build: pr5a_slice::<2, 1, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b > len traps Trap.Bounds, literal (PR5a)", build: pr5a_slice::<0, 4, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b > len traps Trap.Bounds, opaque (PR5a)", build: pr5a_slice::<0, 4, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with a = 2^32 traps Trap.Bounds, literal (PR5a)", build: pr5a_slice::<TWO_32, { TWO_32 + 3 }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with a = 2^32 traps Trap.Bounds, opaque (PR5a)", build: pr5a_slice::<TWO_32, { TWO_32 + 3 }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b = 2^32 + len traps Trap.Bounds, literal (PR5a)", build: pr5a_slice::<1, { TWO_32 + 3 }, false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice with b = 2^32 + len traps Trap.Bounds, opaque (PR5a)", build: pr5a_slice::<1, { TWO_32 + 3 }, true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_slice of [0, len) and [len, len) fit, opaque (PR5a)", build: pr5a_slice::<3, 3, true>, expect: Expect::StdoutBytes(b"B:0") },
        Case { name: "an unused out-of-range str_index still traps, literal (PR5a)", build: pr5a_unused_index::<false>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "an unused out-of-range str_index still traps, opaque (PR5a)", build: pr5a_unused_index::<true>, expect: Expect::StdoutBytesThenTrap(b"B:", BOUNDS) },
        Case { name: "str_concat to 16 fits and to 17 traps Trap.Overflow, literal (PR5a)", build: pr5a_ceiling::<false>, expect: Expect::StdoutBytesThenTrap(b"B:16", OVERFLOW) },
        Case { name: "str_concat to 16 fits and to 17 traps Trap.Overflow, opaque (PR5a)", build: pr5a_ceiling::<true>, expect: Expect::StdoutBytesThenTrap(b"B:16", OVERFLOW) },
        Case { name: "a string op may overwrite its own operand, literal (PR5a)", build: pr5a_aliasing::<false>, expect: Expect::StdoutBytes(b"B:ba2") },
        Case { name: "a string op may overwrite its own operand, opaque (PR5a)", build: pr5a_aliasing::<true>, expect: Expect::StdoutBytes(b"B:ba2") },
        Case { name: "str_len counts bytes, opaque (PR5a)", build: pr5a_opaque_e4::<0>, expect: Expect::StdoutBytes(b"B:5") },
        Case { name: "str_index reads one byte, opaque (PR5a)", build: pr5a_opaque_e4::<1>, expect: Expect::StdoutBytes(b"B:69") },
        Case { name: "str_slice takes [start, end), opaque (PR5a)", build: pr5a_opaque_e4::<2>, expect: Expect::StdoutBytes(b"B:EL2") },
        Case { name: "str_concat joins, opaque (PR5a)", build: pr5a_opaque_e4::<3>, expect: Expect::StdoutBytes(b"B:ABCD4") },
        Case { name: "str_eq compares bytes, opaque (PR5a)", build: pr5a_opaque_e4::<4>, expect: Expect::StdoutBytes(b"B:1") },
        Case { name: "str_cmp orders lexically, opaque (PR5a)", build: pr5a_opaque_e4::<5>, expect: Expect::StdoutBytes(b"B:-1") },
    ]
}

// ---------------------------------------------------------------------------
// LANG83 PR5b — strings are bytes, not text
// ---------------------------------------------------------------------------
//
// Every case below writes bytes outside printable ASCII and compares them
// exactly (`Expect::StdoutBytes`). Each runs in two forms:
//
// | form    | where the strings are                       | what it proves                  |
// |---------|---------------------------------------------|---------------------------------|
// | literal | `str_const`s in `main`, foldable            | the literal encoding and folds  |
// | opaque  | parameters of a call-free `f` (`main` calls it) | the run-time representation |
//
// Before PR5b every non-literal-ASCII case here was a compile-time refusal on
// native, LLVM, WASM, JVM, CLR and BEAM; the VM refused the lone-byte slice;
// and the JVM and CLR wrote a char ≥ 0x80 as two UTF-8 bytes.

/// The UTF-8 literal of the conformance table: 14 bytes, 1 + 2 + 3 + 4-byte
/// characters (`é` = C3 A9, `€` = E2 82 AC, `𝄞` = F0 9D 84 9E).
const PR5B_UTF8: &str = "h\u{e9}llo \u{20ac}\u{1d11e}";

fn sep() -> IIRInstr {
    text("pr5b.sep", ",")
}
fn print_sep() -> IIRInstr {
    print_str("pr5b.sep")
}
fn putchar_const(d: &str, c: i64) -> Vec<IIRInstr> {
    vec![int(d, c), builtin("putchar", d)]
}

/// One PR5b program. `strings` are `(register, literal)`; `body` reads them.
/// Literal form: everything in `main`. Opaque form: `body` is `f(strings…)`,
/// and `main` sets up the literals and calls `f`.
fn pr5b_program(strings: Vec<(&str, Operand)>, body: Vec<IIRInstr>, opaque: bool) -> IIRModule {
    let lit = |d: &str, o: &Operand| IIRInstr::new("str_const", Some(d.into()), vec![o.clone()], "str");
    if !opaque {
        let mut main: Vec<IIRInstr> = strings.iter().map(|(d, o)| lit(d, o)).collect();
        main.extend(body);
        return program(main);
    }
    let mut f_body = body;
    f_body.push(int("pr5b.zero", 0));
    f_body.push(IIRInstr::new("ret", None, vec![var("pr5b.zero")], "i64"));
    let mut main: Vec<IIRInstr> = strings.iter().map(|(d, o)| lit(&format!("arg.{d}"), o)).collect();
    let mut call = vec![var("f")];
    call.extend(strings.iter().map(|(d, _)| var(&format!("arg.{d}"))));
    main.push(IIRInstr::new("call", Some("pr5b.r".into()), call, "i64"));
    let mut module = program(main);
    module.add_or_replace(IIRFunction::new(
        "f",
        strings.iter().map(|(d, _)| (d.to_string(), "str".to_string())).collect(),
        "i64",
        f_body,
    ));
    module
}

/// `str_len s`, then `str_index s` at 0, 1, 2 and 13, each written and
/// followed by `,`; then `print_str s`.
fn pr5b_stats_of_s() -> Vec<IIRInstr> {
    let mut body = vec![sep(), op("str_len", "n", &["s"], "i64"), builtin("write_i64", "n"), print_sep()];
    for i in [0, 1, 2, 13] {
        let (iv, bv) = (format!("i{i}"), format!("b{i}"));
        body.push(int(&iv, i));
        body.push(op("str_index", &bv, &["s", &iv], "i64"));
        body.push(builtin("write_i64", &bv));
        body.push(print_sep());
    }
    body.push(print_str("s"));
    body
}

/// The UTF-8 literal: its byte count, four of its bytes, then itself.
fn pr5b_utf8<const OPAQUE: bool>() -> IIRModule {
    pr5b_program(vec![("s", Operand::Str(PR5B_UTF8.into()))], pr5b_stats_of_s(), OPAQUE)
}

/// The same bytes as a raw-byte literal: `str_eq` with the `Str` spelling is
/// 1, and the stats are identical.
fn pr5b_raw_utf8<const OPAQUE: bool>() -> IIRModule {
    let mut body = vec![op("str_eq", "same", &["s", "t"], "i64"), builtin("write_i64", "same")];
    body.extend(pr5b_stats_of_s());
    pr5b_program(
        vec![("s", Operand::Bytes(PR5B_UTF8.as_bytes().to_vec())), ("t", Operand::Str(PR5B_UTF8.into()))],
        body,
        OPAQUE,
    )
}

/// A raw-byte literal no `Str` can carry: NUL, LF, CR, DEL and three bytes
/// that are not UTF-8 on their own; printed, then its length.
fn pr5b_raw_bytes<const OPAQUE: bool>() -> IIRModule {
    pr5b_program(
        vec![("s", Operand::Bytes(vec![0x00, 0x0A, 0x0D, 0x7F, 0x80, 0xA9, 0xFF]))],
        vec![print_str("s"), op("str_len", "n", &["s"], "i64"), builtin("write_i64", "n")],
        OPAQUE,
    )
}

/// `str_slice "é", 1, 2`: the lone byte A9, printed.
fn pr5b_lone_byte<const OPAQUE: bool>() -> IIRModule {
    pr5b_program(
        vec![("s", Operand::Str("\u{e9}".into()))],
        vec![int("a", 1), int("b", 2), op("str_slice", "t", &["s", "a", "b"], "str"), print_str("t")],
        OPAQUE,
    )
}

/// Every byte 0x80–0xFF in a raw-byte literal through `print_str`, then
/// `putchar 0xFF` (the CLR wrote it as C3 BF).
fn pr5b_byte_table<const OPAQUE: bool>() -> IIRModule {
    let mut body = vec![print_str("s")];
    body.extend(putchar_const("ff", 0xFF));
    pr5b_program(vec![("s", Operand::Bytes((0x80..=0xFF).collect()))], body, OPAQUE)
}

/// Bytes 0x80..=0xFF, then 0xFF again.
const PR5B_BYTE_TABLE_OUT: [u8; 129] = {
    let mut out = [0xFFu8; 129];
    let mut i = 0;
    while i < 128 {
        out[i] = 0x80 + i as u8;
        i += 1;
    }
    out
};

/// Edges on a two-byte string `s = "é"` and `e = ""`: `str_len ""`; `s ++ ""`
/// equals `s`; slices `[0,0)`, `[len,len)` and `[0,len)`; and `str_cmp` of
/// the byte 7F with `s` is -1 (unsigned: 0x7F < 0xC3).
fn pr5b_edges<const OPAQUE: bool>() -> IIRModule {
    let w = |d: &str| vec![builtin("write_i64", d), print_sep()];
    let mut body = vec![sep(), op("str_len", "le", &["e"], "i64")];
    body.extend(w("le"));
    body.extend([op("str_concat", "se", &["s", "e"], "str"), op("str_eq", "q1", &["se", "s"], "i64")]);
    body.extend(w("q1"));
    body.extend([int("z", 0), int("two", 2)]);
    body.extend([op("str_slice", "s00", &["s", "z", "z"], "str"), op("str_len", "l00", &["s00"], "i64")]);
    body.extend(w("l00"));
    body.extend([op("str_slice", "s22", &["s", "two", "two"], "str"), op("str_len", "l22", &["s22"], "i64")]);
    body.extend(w("l22"));
    body.extend([op("str_slice", "s02", &["s", "z", "two"], "str"), op("str_eq", "q2", &["s02", "s"], "i64")]);
    body.extend(w("q2"));
    body.extend([op("str_cmp", "c", &["d", "s"], "i64"), builtin("write_i64", "c")]);
    pr5b_program(
        vec![("s", Operand::Str("\u{e9}".into())), ("e", Operand::Str(String::new())), ("d", Operand::Bytes(vec![0x7F]))],
        body,
        OPAQUE,
    )
}

/// The ordered stream with bytes ≥ 0x80 on every writer: `print_str`,
/// `write_i64`, `putchar`, `print_str`, with no flush between them.
fn pr5b_ordered<const OPAQUE: bool>() -> IIRModule {
    let mut body = vec![print_str("s"), int("one", 1), builtin("write_i64", "one")];
    body.extend(putchar_const("ff", 0xFF));
    body.push(print_str("t"));
    pr5b_program(vec![("s", Operand::Str("\u{e9}".into())), ("t", Operand::Bytes(vec![0xA9]))], body, OPAQUE)
}

/// What [`pr5b_utf8`] writes after its stats prefix.
const PR5B_UTF8_OUT: &[u8] = b"14,104,195,169,158,h\xc3\xa9llo \xe2\x82\xac\xf0\x9d\x84\x9e";
const PR5B_RAW_UTF8_OUT: &[u8] = b"114,104,195,169,158,h\xc3\xa9llo \xe2\x82\xac\xf0\x9d\x84\x9e";

/// The PR5b cases. Each name ends in `(PR5b)`, which is how the gate finds them.
fn pr5b_cases() -> Vec<Case> {
    vec![
        Case { name: "a UTF-8 literal is its bytes, literal (PR5b)", build: pr5b_utf8::<false>, expect: Expect::StdoutBytes(PR5B_UTF8_OUT) },
        Case { name: "a UTF-8 literal is its bytes, opaque (PR5b)", build: pr5b_utf8::<true>, expect: Expect::StdoutBytes(PR5B_UTF8_OUT) },
        Case { name: "a raw-byte literal of the same bytes is the same string, literal (PR5b)", build: pr5b_raw_utf8::<false>, expect: Expect::StdoutBytes(PR5B_RAW_UTF8_OUT) },
        Case { name: "a raw-byte literal of the same bytes is the same string, opaque (PR5b)", build: pr5b_raw_utf8::<true>, expect: Expect::StdoutBytes(PR5B_RAW_UTF8_OUT) },
        Case { name: "a raw-byte literal holds any byte, literal (PR5b)", build: pr5b_raw_bytes::<false>, expect: Expect::StdoutBytes(b"\x00\n\r\x7f\x80\xa9\xff7") },
        Case { name: "a raw-byte literal holds any byte, opaque (PR5b)", build: pr5b_raw_bytes::<true>, expect: Expect::StdoutBytes(b"\x00\n\r\x7f\x80\xa9\xff7") },
        Case { name: "str_slice may split a UTF-8 character, literal (PR5b)", build: pr5b_lone_byte::<false>, expect: Expect::StdoutBytes(b"\xa9") },
        Case { name: "str_slice may split a UTF-8 character, opaque (PR5b)", build: pr5b_lone_byte::<true>, expect: Expect::StdoutBytes(b"\xa9") },
        Case { name: "every byte 0x80-0xFF prints as itself, and so does putchar 0xFF, literal (PR5b)", build: pr5b_byte_table::<false>, expect: Expect::StdoutBytes(&PR5B_BYTE_TABLE_OUT) },
        Case { name: "every byte 0x80-0xFF prints as itself, and so does putchar 0xFF, opaque (PR5b)", build: pr5b_byte_table::<true>, expect: Expect::StdoutBytes(&PR5B_BYTE_TABLE_OUT) },
        Case { name: "string edges and unsigned byte order, literal (PR5b)", build: pr5b_edges::<false>, expect: Expect::StdoutBytes(b"0,1,0,0,1,-1") },
        Case { name: "string edges and unsigned byte order, opaque (PR5b)", build: pr5b_edges::<true>, expect: Expect::StdoutBytes(b"0,1,0,0,1,-1") },
        Case { name: "print_str, write_i64 and putchar share one ordered byte stream, literal (PR5b)", build: pr5b_ordered::<false>, expect: Expect::StdoutBytes(b"\xc3\xa91\xff\xa9") },
        Case { name: "print_str, write_i64 and putchar share one ordered byte stream, opaque (PR5b)", build: pr5b_ordered::<true>, expect: Expect::StdoutBytes(b"\xc3\xa91\xff\xa9") },
    ]
}

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;

/// 2⁶³: the first double a `real_to_int_*` narrowing cannot hold.
const TWO_63: f64 = 9_223_372_036_854_775_808.0;

/// Print `1`, then narrow `x` with `name` — which must trap `Trap.ConvRange`
/// — then print the result (never reached).
fn narrow_then_trap(name: &str, x: f64) -> IIRModule {
    program(vec![int("p", 1), print_i64("p"), real("x", x), op(name, "n", &["x"], "i64"), print_i64("n")])
}

// --- LANG83 PR6a: the float-output table -------------------------------------
//
// The layout edges of the spec's "PR6a — Conformance cases" plus 60 seeded
// random doubles. `float_table_matches_host_python3_and_node` re-checks every
// row against the host oracles, so the strings below cannot drift from what
// Python and JavaScript actually print.
// (bits, python3 repr, node String) — generated by host python3 3.13.16 and node v22.22.0.
const FLOAT_TABLE: &[(u64, &str, &str)] = &[
    (0x0000000000000000, "0.0", "0"),
    (0x8000000000000000, "-0.0", "0"),
    (0x7FF0000000000000, "inf", "Infinity"),
    (0xFFF0000000000000, "-inf", "-Infinity"),
    (0x7FF8000000000000, "nan", "NaN"),
    (0x3FF0000000000000, "1.0", "1"),
    (0x4059000000000000, "100.0", "100"),
    (0x3FB999999999999A, "0.1", "0.1"),
    (0x430C6BF526340000, "1000000000000000.0", "1000000000000000"),
    (0x4341C37937E08000, "1e+16", "10000000000000000"),
    (0x4415AF1D78B58C40, "1e+20", "100000000000000000000"),
    (0x444B1AE4D6E2EF50, "1e+21", "1e+21"),
    (0x3F1A36E2EB1C432D, "0.0001", "0.0001"),
    (0x3EE4F8B588E368F1, "1e-05", "0.00001"),
    (0x3EB0C6F7A0B5ED8D, "1e-06", "0.000001"),
    (0x3E7AD7F29ABCAF48, "1e-07", "1e-7"),
    (0x3E8421F5F40D8376, "1.5e-07", "1.5e-7"),
    (0x405EDD2F1A9FBE77, "123.456", "123.456"),
    (0xC004000000000000, "-2.5", "-2.5"),
    (0x0000000000000001, "5e-324", "5e-324"),
    (0x0010000000000000, "2.2250738585072014e-308", "2.2250738585072014e-308"),
    (0x7FEFFFFFFFFFFFFF, "1.7976931348623157e+308", "1.7976931348623157e+308"),
    (0x3E60000000000000, "2.9802322387695312e-08", "2.9802322387695312e-8"),
    (0x4340000000000001, "9007199254740994.0", "9007199254740994"),
    (0x44C52D02C7E14AF6, "2e+23", "2e+23"),
    (0x3FD3333333333334, "0.30000000000000004", "0.30000000000000004"),
    (0x430E77EA4376F602, "1072012167405248.2", "1072012167405248.2"),
    (0x437B69B4BA630F35, "1.2345678901234568e+17", "123456789012345680"),
    (0x81A56E1FC2F8F359, "-1e-300", "-1e-300"),
    (0x4011666666666666, "4.35", "4.35"),
    (0xD802DB897F041728, "-9.287824536391492e+115", "-9.287824536391492e+115"),
    (0x75BD5125DB54E35F, "1.4086255999159193e+259", "1.4086255999159193e+259"),
    (0x20CBF68A16B4AA27, "1.0678165209952348e-150", "1.0678165209952348e-150"),
    (0x08CE66D960E04456, "2.9464053386017267e-266", "2.9464053386017267e-266"),
    (0xF3C0A0E50C95D61B, "-3.7204833670429256e+249", "-3.7204833670429256e+249"),
    (0x36539DC5A658D453, "5.368806067798381e-47", "5.368806067798381e-47"),
    (0x2C80A15F306E7B43, "2.491454630318929e-94", "2.491454630318929e-94"),
    (0x152D01E64F17B9BA, "1.1293852148835315e-206", "1.1293852148835315e-206"),
    (0xFA231E76D1930214, "-2.1690580031879166e+280", "-2.1690580031879166e+280"),
    (0x691ED3920DFB649F, "2.3043086018535117e+198", "2.3043086018535117e+198"),
    (0x5A54D551274E53BE, "1.4102484161097585e+127", "1.4102484161097585e+127"),
    (0x8F8D17F44D22AC04, "-9.15019639745717e-234", "-9.15019639745717e-234"),
    (0x33C1B3F1CFC3ED46, "2.2033094014034217e-59", "2.2033094014034217e-59"),
    (0xC93BF9A679B68A27, "-6.238677282146316e+44", "-6.238677282146316e+44"),
    (0xB50A7691A04B8761, "-3.4536034687305475e-53", "-3.4536034687305475e-53"),
    (0xED9EDE68E6A24F73, "-1.0896736081856203e+220", "-1.0896736081856203e+220"),
    (0xD24149293AD63D44, "-1.7193245445503562e+88", "-1.7193245445503562e+88"),
    (0xF7242ACA7222B702, "-8.128506383860937e+265", "-8.128506383860937e+265"),
    (0x0B5BBCCE2AB0CC72, "5.911413105048698e-254", "5.911413105048698e-254"),
    (0x07CCF96BC815D3AE, "4.2847637478370544e-271", "4.2847637478370544e-271"),
    (0xF7AC0685051557DA, "-2.8917384894280522e+268", "-2.8917384894280522e+268"),
    (0x777125B3C1C4578B, "2.2116238141801078e+267", "2.2116238141801078e+267"),
    (0x62F46C925963D9C7, "4.817488535775391e+168", "4.817488535775391e+168"),
    (0xB93B967192FADFF5, "-5.313192071926005e-33", "-5.313192071926005e-33"),
    (0x8CE38FEE2FB932C4, "-1.3989167825577705e-246", "-1.3989167825577705e-246"),
    (0xDF92355AEE87C2E6, "-2.3841306779525437e+152", "-2.3841306779525437e+152"),
    (0x7A2CEC265132D40E, "3.2812642341384877e+280", "3.2812642341384877e+280"),
    (0xCBE782C1746553A4, "-4.6118581223499435e+57", "-4.6118581223499435e+57"),
    (0xDEF0DF69F03C6967, "-2.1574598139070555e+149", "-2.1574598139070555e+149"),
    (0xD8F1B39298A733C6, "-2.856848852987085e+120", "-2.856848852987085e+120"),
    (0x70D0CDBC524A32C7, "2.67140895794731e+235", "2.67140895794731e+235"),
    (0x22D05E9A3AE8FD30, "5.369557981821192e-141", "5.369557981821192e-141"),
    (0xC0826248E92243B8, "-588.2856009175221", "-588.2856009175221"),
    (0x684BDBB4B7B2484B, "2.542030062404305e+194", "2.542030062404305e+194"),
    (0x82AA9609A0ED9BF6, "-8.1303389302721e-296", "-8.1303389302721e-296"),
    (0x2F977CC78DF3596C, "1.9808722431475594e-79", "1.9808722431475594e-79"),
    (0x91B2D5E5EBF7C6BC, "-2.03545399911944e-223", "-2.03545399911944e-223"),
    (0x033F737120A4CDF7, "4.924452245226261e-293", "4.924452245226261e-293"),
    (0x713A83505E7F5BD2, "2.6975814893955876e+237", "2.6975814893955876e+237"),
    (0x304A2209D00A3D4E, "4.5137733331146937e-76", "4.5137733331146937e-76"),
    (0x40B6EACCCCCCCCCD, "5866.8", "5866.8"),
    (0xC072B4EB1C432CA5, "-299.3074", "-299.3074"),
    (0xC0C217C666666666, "-9263.55", "-9263.55"),
    (0x40A970BE157D3FC7, "3256.3712577", "3256.3712577"),
    (0x40C22F2D7F62B6AE, "9310.35545", "9310.35545"),
    (0xC0AC380000000000, "-3612.0", "-3612"),
    (0xC0B1C209BA5E353F, "-4546.038", "-4546.038"),
    (0x40B1F47F2495E17E, "4596.496652", "4596.496652"),
    (0x40C009BA1A8CDEA0, "8211.45393525", "8211.45393525"),
    (0x40AC7211EB851EB8, "3641.035", "3641.035"),
    (0x4093D172474538EF, "1268.3616", "1268.3616"),
    (0x40A9740000000000, "3258.0", "3258"),
    (0x40BAE5A5810624DD, "6885.6465", "6885.6465"),
    (0x40C3265CF12C27A6, "9804.72611", "9804.72611"),
    (0x40B775A786C22681, "6005.6544", "6005.6544"),
    (0xC0911F3333333333, "-1095.8", "-1095.8"),
    (0x40B3F76083126E98, "5111.377", "5111.377"),
    (0xC0BF710000000000, "-8049.0", "-8049"),
    (0x409BAD95810624DD, "1771.396", "1771.396"),
    (0x40B99E157689CA19, "6558.08384", "6558.08384"),
];

// --- LANG83 PR7e: the PR6a table, printed by the compilers' own helpers --------
//
// PR6a printed each table value with the `write_f64_py` / `write_f64_js`
// builtins, which every backend lowered. LANG83 PR7e removed them: since
// PR7c-js / PR7c-py each compiler lays a float out itself, in module-local
// helpers built from `f64_class` / `f64_digits` / `f64_exp10`, integer ops and
// `putchar`. So the table now runs through those helpers, taken verbatim from
// the module each compiler produces, and the PR6a strings stay the oracle:
// every value must print byte for byte what host `python3` / `node` print, on
// every column.

/// A float layout: the compiler that emits it, a one-print program in that
/// language, the helpers' common prefix and the entry helper's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// python-iir-compiler's `__py_write_float` (`repr(float)`).
    Python,
    /// javascript-iir-compiler's `__js_write_number` (`Number::toString`).
    JavaScript,
}

impl Layout {
    fn entry(self) -> &'static str {
        match self {
            Layout::Python => "__py_write_float",
            Layout::JavaScript => "__js_write_number",
        }
    }

    /// The helper functions the compiler emits into a module that prints a
    /// float: compiled once from a one-print program and taken by prefix, so
    /// they are exactly what every float-printing module of that language
    /// carries.
    fn helpers(self) -> Vec<IIRFunction> {
        static PY: std::sync::OnceLock<Vec<IIRFunction>> = std::sync::OnceLock::new();
        static JS: std::sync::OnceLock<Vec<IIRFunction>> = std::sync::OnceLock::new();
        let (cell, language, source, prefix) = match self {
            Layout::Python => (&PY, Language::Python, "print(1.5)\n", "__py_"),
            Layout::JavaScript => (&JS, Language::JavaScript, "console.log(1.5);\n", "__js_"),
        };
        cell.get_or_init(|| {
            let module = lang_aot::compile_source_to_iir(language, source, "layout")
                .unwrap_or_else(|e| panic!("{language}: the one-print program must compile: {e}"));
            let helpers: Vec<IIRFunction> =
                module.functions.into_iter().filter(|f| f.name.starts_with(prefix)).collect();
            assert_eq!(helpers.len(), 4, "{language}: the layout, digit count, digit writer and uint writer");
            assert!(helpers.iter().any(|f| f.name == self.entry()), "{language}: {} is emitted", self.entry());
            helpers
        })
        .clone()
    }
}

/// `program(body)` plus the helpers of every layout in `layouts`.
fn program_with(body: Vec<IIRInstr>, layouts: &[Layout]) -> IIRModule {
    let mut module = program(body);
    for layout in layouts {
        for helper in layout.helpers() {
            module.add_or_replace(helper);
        }
    }
    module
}

/// `call <layout>(x)`, the way the compilers print a float. The unused `0`
/// result goes to one register, `printed`, as python-iir-compiler's does.
fn print_float(layout: Layout, x: &str) -> IIRInstr {
    IIRInstr::new("call", Some("printed".into()), vec![var(layout.entry()), var(x)], "i64")
}

/// `layout(x); putchar('\n')` for every value of [`FLOAT_TABLE`] that `keep`
/// accepts.
fn float_table_program(layout: Layout, keep: fn(f64) -> bool) -> IIRModule {
    let mut body = vec![int("nl", 10)];
    for (i, (bits, _, _)) in FLOAT_TABLE.iter().enumerate() {
        let x = f64::from_bits(*bits);
        if !keep(x) {
            continue;
        }
        let name = format!("x{i}");
        body.push(real(&name, x));
        body.push(print_float(layout, &name));
        body.push(builtin("putchar", "nl"));
    }
    program_with(body, &[layout])
}

/// The expected (trimmed) stdout of [`float_table_program`]: the PR6a
/// table's Python or JavaScript column, one line per kept row.
fn float_table_expect(layout: Layout, finite_rows: bool) -> &'static str {
    static CELLS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        let column = |js: bool, keep: fn(f64) -> bool| {
            FLOAT_TABLE
                .iter()
                .filter(|(bits, _, _)| keep(f64::from_bits(*bits)))
                .map(|(_, py, js_text)| if js { *js_text } else { *py })
                .collect::<Vec<&str>>()
                .join("\n")
        };
        vec![column(false, finite), column(false, non_finite), column(true, finite), column(true, non_finite)]
    });
    &cells[usize::from(layout == Layout::JavaScript) * 2 + usize::from(!finite_rows)]
}

// --- LANG83 PR7b: the digits on their own -----------------------------------
//
// `f64_class`, `f64_digits` and `f64_exp10` of every PR6a table value, as
// host `python3` gives them: its `repr`, split at the digits.
// `float_digits_table_matches_host_python3` re-derives every row whenever
// `python3` is installed, and `float_digits_table_follows_the_float_table`
// pins the rows to `FLOAT_TABLE`'s values and Python strings.
// (bits, class, digits, exp10) — generated by host python3 3.13.16 from `repr`,
// split at the digits (the PR6a table's values, in the same order).
const FLOAT_DIGITS_TABLE: &[(u64, i64, i64, i64)] = &[
    (0x0000000000000000, 0, 0, 0),
    (0x8000000000000000, 0, 0, 0),
    (0x7FF0000000000000, 2, 0, 0),
    (0xFFF0000000000000, 2, 0, 0),
    (0x7FF8000000000000, 3, 0, 0),
    (0x3FF0000000000000, 1, 1, 0),
    (0x4059000000000000, 1, 1, 2),
    (0x3FB999999999999A, 1, 1, -1),
    (0x430C6BF526340000, 1, 1, 15),
    (0x4341C37937E08000, 1, 1, 16),
    (0x4415AF1D78B58C40, 1, 1, 20),
    (0x444B1AE4D6E2EF50, 1, 1, 21),
    (0x3F1A36E2EB1C432D, 1, 1, -4),
    (0x3EE4F8B588E368F1, 1, 1, -5),
    (0x3EB0C6F7A0B5ED8D, 1, 1, -6),
    (0x3E7AD7F29ABCAF48, 1, 1, -7),
    (0x3E8421F5F40D8376, 1, 15, -7),
    (0x405EDD2F1A9FBE77, 1, 123456, 2),
    (0xC004000000000000, 1, 25, 0),
    (0x0000000000000001, 1, 5, -324),
    (0x0010000000000000, 1, 22250738585072014, -308),
    (0x7FEFFFFFFFFFFFFF, 1, 17976931348623157, 308),
    (0x3E60000000000000, 1, 29802322387695312, -8),
    (0x4340000000000001, 1, 9007199254740994, 15),
    (0x44C52D02C7E14AF6, 1, 2, 23),
    (0x3FD3333333333334, 1, 30000000000000004, -1),
    (0x430E77EA4376F602, 1, 10720121674052482, 15),
    (0x437B69B4BA630F35, 1, 12345678901234568, 17),
    (0x81A56E1FC2F8F359, 1, 1, -300),
    (0x4011666666666666, 1, 435, 0),
    (0xD802DB897F041728, 1, 9287824536391492, 115),
    (0x75BD5125DB54E35F, 1, 14086255999159193, 259),
    (0x20CBF68A16B4AA27, 1, 10678165209952348, -150),
    (0x08CE66D960E04456, 1, 29464053386017267, -266),
    (0xF3C0A0E50C95D61B, 1, 37204833670429256, 249),
    (0x36539DC5A658D453, 1, 5368806067798381, -47),
    (0x2C80A15F306E7B43, 1, 2491454630318929, -94),
    (0x152D01E64F17B9BA, 1, 11293852148835315, -206),
    (0xFA231E76D1930214, 1, 21690580031879166, 280),
    (0x691ED3920DFB649F, 1, 23043086018535117, 198),
    (0x5A54D551274E53BE, 1, 14102484161097585, 127),
    (0x8F8D17F44D22AC04, 1, 915019639745717, -234),
    (0x33C1B3F1CFC3ED46, 1, 22033094014034217, -59),
    (0xC93BF9A679B68A27, 1, 6238677282146316, 44),
    (0xB50A7691A04B8761, 1, 34536034687305475, -53),
    (0xED9EDE68E6A24F73, 1, 10896736081856203, 220),
    (0xD24149293AD63D44, 1, 17193245445503562, 88),
    (0xF7242ACA7222B702, 1, 8128506383860937, 265),
    (0x0B5BBCCE2AB0CC72, 1, 5911413105048698, -254),
    (0x07CCF96BC815D3AE, 1, 42847637478370544, -271),
    (0xF7AC0685051557DA, 1, 28917384894280522, 268),
    (0x777125B3C1C4578B, 1, 22116238141801078, 267),
    (0x62F46C925963D9C7, 1, 4817488535775391, 168),
    (0xB93B967192FADFF5, 1, 5313192071926005, -33),
    (0x8CE38FEE2FB932C4, 1, 13989167825577705, -246),
    (0xDF92355AEE87C2E6, 1, 23841306779525437, 152),
    (0x7A2CEC265132D40E, 1, 32812642341384877, 280),
    (0xCBE782C1746553A4, 1, 46118581223499435, 57),
    (0xDEF0DF69F03C6967, 1, 21574598139070555, 149),
    (0xD8F1B39298A733C6, 1, 2856848852987085, 120),
    (0x70D0CDBC524A32C7, 1, 267140895794731, 235),
    (0x22D05E9A3AE8FD30, 1, 5369557981821192, -141),
    (0xC0826248E92243B8, 1, 5882856009175221, 2),
    (0x684BDBB4B7B2484B, 1, 2542030062404305, 194),
    (0x82AA9609A0ED9BF6, 1, 81303389302721, -296),
    (0x2F977CC78DF3596C, 1, 19808722431475594, -79),
    (0x91B2D5E5EBF7C6BC, 1, 203545399911944, -223),
    (0x033F737120A4CDF7, 1, 4924452245226261, -293),
    (0x713A83505E7F5BD2, 1, 26975814893955876, 237),
    (0x304A2209D00A3D4E, 1, 45137733331146937, -76),
    (0x40B6EACCCCCCCCCD, 1, 58668, 3),
    (0xC072B4EB1C432CA5, 1, 2993074, 2),
    (0xC0C217C666666666, 1, 926355, 3),
    (0x40A970BE157D3FC7, 1, 32563712577, 3),
    (0x40C22F2D7F62B6AE, 1, 931035545, 3),
    (0xC0AC380000000000, 1, 3612, 3),
    (0xC0B1C209BA5E353F, 1, 4546038, 3),
    (0x40B1F47F2495E17E, 1, 4596496652, 3),
    (0x40C009BA1A8CDEA0, 1, 821145393525, 3),
    (0x40AC7211EB851EB8, 1, 3641035, 3),
    (0x4093D172474538EF, 1, 12683616, 3),
    (0x40A9740000000000, 1, 3258, 3),
    (0x40BAE5A5810624DD, 1, 68856465, 3),
    (0x40C3265CF12C27A6, 1, 980472611, 3),
    (0x40B775A786C22681, 1, 60056544, 3),
    (0xC0911F3333333333, 1, 10958, 3),
    (0x40B3F76083126E98, 1, 5111377, 3),
    (0xC0BF710000000000, 1, 8049, 3),
    (0x409BAD95810624DD, 1, 1771396, 3),
    (0x40B99E157689CA19, 1, 655808384, 3),
];

/// `op(x)` printed with `write_i64`, `\n` after each, for every row of
/// [`FLOAT_DIGITS_TABLE`] whose value `keep` accepts.
fn digits_table_program(op: &str, keep: fn(f64) -> bool) -> IIRModule {
    let mut body = vec![int("nl", 10)];
    for (i, (bits, ..)) in FLOAT_DIGITS_TABLE.iter().enumerate() {
        let x = f64::from_bits(*bits);
        if !keep(x) {
            continue;
        }
        // One result name for every row: each result is printed at once, and
        // the JIT's register file (256 i64 slots) would not hold 87 distinct
        // results plus the literals constant propagation materialises.
        let c = format!("x{i}");
        body.push(real(&c, x));
        body.push(op_instr(op, "r", &c));
        body.push(builtin("write_i64", "r"));
        body.push(builtin("putchar", "nl"));
    }
    program(body)
}

/// `name x -> d : i64` (the PR7b ops write an integer).
fn op_instr(name: &str, d: &str, x: &str) -> IIRInstr {
    op(name, d, &[x], "i64")
}

/// The expected stdout of [`digits_table_program`]: column `col` (0 class,
/// 1 digits, 2 exp10) of every kept row.
fn digits_table_expect(col: usize, keep: fn(f64) -> bool) -> String {
    FLOAT_DIGITS_TABLE
        .iter()
        .filter(|(bits, ..)| keep(f64::from_bits(*bits)))
        .map(|&(_, c, d, e)| [c, d, e][col].to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The finite values: what BEAM can hold (its floats have no ±Infinity or
/// NaN, and `validate_for_beam` refuses such a constant).
fn finite(x: f64) -> bool {
    x.is_finite()
}

fn non_finite(x: f64) -> bool {
    !x.is_finite()
}

/// The expected stdouts, computed once (a `Case` holds `&'static str`).
fn digits_expect(col: usize, finite_rows: bool) -> &'static str {
    static CELLS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let cells = CELLS.get_or_init(|| {
        (0..3).flat_map(|c| [digits_table_expect(c, finite), digits_table_expect(c, non_finite)]).collect()
    });
    &cells[col * 2 + usize::from(!finite_rows)]
}

fn trap_instr(kind: &str) -> IIRInstr {
    IIRInstr::new("trap", None, vec![Operand::Str(kind.into())], "void")
}

/// Comparison `name(a, b)`, printed as 1/0 through a branch (the result is
/// never printed directly, so no backend's bool-to-integer convention matters).
///
/// The comparison carries the `i64` type hint, the shape ALGOL emits. The `bool`
/// hint Twig emits was not used here because the native pipeline miscompiled it
/// for negative operands: `jit_core::specialise` names the CIR op after the
/// *result* hint (`cmp_lt_bool`), and x86-64 / AArch64 took a `bool` suffix as
/// unsigned, so `-1 < 2` was false (Twig `(if (< (- 0 1) 2) 7 3)` exited 3
/// natively). This suite found it. #18453 fixed it: both backends now compare
/// a `bool`-suffixed op as signed, and `lang_matrix.rs` has Twig rows with
/// negative operands.
fn compare(name: &str, a: i64, b: i64) -> IIRModule {
    program(vec![
        int("a", a),
        int("b", b),
        int("one", 1),
        int("zero", 0),
        op(name, "c", &["a", "b"], "i64"),
        IIRInstr::new("jmp_if_false", None, vec![var("c"), var("is_false")], "void"),
        print_i64("one"),
        IIRInstr::new("jmp", None, vec![var("done")], "void"),
        IIRInstr::new("label", None, vec![var("is_false")], "void"),
        print_i64("zero"),
        IIRInstr::new("label", None, vec![var("done")], "void"),
    ])
}

/// LANG83 PR6c: for each `(op, operands, print_sign)`, compute `op` on float
/// constants and print the result as `class digits exp10` (`f64_class`,
/// `f64_digits`, `f64_exp10` via `write_i64`) and — when `print_sign` — a
/// space and its `f64_signbit`; a `\n` after each row. Those four integers
/// identify every value (`2 0 0 1` is -Infinity, `0 0 0 1` is -0). A NaN row
/// does not print its sign: the spec leaves a NaN's sign unspecified (x86's
/// default NaN has it set, AArch64's does not).
///
/// PR6a printed the result with `write_f64_js` (gone since PR7e). The digit
/// ops keep `main` call-free, so the JIT column still runs the IEEE ops as
/// compiled bytecode.
fn ieee_rows(rows: &[(&str, &[f64], bool)]) -> IIRModule {
    let mut body = vec![int("sp", 32), int("nl", 10)];
    for (i, (name, xs, sign)) in rows.iter().enumerate() {
        let srcs: Vec<String> = (0..xs.len()).map(|k| format!("c{i}_{k}")).collect();
        for (src, x) in srcs.iter().zip(xs.iter()) {
            body.push(real(src, *x));
        }
        let (r, s) = (format!("r{i}"), format!("s{i}"));
        let src_refs: Vec<&str> = srcs.iter().map(String::as_str).collect();
        body.push(op(name, &r, &src_refs, "f64"));
        for (k, query) in ["f64_class", "f64_digits", "f64_exp10"].iter().enumerate() {
            let q = format!("q{i}_{k}");
            body.push(op(query, &q, &[&r], "i64"));
            body.push(builtin("write_i64", &q));
            if k < 2 {
                body.push(builtin("putchar", "sp"));
            }
        }
        if *sign {
            body.push(builtin("putchar", "sp"));
            body.push(op("f64_signbit", &s, &[&r], "i64"));
            body.push(builtin("write_i64", &s));
        }
        body.push(builtin("putchar", "nl"));
    }
    program(body)
}

/// The PR6c spec's edge table, as `ieee_rows` rows: infinities from
/// division and overflow, NaN, signed zeros, an inexact sum.
const IEEE_EDGES: &[(&str, &[f64], bool)] = &[
    ("f64_div", &[1.0, 0.0], true),
    ("f64_div", &[-1.0, 0.0], true),
    ("f64_div", &[1.0, -0.0], true),
    ("f64_div", &[0.0, 0.0], false),
    ("f64_sub", &[f64::INFINITY, f64::INFINITY], false),
    ("f64_mul", &[0.0, f64::INFINITY], false),
    ("f64_add", &[1e308, 1e308], true),
    ("f64_mul", &[-1.0, 0.0], true),
    ("f64_mul", &[-1.0, -0.0], true),
    ("f64_add", &[-0.0, -0.0], true),
    ("f64_add", &[-0.0, 0.0], true),
    ("f64_sub", &[0.0, 0.0], true),
    ("f64_neg", &[0.0], true),
    ("f64_neg", &[-0.0], true),
    ("f64_add", &[0.1, 0.2], true),
    ("f64_neg", &[-f64::INFINITY], true),
];

/// `(class, digits, exp10)` of `x`, computed on the host and independently
/// of vm-core: the class from `f64::classify`; the digits and exponent from
/// the shortest spelling that reads back as `x`, choosing among the
/// spellings of that length the one nearest `x`, ties to even, as Python's
/// `repr` and the PR7b ops do (`3.0000000000000004e-1` is digits
/// `30000000000000004`, exp10 `-1`). 0 for both unless the class is 1
/// (finite, non-zero).
///
/// Two halves, because neither Rust formatter is the oracle alone:
///
/// | spelling | length | which digits of that length |
/// | --- | --- | --- |
/// | shortest `{:e}` | right | breaks an exact tie *up* (`2⁻²⁵`: `…695313e-8`, `repr` `…695312e-8`) |
/// | `{:.*e}` at that length | given | the correctly rounded one, half to even, but it may not round-trip |
///
/// So the length comes from `{:e}`, and the digits from `{:.*e}` at that
/// length when they read back as `x`, else from `{:e}` itself. The fallback
/// is the power-of-two case: at `2^k` the round-trip interval is lopsided,
/// so the nearest 16-digit spelling of `0x0060000000000000` misses while
/// `{:e}`'s `7.120236347223045e-307` (also `repr`'s) reads back. Searching
/// for the fewest correctly rounded digits that round-trip, as a first
/// draft did, gives 17 digits there.
fn host_float_parts(x: f64) -> (i64, i64, i64) {
    use std::num::FpCategory;
    match x.classify() {
        FpCategory::Zero => (0, 0, 0),
        FpCategory::Infinite => (2, 0, 0),
        FpCategory::Nan => (3, 0, 0),
        FpCategory::Normal | FpCategory::Subnormal => {
            let ax = x.abs();
            let shortest = format!("{ax:e}");
            let length = shortest
                .split_once('e')
                .expect("`{:e}` has an exponent")
                .0
                .chars()
                .filter(char::is_ascii_digit)
                .count();
            let nearest = format!("{:.*e}", length - 1, ax);
            let spelled = if nearest.parse::<f64>() == Ok(ax) { nearest } else { shortest };
            let (mantissa, exp10) = spelled.split_once('e').expect("`{:e}` has an exponent");
            let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
            let digits = digits.trim_end_matches('0');
            (1, digits.parse().expect("digits"), exp10.parse().expect("exp10"))
        }
    }
}

/// The expected stdout of [`ieee_rows`]: each row's op applied on the host,
/// printed as `class digits exp10` and, when the row asks, its sign bit.
fn ieee_rows_expect(rows: &[(&str, &[f64], bool)]) -> String {
    rows.iter()
        .map(|&(name, xs, sign)| {
            let r = match (name, xs) {
                ("f64_add", &[a, b]) => a + b,
                ("f64_sub", &[a, b]) => a - b,
                ("f64_mul", &[a, b]) => a * b,
                ("f64_div", &[a, b]) => a / b,
                ("f64_neg", &[a]) => -a,
                other => panic!("no host op for {other:?}"),
            };
            let (c, d, e) = host_float_parts(r);
            let row = format!("{c} {d} {e}");
            if sign {
                format!("{row} {}", u8::from(r.is_sign_negative()))
            } else {
                row
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// [`ieee_rows_expect`] of [`IEEE_EDGES`], computed once (a `Case` holds
/// `&'static str`).
fn ieee_edges_expect() -> &'static str {
    static EXPECT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    EXPECT.get_or_init(|| ieee_rows_expect(IEEE_EDGES))
}

/// Unary `f64` op `name(x)`, truncated to an integer and printed.
fn f64_unary(name: &str, x: f64) -> IIRModule {
    program(vec![
        real("x", x),
        op(name, "y", &["x"], "f64"),
        op("real_to_int_trunc", "n", &["y"], "i64"),
        print_i64("n"),
    ])
}

/// The corpus. Values avoid every backend-dependent edge (no zero divisors, no
/// `MIN / -1`, results exactly representable), because those edges are what the
/// PR1b ops define — the legacy ops are only required to agree away from them.
fn cases() -> Vec<Case> {
    let mut all = vec![
        // ── Output ──────────────────────────────────────────────────────────
        Case {
            name: "print_i64 writes the value and a newline",
            build: || program(vec![int("a", 42), int("b", -7), print_i64("a"), print_i64("b")]),
            expect: Expect::Stdout("42\n-7"),
        },
        Case {
            name: "putchar and print_i64 share one ordered stream",
            build: || {
                program(vec![
                    int("a", 65),
                    builtin("putchar", "a"),
                    int("n", 1),
                    print_i64("n"),
                    int("b", 66),
                    builtin("putchar", "b"),
                ])
            },
            expect: Expect::Stdout("A1\nB"),
        },
        Case {
            name: "print_str writes bytes without a newline",
            build: || program(vec![text("a", "he"), text("b", "llo"), print_str("a"), print_str("b")]),
            expect: Expect::Stdout("hello"),
        },
        Case {
            name: "write_i64 writes the value without a newline (PR1b)",
            build: || program(vec![int("a", 4), int("b", 2), builtin("write_i64", "a"), builtin("write_i64", "b")]),
            expect: Expect::Stdout("42"),
        },
        Case {
            name: "write_i64 of 0, a negative, MIN and MAX interleaves with putchar and print_i64",
            build: || {
                program(vec![
                    int("z", 0),
                    int("n", -12),
                    int("lo", i64::MIN),
                    int("hi", i64::MAX),
                    int("sp", 32),
                    int("one", 1),
                    builtin("write_i64", "z"),
                    builtin("putchar", "sp"),
                    builtin("write_i64", "n"),
                    print_i64("one"),
                    builtin("write_i64", "lo"),
                    builtin("putchar", "sp"),
                    builtin("write_i64", "hi"),
                ])
            },
            expect: Expect::Stdout("0 -121\n-9223372036854775808 9223372036854775807"),
        },
        // ── Strings ─────────────────────────────────────────────────────────
        Case {
            name: "str_len counts bytes",
            build: || program(vec![text("s", "hello"), op("str_len", "n", &["s"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("5"),
        },
        Case {
            name: "str_index reads one byte",
            build: || {
                program(vec![text("s", "HELLO"), int("i", 1), op("str_index", "c", &["s", "i"], "i64"), print_i64("c")])
            },
            expect: Expect::Stdout("69"),
        },
        Case {
            name: "str_slice takes [start, end)",
            build: || {
                program(vec![
                    text("s", "HELLO"),
                    int("a", 1),
                    int("b", 3),
                    op("str_slice", "t", &["s", "a", "b"], "str"),
                    print_str("t"),
                ])
            },
            expect: Expect::Stdout("EL"),
        },
        Case {
            name: "str_concat joins",
            build: || {
                program(vec![text("a", "AB"), text("b", "CD"), op("str_concat", "t", &["a", "b"], "str"), print_str("t")])
            },
            expect: Expect::Stdout("ABCD"),
        },
        Case {
            name: "str_eq compares bytes",
            build: || {
                program(vec![text("a", "xy"), text("b", "xy"), op("str_eq", "r", &["a", "b"], "i64"), print_i64("r")])
            },
            expect: Expect::Stdout("1"),
        },
        Case {
            name: "str_cmp orders lexically",
            build: || {
                program(vec![text("a", "a"), text("b", "b"), op("str_cmp", "r", &["a", "b"], "i64"), print_i64("r")])
            },
            expect: Expect::Stdout("-1"),
        },
        // ── Arithmetic ──────────────────────────────────────────────────────
        Case { name: "add", build: || int_binop("add", 40, 2), expect: Expect::Stdout("42") },
        Case { name: "sub", build: || int_binop("sub", -7, 2), expect: Expect::Stdout("-9") },
        Case { name: "mul", build: || int_binop("mul", 6, -7), expect: Expect::Stdout("-42") },
        Case { name: "div truncates", build: || int_binop("div", -7, 2), expect: Expect::Stdout("-3") },
        Case { name: "mod truncates", build: || int_binop("mod", -7, 2), expect: Expect::Stdout("-1") },
        Case {
            name: "neg",
            build: || program(vec![int("a", 5), op("neg", "r", &["a"], "i64"), print_i64("r")]),
            expect: Expect::Stdout("-5"),
        },
        Case {
            name: "i64_div_floor rounds toward -inf (PR1b)",
            build: || int_binop("i64_div_floor", -7, 2),
            expect: Expect::Stdout("-4"),
        },
        Case {
            name: "i64_mod_floor takes the divisor's sign (PR1b)",
            build: || int_binop("i64_mod_floor", -7, 2),
            expect: Expect::Stdout("1"),
        },
        Case {
            name: "i64_div_floor sign table (+-7, +-2)",
            build: || floor_table("i64_div_floor", &[(7, 2), (-7, 2), (7, -2), (-7, -2)]),
            expect: Expect::Stdout("3\n-4\n-4\n3"),
        },
        Case {
            name: "i64_mod_floor sign table (+-7, +-2)",
            build: || floor_table("i64_mod_floor", &[(7, 2), (-7, 2), (7, -2), (-7, -2)]),
            expect: Expect::Stdout("1\n1\n-1\n-1"),
        },
        Case {
            name: "i64 floor ops of MIN by 1",
            build: || {
                program(vec![
                    int("a", i64::MIN),
                    int("b", 1),
                    op("i64_div_floor", "q", &["a", "b"], "i64"),
                    op("i64_mod_floor", "r", &["a", "b"], "i64"),
                    print_i64("q"),
                    print_i64("r"),
                ])
            },
            expect: Expect::Stdout("-9223372036854775808\n0"),
        },
        Case {
            name: "i64_mod_floor of MIN by -1 is 0",
            build: || int_binop("i64_mod_floor", i64::MIN, -1),
            expect: Expect::Stdout("0"),
        },
        Case {
            name: "i64_div_floor of MIN by -1 traps Trap.Overflow",
            build: || floor_then_trap("i64_div_floor", 7, i64::MIN, -1),
            expect: Expect::StdoutThenTrap("7", "Trap.Overflow"),
        },
        Case {
            name: "i64_div_floor by zero traps Trap.DivZero",
            build: || floor_then_trap("i64_div_floor", 5, 7, 0),
            expect: Expect::StdoutThenTrap("5", "Trap.DivZero"),
        },
        Case {
            name: "i64_mod_floor by zero traps Trap.DivZero",
            build: || floor_then_trap("i64_mod_floor", 6, -7, 0),
            expect: Expect::StdoutThenTrap("6", "Trap.DivZero"),
        },
        Case {
            // The quotient is never read: an optimiser that treated the op as
            // pure would delete it, and its trap with it.
            name: "an i64_div_floor whose result is unused still traps",
            build: || {
                program(vec![
                    int("a", 1),
                    int("z", 0),
                    builtin("write_i64", "a"),
                    op("i64_div_floor", "dead", &["a", "z"], "i64"),
                    builtin("write_i64", "a"),
                ])
            },
            expect: Expect::StdoutThenTrap("1", "Trap.DivZero"),
        },
        // ── LANG83 PR6a → PR7e: shortest round-trip float output ────────────
        //
        // PR6a's float writers are gone (PR7e). The same table, the same
        // expected strings, now printed by the compilers' own helpers. The
        // finite rows run on all nine backends (BEAM included); ±Infinity and
        // NaN, which a BEAM float cannot hold, are cases of their own.
        Case {
            name: "Python's repr helper prints every finite table value (PR7e)",
            build: || float_table_program(Layout::Python, finite),
            expect: Expect::Stdout(float_table_expect(Layout::Python, true)),
        },
        Case {
            name: "Python's repr helper prints the infinities and NaN (PR7e)",
            build: || float_table_program(Layout::Python, non_finite),
            expect: Expect::Stdout(float_table_expect(Layout::Python, false)),
        },
        Case {
            name: "JavaScript's Number::toString helper prints every finite table value (PR7e)",
            build: || float_table_program(Layout::JavaScript, finite),
            expect: Expect::Stdout(float_table_expect(Layout::JavaScript, true)),
        },
        Case {
            name: "JavaScript's Number::toString helper prints the infinities and NaN (PR7e)",
            build: || float_table_program(Layout::JavaScript, non_finite),
            expect: Expect::Stdout(float_table_expect(Layout::JavaScript, false)),
        },
        Case {
            name: "the layout helpers share one ordered stream with write_i64, print_i64 and putchar (PR7e)",
            build: || {
                program_with(
                    vec![
                        int("seven", 7),
                        real("half", 0.5),
                        int("one", 1),
                        real("negzero", -0.0),
                        int("sp", 32),
                        real("big", 1e21),
                        builtin("write_i64", "seven"),
                        print_float(Layout::Python, "half"),
                        print_i64("one"),
                        print_float(Layout::JavaScript, "negzero"),
                        builtin("putchar", "sp"),
                        print_float(Layout::Python, "negzero"),
                        builtin("putchar", "sp"),
                        print_float(Layout::JavaScript, "big"),
                    ],
                    &[Layout::Python, Layout::JavaScript],
                )
            },
            expect: Expect::Stdout("70.51\n0 -0.0 1e+21"),
        },
        Case {
            name: "the layout helpers print computed floats, not only constants (PR7e)",
            build: || {
                program_with(
                    vec![
                        real("a", 0.1),
                        real("b", 0.2),
                        op("add", "c", &["a", "b"], "f64"),
                        int("sp", 32),
                        int("n", 7),
                        op("int_to_real", "r", &["n"], "f64"),
                        print_float(Layout::Python, "c"),
                        builtin("putchar", "sp"),
                        print_float(Layout::JavaScript, "c"),
                        builtin("putchar", "sp"),
                        print_float(Layout::Python, "r"),
                        builtin("putchar", "sp"),
                        print_float(Layout::JavaScript, "r"),
                    ],
                    &[Layout::Python, Layout::JavaScript],
                )
            },
            expect: Expect::Stdout("0.30000000000000004 0.30000000000000004 7.0 7"),
        },
        // ── LANG83 PR6c: IEEE f64 arithmetic that never traps ───────────────
        //
        // The spec's edge table: infinities from division and overflow, NaN,
        // signed zeros, an inexact sum. Every row's value is printed as
        // `class digits exp10` and (except NaN) its sign bit, so `-0` is told
        // apart from `+0` (see `ieee_rows`; PR6a printed it with
        // `write_f64_js`, removed in PR7e).
        Case {
            name: "IEEE edges: infinities, NaN, signed zeros (PR6c)",
            build: || ieee_rows(IEEE_EDGES),
            // Infinity, -Infinity, -Infinity, NaN ×3, Infinity, -0, 0, -0,
            // 0, 0, -0, 0, 0.30000000000000004, Infinity — computed on the
            // host, not typed (see `ieee_rows_expect`).
            expect: Expect::Stdout(ieee_edges_expect()),
        },
        // Constants only, finite values only, no arithmetic: the one PR6c case
        // BEAM runs too (its floats have no ±Infinity or NaN to test).
        Case {
            name: "f64_signbit of finite constants (PR6c)",
            build: || {
                let mut body = Vec::new();
                for (i, x) in [0.0, -0.0, 2.5, -2.5, 5e-324, -5e-324, f64::MAX, f64::MIN].iter().enumerate() {
                    let (c, s) = (format!("x{i}"), format!("s{i}"));
                    body.push(real(&c, *x));
                    body.push(op("f64_signbit", &s, &[&c], "i64"));
                    body.push(print_i64(&s));
                }
                program(body)
            },
            expect: Expect::Stdout("0\n1\n0\n1\n0\n1\n0\n1"),
        },
        // Every op with its dest also a source: each backend must read the
        // operands before it writes the result. 1.5+1.5 = 3, 3·3 = 9, 9-0 = 9,
        // -9, then -9/0 = -inf.
        Case {
            name: "IEEE ops read their operands before writing an aliased dest (PR6c)",
            build: || {
                program(vec![
                    real("x", 1.5),
                    real("z", 0.0),
                    int("sp", 32),
                    op("f64_add", "x", &["x", "x"], "f64"),
                    op("f64_mul", "x", &["x", "x"], "f64"),
                    op("f64_sub", "x", &["x", "z"], "f64"),
                    op("f64_neg", "x", &["x"], "f64"),
                    op("f64_digits", "d", &["x"], "i64"),
                    builtin("write_i64", "d"),
                    builtin("putchar", "sp"),
                    op("f64_exp10", "e", &["x"], "i64"),
                    builtin("write_i64", "e"),
                    builtin("putchar", "sp"),
                    op("f64_signbit", "s", &["x"], "i64"),
                    builtin("write_i64", "s"),
                    builtin("putchar", "sp"),
                    op("f64_div", "x", &["x", "z"], "f64"),
                    op("f64_class", "c", &["x"], "i64"),
                    builtin("write_i64", "c"),
                    builtin("putchar", "sp"),
                    op("f64_signbit", "s2", &["x"], "i64"),
                    builtin("write_i64", "s2"),
                ])
            },
            // -9 (digits 9, exp10 0, sign 1), then -9 / 0 = -Infinity
            // (class 2, sign 1).
            expect: Expect::Stdout("9 0 1 2 1"),
        },
        // ── LANG83 PR7b: the language-neutral float primitives ──────────────
        //
        // `f64_class`, `f64_digits` and `f64_exp10` of every PR6a table value,
        // each printed with `write_i64`. The finite rows run on all nine
        // backends (BEAM included); ±Infinity and NaN, which a BEAM float
        // cannot hold, are a case of their own on the other eight.
        Case {
            name: "f64_class of every finite table value (PR7b)",
            build: || digits_table_program("f64_class", finite),
            expect: Expect::Stdout(digits_expect(0, true)),
        },
        Case {
            name: "f64_digits of every finite table value (PR7b)",
            build: || digits_table_program("f64_digits", finite),
            expect: Expect::Stdout(digits_expect(1, true)),
        },
        Case {
            name: "f64_exp10 of every finite table value (PR7b)",
            build: || digits_table_program("f64_exp10", finite),
            expect: Expect::Stdout(digits_expect(2, true)),
        },
        Case {
            name: "f64_class, f64_digits and f64_exp10 of the infinities and NaN (PR7b)",
            build: || {
                let mut body = vec![int("sp", 32), int("nl", 10)];
                for (i, (bits, ..)) in FLOAT_DIGITS_TABLE.iter().enumerate().filter(|(_, (b, ..))| non_finite(f64::from_bits(*b))) {
                    let c = format!("x{i}");
                    body.push(real(&c, f64::from_bits(*bits)));
                    for (k, name) in ["f64_class", "f64_digits", "f64_exp10"].iter().enumerate() {
                        let r = format!("{name}{i}");
                        body.push(op_instr(name, &r, &c));
                        body.push(builtin("write_i64", &r));
                        body.push(builtin("putchar", if k == 2 { "nl" } else { "sp" }));
                    }
                }
                program(body)
            },
            expect: Expect::Stdout("2 0 0\n2 0 0\n3 0 0"),
        },
        // Security-review regression (PR7b): a string op makes the LLVM column
        // link the native runtime (`twig_runtime.c`, via `@__twig_str_eq`) into
        // the same program as the module's own float helpers. A runtime symbol
        // that shared a name with one of them (PR7b first named its C helper
        // `__iir_f64_digits`, the LLVM core's name) would interpose on it:
        // `write_f64_py 0.1` printed `0.0` and `123.456` crashed. The LLVM
        // helpers are now `internal` and the C ones `__twig_*`. Since PR7e the
        // floats are printed by Python's layout helper, over the same digits.
        Case {
            name: "float digits and the layout helper survive a linked native runtime (PR7b review)",
            build: || {
                program_with(vec![
                    text("a", "xy"),
                    text("b", "xy"),
                    int("sp", 32),
                    real("x", 123.456),
                    real("y", 0.1),
                    real("h", 1.5),
                    op("str_eq", "e", &["a", "b"], "i64"),
                    builtin("write_i64", "e"),
                    builtin("putchar", "sp"),
                    print_float(Layout::Python, "x"),
                    builtin("putchar", "sp"),
                    print_float(Layout::Python, "y"),
                    builtin("putchar", "sp"),
                    op("f64_digits", "d", &["x"], "i64"),
                    builtin("write_i64", "d"),
                    builtin("putchar", "sp"),
                    op("f64_exp10", "k", &["x"], "i64"),
                    builtin("write_i64", "k"),
                    builtin("putchar", "sp"),
                    op("f64_digits", "d2", &["h"], "i64"),
                    builtin("write_i64", "d2"),
                ], &[Layout::Python])
            },
            expect: Expect::Stdout("1 123.456 0.1 123456 2 15"),
        },
        // ── LANG83 PR4: overflow-checked arithmetic ─────────────────────────
        //
        // Exact results right at the i64 bounds, then one step past them,
        // where every column must trap `Trap.Overflow` after flushing the
        // output written before.
        Case {
            name: "i64_add_ovf is exact up to the range edges (PR4)",
            build: || checked_table("i64_add_ovf", &[(MAX, 0), (MAX - 1, 1), (MIN, MAX), (MIN, 0), (-5, 3)]),
            expect: Expect::Stdout("9223372036854775807\n9223372036854775807\n-1\n-9223372036854775808\n-2"),
        },
        Case {
            name: "i64_sub_ovf is exact up to the range edges (PR4)",
            build: || checked_table("i64_sub_ovf", &[(MIN, 0), (MIN + 1, 1), (-1, MAX), (MAX, MAX), (-5, 3)]),
            expect: Expect::Stdout("-9223372036854775808\n-9223372036854775808\n-9223372036854775808\n0\n-8"),
        },
        Case {
            name: "i64_mul_ovf is exact up to the range edges (PR4)",
            build: || {
                checked_table(
                    "i64_mul_ovf",
                    &[(MIN, 1), (-(1 << 32), 1 << 31), (3_037_000_499, 3_037_000_499), (-1, MAX), (MIN, 0), (-6, 7)],
                )
            },
            expect: Expect::Stdout(
                "-9223372036854775808\n-9223372036854775808\n9223372030926249001\n-9223372036854775807\n0\n-42",
            ),
        },
        Case {
            name: "i64_neg_ovf is exact except for MIN (PR4)",
            build: || checked_table("i64_neg_ovf", &[(MAX, 0), (0, 0), (-7, 0)]),
            expect: Expect::Stdout("-9223372036854775807\n0\n7"),
        },
        Case {
            name: "i64_add_ovf of MAX + 1 traps Trap.Overflow",
            build: || checked_then_trap("i64_add_ovf", 11, MAX, 1),
            expect: Expect::StdoutThenTrap("11", "Trap.Overflow"),
        },
        Case {
            name: "i64_add_ovf of MIN + -1 traps Trap.Overflow",
            build: || checked_then_trap("i64_add_ovf", 12, MIN, -1),
            expect: Expect::StdoutThenTrap("12", "Trap.Overflow"),
        },
        Case {
            name: "i64_sub_ovf of MIN - 1 traps Trap.Overflow",
            build: || checked_then_trap("i64_sub_ovf", 13, MIN, 1),
            expect: Expect::StdoutThenTrap("13", "Trap.Overflow"),
        },
        Case {
            name: "i64_sub_ovf of MAX - -1 traps Trap.Overflow",
            build: || checked_then_trap("i64_sub_ovf", 14, MAX, -1),
            expect: Expect::StdoutThenTrap("14", "Trap.Overflow"),
        },
        Case {
            name: "i64_mul_ovf of MIN * -1 traps Trap.Overflow",
            build: || checked_then_trap("i64_mul_ovf", 15, MIN, -1),
            expect: Expect::StdoutThenTrap("15", "Trap.Overflow"),
        },
        Case {
            name: "i64_mul_ovf of -1 * MIN traps Trap.Overflow",
            build: || checked_then_trap("i64_mul_ovf", 16, -1, MIN),
            expect: Expect::StdoutThenTrap("16", "Trap.Overflow"),
        },
        Case {
            name: "i64_mul_ovf of 2^32 * 2^31 traps Trap.Overflow",
            build: || checked_then_trap("i64_mul_ovf", 17, 1 << 32, 1 << 31),
            expect: Expect::StdoutThenTrap("17", "Trap.Overflow"),
        },
        Case {
            name: "i64_mul_ovf of 3037000500 squared traps Trap.Overflow",
            build: || checked_then_trap("i64_mul_ovf", 18, 3_037_000_500, 3_037_000_500),
            expect: Expect::StdoutThenTrap("18", "Trap.Overflow"),
        },
        Case {
            name: "i64_neg_ovf of MIN traps Trap.Overflow",
            build: || checked_then_trap("i64_neg_ovf", 19, MIN, 0),
            expect: Expect::StdoutThenTrap("19", "Trap.Overflow"),
        },
        Case {
            // The sum is never read: an optimiser that treated the op as pure
            // would delete it, and its trap with it. `write_i64` does not
            // flush per call natively, so the "1" also proves the flush.
            name: "an i64_add_ovf whose result is unused still traps",
            build: || {
                program(vec![
                    int("a", 1),
                    int("big", MAX),
                    builtin("write_i64", "a"),
                    op("i64_add_ovf", "dead", &["big", "a"], "i64"),
                    builtin("write_i64", "a"),
                ])
            },
            expect: Expect::StdoutThenTrap("1", "Trap.Overflow"),
        },
        Case {
            // No `print_i64`/`write_i64` and no literal wider than i32: the
            // only i64 evidence is the checked op itself, so this exercises
            // the JVM scalar concretizer, which must keep the module on
            // `long`. 2e9 + 2e9 = 4e9 (overflows i32), / 10^8 = 40, + 25 = 'A'.
            name: "i64_add_ovf keeps a putchar-only module on the i64 model",
            build: || {
                program(vec![
                    int("a", 2_000_000_000),
                    int("scale", 100_000_000),
                    int("k", 25),
                    op("i64_add_ovf", "s", &["a", "a"], "i64"),
                    op("div", "q", &["s", "scale"], "i64"),
                    op("add", "c", &["q", "k"], "i64"),
                    builtin("putchar", "c"),
                ])
            },
            expect: Expect::Stdout("A"),
        },
        // ── Comparison ──────────────────────────────────────────────────────
        Case { name: "cmp_eq", build: || compare("cmp_eq", 3, 3), expect: Expect::Stdout("1") },
        Case { name: "cmp_ne", build: || compare("cmp_ne", 3, 3), expect: Expect::Stdout("0") },
        Case { name: "cmp_lt", build: || compare("cmp_lt", -1, 2), expect: Expect::Stdout("1") },
        Case { name: "cmp_le", build: || compare("cmp_le", 2, 2), expect: Expect::Stdout("1") },
        Case { name: "cmp_gt", build: || compare("cmp_gt", -1, 2), expect: Expect::Stdout("0") },
        Case { name: "cmp_ge", build: || compare("cmp_ge", 1, 2), expect: Expect::Stdout("0") },
        // ── Float math (exact results only) ─────────────────────────────────
        Case { name: "f64_sqrt", build: || f64_unary("f64_sqrt", 16.0), expect: Expect::Stdout("4") },
        Case { name: "f64_sin", build: || f64_unary("f64_sin", 0.0), expect: Expect::Stdout("0") },
        Case { name: "f64_cos", build: || f64_unary("f64_cos", 0.0), expect: Expect::Stdout("1") },
        Case { name: "f64_ln", build: || f64_unary("f64_ln", 1.0), expect: Expect::Stdout("0") },
        Case { name: "f64_exp", build: || f64_unary("f64_exp", 0.0), expect: Expect::Stdout("1") },
        Case { name: "f64_atan", build: || f64_unary("f64_atan", 0.0), expect: Expect::Stdout("0") },
        Case { name: "f64_tan", build: || f64_unary("f64_tan", 0.0), expect: Expect::Stdout("0") },
        Case {
            name: "f64_pow",
            build: || {
                program(vec![
                    real("b", 2.0),
                    real("e", 10.0),
                    op("f64_pow", "y", &["b", "e"], "f64"),
                    op("real_to_int_trunc", "n", &["y"], "i64"),
                    print_i64("n"),
                ])
            },
            expect: Expect::Stdout("1024"),
        },
        // ── Conversions ─────────────────────────────────────────────────────
        Case {
            name: "int_to_real round-trips",
            build: || {
                program(vec![
                    int("n", -7),
                    op("int_to_real", "x", &["n"], "f64"),
                    op("real_to_int_floor", "m", &["x"], "i64"),
                    print_i64("m"),
                ])
            },
            expect: Expect::Stdout("-7"),
        },
        Case {
            name: "real_to_int_trunc rounds toward zero",
            build: || program(vec![real("x", -2.5), op("real_to_int_trunc", "n", &["x"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("-2"),
        },
        Case {
            name: "real_to_int_floor rounds toward -inf",
            build: || program(vec![real("x", -2.5), op("real_to_int_floor", "n", &["x"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("-3"),
        },
        // A narrowing that cannot fit raises the typed `Trap.ConvRange` on
        // every column, after the output written before it. BEAM runs only
        // the finite rows: an Erlang float is never NaN or ±∞.
        Case {
            name: "real_to_int_trunc of NaN traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_trunc", f64::NAN),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_trunc of +inf traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_trunc", f64::INFINITY),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_trunc of -inf traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_trunc", f64::NEG_INFINITY),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_trunc of 2^63 traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_trunc", TWO_63),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_trunc of -1e30 traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_trunc", -1e30),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_floor of NaN traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_floor", f64::NAN),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_floor of +inf traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_floor", f64::INFINITY),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_floor of -inf traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_floor", f64::NEG_INFINITY),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_floor of 2^63 traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_floor", TWO_63),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int_floor of -1e30 traps Trap.ConvRange",
            build: || narrow_then_trap("real_to_int_floor", -1e30),
            expect: Expect::StdoutThenTrap("1", "Trap.ConvRange"),
        },
        Case {
            name: "real_to_int of -2^63 fits (the inclusive lower bound)",
            build: || program(vec![real("x", -TWO_63), op("real_to_int_trunc", "n", &["x"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("-9223372036854775808"),
        },
        // ── Errors ──────────────────────────────────────────────────────────
        Case {
            name: "trap flushes earlier output then reports its kind (PR1b)",
            build: || {
                program(vec![
                    text("s", "before"),
                    print_str("s"),
                    IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
                ])
            },
            expect: Expect::StdoutThenTrap("before", "Trap.User"),
        },
        // ── Dynamic values (LANG83 PR7f) ────────────────────────────────────
        Case {
            name: "dynamic-value pairs, tags, nil and the pair/nil tests (PR7f)",
            build: dynamic_value_program,
            expect: Expect::Stdout("41\n-7\n7\n0\n1\n5\n3\n5\n3\n2"),
        },
        Case {
            name: "Lisp truthiness, not, equal? and exit code as module IIR (PR7f-2)",
            build: lisp_rules_program,
            // Truthiness and `not` (#t = 5, #f = 3) of 0, 7, nil, #t, #f,
            // (1 . 2) and A; the exit codes of 7, #t, #f, nil and 0; then
            // `equal?` of each LISP_RULE_EQUAL pair.
            expect: Expect::Stdout(
                "1\n3\n1\n3\n0\n5\n1\n3\n0\n5\n1\n3\n1\n3\n\
                 7\n1\n0\n0\n0\n\
                 5\n3\n5\n5\n3\n3\n5\n3\n5\n3\n5\n3",
            ),
        },
        Case {
            // No compiled backend lowers a symbol built from a literal: a
            // symbol reaches the tagged-word backends as an interned
            // `const … : symbol`, so they refuse this case before any pass
            // runs. The VM lowers it (#18470): `dyn_symbol "A"` interns the
            // name to a `Value::Symbol`, whose `dyn_tag` is 2.
            name: "dyn_symbol of a literal runs on the VM only (PR7f, #18470)",
            build: || {
                program(vec![
                    IIRInstr::new(
                        "call_builtin",
                        Some("s".into()),
                        vec![var("dyn_symbol"), Operand::Str("A".into())],
                        DYN,
                    ),
                    builtin_to("dyn_tag", "t", &["s"], "i64"),
                    print_i64("t"),
                ])
            },
            expect: Expect::Stdout("2"),
        },
        Case {
            // `write_i64`/`putchar` do not flush per call on the native and LLVM
            // columns (libc stdout is block-buffered on a pipe), so this output
            // reaches the reader only if the trap flushes it first.
            name: "trap keeps unflushed write_i64 and putchar output (PR1b)",
            build: || {
                program(vec![
                    int("n", -3),
                    int("bang", 33),
                    builtin("write_i64", "n"),
                    builtin("putchar", "bang"),
                    trap_instr("Trap.User"),
                ])
            },
            expect: Expect::StdoutThenTrap("-3!", "Trap.User"),
        },
    ];
    // LANG83 PR5a: the string-failure, ceiling, DCE, aliasing and opaque cases.
    all.extend(pr5a_cases());
    all.extend(pr5b_cases());
    all
}

// ===========================================================================
// Column runners
// ===========================================================================

/// The code-generation backends a case is compiled on.
const COMPILE_BACKENDS: &[Backend] =
    &[Backend::Llvm, Backend::Wasm, Backend::Jvm, Backend::Clr, Backend::Beam];

/// Compile `module` through `backend`'s lang-aot entry point; the artifact is
/// discarded (execution, where available, is a separate runner).
fn compile_on(backend: Backend, module: IIRModule) -> Result<(), LangAotError> {
    match backend {
        Backend::Llvm => lang_aot::compile_iir_to_llvm(module, "opconf", "x86_64-unknown-linux-gnu").map(drop),
        Backend::Wasm => lang_aot::compile_iir_to_wasm(module).map(drop),
        Backend::Jvm => lang_aot::compile_iir_to_jvm_class(module, "OpConf").map(drop),
        Backend::Clr => lang_aot::compile_iir_to_cil_text(module, "OpConf").map(drop),
        Backend::Beam => lang_aot::compile_iir_to_beam(module, "opconf").map(drop),
        Backend::X86_64 | Backend::Aarch64 => {
            let dir = tempfile::tempdir().expect("native: temp dir");
            native_compile(&module, &dir.path().join("prog"))
        }
        Backend::Vm | Backend::Jit => check_module_for(backend, &module).map_err(LangAotError::from),
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn native_compile(module: &IIRModule, exe: &std::path::Path) -> Result<(), LangAotError> {
    lang_aot::compile_iir_to_host_executable(module, exe)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn native_compile(module: &IIRModule, _exe: &std::path::Path) -> Result<(), LangAotError> {
    check_module_for(Backend::native_host(), module).map_err(LangAotError::from)
}

/// One shared, ordered output buffer (LANG83): every output builtin appends here.
type Sink = Arc<Mutex<Vec<u8>>>;

/// Every byte the program wrote, exactly (LANG83 PR5a: no decode, no trim;
/// [`assert_outcome`] decides how to compare).
fn sink_text(sink: &Sink) -> Vec<u8> {
    sink.lock().expect("output sink poisoned").clone()
}

fn register_vm_output(vm: &mut vm_core::VMCore, sink: &Sink) {
    use vm_core::value::Value;
    let s = Arc::clone(sink);
    vm.builtins_mut().register("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
        Ok(Value::Null)
    });
    let s = Arc::clone(sink);
    vm.builtins_mut().register("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(Value::as_i64).unwrap_or(0) & 0xFF) as u8;
        s.lock().expect("sink").push(b);
        Ok(Value::Null)
    });
    let s = Arc::clone(sink);
    vm.builtins_mut().register("write_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(n.to_string().as_bytes());
        Ok(Value::Null)
    });
    // No float writer: LANG83 PR7e removed `write_f64_py` / `write_f64_js`;
    // a float is printed by its compiler's layout helper, through `putchar`.
    let s = Arc::clone(sink);
    vm.builtins_mut().register("print_str", move |args: &[Value]| {
        // The bytes, unchanged (LANG83 PR5b): `as_str` would drop a string
        // that is not UTF-8, so a lone `0xA9` printed nothing.
        let bytes = args.first().and_then(Value::as_bytes).unwrap_or(b"");
        s.lock().expect("sink").extend_from_slice(bytes);
        Ok(Value::Null)
    });
}

/// LANG83 PR7e: no builtin a runner registers may name a source language,
/// exactly like the registry's ops and trap kinds. (PR6a's runners had
/// `write_f64_py` / `write_f64_js` sinks.)
fn assert_runner_builtins_name_no_language<'a>(column: &str, names: impl IntoIterator<Item = &'a str>) {
    for name in names {
        if let Some(why) = language_name_violation(name) {
            panic!("{column} runner builtin: {why}");
        }
    }
}

fn run_vm(module: IIRModule) -> RunResult {
    let mut module = module;
    let mut vm = vm_core::VMCore::new();
    let sink: Sink = Arc::default();
    register_vm_output(&mut vm, &sink);
    assert_runner_builtins_name_no_language("Vm", vm.builtins_mut().names());
    match vm.execute(&mut module, "main", &[]) {
        Ok(_) => RunResult::Completed { stdout: sink_text(&sink) },
        Err(vm_core::errors::VMError::Trap { kind }) => RunResult::Trapped { stdout: sink_text(&sink), kind: Some(kind) },
        Err(_) => RunResult::Trapped { stdout: sink_text(&sink), kind: None },
    }
}

/// The JIT column. Only cases whose every op the JIT declares `Lowered` reach
/// here, so every call-free function (`main`, unless the case prints with a
/// compiler's layout helper) must genuinely be JIT-compiled — asserting it is
/// what stops the interpreter fallback from passing as JIT support.
///
/// `main` is compiled explicitly with `JITCore::compile` rather than left to
/// `execute_with_jit`'s eager phase, because that phase only compiles
/// `FullyTyped` functions and `infer_type_status` does not count `"void"` as a
/// concrete type — so any function containing a `call_builtin … : void` (every
/// function that prints) is `PartiallyTyped` and would silently run on the
/// interpreter tier. (That is also true of the `lang_matrix` JIT column today;
/// LANG83 records it as a follow-up.)
fn run_jit(module: IIRModule, case: &Case) -> RunResult {
    use jit_core::core::JITCore;
    use jit_core::GenericCirJit;
    use vm_core::value::Value;
    let mut module = module;
    let sink: Sink = Arc::default();
    let mut vm = vm_core::VMCore::new();
    register_vm_output(&mut vm, &sink);
    let backend = GenericCirJit::new();
    let s = Arc::clone(&sink);
    backend.register_builtin("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
        Value::Null
    });
    let s = Arc::clone(&sink);
    backend.register_builtin("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(Value::as_i64).unwrap_or(0) & 0xFF) as u8;
        s.lock().expect("sink").push(b);
        Value::Null
    });
    let s = Arc::clone(&sink);
    backend.register_builtin("write_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(n.to_string().as_bytes());
        Value::Null
    });
    // `run` cannot return an error, so a JIT-side trap lands in this slot;
    // `JITCore` must drain it into an `Err` (fail closed). A message still
    // sitting here after `execute` returns means the trap failed open.
    let error_slot = Arc::clone(&backend.error);
    assert_runner_builtins_name_no_language("Jit", backend.builtin_names().iter().map(String::as_str));
    assert_runner_builtins_name_no_language("Jit (interpreter tier)", vm.builtins_mut().names());
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    // Every function without a `call` must be JIT-compiled. The generic JIT
    // has no bytecode for `call`, so a function that makes one (`main` of a
    // case that prints with a compiler's layout helper, the helper itself)
    // runs on the interpreter tier and calls the compiled ones — the split
    // the `lang_matrix` Python and JavaScript rows have (LANG83 PR7c).
    let call_free: Vec<String> = module
        .functions
        .iter()
        .filter(|f| f.instructions.iter().all(|i| i.op != "call"))
        .map(|f| f.name.clone())
        .collect();
    assert!(
        !call_free.is_empty(),
        "Jit: case `{}` must have a call-free function for the JIT to compile",
        case.name
    );
    for name in &call_free {
        let compiled = jit.compile(&mut vm, &module, name);
        assert!(
            matches!(compiled, Ok(true)) && jit.is_compiled(name),
            "Jit: case `{}` uses only JIT-Lowered ops, so `{name}` must be JIT-compiled ({compiled:?})",
            case.name
        );
    }
    let result = jit.execute(&mut vm, &mut module, "main", &[]);
    // A compiled function that ran really ran as JIT code: `main` when it is
    // call-free, otherwise the helpers it reaches. A layout helper reaches
    // its compiled digit writer for every finite non-zero value, so a case
    // whose output holds a digit must have run one; `inf`, `nan` and the
    // zeros are written by the (interpreted) entry helper alone.
    let prints_a_digit = case.expect.out_text().bytes().any(|b| b.is_ascii_digit());
    if !call_free.iter().any(|n| n == "main") && prints_a_digit {
        let stats = jit.cache_stats();
        let ran_compiled = call_free.iter().any(|n| {
            stats.get(n).and_then(|s| s.get("exec_count")).and_then(|c| c.parse::<u64>().ok()).is_some_and(|c| c > 0)
        });
        assert!(ran_compiled, "Jit: case `{}` never ran a compiled function ({stats:?})", case.name);
    }
    let leftover = error_slot.lock().expect("jit error slot").take();
    assert!(
        leftover.is_none(),
        "Jit: case `{}` left {leftover:?} in the JIT error slot — a JIT failure must surface as Err",
        case.name
    );
    match result {
        Ok(_) => RunResult::Completed { stdout: sink_text(&sink) },
        Err(vm_core::errors::VMError::Trap { kind }) => {
            RunResult::Trapped { stdout: sink_text(&sink), kind: Some(kind) }
        }
        Err(_) => RunResult::Trapped { stdout: sink_text(&sink), kind: None },
    }
}

/// The body of a host import: arguments (and linear memory) in, results out.
type HostBody = Box<
    dyn Fn(
        &[wasm_execution::WasmValue],
        Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError>,
>;

/// A host import implemented by a closure, so the WASM host below stays a table.
struct HostFn {
    ty: wasm_types::FuncType,
    f: HostBody,
}

impl wasm_execution::HostFunction for HostFn {
    fn func_type(&self) -> &wasm_types::FuncType {
        &self.ty
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        (self.f)(args, memory)
    }
}

/// The WASM host: the output imports append to ONE ordered sink; the `f64`
/// math imports are libm through Rust's `f64` methods.
struct ConformanceHost {
    sink: Sink,
    /// The kind id `env.__trap` received, if the program trapped (LANG83).
    trap_kind: Arc<Mutex<Option<u32>>>,
}

fn trap(msg: &str) -> wasm_execution::TrapError {
    wasm_execution::TrapError::new(msg)
}

fn arg_i64(args: &[wasm_execution::WasmValue], i: usize) -> Result<i64, wasm_execution::TrapError> {
    match args.get(i) {
        Some(wasm_execution::WasmValue::I64(v)) => Ok(*v),
        Some(wasm_execution::WasmValue::I32(v)) => Ok(i64::from(*v)),
        _ => Err(trap("expected an integer argument")),
    }
}

fn arg_f64(args: &[wasm_execution::WasmValue], i: usize) -> Result<f64, wasm_execution::TrapError> {
    match args.get(i) {
        Some(wasm_execution::WasmValue::F64(v)) => Ok(*v),
        _ => Err(trap("expected an f64 argument")),
    }
}

impl wasm_execution::HostInterface for ConformanceHost {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn wasm_execution::HostFunction>> {
        use wasm_execution::WasmValue;
        use wasm_types::{FuncType, ValueType::{F64, I32, I64}};
        if module != "env" {
            return None;
        }
        let sink = Arc::clone(&self.sink);
        let trap_kind = Arc::clone(&self.trap_kind);
        let unary = |g: fn(f64) -> f64| -> Option<Box<dyn wasm_execution::HostFunction>> {
            Some(Box::new(HostFn {
                ty: FuncType { params: vec![F64], results: vec![F64] },
                f: Box::new(move |a, _| Ok(vec![WasmValue::F64(g(arg_f64(a, 0)?))])),
            }))
        };
        match name {
            "__print_i64" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I64], results: vec![] },
                f: Box::new(move |a, _| {
                    let n = arg_i64(a, 0)?;
                    sink.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
                    Ok(vec![])
                }),
            })),
            // LANG83 PR1b: decimal, no newline, into the same ordered sink.
            "__write_i64" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I64], results: vec![] },
                f: Box::new(move |a, _| {
                    let n = arg_i64(a, 0)?;
                    sink.lock().expect("sink").extend_from_slice(n.to_string().as_bytes());
                    Ok(vec![])
                }),
            })),
            // LANG83 PR1b: record the kind id and return; the module's own
            // `unreachable` must then stop the program.
            "__trap" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I32], results: vec![] },
                f: Box::new(move |a, _| {
                    let id = u32::try_from(arg_i64(a, 0)?).map_err(|_| trap("__trap: bad kind id"))?;
                    *trap_kind.lock().expect("trap slot") = Some(id);
                    Ok(vec![])
                }),
            })),
            "putchar" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I32], results: vec![] },
                f: Box::new(move |a, _| {
                    sink.lock().expect("sink").push((arg_i64(a, 0)? & 0xFF) as u8);
                    Ok(vec![])
                }),
            })),
            "__print_str" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I32, I32], results: vec![] },
                f: Box::new(move |a, memory| {
                    let ptr = usize::try_from(arg_i64(a, 0)?).map_err(|_| trap("__print_str: bad ptr"))?;
                    let len = usize::try_from(arg_i64(a, 1)?).map_err(|_| trap("__print_str: bad len"))?;
                    let memory = memory.ok_or_else(|| trap("__print_str: no linear memory"))?;
                    let mut bytes = Vec::with_capacity(len);
                    for offset in 0..len {
                        bytes.push(memory.load_i32_8u(ptr + offset)? as u8);
                    }
                    sink.lock().expect("sink").extend_from_slice(&bytes);
                    Ok(vec![])
                }),
            })),
            "__sin" => unary(f64::sin),
            "__cos" => unary(f64::cos),
            "__ln" => unary(f64::ln),
            "__exp" => unary(f64::exp),
            "__atan" => unary(f64::atan),
            "__tan" => unary(f64::tan),
            "__pow" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![F64, F64], results: vec![F64] },
                f: Box::new(|a, _| Ok(vec![WasmValue::F64(arg_f64(a, 0)?.powf(arg_f64(a, 1)?))])),
            })),
            _ => None,
        }
    }

    fn resolve_global(
        &self,
        _module: &str,
        _name: &str,
    ) -> Option<(wasm_types::GlobalType, std::rc::Rc<std::cell::RefCell<wasm_execution::GlobalStorage>>)> {
        None
    }

    fn resolve_memory(&self, _module: &str, _name: &str) -> Option<wasm_execution::LinearMemory> {
        None
    }

    fn resolve_table(&self, _module: &str, _name: &str) -> Option<wasm_execution::Table> {
        None
    }
}

fn run_wasm(module: IIRModule, case: &Case) -> RunResult {
    let bytes = lang_aot::compile_iir_to_wasm(module)
        .unwrap_or_else(|e| panic!("Wasm: case `{}` failed to compile: {e}", case.name));
    let sink: Sink = Arc::default();
    let trap_kind: Arc<Mutex<Option<u32>>> = Arc::default();
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(ConformanceHost {
        sink: Arc::clone(&sink),
        trap_kind: Arc::clone(&trap_kind),
    }));
    let result = rt.load_and_run(&bytes, "main", &[]);
    let kind = trap_kind
        .lock()
        .expect("trap slot")
        .and_then(interpreter_ir::exception_kind::trap_kind_name)
        .map(str::to_string);
    match (result, kind) {
        (Ok(_), None) => RunResult::Completed { stdout: sink_text(&sink) },
        (Ok(_), Some(kind)) => panic!("Wasm: case `{}` returned after env.__trap({kind})", case.name),
        (Err(_), kind) => RunResult::Trapped { stdout: sink_text(&sink), kind },
    }
}

fn clang_ok() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// The LLVM column's print helpers (the harness C runtime LANG83 PR1b retires by
/// emitting self-contained definitions; until then it is linked like the
/// matrix's `PRINT_RUNTIME_C`).
const PRINT_RUNTIME_C: &str = "#include <stdio.h>\n#include <stdint.h>\n\
void __print_i64(int64_t x){printf(\"%lld\\n\",(long long)x);}\n\
void __print_str(const char* p,int64_t len){if(len>0){fwrite(p,1,(size_t)len,stdout);}}\n";

fn run_llvm(module: IIRModule, case: &Case) -> Option<RunResult> {
    if !clang_ok() {
        return None;
    }
    let triple = String::from_utf8(Command::new("clang").arg("-dumpmachine").output().ok()?.stdout)
        .ok()?
        .trim()
        .to_string();
    let ll = lang_aot::compile_iir_to_llvm(module, "opconf", &triple)
        .unwrap_or_else(|e| panic!("Llvm: case `{}` failed to compile: {e}", case.name));
    let dir = tempfile::tempdir().expect("llvm: temp dir");
    let ll_path = dir.path().join("prog.ll");
    std::fs::write(&ll_path, &ll).expect("llvm: write .ll");
    let exe = dir.path().join("prog");
    let mut cmd = Command::new("clang");
    cmd.arg("-x").arg("ir").arg(&ll_path);
    if ll.contains("@__twig_") || ll.contains("@__dyn_") {
        let rt = |n: &str| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(n);
        cmd.arg("-x").arg("none").arg(rt("dynval_runtime.c")).args(common::gc_link_args()).arg(rt("twig_runtime.c"));
    }
    if ll.contains("@__print_") {
        let rt_path = dir.path().join("print_rt.c");
        std::fs::write(&rt_path, PRINT_RUNTIME_C).expect("llvm: write print runtime");
        cmd.arg("-x").arg("c").arg(&rt_path);
    }
    if cfg!(target_os = "linux") {
        cmd.arg("-x").arg("none").arg("-lm");
    }
    let built = cmd.arg("-o").arg(&exe).output().expect("llvm: spawn clang");
    assert!(
        built.status.success(),
        "Llvm: case `{}` did not link: {}",
        case.name,
        String::from_utf8_lossy(&built.stderr)
    );
    Some(run_process(Command::new(&exe)))
}

fn java_ok() -> bool {
    ["java", "javac"]
        .iter()
        .all(|tool| Command::new(tool).arg("-version").output().map(|o| o.status.success()).unwrap_or(false))
}

/// The two host classes the JVM backend's `print_i64` / `putchar` call (the
/// same contract as `lang_matrix`'s `BASIC_RUNTIME_JAVA` / `BF_RUNTIME_JAVA`),
/// both writing to `System.out` like `write_i64` and `print_str` do.
const ENV_RUNTIME_JAVA: &[(&str, &str)] = &[
    (
        "BasicRuntime.java",
        "package env; public final class BasicRuntime { \
         public static void println(long x) { System.out.println(x); } }",
    ),
    (
        "BFRuntime.java",
        "package env; public final class BFRuntime { public static byte[] __tape = new byte[30000]; \
         public static void putchar(int c) { System.out.write(c & 0xFF); System.out.flush(); } \
         public static int getchar() { return 0; } }",
    ),
];

/// Compile the `env.*` host classes once per test process; `None` if that fails.
fn env_classes_dir() -> Option<&'static std::path::Path> {
    static DIR: std::sync::OnceLock<Option<tempfile::TempDir>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().ok()?;
        let mut javac = Command::new("javac");
        javac.arg("-d").arg(dir.path());
        for (file, source) in ENV_RUNTIME_JAVA {
            let path = dir.path().join(file);
            std::fs::write(&path, source).ok()?;
            javac.arg(path);
        }
        let out = javac.output().ok()?;
        assert!(out.status.success(), "javac env runtime: {}", String::from_utf8_lossy(&out.stderr));
        Some(dir)
    })
    .as_ref()
    .map(|d| d.path())
}

/// The JVM column: compile through lang-aot, add a `main(String[])` launcher
/// that calls the entry and discards its result, and run it on real `java` with
/// the class-file verifier ON (no `-Xverify:none`) — so the PR1b bytecode
/// (guards, `athrow` and the dead code after it) is verified, not just run.
fn run_jvm(module: IIRModule, case: &Case) -> Option<RunResult> {
    use jvm_class_file::{
        JvmCodeAttribute, JvmConstantPoolEntry as Cp, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC, ACC_STATIC,
    };
    if !java_ok() {
        return None;
    }
    let env_dir = env_classes_dir()?;
    let mut class = lang_aot::compile_iir_to_jvm_class(module, "Main")
        .unwrap_or_else(|e| panic!("Jvm: case `{}` failed to compile: {e}", case.name));
    let entry_desc = class.methods.iter().find(|m| m.name == "main").expect("entry `main`").descriptor.clone();
    let ret = entry_desc.rsplit(')').next().unwrap_or("V").to_string();
    let cp = &mut class.constant_pool;
    let mut add = |e: Cp| {
        cp.push(Some(e));
        (cp.len() - 1) as u16
    };
    let class_utf8 = add(Cp::Utf8("Main".into()));
    let class_ref = add(Cp::Class { name_index: class_utf8 });
    let name = add(Cp::Utf8("main".into()));
    let desc = add(Cp::Utf8(entry_desc.clone()));
    let nat = add(Cp::NameAndType { name_index: name, descriptor_index: desc });
    let entry_ref = add(Cp::Methodref { class_index: class_ref, name_and_type_index: nat });
    let _ = add(Cp::Utf8("([Ljava/lang/String;)V".into()));
    let [hi, lo] = entry_ref.to_be_bytes();
    let mut code = vec![0xB8, hi, lo]; // invokestatic Main.main()<R>
    let width: u16 = match ret.as_str() {
        "J" | "D" => {
            code.push(0x58); // pop2
            2
        }
        "V" => 0,
        _ => {
            code.push(0x57); // pop
            1
        }
    };
    code.push(0xB1); // return
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack: width.max(1),
            max_locals: 1,
            code,
            nested_attributes: vec![],
        })],
    });
    let dir = tempfile::tempdir().expect("jvm: temp dir");
    std::fs::write(dir.path().join("Main.class"), iir_to_jvm_class_file::serialize_jvm_class_file(&class))
        .expect("jvm: write Main.class");
    let classpath = std::env::join_paths([dir.path(), env_dir]).expect("jvm: classpath");
    // HotSpot keeps perf counters in /tmp/hsperfdata_<user>/<pid>. With many JVMs
    // running in parallel a recycled PID's file can still be locked, and the JVM
    // then prints a unified-logging warning to STDOUT, which this test compares
    // byte-for-byte. `-XX:-UsePerfData` removes the file, hence the warning.
    let mut cmd = Command::new("java");
    cmd.arg("-XX:-UsePerfData").arg("-cp").arg(classpath).arg("Main");
    let result = run_process(cmd);
    if let RunResult::Trapped { kind: None, .. } = &result {
        panic!("Jvm: case `{}` failed without a LANG83 trap kind: {result:?}", case.name);
    }
    Some(result)
}

fn run_process(mut cmd: Command) -> RunResult {
    let out = cmd.output().expect("spawn compiled program");
    let stdout = out.stdout;
    if out.status.success() {
        RunResult::Completed { stdout }
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // `trap: Trap.DivZero` (native, LLVM) or
        // `java.lang.RuntimeException: Trap.DivZero` (JVM): the first word that
        // names a kind, without surrounding punctuation.
        let kind = stderr
            .split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | ',' | '{' | '}' | '(' | ')'))
            .find(|w| w.starts_with("Trap."))
            .map(str::to_string);
        RunResult::Trapped { stdout, kind }
    }
}

#[cfg(target_os = "linux")]
fn run_native(module: IIRModule, case: &Case) -> Option<RunResult> {
    let dir = tempfile::tempdir().expect("native: temp dir");
    let exe = dir.path().join("prog");
    lang_aot::compile_iir_to_host_executable(&module, &exe)
        .unwrap_or_else(|e| panic!("NativeAot: case `{}` failed to compile: {e}", case.name));
    Some(run_process(Command::new(&exe)))
}

#[cfg(not(target_os = "linux"))]
fn run_native(_module: IIRModule, _case: &Case) -> Option<RunResult> {
    None
}

/// Run `case` on every executable column that lowers all of its ops; returns
/// the columns that ran.
fn execute_everywhere(case: &Case) -> Vec<&'static str> {
    let mut ran = Vec::new();
    if case.lowered_on(Backend::Wasm) {
        assert_outcome("Wasm", case, run_wasm(case.module(), case));
        ran.push("Wasm");
    }
    if case.lowered_on(Backend::Vm) {
        assert_outcome("Vm", case, run_vm(case.module()));
        ran.push("Vm");
    }
    if case.lowered_on(Backend::Jit) {
        assert_outcome("Jit", case, run_jit(case.module(), case));
        ran.push("Jit");
    }
    if case.lowered_on(Backend::Llvm) {
        if let Some(result) = run_llvm(case.module(), case) {
            assert_outcome("Llvm", case, result);
            ran.push("Llvm");
        }
    }
    if case.lowered_on(Backend::native_host()) {
        if let Some(result) = run_native(case.module(), case) {
            assert_outcome("NativeAot", case, result);
            ran.push("NativeAot");
        }
    }
    if case.lowered_on(Backend::Jvm) {
        if let Some(result) = run_jvm(case.module(), case) {
            assert_outcome("Jvm", case, result);
            ran.push("Jvm");
        }
    }
    ran
}

// ===========================================================================
// The gates
// ===========================================================================

/// The host oracle behind [`ieee_rows_expect`] agrees with every row of
/// the PR7b digits table (whose columns were written from host `python3`),
/// and with the PR6c edge results it describes: `+Infinity` is `2 0 0 0`,
/// `-0` is `0 0 0 1`, a NaN row has no sign, and `0.1 + 0.2` is digits
/// `30000000000000004` at exp10 `-1`.
/// Every power of two, `2⁻¹⁰⁷⁴ ..= 2¹⁰²³`, where the round-trip interval
/// is lopsided, against host `python3`'s `repr` digits (skipped when
/// `python3` is absent). A first draft of [`host_float_parts`] disagreed
/// with `repr` at 46 of them.
#[test]
fn the_host_float_oracle_matches_python_at_every_power_of_two() {
    let script = "import math\nfrom decimal import Decimal\n\
        for k in range(-1074, 1024):\n\
        \x20   t = Decimal(repr(math.ldexp(1.0, k))).normalize().as_tuple()\n\
        \x20   print(k, int(''.join(map(str, t.digits))), t.exponent + len(t.digits) - 1)\n";
    let Ok(out) = std::process::Command::new("python3").args(["-I", "-c", script]).output() else {
        eprintln!("SKIP: python3 not found");
        return;
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).expect("utf-8");
    let mut checked = 0;
    for line in text.lines() {
        let f: Vec<i64> = line.split(' ').map(|w| w.parse().expect("number")).collect();
        // Built from bits: `powi` would round 2⁻¹⁰⁷⁴ to 0 through 1/2¹⁰⁷⁴.
        let k = f[0];
        let bits = if k >= -1022 { ((k + 1023) as u64) << 52 } else { 1u64 << (k + 1074) };
        let x = f64::from_bits(bits);
        assert_eq!(host_float_parts(x), (1, f[1], f[2]), "2^{}", f[0]);
        checked += 1;
    }
    assert_eq!(checked, 2098);
}

#[test]
fn the_host_float_oracle_matches_the_digits_table() {
    for &(bits, class, digits, exp10) in FLOAT_DIGITS_TABLE {
        let x = f64::from_bits(bits);
        assert_eq!(host_float_parts(x), (class, digits, exp10), "{x:e}");
    }
    // The power-of-two case the shortest-length rule exists for.
    assert_eq!(
        host_float_parts(f64::from_bits(0x0060_0000_0000_0000)),
        (1, 7_120_236_347_223_045, -307)
    );
    let rows: Vec<String> = ieee_edges_expect().lines().map(str::to_owned).collect();
    assert_eq!(rows.len(), IEEE_EDGES.len());
    assert_eq!(rows[0], "2 0 0 0");
    assert_eq!(rows[3], "3 0 0");
    assert_eq!(rows[7], "0 0 0 1");
    assert_eq!(rows[14], "1 30000000000000004 -1 0");
}

#[test]
fn every_registry_op_has_cases() {
    let corpus = cases();
    let missing: Vec<&str> = STANDARD_OPS
        .iter()
        .map(|s| s.name)
        .filter(|name| !corpus.iter().any(|c| c.registry_ops().contains(name)))
        .collect();
    assert!(missing.is_empty(), "registry ops with no conformance case: {missing:?}");
    // Case names are how failures are reported; keep them unambiguous.
    let mut names: Vec<&str> = corpus.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), corpus.len(), "duplicate case names");
}

#[test]
fn every_lowered_op_runs_on_every_backend() {
    let mut compiled = 0usize;
    let mut executed = 0usize;
    for case in cases() {
        // Compile on every code-generation backend that lowers all the case's ops.
        for &backend in COMPILE_BACKENDS.iter().chain(std::iter::once(&Backend::native_host())) {
            if case.lowered_on(backend) {
                if let Err(e) = compile_on(backend, case.module()) {
                    panic!("{backend}: case `{}` declares Lowered but failed to compile: {e}", case.name);
                }
                compiled += 1;
            }
        }
        // Execute on every executable column that lowers them.
        let ran = execute_everywhere(&case);
        // The in-process columns never skip a case their declarations accept.
        for (column, backend) in [("Wasm", Backend::Wasm), ("Vm", Backend::Vm)] {
            if case.lowered_on(backend) {
                assert!(ran.contains(&column), "{column} skipped case `{}`", case.name);
            }
        }
        executed += ran.len();
    }
    eprintln!("LANG83 conformance: {compiled} compilations, {executed} executions");
    assert!(compiled > 0 && executed > 0);
}

#[test]
fn refused_ops_are_rejected_at_compile_time() {
    let mut refusals = 0usize;
    for case in cases() {
        for backend in Backend::ALL {
            let refused: Vec<&str> = case
                .registry_ops()
                .into_iter()
                .filter(|op| matches!(backend.op_support(op), OpSupport::Refused(_)))
                .collect();
            if refused.is_empty() {
                continue;
            }
            // 1. The shared check names exactly the refused ops.
            let err = check_module_for(backend, &case.module())
                .expect_err("a module using a refused op must not pass check_module_for");
            // A refusal is listed once per (function, op); a case whose
            // module has several functions (PR7f-2's Lisp helpers) can name
            // one op in more than one of them.
            let mut named: Vec<&str> = err.refusals.iter().map(|r| r.op).collect();
            named.sort_unstable();
            named.dedup();
            assert_eq!(named, refused, "{backend}: case `{}`", case.name);
            // 2. The backend's own compile entry point refuses it the same way,
            //    before any lowering pass. Only the host's native backend has a
            //    native entry point here; the other one is checked by step 1.
            let native = matches!(backend, Backend::X86_64 | Backend::Aarch64);
            if !native || backend == Backend::native_host() {
                match compile_on(backend, case.module()) {
                    Err(LangAotError::UnsupportedOps(e)) => assert_eq!(e.backend, backend),
                    other => panic!("{backend}: case `{}` must be refused with UnsupportedOps, got {other:?}", case.name),
                }
            }
            refusals += 1;
        }
    }
    // Since PR1b no backend refuses a PR1b op; the refusals left are the
    // generic JIT's interpreter-tier-only ops (strings, `f64` math,
    // conversions, `mod`, `print_str`) and BEAM's PR6c IEEE arithmetic, each
    // of which has a case. Derive the bound from the JIT's share rather than
    // pinning a count.
    let jit_refused_cases =
        cases().iter().filter(|c| c.registry_ops().iter().any(|op| !Backend::Jit.op_support(op).is_lowered())).count();
    assert!(jit_refused_cases > 0, "the JIT refuses interpreter-tier ops, so some case must exercise a refusal");
    assert!(refusals >= jit_refused_cases, "only {refusals} refusals exercised (JIT alone has {jit_refused_cases})");
}

#[test]
fn pr1b_ops_are_lowered_on_every_backend() {
    for spec in STANDARD_OPS.iter().filter(|s| s.is_pr1b()) {
        for backend in Backend::ALL {
            assert!(
                backend.op_support(spec.name).is_lowered(),
                "{backend} still refuses {}: {:?}",
                spec.name,
                backend.op_support(spec.name)
            );
        }
    }
}

/// LANG83 PR5a — strings fail closed. The gate:
///
/// 1. the support matrix the spec pins: the E4 string ops and `print_str`
///    are `Lowered` on every backend but the generic JIT, which refuses them
///    (`INTERPRETER_TIER_ONLY`) until PR5d;
/// 2. the registry: `str_index` / `str_slice` may trap `Trap.Bounds`, and
///    `str_concat` `Trap.Overflow`, and all three survive dead-code
///    elimination;
/// 3. every PR5a case runs, with the exact bytes before the trap and the
///    registered kind, on every executable column whose toolchain is present
///    (WASM and VM always; LLVM with `clang`; native x86-64 on Linux; JVM with
///    `java`), and compiles for CLR and BEAM, whose execution is CI's.
#[test]
fn pr5a_string_failures_are_registered_traps_on_every_column() {
    const STRING_OPS: [&str; 8] =
        ["str_const", "str_len", "str_index", "str_slice", "str_concat", "str_eq", "str_cmp", "print_str"];
    for op in STRING_OPS {
        for backend in Backend::ALL {
            let support = backend.op_support(op);
            if backend == Backend::Jit {
                assert!(matches!(support, OpSupport::Refused(_)), "the JIT refuses {op} until PR5d: {support:?}");
            } else {
                assert!(support.is_lowered(), "{backend} must lower {op}: {support:?}");
            }
        }
    }
    use interpreter_ir::opcodes::{has_side_effects, trap_kinds};
    assert_eq!(trap_kinds("str_index"), ["Trap.Bounds"]);
    assert_eq!(trap_kinds("str_slice"), ["Trap.Bounds"]);
    assert_eq!(trap_kinds("str_concat"), ["Trap.Overflow"]);
    for op in ["str_index", "str_slice", "str_concat"] {
        assert!(has_side_effects(op), "{op} must survive DCE");
    }

    let mut required = vec!["Wasm", "Vm"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let pr5a: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("(PR5a)")).collect();
    assert_eq!(pr5a.len(), 34, "the PR5a case table");
    let traps = pr5a.iter().filter(|c| c.expect.trap_kind().is_some()).count();
    assert_eq!(traps, 24, "twenty bounds edges, two unused reads, two ceilings");
    for case in pr5a {
        assert!(!case.lowered_on(Backend::Jit), "{}: the JIT refuses strings", case.name);
        let ran = execute_everywhere(case);
        for column in &required {
            assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
        }
        for backend in [Backend::Clr, Backend::Beam] {
            compile_on(backend, case.module()).unwrap_or_else(|e| panic!("{}: {backend}: {e}", case.name));
        }
        eprintln!("LANG83 PR5a `{}` proven on {ran:?}", case.name);
    }
}

/// LANG83 PR5b gate: strings are bytes on every column. Pins that every
/// string op stays lowered (the JIT refuses them until PR5d), then runs the
/// PR5b cases on every executable column whose toolchain is present (WASM,
/// VM always; LLVM with `clang`; native x86-64 on Linux; JVM with `java`)
/// and compiles them for CLR and BEAM, whose execution is CI's.
#[test]
fn pr5b_strings_are_bytes_on_every_column() {
    for op in ["str_const", "str_len", "str_index", "str_slice", "str_concat", "str_eq", "str_cmp", "print_str"] {
        for backend in Backend::ALL {
            let support = backend.op_support(op);
            if backend == Backend::Jit {
                assert!(matches!(support, OpSupport::Refused(_)), "the JIT refuses {op} until PR5d: {support:?}");
            } else {
                assert!(support.is_lowered(), "{backend} must lower {op}: {support:?}");
            }
        }
    }
    let mut required = vec!["Wasm", "Vm"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let pr5b: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("(PR5b)")).collect();
    assert_eq!(pr5b.len(), 14, "the PR5b case table");
    for case in pr5b {
        assert!(!case.lowered_on(Backend::Jit), "{}: the JIT refuses strings", case.name);
        let ran = execute_everywhere(case);
        for column in &required {
            assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
        }
        for backend in [Backend::Clr, Backend::Beam] {
            compile_on(backend, case.module()).unwrap_or_else(|e| panic!("{}: {backend}: {e}", case.name));
        }
        eprintln!("LANG83 PR5b `{}` proven on {ran:?}", case.name);
    }
}

/// Bare libc output must select byte mode too: no `print_str` or native runtime
/// is linked here to initialize stdout on behalf of `putchar`.
#[test]
fn putchar_preserves_lf_cr_nul_and_high_bytes_without_a_string_runtime() {
    let case = Case {
        name: "bare putchar is a byte stream",
        build: || program(vec![
            int("lf", 10), builtin("putchar", "lf"),
            int("cr", 13), builtin("putchar", "cr"),
            int("nul", 0), builtin("putchar", "nul"),
            int("high", 255), builtin("putchar", "high"),
        ]),
        expect: Expect::StdoutBytes(b"\n\r\x00\xff"),
    };
    let ran = execute_everywhere(&case);
    assert!(ran.contains(&"Wasm") && ran.contains(&"Vm"), "ran on {ran:?}");
    if clang_ok() {
        assert!(ran.contains(&"Llvm"), "ran on {ran:?}");
    }
}

#[test]
fn pr4_ops_are_lowered_on_every_backend() {
    let pr4: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr4()).map(|s| s.name).collect();
    assert_eq!(pr4.len(), 4, "the overflow family is i64_add/sub/mul/neg_ovf");
    for name in pr4 {
        for backend in Backend::ALL {
            assert!(
                backend.op_support(name).is_lowered(),
                "{backend} refuses {name}: {:?}",
                backend.op_support(name)
            );
        }
    }
}

/// LANG83 PR7e: PR6a's float writers are gone — from the registry, from
/// every backend's `OP_SUPPORT`, and from every compile entry point, which
/// now refuses a module that still calls one — and the PR6a table, printed
/// by the Python and JavaScript compilers' own layout helpers, runs (with
/// the PR6a strings) on every executable column whose toolchain is present.
/// BEAM compiles the finite cases and refuses the ones holding ±Infinity and
/// NaN constants.
#[test]
fn pr7e_float_writers_are_gone_and_the_layouts_run_everywhere() {
    for name in ["write_f64_py", "write_f64_js"] {
        assert!(lookup(name).is_none(), "{name} must not be a registry entry");
        assert!(language_name_violation(name).is_some(), "the language-name rule must catch {name}");
        for backend in Backend::ALL {
            assert!(!backend.op_support(name).is_lowered(), "{backend} still declares {name}");
        }
        // A module that still calls one is refused by every code-generation
        // backend (an unknown builtin), never compiled into a call nothing
        // defines; the VM has no such builtin either.
        let module = || program(vec![int("nl", 10), real("x", 1.5), builtin(name, "x"), builtin("putchar", "nl")]);
        for &backend in COMPILE_BACKENDS.iter().chain(std::iter::once(&Backend::native_host())) {
            assert!(compile_on(backend, module()).is_err(), "{backend} compiled a call to {name}");
        }
        assert!(matches!(run_vm(module()), RunResult::Trapped { kind: None, .. }), "Vm ran {name}");
    }
    let mut required = vec!["Wasm", "Vm", "Jit"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let pr7e: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("(PR7e)")).collect();
    assert_eq!(pr7e.len(), 6);
    let columns = [
        ("Wasm", Backend::Wasm),
        ("Vm", Backend::Vm),
        ("Jit", Backend::Jit),
        ("Llvm", Backend::Llvm),
        ("NativeAot", Backend::native_host()),
        ("Jvm", Backend::Jvm),
    ];
    let mut table_rows = 0;
    for case in pr7e {
        let table = case.name.contains("table value") || case.name.contains("infinities");
        let ran = execute_everywhere(case);
        for (column, backend) in columns {
            // Every PR7e case uses only ops every column but BEAM lowers. The
            // computed-float case's `int_to_real` once kept it off the JIT;
            // jit-core now lowers the LANG83 conversions, so it is held to
            // the JIT column like the rest.
            assert!(case.lowered_on(backend) || backend == Backend::Beam, "{}: {column}", case.name);
            if required.contains(&column) && case.lowered_on(backend) {
                assert!(ran.contains(&column), "{}: did not run on {column} (ran {ran:?})", case.name);
            }
        }
        match (case.lowered_on(Backend::Beam), case.has_non_finite_float_const(), compile_on(Backend::Beam, case.module())) {
            (true, _, Ok(())) => {}
            (false, true, Err(e)) => assert!(e.to_string().contains("non-finite float"), "{}: {e}", case.name),
            (false, false, Err(LangAotError::UnsupportedOps(e))) => assert_eq!(e.backend, Backend::Beam),
            (_, _, other) => panic!("{}: BEAM gave {other:?}", case.name),
        }
        if table {
            let Expect::Stdout(out) = case.expect else { unreachable!() };
            table_rows += out.lines().count();
        }
        eprintln!("LANG83 PR7e `{}` proven on {ran:?}", case.name);
    }
    // Every PR6a table value, in both layouts.
    assert_eq!(table_rows, 2 * FLOAT_TABLE.len());
}

/// LANG83 PR7e: no builtin either runner registers names a source language
/// (the runners also check this on every run).
#[test]
fn harness_builtins_name_no_language() {
    let mut vm = vm_core::VMCore::new();
    register_vm_output(&mut vm, &Sink::default());
    let names = vm.builtins_mut().names();
    assert!(names.contains(&"putchar") && !names.contains(&"write_f64_py"), "{names:?}");
    assert_runner_builtins_name_no_language("Vm", names);
}

/// LANG83 PR6c: the IEEE arithmetic is lowered on the eight IEEE backends and
/// refused on BEAM — both at `check_module_for` and by the BEAM compile entry
/// point — while `f64_signbit` is lowered on all nine; and every PR6c case runs
/// (with the expected output) on every executable column that can run it.
#[test]
fn pr6c_ops_follow_the_ieee_contract() {
    let pr6c: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr6c()).map(|s| s.name).collect();
    assert_eq!(pr6c, ["f64_add", "f64_sub", "f64_mul", "f64_div", "f64_neg", "f64_signbit"]);
    for name in &pr6c {
        for backend in Backend::ALL {
            let support = backend.op_support(name);
            if backend == Backend::Beam && *name != "f64_signbit" {
                assert!(matches!(support, OpSupport::Refused(_)), "BEAM must refuse {name}: {support:?}");
            } else {
                assert!(support.is_lowered(), "{backend} refuses {name}: {support:?}");
            }
        }
    }
    let mut required = vec!["Wasm", "Vm", "Jit"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let pr6c_cases: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("(PR6c)")).collect();
    assert_eq!(pr6c_cases.len(), 3);
    for case in pr6c_cases {
        let ran = execute_everywhere(case);
        for column in &required {
            assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
        }
        // BEAM: the arithmetic cases are refused before lowering; the
        // sign-bit case compiles.
        let arithmetic = case.registry_ops().iter().any(|op| *op != "f64_signbit" && pr6c.contains(op));
        let on_beam = compile_on(Backend::Beam, case.module());
        match (arithmetic, on_beam) {
            (true, Err(LangAotError::UnsupportedOps(e))) => assert_eq!(e.backend, Backend::Beam),
            (false, Ok(_)) => {}
            (_, other) => panic!("{}: BEAM gave {other:?}", case.name),
        }
        eprintln!("LANG83 PR6c `{}` proven on {ran:?}", case.name);
    }
}

/// LANG83 PR7b: all three neutral float primitives are lowered on all nine
/// backends; every PR7b case runs (with the expected output) on every
/// executable column whose toolchain is present; and BEAM compiles the finite
/// cases while refusing the one that holds ±Infinity and NaN constants.
#[test]
fn pr7b_ops_are_lowered_on_every_backend() {
    let pr7b: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr7b()).map(|s| s.name).collect();
    assert_eq!(pr7b, ["f64_class", "f64_digits", "f64_exp10"], "the PR7b primitives");
    for name in &pr7b {
        for backend in Backend::ALL {
            assert!(backend.op_support(name).is_lowered(), "{backend} refuses {name}: {:?}", backend.op_support(name));
        }
    }
    let mut required = vec!["Wasm", "Vm", "Jit"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let pr7b_cases: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("(PR7b)")).collect();
    assert_eq!(pr7b_cases.len(), 4);
    for case in pr7b_cases {
        let ran = execute_everywhere(case);
        for column in &required {
            assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
        }
        match (case.has_non_finite_float_const(), compile_on(Backend::Beam, case.module())) {
            (false, Ok(())) => {}
            (true, Err(e)) => assert!(e.to_string().contains("non-finite float"), "{}: {e}", case.name),
            (_, other) => panic!("{}: BEAM gave {other:?}", case.name),
        }
        eprintln!("LANG83 PR7b `{}` proven on {ran:?}", case.name);
    }
}

/// LANG83 PR7f-2: the Lisp-rule case's expected output is what
/// `dynval-runtime`'s Rust value model (the VM's `LispyValue`, the former C
/// runtime's twin) says — so the compiler-written rules agree with the
/// runtime rules they replace — and the case runs on LLVM and native x86-64
/// whenever they are present. No tagged-word backend lowers the old
/// language-semantic builtins any more.
#[test]
fn pr7f2_lisp_rules_are_module_code_over_the_primitives() {
    use dynval_runtime::{builtins, intern, LispyValue};
    let value = |name: &str| -> LispyValue {
        let pair = |a, b| builtins::cons(&[a, b]).expect("cons");
        let list = || pair(LispyValue::int(1), pair(LispyValue::int(2), LispyValue::NIL));
        match name {
            "i0" => LispyValue::int(0),
            "i7" => LispyValue::int(7),
            "nil" => LispyValue::NIL,
            "t" => LispyValue::TRUE,
            "f" => LispyValue::FALSE,
            "p12" | "p12b" => pair(LispyValue::int(1), LispyValue::int(2)),
            "p13" => pair(LispyValue::int(1), LispyValue::int(3)),
            "l" | "lb" => list(),
            "sa" | "sa2" => LispyValue::symbol(intern("A")),
            "sb" => LispyValue::symbol(intern("B")),
            other => panic!("{other}"),
        }
    };
    assert_eq!(LISP_RULE_VALUES.len(), 13);
    let word = |v: LispyValue| v.bits() as i64;
    let mut want = Vec::new();
    for x in LISP_RULE_TESTED {
        want.push(value(x).is_truthy() as i64);
        want.push(word(builtins::not(&[value(x)]).expect("not")));
    }
    for x in LISP_RULE_EXITS {
        let v = value(x);
        want.push(match v.as_int() {
            Some(n) => n,
            None if v.is_true() => 1,
            None => 0,
        });
    }
    for (x, y) in LISP_RULE_EQUAL {
        want.push(word(builtins::equal_p(&[value(x), value(y)]).expect("equal?")));
    }
    let want: Vec<String> = want.iter().map(i64::to_string).collect();
    let corpus = cases();
    let case = corpus.iter().find(|c| c.name.ends_with("(PR7f-2)")).expect("the PR7f-2 case");
    let Expect::Stdout(expected) = case.expect else { panic!("stdout case") };
    assert_eq!(expected, want.join("\n"), "the case's expectation is the Rust value model's");

    let mut required = Vec::new();
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    let ran = execute_everywhere(case);
    for column in &required {
        assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
    }
    // The rules are module code: the LLVM module defines `__lisp_equal` and
    // `__lisp_exit_code` and calls no runtime rule.
    let ll = lang_aot::compile_iir_to_llvm(case.module(), "opconf", "x86_64-unknown-linux-gnu").expect("llvm");
    for defined in ["@__lisp_equal(", "@__lisp_exit_code("] {
        // `internal`: only the entry point is exported (#18416).
        assert!(ll.contains(&format!("define internal i64 {defined}")), "{defined} is defined in the module");
    }
    for gone in ["__dyn_truthy", "__dyn_not", "__dyn_equal", "__dyn_to_exit_code"] {
        assert!(!ll.contains(gone), "{gone} is no longer a runtime call");
    }
    eprintln!("LANG83 PR7f-2 `{}` proven on {ran:?}", case.name);
}

/// LANG83 PR7f: the dynamic-value primitives are calls into the tagged-word
/// runtime, so they are lowered where that runtime is linked (native x86-64
/// and AArch64, LLVM) and refused, with the shared reason, by the other
/// compiled backends; `dyn_symbol` is refused there too (a symbol is an
/// interned constant on the tagged-word backends). The VM is the exception
/// (#18470): it lowers the five primitives that read or make a value's kind
/// (`dyn_tag`, `dyn_is_nil`, `dyn_is_pair`, `dyn_nil`, `dyn_symbol`) over its
/// own `Value` kinds, and still refuses the five that build or take apart a
/// tagged word or a cell. The pair case runs, with the expected output, on
/// LLVM and native x86-64 whenever their toolchains are present, and every
/// other backend — the VM included, for `dyn_box_int` — refuses it before
/// any lowering pass.
#[test]
fn pr7f_dynamic_values_run_on_the_tagged_word_backends() {
    use interpreter_ir::op_registry::{NO_TAGGED_WORD_RUNTIME, SYMBOLS_ARE_INTERNED_CONSTANTS};
    let pr7f: Vec<&str> = STANDARD_OPS.iter().filter(|s| s.is_pr7f()).map(|s| s.name).collect();
    assert_eq!(pr7f.len(), 10, "the PR7f primitives: {pr7f:?}");
    let tagged = [Backend::X86_64, Backend::Aarch64, Backend::Llvm];
    let vm_kind_primitives = ["dyn_tag", "dyn_is_nil", "dyn_is_pair", "dyn_nil", "dyn_symbol"];
    for name in &pr7f {
        for backend in Backend::ALL {
            let want = match (tagged.contains(&backend), *name) {
                (true, "dyn_symbol") => OpSupport::Refused(SYMBOLS_ARE_INTERNED_CONSTANTS),
                (true, _) => OpSupport::Lowered,
                (false, name) if backend == Backend::Vm && vm_kind_primitives.contains(&name) => {
                    OpSupport::Lowered
                }
                (false, _) => OpSupport::Refused(NO_TAGGED_WORD_RUNTIME),
            };
            assert_eq!(backend.op_support(name), want, "{backend} {name}");
        }
    }
    let corpus = cases();
    let case = corpus.iter().find(|c| c.name.starts_with("dynamic-value pairs")).expect("the PR7f case");
    let mut required = Vec::new();
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    let ran = execute_everywhere(case);
    for column in &required {
        assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
    }
    for backend in [Backend::Wasm, Backend::Jvm, Backend::Clr, Backend::Beam, Backend::Vm, Backend::Jit] {
        match compile_on(backend, case.module()) {
            Err(LangAotError::UnsupportedOps(e)) => {
                assert_eq!(e.backend, backend);
                assert!(e.to_string().contains("tagged-word runtime"), "{backend}: {e}");
            }
            other => panic!("{backend}: the PR7f case must be refused, got {other:?}"),
        }
    }
    eprintln!("LANG83 PR7f `{}` proven on {ran:?}", case.name);
}

/// The PR7b review regression really exercises the hazard: its LLVM module
/// references the native runtime (so `run_llvm` links `twig_runtime.c` into
/// it), and every float helper the module defines is `internal`, never an
/// interposable global; and the case runs on the LLVM column when `clang` is
/// present.
#[test]
fn llvm_float_helpers_cannot_be_interposed_by_the_native_runtime() {
    let corpus = cases();
    let case = corpus.iter().find(|c| c.name.ends_with("(PR7b review)")).expect("the regression case");
    let ll = lang_aot::compile_iir_to_llvm(case.module(), "opconf", "x86_64-unknown-linux-gnu").expect("compiles");
    assert!(ll.contains("@__twig_"), "the module must make run_llvm link the native runtime");
    for name in ["__iir_f64_digits", "__iir_f64_shortest_digits", "__iir_f64_shortest_exp10"] {
        let def = ll.lines().find(|l| l.starts_with("define") && l.contains(&format!("@{name}("))).unwrap_or_else(|| panic!("{name} defined"));
        assert!(def.starts_with("define internal "), "{name} must not be interposable: {def}");
    }
    // No runtime symbol may take a name the LLVM module defines.
    let runtime = include_str!("../../twig-aot/runtime/twig_runtime.c");
    let code_lines = runtime.lines().filter(|l| {
        let t = l.trim_start();
        !(t.starts_with('*') || t.starts_with("/*") || t.starts_with("//"))
    });
    for line in code_lines {
        assert!(!line.contains("__iir_"), "twig_runtime.c must not define `__iir_*` names: {line}");
    }
    let ran = execute_everywhere(case);
    if clang_ok() {
        assert!(ran.contains(&"Llvm"), "ran on {ran:?}");
    }
}

/// The PR7b table is the PR6a table's values, in order, and each row is
/// what the checked-in Python string says: its class, and its digits and
/// leading exponent once the point, the exponent and the zeros are removed.
#[test]
fn float_digits_table_follows_the_float_table() {
    assert_eq!(FLOAT_DIGITS_TABLE.len(), FLOAT_TABLE.len());
    for (&(bits, c, d, e), &(fbits, py, _)) in FLOAT_DIGITS_TABLE.iter().zip(FLOAT_TABLE) {
        assert_eq!(bits, fbits);
        let r = py.trim_start_matches('-');
        let want = match r {
            "inf" => (2, 0, 0),
            "nan" => (3, 0, 0),
            "0.0" => (0, 0, 0),
            _ => {
                let (mant, exp) = r.split_once('e').map_or((r, 0), |(m, x)| (m, x.parse::<i64>().unwrap()));
                let (int_part, frac) = mant.split_once('.').unwrap_or((mant, ""));
                let all = format!("{int_part}{frac}");
                let lead_zeros = (all.len() - all.trim_start_matches('0').len()) as i64;
                let digits = all.trim_matches('0');
                (1, digits.parse().unwrap(), int_part.len() as i64 - 1 + exp - lead_zeros)
            }
        };
        assert_eq!((c, d, e), want, "{py}");
        // The definition: |x| = D × 10^(e10 − n + 1).
        if c == 1 {
            let n = d.to_string().len() as i64;
            let back: f64 = format!("{d}e{}", e - n + 1).parse().unwrap();
            assert_eq!(back.to_bits(), f64::from_bits(bits).abs().to_bits(), "{py}");
        }
    }
}

/// The PR7b oracle check: every row is what host `python3` gives, re-derived
/// from `repr` whenever `python3` is installed (CI installs it).
#[test]
fn float_digits_table_matches_host_python3() {
    let hexes: Vec<String> = FLOAT_DIGITS_TABLE.iter().map(|(b, ..)| format!("{b:016x}")).collect();
    let python = r#"import struct, sys
def split(x):
    if x != x: return (3, 0, 0)
    if x in (float('inf'), float('-inf')): return (2, 0, 0)
    if x == 0: return (0, 0, 0)
    m, _, e = repr(abs(x)).partition('e')
    ip, _, fp = m.partition('.')
    s = (ip + fp).lstrip('0')
    return (1, int(s.rstrip('0')), len(ip) - 1 + (int(e) if e else 0) - (len(ip + fp) - len(s)))
for h in sys.argv[1:]: print(*split(struct.unpack('<d', bytes.fromhex(h)[::-1])[0]))
"#;
    if let Ok(out) = Command::new("python3").arg("-c").arg(python).args(&hexes).output() {
        if out.status.success() {
            let got: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect();
            let want: Vec<String> = FLOAT_DIGITS_TABLE.iter().map(|(_, c, d, e)| format!("{c} {d} {e}")).collect();
            assert_eq!(got, want, "python3's digits drifted from the checked-in table");
            eprintln!("LANG83 PR7b: python3 agrees on {} values", got.len());
        }
    }
}

/// A failed `real_to_int_*` narrowing — NaN, ±∞, 2⁶³, −1e30 — raises the
/// typed `Trap.ConvRange` (id 4) after the output written before it, on every
/// executable column whose toolchain is present (and always on WASM, VM and
/// JIT). CLR and BEAM compile here and execute in CI's `lang_matrix`
/// columns. BEAM is given only the finite rows, since its floats are never
/// NaN or ±∞, and must compile them.
#[test]
fn failed_narrowing_traps_conv_range_on_every_executable_column() {
    let mut required = vec!["Wasm", "Vm", "Jit"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    let corpus = cases();
    let conv: Vec<&Case> = corpus.iter().filter(|c| c.name.ends_with("traps Trap.ConvRange")).collect();
    assert_eq!(conv.len(), 10, "five values × two narrowing ops");
    for case in conv {
        assert!(matches!(case.expect, Expect::StdoutThenTrap("1", "Trap.ConvRange")), "{}", case.name);
        let ran = execute_everywhere(case);
        for column in &required {
            assert!(ran.contains(column), "{}: did not run on {column} (ran {ran:?})", case.name);
        }
        compile_on(Backend::Clr, case.module()).unwrap_or_else(|e| panic!("{}: CLR: {e}", case.name));
        let finite = !case.has_non_finite_float_const();
        assert_eq!(case.lowered_on(Backend::Beam), finite, "{}", case.name);
        if finite {
            compile_on(Backend::Beam, case.module()).unwrap_or_else(|e| panic!("{}: BEAM: {e}", case.name));
        }
        eprintln!("Trap.ConvRange `{}` proven on {ran:?}", case.name);
    }
}

/// The PR6a oracle check: every checked-in expected string is what host
/// `python3` and `node` print for that exact double. A tool that is not
/// installed is skipped (CI installs both); a mismatch fails.
#[test]
fn float_table_matches_host_python3_and_node() {
    let hexes: Vec<String> = FLOAT_TABLE.iter().map(|(b, _, _)| format!("{b:016x}")).collect();
    let python = "import struct,sys\nfor h in sys.argv[1:]: print(repr(struct.unpack('<d', bytes.fromhex(h)[::-1])[0]))";
    if let Ok(out) = Command::new("python3").arg("-c").arg(python).args(&hexes).output() {
        if out.status.success() {
            let got: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect();
            let want: Vec<&str> = FLOAT_TABLE.iter().map(|(_, py, _)| *py).collect();
            assert_eq!(got, want, "python3 repr drifted from the checked-in table");
            eprintln!("LANG83 PR6a: python3 agrees on {} values", got.len());
        }
    }
    let node = "const b=Buffer.alloc(8);for(const h of process.argv.slice(1)){b.writeBigUInt64BE(BigInt('0x'+h));console.log(String(b.readDoubleBE(0)));}";
    if let Ok(out) = Command::new("node").arg("-e").arg(node).args(&hexes).output() {
        if out.status.success() {
            let got: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect();
            let want: Vec<&str> = FLOAT_TABLE.iter().map(|(_, _, js)| *js).collect();
            assert_eq!(got, want, "node String(x) drifted from the checked-in table");
            eprintln!("LANG83 PR6a: node agrees on {} values", got.len());
        }
    }
}

#[test]
fn flush_before_trap_is_proven_on_every_executable_column() {
    let corpus = cases();
    let traps: Vec<&Case> = corpus
        .iter()
        .filter(|c| c.expect.trap_kind().is_some() && !c.expect.out_text().is_empty())
        .collect();
    assert!(!traps.is_empty());
    let mut proven: Vec<&str> = Vec::new();
    for case in traps {
        for column in execute_everywhere(case) {
            if !proven.contains(&column) {
                proven.push(column);
            }
        }
    }
    // In-process columns never skip; toolchain columns only when the tool is absent.
    let mut required = vec!["Wasm", "Vm", "Jit"];
    if clang_ok() {
        required.push("Llvm");
    }
    if cfg!(target_os = "linux") {
        required.push("NativeAot");
    }
    if java_ok() {
        required.push("Jvm");
    }
    for column in required {
        assert!(proven.contains(&column), "flush-before-trap not proven on {column} (proven: {proven:?})");
    }
    eprintln!("LANG83 flush-before-trap proven on {proven:?}");
}

#[test]
fn print_i64_and_putchar_share_one_ordered_stream() {
    let corpus = cases();
    let case = corpus
        .iter()
        .find(|c| c.registry_ops() == ["print_i64", "putchar"])
        .expect("the ordered-stream case must stay in the corpus");
    let ran = execute_everywhere(case);
    for column in ["Wasm", "Vm", "Jit"] {
        assert!(ran.contains(&column), "{column} must run the ordered-stream case");
    }
    eprintln!("LANG83 print_i64 + putchar ordering proven on {ran:?}");
}

/// BEAM refuses a float-table module that holds `inf`/`nan` at compile time
/// instead of building a `.beam` whose literal table no Erlang loader reads.
/// The finite table, helpers included, still compiles.
#[test]
fn beam_refuses_non_finite_float_constants() {
    for layout in [Layout::Python, Layout::JavaScript] {
        match compile_on(Backend::Beam, float_table_program(layout, non_finite)) {
            Err(e) => assert!(e.to_string().contains("non-finite float"), "{layout:?}: {e}"),
            Ok(()) => panic!("{layout:?}: BEAM compiled a module holding inf/nan constants"),
        }
        compile_on(Backend::Beam, float_table_program(layout, finite))
            .unwrap_or_else(|e| panic!("{layout:?}: finite float constants compile on BEAM: {e}"));
    }
    let digits = program(vec![real("x", 1.5), op("f64_digits", "d", &["x"], "i64"), builtin("write_i64", "d")]);
    compile_on(Backend::Beam, digits).expect("a finite digit query compiles on BEAM");
}
