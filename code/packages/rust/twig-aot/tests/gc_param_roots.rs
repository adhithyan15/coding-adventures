//! # Heap-valued parameters stay GC roots (#18492) — end to end on Linux x86-64
//!
//! A Twig parameter is declared `any`: one 64-bit *tagged word* that is an
//! integer as often as it is a pair, a closure or a string. Type propagation
//! reads it as `i64` (signed comparisons), and until #18492 the param was
//! **retyped** `i64` in place. The native backends build each function's GC
//! stack map from the param types, and `i64` means "never a reference", so the
//! frame slot holding the param was missing from every stack map.
//!
//! ## What that broke, and what it did not
//!
//! The native runtime has two ways of finding stack roots:
//!
//! | collection                         | how stack roots are found               | moves? | param slot |
//! |------------------------------------|-----------------------------------------|--------|------------|
//! | allocation-triggered (`__gc_alloc_kind` past the 1 MiB threshold) | conservative scan of the whole `[sp, base)` stack + spilled callee-saved registers | no | scanned like every other word — safe |
//! | `gc_collect_precise`               | frame-pointer walk + per-frame stack maps | no   | skipped, but the **caller's** `any` temporary still roots the object — safe |
//! | `gc_collect_compacting` (and `__gc_safepoint` when fragmentation says compact) | same precise walk | **yes** | skipped: the object moves, the caller's slot is rewritten, the callee's param slot still points at the freed from-space copy — **use-after-free** |
//!
//! Twig source cannot name the precise or compacting collector today (no
//! builtin, and nothing emits `gc_safepoint`), so a Twig *program* only ever
//! gets the conservative row. The tests below therefore:
//!
//! 1. prove the conservative row really runs under GC pressure in a pure Twig
//!    program (the mechanism that has been keeping parameters alive), and
//! 2. drive the precise and compacting rows by splicing the collection builtin
//!    into the compiled IIR of the callee, exactly where `(alloc-a-lot)` sits
//!    in the issue's reproduction.
//!
//! Before the fix the compacting cases exit with a value read from a *different*
//! pair (the freed cell was handed straight back to the next `cons`), and under
//! valgrind memcheck `__dyn_pair_first` reports an invalid read of a block freed
//! by `FlatHeap::sweep` inside `collect_compacting`.

#![cfg(target_os = "linux")]

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use std::process::{Command, Output};

/// Allocation helpers, written without integer recursion so they stay clear of
/// the unrelated boxed/raw representation bugs tracked in #18480: `(g64)` conses
/// a fresh 64-element list (64 pairs), and each `wN` calls the level below four
/// times, so `(churn)` allocates 4 × 4⁵ × 64 = 262 144 pairs — 4 MiB of payload,
/// four times the collector's 1 MiB initial threshold.
const CHURN: &str = "\
(define (g64) (list 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 \
33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64))
(define (w4) (begin (g64) (g64) (g64) (g64)))
(define (w16) (begin (w4) (w4) (w4) (w4)))
(define (w64) (begin (w16) (w16) (w16) (w16)))
(define (w256) (begin (w64) (w64) (w64) (w64)))
(define (w1024) (begin (w256) (w256) (w256) (w256)))
(define (churn) (begin (w1024) (w1024) (w1024) (w1024)))
";

/// Payload bytes `(churn)` allocates: 262 144 pairs × 16 bytes.
const CHURN_BYTES: i64 = 262_144 * 16;

/// The issue's shape: the callee's parameter is the only reference *it* has to
/// the pair, it allocates, then it reads the pair.
const PARAM_PROGRAM: &str = "(define (f p) (begin (w4) (car p)))\n(f (cons 7 8))\n";

/// The closure shape: `p` is captured, and reaches the lifted lambda body as one
/// of its parameters when `__dyn_call_closure` dispatches the call.
const CAPTURE_PROGRAM: &str =
    "(define (g p) (lambda () (begin (w4) (car p))))\n((g (cons 7 8)))\n";

