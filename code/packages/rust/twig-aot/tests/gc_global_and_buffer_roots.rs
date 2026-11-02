//! # Globals and string buffers stay GC roots (#18765, #18766) — end to end on Linux x86-64
//!
//! `gc_param_roots.rs` (#18492) proved that every *stack slot* that may hold a
//! heap word is named by the function's stack map. Two other places hold heap
//! words, and neither was a root for the precise or the compacting collector:
//!
//! | where the word lives                     | issue  | why it was missed                                                                 |
//! |------------------------------------------|--------|-----------------------------------------------------------------------------------|
//! | a top-level `define` (a `.data` slot)    | #18765 | the collector only looked at the machine stack; `_twig_globals` is not on it      |
//! | an `alloc_bytes` string buffer (a local) | #18766 | the buffer's CIR type was `i64`, so its slot (and every copy of it) was left out of the stack map |
//!
//! ## How the tests drive the collector
//!
//! Twig source cannot call the precise or compacting collector, so — exactly as
//! `gc_param_roots.rs` does — the tests compile a Twig program and **splice** a
//! `gc_collect_precise` / `gc_collect_compacting` call into its IIR, right
//! before the instruction that reads the value back. With `reuse`, a second
//! call to the program's allocating helper `(w4)` follows the collection: the
//! allocator then hands a just-freed cell straight back, so a stale handle reads
//! someone else's bytes instead of happening to find the old ones intact.
//!
//! ## The global cases
//!
//! ```text
//!   main:  r  = cons 7 8          ; r's frame slot is a root (stack map)
//!          global_store g, r      ; the .data slot was NOT a root
//!          call f
//!   f:     <collection spliced here>
//!          x = global_load g      ; stale after a compaction / freed after a precise GC
//!          car x
//! ```
//!
//! `main`'s own slot for `r` still holds the pair for the whole program (frame
//! slots are never reused). Before the fix a **compacting** collection moved the
//! pair and rewrote that slot, but not the global, so `(car g)` read the freed
//! from-space cell; with `reuse` it read an element of one of `(w4)`'s lists
//! (`keep_main` rows). The `sole` rows also clear `main`'s copy right after the
//! store — what a liveness pass (the §4 refinement rung of
//! `AOT00-T1-stackmap-emission.md`) would do — so the global is the only live
//! reference; they fail the same way.
//!
//! The **precise** (non-moving) rows and the allocation-triggered test pass
//! before the fix too, by luck that cannot be removed from a Twig program: the
//! stack map names every reference slot of `f` at every safepoint (rule R1), and
//! `f`'s not-yet-written slots still hold stale words from `main`'s earlier call
//! into the allocator — one of them the pair's address. That keeps the pair
//! alive; it does not keep it in place, which is why compaction exposes the bug.
//! They stay as guards. The deterministic proof that a registered global slot is
//! a root of the precise and conservative collectors is gc-core-capi's
//! `registered_global_slots_are_roots_of_every_collector` unit test, which parks
//! objects where no stack word can reach them.
//!
//! ## The buffer cases
//!
//! Twig string literals reach twig-aot as `const Str(..)`, which its string
//! lowering does not handle (it expects the `str_const` that BASIC / ALGOL emit
//! through `lang-aot`). [`as_runtime_strings`] rewrites the IIR into that shape:
//! each branch of an `if` writes the join variable with its own `str_const`, so
//! the variable is a *runtime* string, lowered to an `alloc_bytes` buffer plus a
//! `mov` alias, and `string-length` reads the buffer's length header at run time.
//! Before the fix the buffer's slot and the alias were both typed `i64`, so a
//! compaction or precise collection freed the buffer and `string-length` read a
//! reused cell.

#![cfg(target_os = "linux")]

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use std::collections::HashMap;
use std::process::{Command, Output};

/// `(w4)` conses four fresh 64-element lists (256 pairs, 4 KiB of payload) —
/// the reuse step. `(churn)` allocates 4 MiB, four times the collector's
/// 1 MiB initial threshold, so allocation-triggered (conservative) collections
/// certainly run. Same helpers as `gc_param_roots.rs`.
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

