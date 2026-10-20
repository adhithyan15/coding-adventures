//! LANG83 PR1b on the JVM backend: bytecode shape of `i64_div_floor`,
//! `i64_mod_floor`, `trap` and `write_i64` — and of the PR4 overflow-checked
//! `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf`, `i64_neg_ovf` (end of file).
//!
//! Execution on a real `java` (with the class-file verifier on) is proven by
//! `lang-aot/tests/op_conformance.rs`'s JVM column; these tests pin the
//! instruction sequences so a regression is caught without a JDK.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_jvm_class_file::{lower_iir_to_jvm, validate_for_jvm, IIRJvmConfig, JvmClassFile};
use jvm_class_file::JvmConstantPoolEntry as Cp;

const GETSTATIC: u8 = 0xB2;
const INVOKEVIRTUAL: u8 = 0xB6;
const INVOKESPECIAL: u8 = 0xB7;
const INVOKESTATIC: u8 = 0xB8;
const NEW: u8 = 0xBB;
const DUP: u8 = 0x59;
const LDC_W: u8 = 0x13;
const ATHROW: u8 = 0xBF;
const IFNE: u8 = 0x9A;
const LCMP: u8 = 0x94;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn lower(body: Vec<IIRInstr>, params: Vec<(String, String)>) -> JvmClassFile {
    let module = IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new("main", params, "i64", body)],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
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

fn floor_fn(op: &str) -> JvmClassFile {
    lower(
        vec![
            IIRInstr::new(op, Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    )
}

#[test]
fn i64_div_floor_checks_both_edges_then_calls_math_floor_div() {
    let class = floor_fn("i64_div_floor");
    let code = main_code(&class);
    let flushes: Vec<usize> = refs(&class, &code, INVOKEVIRTUAL)
        .into_iter()
        .filter(|(_, m)| m == "java/io/PrintStream.flush:()V")
        .map(|(pc, _)| pc - 3)
        .collect();
    assert_eq!(flushes.len(), 2, "Trap.DivZero and Trap.Overflow");
    assert_trap_at(&class, &code, flushes[0], "Trap.DivZero");
    assert_trap_at(&class, &code, flushes[1], "Trap.Overflow");
    // Each trap is skipped by an `lcmp; ifne +N` that lands just past it.
    for &t in &flushes {
        assert_eq!(code[t - 4], LCMP);
        assert_eq!(code[t - 3], IFNE);
        let target = (t - 3) as i64 + i16::from_be_bytes([code[t - 2], code[t - 1]]) as i64;
        assert!(target as usize >= t + 17, "ifne skips the 17-byte trap");
    }
    let call = refs(&class, &code, INVOKESTATIC);
    assert_eq!(call.len(), 1);
    assert_eq!(call[0].1, "java/lang/Math.floorDiv:(JJ)J");
    assert!(call[0].0 > flushes[1] + 16, "floorDiv is reached only after both checks");
}

#[test]
fn i64_mod_floor_has_only_the_zero_check_and_calls_math_floor_mod() {
    let class = floor_fn("i64_mod_floor");
    let code = main_code(&class);
    let flushes: Vec<usize> = refs(&class, &code, INVOKEVIRTUAL)
        .into_iter()
        .filter(|(_, m)| m.ends_with("flush:()V"))
        .map(|(pc, _)| pc - 3)
        .collect();
    assert_eq!(flushes.len(), 1, "MIN mod -1 is 0 — no overflow trap");
    assert_trap_at(&class, &code, flushes[0], "Trap.DivZero");
    assert_eq!(refs(&class, &code, INVOKESTATIC)[0].1, "java/lang/Math.floorMod:(JJ)J");
}

#[test]
fn trap_flushes_then_throws_a_runtime_exception_naming_the_kind() {
    let class = lower(
        vec![
            IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ],
        vec![],
    );
    let code = main_code(&class);
    assert_trap_at(&class, &code, 0, "Trap.User");
}

#[test]
fn write_i64_prints_through_system_out_without_a_newline() {
    let class = lower(
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(-5)], "i64"),
            IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("n")], "void"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
        vec![],
    );
    let code = main_code(&class);
    let calls: Vec<String> = refs(&class, &code, INVOKEVIRTUAL).into_iter().map(|(_, m)| m).collect();
    assert_eq!(calls, ["java/io/PrintStream.print:(J)V"]);
    assert!(refs(&class, &code, INVOKESTATIC).is_empty(), "no host runtime class needed");
}

#[test]
fn unregistered_trap_kind_is_refused() {
    let module = IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("trap", None, vec![Operand::Str("Nope".into())], "void")],
        )],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert!(lower_iir_to_jvm(&module, &IIRJvmConfig::new("Main")).is_err());
}