/// Copies of a heap word in a *local*, not just in the parameter itself. Each
/// is a `mov` that type propagation (or `default_any_to_i64`) types `i64`, so
/// before the follow-up fix the copy's slot was missing from the stack map
/// even once the parameter's was named; a compaction rewrote the original and
/// left the copy pointing at the freed cell. The reading function reads the
/// copy, never the original.
///
/// | tag            | the copy                                          | `mov` typed `i64` by        |
/// |----------------|---------------------------------------------------|-----------------------------|
/// | `let_copy`     | `(let ((q p)) …)`                                 | propagation from `p`        |
/// | `copy_chain`   | `a ← p`, `b ← a`, `c ← b`                         | propagation, three times    |
/// | `call_result`  | `(let ((x (f p 0))) …)`, a recursive call's result | `default_any_to_i64` (the call's own type is unknown while `f` is being compiled) |
/// | `join_int`     | `(if (pair? c) p 0)` — both arms are `i64` movs   | propagation / literal       |
/// | `join_pair`    | `(if (pair? c) p (cons 9 9))`                     | (rooted already: the other arm's `mov` is `any`, so the shared slot was declared — a guard) |
const COPY_PROGRAMS: [(&str, &str); 5] = [
    ("let_copy", "(define (f p) (let ((q p)) (begin (w4) (car q))))\n(f (cons 7 8))\n"),
    (
        "copy_chain",
        "(define (f p) (let ((a p)) (let ((b a)) (let ((c b)) (begin (w4) (car c))))))\n\
         (f (cons 7 8))\n",
    ),
    (
        "call_result",
        "(define (f p n) (if (pair? n) (let ((x (f p 0))) (begin (w4) (car x))) p))\n\
         (f (cons 7 8) (cons 1 1))\n",
    ),
    (
        "join_int",
        "(define (f p c) (let ((x (if (pair? c) p 0))) (begin (w4) (car x))))\n\
         (f (cons 7 8) (cons 1 1))\n",
    ),
    (
        "join_pair",
        "(define (f p c) (let ((x (if (pair? c) p (cons 9 9)))) (begin (w4) (car x))))\n\
         (f (cons 7 8) (cons 1 1))\n",
    ),
];

/// Every shape the compacting tests drive: the parameter itself, a captured
/// value, and the local copies above.
fn compacting_shapes() -> Vec<(&'static str, &'static str)> {
    let mut v = vec![("param", PARAM_PROGRAM), ("captured", CAPTURE_PROGRAM)];
    v.extend(COPY_PROGRAMS);
    v
}

fn compile(src: &str) -> IIRModule {
    twig_ir_compiler::compile_source(&format!("{CHURN}{src}"), "gc_param_roots")
        .unwrap_or_else(|e| panic!("compile: {e}"))
}

/// The function holding `(car <param>)` — `f`, or the lifted lambda body.
fn reader_of_car(m: &mut IIRModule) -> &mut interpreter_ir::function::IIRFunction {
    m.functions
        .iter_mut()
        .find(|f| f.instructions.iter().any(is_car))
        .expect("a function reads (car p)")
}

fn is_car(i: &IIRInstr) -> bool {
    i.op == "call_builtin" && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "car")
}

/// Splice `collect` (a collection builtin) into the function that reads `car p`,
/// right before the read — after the callee's own allocation, where the issue's
/// `(alloc-a-lot)` triggers it. With `reuse`, follow it with a second call to the
/// function's allocating helper (`(w4)`, 256 fresh pairs): the allocator hands a
/// just-freed 16-byte cell straight back, so a stale `p` reads someone else's
/// pair instead of happening to find the old bytes intact.
fn splice_collection(m: &mut IIRModule, collect: &str, reuse: bool) {
    let f = reader_of_car(m);
    let at = f.instructions.iter().position(is_car).unwrap();
    let mut extra = vec![IIRInstr::new(
        "call_builtin",
        Some("__gc_freed".into()),
        vec![Operand::Var(collect.into())],
        "i64",
    )];
    if reuse {
        let mut again = f
            .instructions
            .iter()
            .find(|i| i.op == "call" && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "w4"))
            .cloned()
            .expect("the callee calls its allocating helper (w4)");
        again.dest = Some("__reuse".into());
        extra.push(again);
    }
    for (k, instr) in extra.into_iter().enumerate() {
        f.instructions.insert(at + k, instr);
    }
    // The splice shifts positions; drop the positional source map rather than
    // let it point at the wrong instructions.
    f.source_map.clear();
}

/// Splice `print_i64(gc_live_bytes())` before the `car` read: an observer that
/// allocates nothing and collects nothing.
fn splice_live_bytes_probe(m: &mut IIRModule) {
    let f = reader_of_car(m);
    let at = f.instructions.iter().position(is_car).unwrap();
    f.instructions.insert(
        at,
        IIRInstr::new(
            "call_builtin",
            Some("__live".into()),
            vec![Operand::Var("gc_live_bytes".into())],
            "i64",
        ),
    );
    f.instructions.insert(
        at + 1,
        IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print_i64".into()), Operand::Var("__live".into())],
            "void",
        ),
    );
    f.source_map.clear();
}

fn build(m: &IIRModule, dir: &tempfile::TempDir, tag: &str) -> std::path::PathBuf {
    let exe = dir.path().join(tag);
    twig_aot::compile_module_to_linux_executable(m, &exe)
        .unwrap_or_else(|e| panic!("{tag}: compile+link: {e}"));
    exe
}

fn run(exe: &std::path::Path) -> Output {
    Command::new(exe).output().unwrap_or_else(|e| panic!("launch {}: {e}", exe.display()))
}

