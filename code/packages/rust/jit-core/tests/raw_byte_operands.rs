//! PR5b security review (L2): a raw-byte literal is never a name on the JIT.
//!
//! `Operand::Bytes` is legal only as `str_const`'s literal (LANG83 PR5b).
//! The JIT first lifted a `Bytes` whose bytes were UTF-8 to the CIR operand
//! `Var(text)`, which is also how a *name* travels in CIR. So
//! `call_builtin [Bytes(b"record"), …]` called the builtin `record` on the JIT,
//! while vm-core and twig-vm refuse a byte string in a name position: the same
//! module meant different things on two tiers.
//!
//! The JIT now refuses to compile a function that carries a `Bytes` operand
//! anywhere (it refuses every string op until PR5d anyway), and the CIR lift
//! never turns one into its text.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use jit_core::cir::{CIROperand, RAW_BYTES_LITERAL_NAME};
use jit_core::core::JITCore;
use jit_core::generic_jit::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::value::Value;

#[test]
fn a_raw_byte_builtin_name_is_not_compiled_or_called() {
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = GenericCirJit::new();
    let seen = Arc::clone(&calls);
    backend.register_builtin("record", move |_args: &[Value]| {
        seen.fetch_add(1, Ordering::SeqCst);
        Value::Null
    });
    let mut module = IIRModule::new("pr5b", "test");
    module.add_or_replace(IIRFunction::new("f", vec![], "i64", vec![
        IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("call_builtin", None, vec![Operand::Bytes(b"record".to_vec()), Operand::Var("one".into())], "void"),
        IIRInstr::new("ret", None, vec![Operand::Var("one".into())], "i64"),
    ]));
    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let compiled = jit.compile(&mut vm, &module, "f").expect("no JIT error");
    assert!(!compiled && !jit.is_compiled("f"), "a function with a raw-byte operand is not JIT-compiled");
    // The interpreter tier refuses the module at load (BytesOperand).
    assert!(jit.execute(&mut vm, &mut module, "f", &[]).is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0, "the builtin `record` was never called");
}

#[test]
fn a_raw_byte_literal_never_lifts_to_its_text() {
    for bytes in [b"x".to_vec(), vec![0xFF]] {
        assert_eq!(CIROperand::from(Operand::Bytes(bytes.clone())), CIROperand::Var(RAW_BYTES_LITERAL_NAME.into()));
        assert_eq!(CIROperand::from(&Operand::Bytes(bytes)), CIROperand::Var(RAW_BYTES_LITERAL_NAME.into()));
    }
}