/// Globals whose value is read back, after the spliced collection, by `f`.
///
/// | tag     | the global                         | `f` reads          |
/// |---------|------------------------------------|--------------------|
/// | `pair`  | a pair                             | `(car g)`          |
/// | `list`  | a list whose head is a pair        | `(car (car g))` — the inner pair is reached *through* the global, so a compaction moves both and must rewrite the global and the list cell |
/// | `two`   | two globals; the second is the pair | `(car h)` — the root table covers every slot, not just slot 0 |
const GLOBAL_PROGRAMS: [(&str, &str); 3] = [
    ("pair", "(define g (cons 7 8))\n(define (f) (begin (w4) (car g)))\n(f)\n"),
    ("list", "(define g (list (cons 7 8) 2))\n(define (f) (begin (w4) (car (car g))))\n(f)\n"),
    (
        "two",
        "(define k 5)\n(define h (cons 7 8))\n(define (f) (begin (w4) (+ (car h) (- k 5))))\n(f)\n",
    ),
];

/// String programs whose length is read back after the spliced collection.
/// `c` is always a pair, so the first arm is taken and the answer is 7.
///
/// | tag         | the string read by `string-length`                      | lowered to                                   |
/// |-------------|---------------------------------------------------------|----------------------------------------------|
/// | `literal`   | the `if` join of two literals                            | `alloc_bytes` buffer + `mov` alias (`i64`)    |
/// | `concat`    | `(string-append s "lo!!")`, `s` a runtime join           | `call_builtin str_concat` (`__twig_str_concat`) |
/// | `substring` | `(substring s 0 7)`, `s` a runtime join                  | `call_builtin str_slice` (`__twig_str_slice`) |
const BUFFER_PROGRAMS: [(&str, &str); 3] = [
    (
        "literal",
        "(define (f c) (let ((s (if (pair? c) \"hello!!\" \"x\"))) (begin (w4) (string-length s))))\n\
         (f (cons 1 1))\n",
    ),
    (
        "concat",
        "(define (f c) (let ((s (if (pair? c) \"hel\" \"x\"))) \
         (let ((t (string-append s \"lo!!\"))) (begin (w4) (string-length t)))))\n\
         (f (cons 1 1))\n",
    ),
    (
        "substring",
        "(define (f c) (let ((s (if (pair? c) \"hello!!!!\" \"xxxxxxxxx\"))) \
         (let ((t (substring s 0 7))) (begin (w4) (string-length t)))))\n\
         (f (cons 1 1))\n",
    ),
];

fn compile(src: &str) -> IIRModule {
    twig_ir_compiler::compile_source(&format!("{CHURN}{src}"), "gc_global_and_buffer_roots")
        .unwrap_or_else(|e| panic!("compile: {e}"))
}

/// Rewrite Twig's string literals into the runtime-string shape twig-aot lowers
/// (see the module doc): `const Str(..)` becomes `str_const`, and a `mov` of a
/// literal into a join variable becomes a `str_const` of the same literal
/// straight into the join, so the join is written by `str_const` in two blocks
/// and its length is read at run time.
fn as_runtime_strings(m: &mut IIRModule) {
    for f in &mut m.functions {
        let literals: HashMap<String, String> = f
            .instructions
            .iter()
            .filter_map(|i| match (i.op.as_str(), &i.dest, i.srcs.first()) {
                ("const", Some(d), Some(Operand::Str(l))) => Some((d.clone(), l.clone())),
                _ => None,
            })
            .collect();
        for i in &mut f.instructions {
            if i.op == "const" && matches!(i.srcs.first(), Some(Operand::Str(_))) {
                i.op = "str_const".into();
            } else if i.op == "mov" {
                if let Some(Operand::Var(v)) = i.srcs.first() {
                    if let Some(l) = literals.get(v) {
                        i.op = "str_const".into();
                        i.srcs = vec![Operand::Str(l.clone())];
                    }
                }
            }
        }
    }
}

fn is_global_get(i: &IIRInstr) -> bool {
    i.op == "call_builtin" && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "global_get")
}

fn is_global_set(i: &IIRInstr) -> bool {
    i.op == "call_builtin" && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "global_set")
}

fn is_str_len(i: &IIRInstr) -> bool {
    i.op == "str_len"
}

/// Splice `collect` (a collection builtin), and with `reuse` a second `(w4)`,
/// into the function `f`, right before its first instruction matching `at`.
fn splice_collection(m: &mut IIRModule, at: fn(&IIRInstr) -> bool, collect: &str, reuse: bool) {
    let f: &mut IIRFunction =
        m.functions.iter_mut().find(|f| f.name == "f").expect("the program defines f");
    let pos = f.instructions.iter().position(at).expect("f has the read to splice before");
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
            .expect("f calls its allocating helper (w4)");
        again.dest = Some("__reuse".into());
        extra.push(again);
    }
    for (k, instr) in extra.into_iter().enumerate() {
        f.instructions.insert(pos + k, instr);
    }
    // The splice shifts positions; drop the positional source map rather than
    // let it point at the wrong instructions.
    f.source_map.clear();
}