fn assert_reads_seven(out: &Output, what: &str) {
    assert_eq!(
        out.status.code(),
        Some(7),
        "{what}: `(car p)` must still read 7 (a different value or a signal means the \
         parameter's slot was not a root and went stale); stderr={:?}",
        String::from_utf8_lossy(&out.stderr),
    );
}

/// Mechanism check (passes before and after #18492): a pure Twig program whose
/// callee allocates 4 MiB while its parameter holds the only reference the
/// callee has to a pair. The allocation-triggered collections are conservative
/// and non-moving, so the parameter's slot is scanned like any other stack word.
/// The probe proves collections actually ran: far less than the 4 MiB allocated
/// is still live when `car p` executes.
#[test]
fn twig_param_survives_allocation_triggered_collections() {
    let dir = tempfile::tempdir().unwrap();
    for (tag, src) in [
        ("param", "(define (f p) (begin (churn) (car p)))\n(f (cons 7 8))\n"),
        ("returned", "(define (mk) (cons 7 8))\n(define (f p) (begin (churn) (car p)))\n(f (mk))\n"),
        ("captured", "(define (g p) (lambda () (begin (churn) (car p))))\n((g (cons 7 8)))\n"),
    ] {
        let plain = build(&compile(src), &dir, tag);
        assert_reads_seven(&run(&plain), tag);

        let mut probed = compile(src);
        splice_live_bytes_probe(&mut probed);
        let out = run(&build(&probed, &dir, &format!("{tag}_probe")));
        assert_reads_seven(&out, &format!("{tag} (probed)"));
        let live: i64 = String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("{tag}: probe printed {:?}: {e}", out.stdout));
        assert!(
            live < CHURN_BYTES / 2,
            "{tag}: {live} live bytes after allocating {CHURN_BYTES}: no collection ran, so \
             this test is not exercising the collector",
        );
    }
}

/// A precise, **non-moving** collection inside the callee. The callee's own slot
/// for `p` was unrooted before the fix, but the caller's `any` temporary for
/// `(cons 7 8)` is a root, so the pair survived; after the fix both are roots.
#[test]
fn param_survives_precise_collection_inside_callee() {
    let dir = tempfile::tempdir().unwrap();
    for (tag, src) in compacting_shapes() {
        for reuse in [false, true] {
            let mut m = compile(src);
            splice_collection(&mut m, "gc_collect_precise", reuse);
            let exe = build(&m, &dir, &format!("precise_{tag}_{reuse}"));
            assert_reads_seven(&run(&exe), &format!("precise {tag} reuse={reuse}"));
        }
    }
}

/// **The #18492 reproduction.** A compacting collection inside the callee moves
/// the pair (its only *precise* root is the caller's temporary, which the
/// collector rewrites) and frees the old cell. Before the fix the callee's `p`
/// still pointed at that freed cell, the next `cons` reused it, and `(car p)`
/// read 51 — an element of one of `(w4)`'s lists — instead of 7. Now `p`'s slot
/// is a root, so the collector rewrites it too.
#[test]
fn param_survives_compacting_collection_inside_callee() {
    let dir = tempfile::tempdir().unwrap();
    // Every shape runs before the verdict, so one failure does not hide another.
    let mut failures = Vec::new();
    for (tag, src) in compacting_shapes() {
        for reuse in [false, true] {
            let mut m = compile(src);
            splice_collection(&mut m, "gc_collect_compacting", reuse);
            let exe = build(&m, &dir, &format!("compacting_{tag}_{reuse}"));
            let out = run(&exe);
            if out.status.code() != Some(7) {
                failures.push(format!("{tag} reuse={reuse}: exit {:?}", out.status.code()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "`(car p)` must read 7 after a compaction inside the callee; a different value or \
         a signal means a slot holding the pair was not a root and went stale: {failures:?}",
    );
}

/// The same compacting case under valgrind memcheck, which flags the stale read
/// even when the freed bytes happen to be intact (`reuse = false`). Skipped when
/// valgrind is not installed. Only invalid reads/writes are checked: memcheck
/// also reports the precise walk reading not-yet-written frame slots (every
/// reference slot is named at every safepoint), which is a known imprecision
/// and not a use-after-free.
#[test]
fn compacting_param_read_is_clean_under_memcheck() {
    let has_valgrind = Command::new("valgrind")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_valgrind {
        eprintln!("valgrind not installed; skipping the memcheck run");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (tag, src) in compacting_shapes() {
        let mut m = compile(src);
        splice_collection(&mut m, "gc_collect_compacting", false);
        let exe = build(&m, &dir, &format!("memcheck_{tag}"));
        let out = Command::new("valgrind").arg("--quiet").arg(&exe).output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("Invalid read") || stderr.contains("Invalid write") {
            failures.push(format!("{tag}: memcheck saw an access to freed memory"));
        } else if out.status.code() != Some(7) {
            failures.push(format!("{tag}: exit {:?}", out.status.code()));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}
