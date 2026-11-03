//! LANG83 PR5b on the BEAM backend: a string literal is bytes.
//!
//! A runtime string is an Erlang list of integers, one per byte, built by
//! `put_list` from the last byte to the first. Every byte 0–255 is a valid
//! element, so the validator no longer restricts a literal to printable ASCII:
//! a non-ASCII `Str` is its UTF-8, and a raw-byte `Operand::Bytes` its bytes.
//!
//! There is no `erl` on the development hosts, so this pins the instruction
//! stream; CI's BEAM columns run the emulator.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_beam::{lower_iir_to_beam, validate_for_beam, IIRBeamConfig};

/// `put_list` (the compact-term opcode the `str_const` arm emits).
const OP_PUT_LIST: u8 = 69;

#[test]
fn any_literal_byte_becomes_one_list_element() {
    let mut module = IIRModule::new("pr5b", "test");
    module.functions = vec![IIRFunction::new("f", vec![], "i64", vec![
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("é".into())], "str"),
        IIRInstr::new("str_const", Some("t".into()), vec![Operand::Bytes(vec![0x00, 0x80, 0xFF])], "str"),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        IIRInstr::new("print_str", None, vec![Operand::Var("t".into())], "void"),
        IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ])];
    module.entry_point = Some("f".into());
    let errors = validate_for_beam(&module);
    assert!(errors.is_empty(), "any literal validates: {errors:?}");
    let m = lower_iir_to_beam(&module, &IIRBeamConfig::new("pr5b")).expect("lowers");
    let elements: Vec<u64> = m
        .instructions
        .iter()
        .filter(|i| i.opcode == OP_PUT_LIST)
        .map(|i| i.operands[0].value)
        .collect();
    // Built last byte first: "é" is C3 A9, then the raw 00 80 FF.
    assert_eq!(elements, vec![0xA9, 0xC3, 0xFF, 0x80, 0x00]);
}
