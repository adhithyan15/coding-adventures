//! # Review follow-ups to #18785, end to end on Linux x86-64
//!
//! Two native-AOT bugs found by the security review of the global-roots PR.
//!
//! ## Arrays of strings were not traced
//!
//! The native backends allocated every `alloc_array` with `__twig_alloc_bytes`, a
//! **no-reference blob** kind: the collector never looks inside it. A BASIC
//! `DIM A$(n)` array holds string handles, so a string reachable only through the
//! array was freed by any collection while the array still held its handle:
//!
//! ```text
//!   main: arr = alloc_array 16 : array<str>
//!         fill(arr)               ; 16 fresh strings, stored only in arr
//!         <collection>            ; arr marked, its elements never scanned → freed
//!         churn()                 ; the freed cells are handed out again
//!         Σ string-length(arr[i]) ; reads reused cells
//! ```
//!
//! twig-aot now rewrites an `alloc_array` of reference elements to `alloc_ref_array`,
//! which both backends lower to `__twig_alloc_ref_array_bytes` (a kind that traces
//! every element word).
//!
//! ## A parameter overwritten in one branch kept a string literal
//!
//! In `f(s, c)`, `if c then s := "hello"` followed by `print s` / `length(s)`: the
//! string lowering registered the literal for `s` (it counted only instruction
//! definitions, not the parameter), so with `c` false `length(s)` folded to 5 and
//! `print s` used a length register only the `c` arm defines.
//!
//! Both programs are IIR modules: Twig has no arrays, and Twig string literals
//! (`const Str`) are not lowered on the native path, while BASIC / ALGOL emit exactly
//! these `alloc_array` / `str_const` shapes through `lang-aot`.

#![cfg(target_os = "linux")]

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use std::process::{Command, Output};

/// Number of strings the array holds.
const N: i64 = 16;
/// Every string is "hello!!", so the expected sum of lengths is `7 · N`.
const WORD: &str = "hello!!";

fn var(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
}

/// `fill(arr)`: store `N` fresh runtime strings into `arr`. The strings live only in
/// the array once `fill` returns.
fn fill() -> IIRFunction {
    let mut body = Vec::new();
    for i in 0..N {
        let (idx, s) = (format!("i{i}"), format!("s{i}"));
        body.push(ins("const", Some(&idx), vec![Operand::Int(i)], "i64"));
        body.push(ins("str_const", Some(&s), vec![Operand::Str(WORD.into())], "str"));
        body.push(ins("array_set", None, vec![var("arr"), var(&idx), var(&s)], "str"));
    }
    body.push(ins("const", Some("z"), vec![Operand::Int(0)], "i64"));
    body.push(ins("ret", None, vec![var("z")], "i64"));
    IIRFunction::new("fill", vec![("arr".into(), "any".into())], "i64", body)
}

/// `churn()`: 64 allocations the size of one string block (8-byte header + 7 bytes),
/// so the allocator hands a just-freed string cell straight back.
fn churn() -> IIRFunction {
    let mut body = vec![ins("const", Some("n"), vec![Operand::Int(8 + WORD.len() as i64)], "i64")];
    for k in 0..64 {
        body.push(ins("call_builtin", Some(&format!("b{k}")), vec![var("gc_alloc"), var("n")], "i64"));
    }
    body.push(ins("const", Some("z"), vec![Operand::Int(0)], "i64"));
    body.push(ins("ret", None, vec![var("z")], "i64"));
    IIRFunction::new("churn", vec![], "i64", body)
}