// ── LANG83 PR4: overflow-checked arithmetic ─────────────────────────────────

const LADD: u8 = 0x61;
const LSUB: u8 = 0x65;
const LMUL: u8 = 0x69;
const LNEG: u8 = 0x75;
const LSTORE: u8 = 0x37;
const LSTORE_0: u8 = 0x3F;
const IFEQ: u8 = 0x99;
const IFGE: u8 = 0x9C;

fn checked_fn(op: &str) -> JvmClassFile {
    let srcs = if op == "i64_neg_ovf" { vec![v("a")] } else { vec![v("a"), v("b")] };
    lower(
        vec![
            IIRInstr::new(op, Some("q".into()), srcs, "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    )
}

/// Every `Trap.Overflow` trap in `code`, each checked to be the full 17-byte
/// sequence and to be jumped over by the `if<cond>` right before it.
fn overflow_traps(class: &JvmClassFile, code: &[u8], guard: &[u8]) -> Vec<usize> {
    let traps: Vec<usize> = refs(class, code, INVOKEVIRTUAL)
        .into_iter()
        .filter(|(_, m)| m == "java/io/PrintStream.flush:()V")
        .map(|(pc, _)| pc - 3)
        .collect();
    assert_eq!(traps.len(), guard.len(), "one trap per guard");
    for (&t, &if_op) in traps.iter().zip(guard) {
        assert_trap_at(class, code, t, "Trap.Overflow");
        assert_eq!(code[t - 3], if_op, "the guard branch right before the trap");
        let offset = i16::from_be_bytes([code[t - 2], code[t - 1]]);
        assert_eq!(offset, 3 + 17, "the guard skips exactly the trap");
    }
    traps
}

/// The wrapping instruction and its `lstore` come after every guard, and no
/// `Math.*Exact` (which throws `ArithmeticException` with no kind) is called.
fn assert_tail(class: &JvmClassFile, code: &[u8], traps: &[usize], arith: u8) {
    let last_trap_end = traps.last().unwrap() + 17;
    let at = code[last_trap_end..]
        .windows(2)
        .position(|w| w[0] == arith && (w[1] == LSTORE || (LSTORE_0..LSTORE_0 + 4).contains(&w[1])))
        .unwrap_or_else(|| panic!("{arith:#04x} then lstore after the last guard: {code:02x?}"));
    assert!(at > 0);
    assert!(refs(class, code, INVOKESTATIC).is_empty(), "no Math.*Exact");
}

#[test]
fn add_and_sub_ovf_sign_test_then_wrap() {
    for (op, arith) in [("i64_add_ovf", LADD), ("i64_sub_ovf", LSUB)] {
        let class = checked_fn(op);
        let code = main_code(&class);
        let traps = overflow_traps(&class, &code, &[IFGE]);
        // `land; lconst_0; lcmp; ifge` — the sign of the combined xor test.
        assert_eq!(&code[traps[0] - 6..traps[0] - 3], &[0x7F, 0x09, LCMP], "{op}");
        assert_tail(&class, &code, &traps, arith);
    }
}

#[test]
fn neg_ovf_compares_with_min() {
    let class = checked_fn("i64_neg_ovf");
    let code = main_code(&class);
    let traps = overflow_traps(&class, &code, &[IFNE]);
    assert_eq!(code[traps[0] - 4], LCMP);
    assert!(class.constant_pool.iter().any(|e| matches!(e, Some(Cp::Long(n)) if *n == i64::MIN)));
    assert_tail(&class, &code, &traps, LNEG);
}

#[test]
fn mul_ovf_guards_minus_one_times_min_then_divides_back() {
    let class = checked_fn("i64_mul_ovf");
    let code = main_code(&class);
    // Guard 1 (b == MIN, nested under a == -1) ends `ifne`; guard 2
    // ((a*b)/a == b, nested under a != 0) ends `ifeq`.
    let traps = overflow_traps(&class, &code, &[IFNE, IFEQ]);
    assert!(code[traps[0]..traps[1]].contains(&0x6D), "ldiv between the guards");
    assert_tail(&class, &code, &traps, LMUL);
}

#[test]
fn checked_ops_refuse_int_operands() {
    let module = IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new(
            "main",
            vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            "i32",
            vec![
                IIRInstr::new("i64_add_ovf", Some("q".into()), vec![v("a"), v("b")], "i32"),
                IIRInstr::new("ret", None, vec![v("q")], "i32"),
            ],
        )],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert!(lower_iir_to_jvm(&module, &IIRJvmConfig::new("Main")).is_err(), "defined on i64 only");
}
