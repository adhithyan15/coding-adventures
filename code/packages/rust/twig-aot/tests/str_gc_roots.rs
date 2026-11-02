//! # A native string slot is a GC root (LANG83 PR5a) — end to end on Linux x86-64
//!
//! LANG83 PR5a's first "thing to verify": does a `str` register still arrive
//! at the native backend typed as a possible reference after `twig-aot`'s
//! string lowering and `aot-core`'s specialiser, so that the precise stack
//! map names its frame slot at every safepoint?
//!
//! A native string is a gc-core block (`[i64 len][bytes]`, allocated by
//! `__twig_alloc_bytes`). The backends name a frame slot in the stack map
//! when its CIR type is *not* a machine scalar (`is_gc_root_ty`), so the
//! question is which type each string register ends with:
//!
//! | how the string was made          | register(s) holding the handle      | type before PR5a | root? |
//! |----------------------------------|-------------------------------------|------------------|-------|
//! | `str_concat` / `str_slice` at run time | `call_builtin … : str`        | `str`            | yes   |
//! | a `str_const` literal            | `alloc_bytes buf`, `mov dest = buf` | `i64`, `i64`     | **no** |
//!
//! The allocation-triggered collector scans the stack conservatively, so the
//! literal row is safe there. `gc_collect_precise` and
//! `gc_collect_compacting` trust the stack map of a mapped frame, so before
//! PR5a a live literal there was reachable only by luck. The deterministic
//! check is the CIR-level unit test
//! `a_literal_strings_handle_registers_are_gc_roots_after_specialisation` in
//! `src/lib.rs`, which fails on the old `i64` typing.
//!
//! These end-to-end runs are the behaviour check that goes with it: a literal
//! kept across an explicit precise or compacting collection still prints.
//! They also passed before the fix, and that is worth knowing: the precise
//! walk scans the collector's own frames (`[sp, start_fp)`) conservatively,
//! and a stale copy of the handle left there by an earlier call kept the
//! block alive and pinned. A test cannot rely on that copy being absent (see
//! the lesson on "make it unreachable in a callee, then collect in the
//! caller"), which is why the type check, not this run, is the regression
//! test.

#![cfg(target_os = "linux")]

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use std::process::Command;

fn var(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main`: build the literal `keep`, collect with `collect`, allocate two
/// same-sized runtime strings and print `keep` and the second one.
///
/// `half` comes back from a call, so it is a run-time `str` and `main` has at
/// least one root-typed slot. That matters: a function with *no* root-typed
/// slot has no stack-map record, and the precise walk scans an unmapped frame
/// conservatively, which would hide the bug. With `half` in place `main` is
/// mapped, and a literal whose slots are all `i64` is invisible to the walk:
/// the collection frees it, the concat's 24-byte block reuses its memory, and
/// `keep` prints as `zz…`.
fn program(collect: &str) -> IIRModule {
    let keep = "KEEP-THIS-STRING";
    let body = vec![
        IIRInstr::new("str_const", Some("keep".into()), vec![Operand::Str(keep.into())], "str"),
        IIRInstr::new("call", Some("half".into()), vec![var("mk")], "str"),
        IIRInstr::new("call_builtin", Some("freed".into()), vec![var(collect)], "i64"),
        IIRInstr::new("str_concat", Some("other".into()), vec![var("half"), var("half")], "str"),
        IIRInstr::new("str_concat", Some("other2".into()), vec![var("half"), var("half")], "str"),
        IIRInstr::new("print_str", None, vec![var("keep")], "void"),
        IIRInstr::new("print_str", None, vec![var("other2")], "void"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![var("z")], "i64"),
    ];
    let mk = vec![
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("zzzzzzzz".into())], "str"),
        IIRInstr::new("ret", None, vec![var("s")], "str"),
    ];
    let mut m = IIRModule::new("str_gc_roots", "iir");
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
    m.add_or_replace(IIRFunction::new("mk", vec![], "str", mk));
    m
}

fn run(m: &IIRModule, tag: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join(tag);
    twig_aot::compile_module_to_linux_executable(m, &exe).unwrap_or_else(|e| panic!("{tag}: {e}"));
    let out = Command::new(&exe).output().unwrap_or_else(|e| panic!("{tag}: {e}"));
    assert!(out.status.success(), "{tag}: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_literal_string_survives_a_precise_collection() {
    assert_eq!(run(&program("gc_collect_precise"), "precise"), "KEEP-THIS-STRINGzzzzzzzzzzzzzzzz");
}

#[test]
fn a_literal_string_survives_a_compacting_collection() {
    assert_eq!(run(&program("gc_collect_compacting"), "compacting"), "KEEP-THIS-STRINGzzzzzzzzzzzzzzzz");
}