/// Make each global the **only** reference to its value: right after every
/// `global_set` in `main`, overwrite the register that was stored with `0`.
/// `main`'s frame then holds no copy, as it would once a liveness pass stops
/// treating dead temporaries as roots.
fn clear_main_copies(m: &mut IIRModule) {
    let main = m.functions.iter_mut().find(|f| f.name == "main").expect("main");
    let mut k = 0;
    while k < main.instructions.len() {
        if is_global_set(&main.instructions[k]) {
            if let Some(Operand::Var(v)) = main.instructions[k].srcs.get(2).cloned() {
                main.instructions.insert(
                    k + 1,
                    IIRInstr::new("const", Some(v), vec![Operand::Int(0)], "i64"),
                );
                k += 1;
            }
        }
        k += 1;
    }
    main.source_map.clear();
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

fn has_valgrind() -> bool {
    Command::new("valgrind")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Every global shape, with `main`'s copy kept (`keep_main`) and cleared (`sole`).
fn global_variants() -> Vec<(String, IIRModule)> {
    let mut v = Vec::new();
    for (tag, src) in GLOBAL_PROGRAMS {
        v.push((format!("{tag}_keep_main"), compile(src)));
        let mut sole = compile(src);
        clear_main_copies(&mut sole);
        v.push((format!("{tag}_sole"), sole));
    }
    v
}

fn buffer_variants() -> Vec<(String, IIRModule)> {
    BUFFER_PROGRAMS
        .iter()
        .map(|(tag, src)| {
            let mut m = compile(src);
            as_runtime_strings(&mut m);
            (tag.to_string(), m)
        })
        .collect()
}

/// Build and run every `(variant, collector, reuse)` case; collect every
/// failure before the verdict so one does not hide another.
fn run_spliced(
    variants: Vec<(String, IIRModule)>,
    at: fn(&IIRInstr) -> bool,
    collectors: &[&str],
    what: &str,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (tag, m) in &variants {
        for collect in collectors {
            for reuse in [false, true] {
                let mut spliced = m.clone();
                splice_collection(&mut spliced, at, collect, reuse);
                let exe = build(&spliced, &dir, &format!("{tag}_{collect}_{reuse}"));
                let out = run(&exe);
                if out.status.code() != Some(7) {
                    failures.push(format!(
                        "{tag} {collect} reuse={reuse}: exit {:?}",
                        out.status.code()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{what} must read 7 after the collection; a different value or a signal means \
         the word holding it was not a root and went stale: {failures:?}",
    );
}

/// **#18765.** A precise or compacting collection inside `f`, before it reads
/// the global. Fails before the fix: every compacting row with `reuse` (the
/// global still names the freed from-space cell, which `(w4)` has reused). The
/// precise rows are guards (see the module doc).
#[test]
fn global_survives_precise_and_compacting_collection() {
    run_spliced(
        global_variants(),
        is_global_get,
        &["gc_collect_precise", "gc_collect_compacting"],
        "a global's value",
    );
}

/// **#18765, the conservative collector.** No splice: `f` allocates 4 MiB, so
/// the allocation-triggered (conservative, non-moving) collector runs several
/// times while the global is the only live reference to its pair. Before the fix
/// that collector scanned only the machine stack, so only a stale stack word kept
/// the pair alive (see the module doc); it now scans the registered global slots
/// as well. A guard: it passes before the fix too.
#[test]
fn global_survives_allocation_triggered_collections() {
    let dir = tempfile::tempdir().unwrap();
    for (tag, src) in [
        ("pair", "(define g (cons 7 8))\n(define (f) (begin (churn) (car g)))\n(f)\n"),
        ("list", "(define g (list (cons 7 8) 2))\n(define (f) (begin (churn) (car (car g))))\n(f)\n"),
    ] {
        let mut m = compile(src);
        clear_main_copies(&mut m);
        let out = run(&build(&m, &dir, &format!("churn_{tag}")));
        assert_eq!(
            out.status.code(),
            Some(7),
            "{tag}: the global was the only reference while (churn) ran the conservative \
             collector; anything but 7 means the .data slot was not scanned",
        );
    }
}

/// **#18766.** A precise or compacting collection between building a runtime
/// string and reading its length. Before the fix both `literal` rows with
/// `reuse` read a reused cell (exit 0 and 144 instead of 7): the `alloc_bytes`
/// buffer, and the `mov` alias that is the string variable, sat in slots typed
/// `i64`. `concat` and `substring` are guards: `__twig_str_concat` /
/// `__twig_str_slice` results were already typed `str` (a root) on this path,
/// and #18766's fix now types them `any` even when the specialiser says `i64`.
#[test]
fn string_buffer_survives_precise_and_compacting_collection() {
    run_spliced(
        buffer_variants(),
        is_str_len,
        &["gc_collect_precise", "gc_collect_compacting"],
        "`string-length` of a runtime string",
    );
}

/// Both issues under valgrind memcheck, compacting, **without** reuse: the stale
/// read is flagged even when the freed bytes happen to be intact. Before the fix
/// every global row and the `literal` buffer row reported an invalid read. Skipped when
/// valgrind is not installed. Only invalid reads/writes are checked (memcheck
/// also reports the precise walk reading not-yet-written frame slots, a known
/// imprecision of naming every reference slot at every safepoint).
#[test]
fn global_and_buffer_reads_are_clean_under_memcheck() {
    if !has_valgrind() {
        eprintln!("valgrind not installed; skipping the memcheck run");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let cases = global_variants()
        .into_iter()
        .map(|(t, m)| (t, m, is_global_get as fn(&IIRInstr) -> bool))
        .chain(buffer_variants().into_iter().map(|(t, m)| (t, m, is_str_len as fn(&IIRInstr) -> bool)));
    for (tag, mut m, at) in cases {
        splice_collection(&mut m, at, "gc_collect_compacting", false);
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

/// What #18766 changes about *dead* string buffers, measured with
/// `gc_live_bytes` (the Linux twin of `macos_arm64_smoke`'s
/// `end_to_end_gc_manages_runtime_strings`). Three gc-core blocks: the literals
/// "AB" (8 + 2 = 10 bytes) and "CDE" (11) and their concatenation "ABCDE" (13).
///
/// | literals built in | after a precise collection | why |
/// |-------------------|----------------------------|-----|
/// | a helper `mk` that returns the concatenation | 13 | the literals died with `mk`'s frame; only `main`'s `c` is a root |
/// | `main` itself | 34 (was 13 before #18766) | the literals' slots are roots now, and rule R1 names every root slot at every safepoint, so a dead literal is floating garbage until `main` returns. Before the fix they were reclaimed only because their `i64` slots were no root at all — the omission that freed *live* buffers |
#[test]
fn dead_string_buffers_are_reclaimed_once_their_frame_is_gone() {
    fn program(collect: bool, in_helper: bool) -> IIRModule {
        let strings = vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("AB".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("CDE".into())], "str"),
            IIRInstr::new(
                "call_builtin",
                Some("c".into()),
                vec![Operand::Var("str_concat".into()), Operand::Var("a".into()), Operand::Var("b".into())],
                "str",
            ),
        ];
        let mut m = IIRModule::new("gc_strings", "twig");
        let mut body = if in_helper {
            let mut mk = strings;
            mk.push(IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "str"));
            m.add_or_replace(IIRFunction::new("mk", vec![], "str", mk));
            vec![IIRInstr::new("call", Some("c".into()), vec![Operand::Var("mk".into())], "str")]
        } else {
            strings
        };
        if collect {
            body.push(IIRInstr::new(
                "call_builtin",
                Some("freed".into()),
                vec![Operand::Var("gc_collect_precise".into())],
                "i64",
            ));
        }
        body.push(IIRInstr::new(
            "call_builtin",
            Some("lb".into()),
            vec![Operand::Var("gc_live_bytes".into())],
            "i64",
        ));
        body.push(IIRInstr::new("ret", None, vec![Operand::Var("lb".into())], "i64"));
        m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
        m.entry_point = Some("main".into());
        m
    }
    let dir = tempfile::tempdir().unwrap();
    let live = |tag: &str, collect: bool, in_helper: bool| {
        let exe = build(&program(collect, in_helper), &dir, tag);
        run(&exe).status.code()
    };
    assert_eq!(live("strings_tracked", false, true), Some(34), "all three blocks are gc-core blocks");
    assert_eq!(live("strings_helper", true, true), Some(13), "the literals died with mk's frame");
    assert_eq!(live("strings_main", true, false), Some(34), "main's literal slots are roots (R1)");
}
