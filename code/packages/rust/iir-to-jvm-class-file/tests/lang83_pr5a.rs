//! LANG83 PR5a on the JVM backend: string failures are registered traps,
//! checked on the full 64-bit operands BEFORE anything narrows them.
//!
//! Before PR5a `str_index s, i` was `aload s; lload i; l2i; charAt`, so
//! `i = 2^32 + 1` read `s[1]`, and an out-of-range index threw
//! `StringIndexOutOfBoundsException` (no flush, no kind). `str_slice` narrowed
//! both offsets the same way, and `str_concat` had no ceiling.
//!
//! These tests pin the bytecode; `lang-aot/tests/op_conformance.rs` runs the
//! cases on a real `java` with the verifier on.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_jvm_class_file::{lower_iir_to_jvm, validate_for_jvm, IIRJvmConfig, JvmClassFile};
use jvm_class_file::JvmConstantPoolEntry as Cp;

const L2I: u8 = 0x88;

const GETSTATIC: u8 = 0xB2;
const INVOKEVIRTUAL: u8 = 0xB6;
const INVOKESPECIAL: u8 = 0xB7;
const NEW: u8 = 0xBB;
const DUP: u8 = 0x59;
const LDC_W: u8 = 0x13;
const ATHROW: u8 = 0xBF;
const LCMP: u8 = 0x94;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn lower(body: Vec<IIRInstr>, params: Vec<(String, String)>, ceiling: Option<u32>) -> JvmClassFile {
    let mut module = IIRModule::new("lang83", "test");
    module.functions = vec![IIRFunction::new("main", params, "i64", body)];
    module.str_max_bytes = ceiling;
    assert!(validate_for_jvm(&module).is_empty(), "{:?}", validate_for_jvm(&module));
    lower_iir_to_jvm(&module, &IIRJvmConfig::new("Main")).expect("lowers")
}

fn main_code(class: &JvmClassFile) -> Vec<u8> {
    class.methods.iter().find(|m| m.name == "main").unwrap().code_attribute().unwrap().code.clone()
}

fn utf8(cp: &[Option<Cp>], i: u16) -> String {
    match &cp[i as usize] {
        Some(Cp::Utf8(s)) => s.clone(),
        other => panic!("cp[{i}] is not Utf8: {other:?}"),
    }
}

fn class_name(cp: &[Option<Cp>], i: u16) -> String {
    match &cp[i as usize] {
        Some(Cp::Class { name_index }) => utf8(cp, *name_index),
        other => panic!("cp[{i}] is not a Class: {other:?}"),
    }
}

/// `class.name:descriptor` for a Methodref / Fieldref index.
fn member(cp: &[Option<Cp>], i: u16) -> String {
    let (c, nt) = match &cp[i as usize] {
        Some(Cp::Methodref { class_index, name_and_type_index })
        | Some(Cp::Fieldref { class_index, name_and_type_index }) => (*class_index, *name_and_type_index),
        other => panic!("cp[{i}] is not a member ref: {other:?}"),
    };
    match &cp[nt as usize] {
        Some(Cp::NameAndType { name_index, descriptor_index }) => {
            format!("{}.{}:{}", class_name(cp, c), utf8(cp, *name_index), utf8(cp, *descriptor_index))
        }
        other => panic!("cp[{nt}] is not NameAndType: {other:?}"),
    }
}

fn u16_at(code: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([code[at], code[at + 1]])
}

/// Every `(pc, "class.name:desc")` call or field access in `code`.
fn refs(class: &JvmClassFile, code: &[u8], opcode: u8) -> Vec<(usize, String)> {
    let cp = &class.constant_pool;
    code.windows(3)
        .enumerate()
        .filter(|(_, w)| w[0] == opcode)
        .filter_map(|(pc, w)| {
            let idx = u16::from_be_bytes([w[1], w[2]]);
            match cp.get(idx as usize) {
                Some(Some(Cp::Methodref { .. })) | Some(Some(Cp::Fieldref { .. })) => Some((pc, member(cp, idx))),
                _ => None,
            }
        })
        .collect()
}

/// The 17-byte trap sequence for `kind` starts at `pc`.
fn assert_trap_at(class: &JvmClassFile, code: &[u8], pc: usize, kind: &str) {
    let cp = &class.constant_pool;
    assert_eq!(code[pc], GETSTATIC);
    assert_eq!(member(cp, u16_at(code, pc + 1)), "java/lang/System.out:Ljava/io/PrintStream;");
    assert_eq!(code[pc + 3], INVOKEVIRTUAL);
    assert_eq!(member(cp, u16_at(code, pc + 4)), "java/io/PrintStream.flush:()V", "flush before throwing");
    assert_eq!(code[pc + 6], NEW);
    assert_eq!(class_name(cp, u16_at(code, pc + 7)), "java/lang/RuntimeException");
    assert_eq!(code[pc + 9], DUP);
    assert_eq!(code[pc + 10], LDC_W);
    match &cp[u16_at(code, pc + 11) as usize] {
        Some(Cp::String { string_index }) => assert_eq!(utf8(cp, *string_index), kind),
        other => panic!("ldc_w operand is not a String: {other:?}"),
    }
    assert_eq!(code[pc + 13], INVOKESPECIAL);
    assert_eq!(member(cp, u16_at(code, pc + 14)), "java/lang/RuntimeException.<init>:(Ljava/lang/String;)V");
    assert_eq!(code[pc + 16], ATHROW);
}