/// `main`: allocate the array, fill it, collect (`collect`, if any) and churn, then
/// return the sum of the strings' lengths.
fn array_program(collect: Option<&str>) -> IIRModule {
    let mut body = vec![
        ins("const", Some("n"), vec![Operand::Int(N)], "i64"),
        ins("alloc_array", Some("arr"), vec![var("n")], "array<str>"),
        ins("call", Some("filled"), vec![var("fill"), var("arr")], "i64"),
    ];
    if let Some(collect) = collect {
        body.push(ins("call_builtin", Some("freed"), vec![var(collect)], "i64"));
        body.push(ins("call", Some("churned"), vec![var("churn")], "i64"));
    }
    body.push(ins("const", Some("acc0"), vec![Operand::Int(0)], "i64"));
    for i in 0..N {
        let (idx, t, l) = (format!("j{i}"), format!("t{i}"), format!("l{i}"));
        body.push(ins("const", Some(&idx), vec![Operand::Int(i)], "i64"));
        body.push(ins("array_get", Some(&t), vec![var("arr"), var(&idx)], "str"));
        body.push(ins("str_len", Some(&l), vec![var(&t)], "i64"));
        body.push(ins(
            "add",
            Some(&format!("acc{}", i + 1)),
            vec![var(&format!("acc{i}")), var(&l)],
            "i64",
        ));
    }
    body.push(ins("ret", None, vec![var(&format!("acc{N}"))], "i64"));
    let mut m = IIRModule::new("ref_array", "basic");
    m.add_or_replace(fill());
    m.add_or_replace(churn());
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
    m.entry_point = Some("main".into());
    m
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

/// **Arrays of strings are traced.** Without a collection the program reads
/// `7 · 16 = 112`; after a precise or compacting collection and a churn it must read
/// the same. Before the fix both collectors freed the strings (only the array was a
/// root, and its blob kind is never scanned) and the sum came from reused cells.
#[test]
fn strings_held_only_by_an_array_survive_collection() {
    let dir = tempfile::tempdir().unwrap();
    let want = Some((7 * N) as i32);
    assert_eq!(run(&build(&array_program(None), &dir, "plain")).status.code(), want);
    let mut failures = Vec::new();
    for collect in ["gc_collect_precise", "gc_collect_compacting"] {
        let out = run(&build(&array_program(Some(collect)), &dir, collect));
        if out.status.code() != want {
            failures.push(format!("{collect}: exit {:?}", out.status.code()));
        }
    }
    assert!(
        failures.is_empty(),
        "the sum of the array's string lengths must stay {}; a different value or a \
         signal means the elements were not traced: {failures:?}",
        7 * N
    );
}

/// The same program under valgrind memcheck (compacting): the element reads must
/// not touch freed memory. Skipped without valgrind.
#[test]
fn array_element_reads_are_clean_under_memcheck() {
    if !has_valgrind() {
        eprintln!("valgrind not installed; skipping the memcheck run");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let exe = build(&array_program(Some("gc_collect_compacting")), &dir, "memcheck");
    let out = Command::new("valgrind").arg("--quiet").arg(&exe).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("Invalid read") && !stderr.contains("Invalid write"),
        "memcheck saw an access to freed memory:\n{stderr}"
    );
    assert_eq!(out.status.code(), Some((7 * N) as i32));
}

/// `f(s, c)`: `if c then s := "hello"`, then `print s` and return `length(s)`.
/// `main` passes the runtime string "ab" and `c = flag`.
fn param_program(flag: i64) -> IIRModule {
    let f = IIRFunction::new(
        "f",
        vec![("s".into(), "str".into()), ("c".into(), "any".into())],
        "i64",
        vec![
            ins("jmp_if_false", None, vec![var("c"), var("Ldone")], "void"),
            ins("str_const", Some("s"), vec![Operand::Str("hello".into())], "str"),
            ins("label", None, vec![var("Ldone")], "void"),
            ins("print_str", None, vec![var("s")], "void"),
            ins("str_len", Some("n"), vec![var("s")], "i64"),
            ins("ret", None, vec![var("n")], "i64"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("str_const", Some("a"), vec![Operand::Str("ab".into())], "str"),
            ins("const", Some("c"), vec![Operand::Int(flag)], "i64"),
            ins("call", Some("r"), vec![var("f"), var("a"), var("c")], "i64"),
            ins("ret", None, vec![var("r")], "i64"),
        ],
    );
    let mut m = IIRModule::new("param_str", "algol");
    m.add_or_replace(f);
    m.add_or_replace(main);
    m.entry_point = Some("main".into());
    m
}

/// **A parameter overwritten in one branch keeps no literal.** With `c` false, `s`
/// is still the argument "ab": `print s` must print "ab" and `length(s)` must be 2.
/// Before the fix the literal "hello" was registered for `s`, so `length(s)` folded
/// to 5 and `print s` read a length register only the `c` arm defines. With `c` true
/// both see "hello".
#[test]
fn a_parameter_overwritten_in_one_branch_prints_and_measures_the_argument() {
    let dir = tempfile::tempdir().unwrap();
    for (flag, text, len) in [(0, "ab", 2), (1, "hello", 5)] {
        let out = run(&build(&param_program(flag), &dir, &format!("param_{flag}")));
        assert_eq!(out.status.code(), Some(len), "c={flag}: length(s)");
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end(), text, "c={flag}: print s");
    }
}
