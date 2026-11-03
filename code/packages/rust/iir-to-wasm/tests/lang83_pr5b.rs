//! LANG83 PR5b — on WASM a string literal is bytes, not ASCII text.
//!
//! Three facts are pinned here:
//!
//! 1. `validate_for_wasm` accepts any `str_const` literal: a `Str` with
//!    non-ASCII UTF-8 (it accepted ASCII only) and a raw-byte `Operand::Bytes`.
//! 2. The data segment holds the literal's bytes verbatim.
//! 3. A literal promoted to a runtime `[i32 len][bytes]` block is found by its
//!    bytes. The block table used to be keyed by a lossy UTF-8 decode, under
//!    which the one-byte strings `A9` and `FF` are both U+FFFD: two branches
//!    returning them shared one block, so one branch returned the other's byte.

mod common;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, validate_for_wasm, IIRWasmConfig};

fn module(functions: Vec<IIRFunction>) -> IIRModule {
    let mut m = IIRModule::new("pr5b", "test");
    for f in functions {
        m.add_or_replace(f);
    }
    m
}

fn str_const(dest: &str, literal: Operand) -> IIRInstr {
    IIRInstr::new("str_const", Some(dest.into()), vec![literal], "str")
}

/// `main` that writes two literals: `"é"` (UTF-8) and the raw bytes `00 FF`.
fn two_literals() -> IIRModule {
    module(vec![IIRFunction::new("main", vec![], "void", vec![
        str_const("s", Operand::Str("é".into())),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        str_const("t", Operand::Bytes(vec![0x00, 0xFF])),
        IIRInstr::new("print_str", None, vec![Operand::Var("t".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ])])
}

#[test]
fn any_literal_validates_and_its_bytes_are_the_data_segment() {
    let m = two_literals();
    let errors = validate_for_wasm(&m);
    assert!(errors.is_empty(), "any literal validates: {errors:?}");
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering");
    let data: Vec<u8> = wasm.data.iter().flat_map(|d| d.data.iter().copied()).collect();
    assert!(
        data.windows(4).any(|w| w == [0xC3, 0xA9, 0x00, 0xFF]),
        "the two literals' bytes, verbatim: {data:02X?}"
    );
}

/// `pick(n)` returns the one-byte string `A9` when `n > 0`, else `FF`. Each
/// branch writes the result variable, so it is a runtime handle whose two
/// literals are laid down as runtime blocks.
fn pick() -> IIRFunction {
    IIRFunction::new("pick", vec![("n".into(), "i64".into())], "str", vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("cmp_gt", Some("c".into()), vec![Operand::Var("n".into()), Operand::Var("z".into())], "i64"),
        IIRInstr::new("jmp_if_false", None, vec![Operand::Var("c".into()), Operand::Var("else".into())], "void"),
        str_const("r", Operand::Bytes(vec![0xA9])),
        IIRInstr::new("jmp", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("else".into())], "void"),
        str_const("r", Operand::Bytes(vec![0xFF])),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "str"),
    ])
}

/// `str_index (pick n), 0`.
fn byte_picked(n: i64) -> i64 {
    let main = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("n".into()), vec![Operand::Int(n)], "i64"),
        IIRInstr::new("call", Some("s".into()), vec![Operand::Var("pick".into()), Operand::Var("n".into())], "str"),
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("str_index", Some("b".into()), vec![Operand::Var("s".into()), Operand::Var("i".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
    ]);
    let m = module(vec![pick(), main]);
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering");
    let bytes = encode_module(&wasm).expect("encoding");
    common::runtime().load_and_run(&bytes, "main", &[]).expect("runs")[0]
}

#[test]
fn runtime_blocks_are_keyed_by_bytes_not_by_lossy_text() {
    assert_eq!(byte_picked(1), 0xA9);
    assert_eq!(byte_picked(0), 0xFF, "the FF branch must not share the A9 block");
}