/// Every trap sequence in `code`, as `(pc, kind)`, in order.
fn traps(class: &JvmClassFile, code: &[u8]) -> Vec<(usize, String)> {
    refs(class, code, INVOKEVIRTUAL)
        .into_iter()
        .filter(|(_, m)| m == "java/io/PrintStream.flush:()V")
        .map(|(pc, _)| {
            let start = pc - 3;
            let kind = match &class.constant_pool[u16_at(code, start + 11) as usize] {
                Some(Cp::String { string_index }) => utf8(&class.constant_pool, *string_index),
                other => panic!("{other:?}"),
            };
            assert_trap_at(class, code, start, &kind);
            (start, kind)
        })
        .collect()
}

fn pc_of_call(class: &JvmClassFile, code: &[u8], member: &str) -> usize {
    refs(class, code, INVOKEVIRTUAL).into_iter().find(|(_, m)| m == member).unwrap_or_else(|| panic!("{member}")).0
}

fn str_params(names: &[(&str, &str)]) -> Vec<(String, String)> {
    names.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect()
}

#[test]
fn str_index_checks_both_bounds_on_the_long_index_before_l2i() {
    let class = lower(
        vec![
            IIRInstr::new("str_index", Some("b".into()), vec![v("s"), v("i")], "i64"),
            IIRInstr::new("ret", None, vec![v("b")], "i64"),
        ],
        str_params(&[("s", "str"), ("i", "i64")]),
        None,
    );
    let code = main_code(&class);
    let traps = traps(&class, &code);
    assert_eq!(traps.iter().map(|(_, k)| k.as_str()).collect::<Vec<_>>(), ["Trap.Bounds", "Trap.Bounds"]);
    let char_at = pc_of_call(&class, &code, "java/lang/String.charAt:(I)C");
    let first_l2i = code.iter().position(|&b| b == L2I).expect("the index is narrowed for charAt");
    assert!(traps.iter().all(|(pc, _)| *pc < first_l2i), "both guards come before the narrowing");
    assert!(first_l2i < char_at);
    // Each guard compares longs: `lcmp` right before its `if<cond>`.
    for (pc, _) in &traps {
        assert_eq!(code[pc - 4], LCMP, "guard at {pc} is an lcmp");
    }
}

#[test]
fn str_slice_checks_all_three_bounds_before_narrowing() {
    let class = lower(
        vec![
            IIRInstr::new("str_slice", Some("t".into()), vec![v("s"), v("a"), v("b")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
        str_params(&[("s", "str"), ("a", "i64"), ("b", "i64")]),
        None,
    );
    let code = main_code(&class);
    let traps = traps(&class, &code);
    assert_eq!(traps.len(), 3, "a >= 0, b >= a, len >= b");
    assert!(traps.iter().all(|(_, k)| k == "Trap.Bounds"));
    let first_l2i = code.iter().position(|&b| b == L2I).expect("offsets are narrowed for substring");
    assert!(traps.iter().all(|(pc, _)| *pc < first_l2i));
    assert!(first_l2i < pc_of_call(&class, &code, "java/lang/String.substring:(II)Ljava/lang/String;"));
}

#[test]
fn str_concat_checks_the_module_ceiling_before_concat() {
    let body = || {
        vec![
            IIRInstr::new("str_concat", Some("c".into()), vec![v("a"), v("b")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("c")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ]
    };
    for (ceiling, want) in [(Some(16), 16i64), (None, i64::from(interpreter_ir::STR_MAX_BYTES))] {
        let class = lower(body(), str_params(&[("a", "str"), ("b", "str")]), ceiling);
        let code = main_code(&class);
        let traps = traps(&class, &code);
        assert_eq!(traps.len(), 1);
        assert_eq!(traps[0].1, "Trap.Overflow");
        assert!(traps[0].0 < pc_of_call(&class, &code, "java/lang/String.concat:(Ljava/lang/String;)Ljava/lang/String;"));
        let longs: Vec<i64> = class
            .constant_pool
            .iter()
            .filter_map(|e| match e {
                Some(Cp::Long(x)) => Some(*x),
                _ => None,
            })
            .collect();
        assert!(longs.contains(&want), "the ceiling {want} is in the constant pool: {longs:?}");
    }
}

/// #18800 item 1: a `str_const` longer than the module's ceiling, or a
/// ceiling above `STR_MAX_BYTES`, is refused by `validate_for_jvm` and so by
/// `lower_iir_to_jvm`, before any class file is built. Before, both lowered.
#[test]
fn a_literal_past_the_module_ceiling_is_refused_at_compile_time() {
    let module = |len: usize, ceiling: Option<u32>| {
        let mut m = IIRModule::new("lang83", "test");
        m.functions = vec![IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("z".repeat(len))], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("s")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])];
        m.str_max_bytes = ceiling;
        m
    };
    let refused = |m: &IIRModule| match lower_iir_to_jvm(m, &IIRJvmConfig::new("Main")) {
        Err(iir_to_jvm_class_file::IIRJvmError::ValidationFailed(errors)) => errors,
        other => panic!("expected a validation failure, got {:?}", other.map(|_| "a class file")),
    };
    assert!(refused(&module(17, Some(16))).iter().any(|e| e.starts_with("StrTooLong")));
    assert!(refused(&module(0, Some(interpreter_ir::STR_MAX_BYTES + 1))).iter().any(|e| e.starts_with("StrMaxBytes")));
    assert!(validate_for_jvm(&module(16, Some(16))).is_empty());
    lower_iir_to_jvm(&module(16, Some(16)), &IIRJvmConfig::new("Main")).expect("at the ceiling it lowers");
}
