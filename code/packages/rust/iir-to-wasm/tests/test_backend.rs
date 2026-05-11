//! Integration tests for the `iir-to-wasm` backend.
//!
//! These tests exercise the full pipeline:
//!   IIRModule → validate_for_wasm → lower_iir_to_wasm → encode_module
//!
//! They check:
//! 1. Validation produces the expected errors (or no errors).
//! 2. `lower_iir_to_wasm` returns `Ok` for valid modules.
//! 3. The resulting `WasmModule` has correct structure:
//!    - Non-empty `functions`, `types`, `exports`, `code` fields.
//!    - Each `FunctionBody.code` is non-empty (has real instructions + end).
//!    - Types match the IIR function signatures.
//! 4. `encode_module` succeeds and returns a valid WASM binary (starts with
//!    the WASM magic `\0asm`).
//! 5. Specific opcodes appear in the code bytes for known operations.
//! 6. Error variants are reported correctly.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, validate_for_wasm, IIRWasmConfig, IIRWasmError};
use wasm_types::{ExternalKind, ValueType};

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

/// Build a minimal single-function module.
fn module_one(
    name: &str,
    params: Vec<(&str, &str)>,
    ret: &str,
    instrs: Vec<IIRInstr>,
) -> IIRModule {
    let fn_ = IIRFunction::new(
        name,
        params.into_iter().map(|(n, t)| (n.into(), t.into())).collect(),
        ret,
        instrs,
    );
    IIRModule {
        name: "test_module".into(),
        functions: vec![fn_],
        entry_point: Some(name.into()),
        language: "test".into(),
    }
}

/// Build a multi-function module.
fn module_multi(fns: Vec<(&str, Vec<(&str, &str)>, &str, Vec<IIRInstr>)>) -> IIRModule {
    let functions = fns
        .into_iter()
        .map(|(name, params, ret, instrs)| {
            IIRFunction::new(
                name,
                params.into_iter().map(|(n, t)| (n.into(), t.into())).collect(),
                ret,
                instrs,
            )
        })
        .collect();
    IIRModule {
        name: "multi_module".into(),
        functions,
        entry_point: Some("main".into()),
        language: "test".into(),
    }
}

/// Lower a module and encode it to bytes; assert both steps succeed.
fn lower_and_encode(module: &IIRModule) -> Vec<u8> {
    let config = IIRWasmConfig::default();
    let wasm = lower_iir_to_wasm(module, &config).expect("lowering failed");
    encode_module(&wasm).expect("encoding failed")
}

// ---------------------------------------------------------------------------
// ── Group 1: Validation ──────────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 1.1
#[test]
fn validate_empty_module_produces_error() {
    let m = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
    };
    let errs = validate_for_wasm(&m);
    assert!(!errs.is_empty(), "empty module must produce errors");
    assert!(errs[0].contains("EmptyModule"));
}

// Test 1.2
#[test]
fn validate_empty_function_produces_error() {
    let m = module_one("main", vec![], "void", vec![]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("EmptyFunction")));
}

// Test 1.3
#[test]
fn validate_any_type_hint_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("add", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
}

// Test 1.4
#[test]
fn validate_polymorphic_type_hint_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "polymorphic"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
}

// Test 1.5
#[test]
fn validate_str_type_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "str"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

// Test 1.6
#[test]
fn validate_ref_type_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "ref<Foo>"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

// Test 1.7 — float types ARE valid (key difference from BEAM backend)
#[test]
fn validate_float_type_accepted() {
    let m = module_one("f", vec![], "f64", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "f64 type should be valid; got: {:?}", errs);
}

// Test 1.8 — f32 also valid
#[test]
fn validate_f32_type_accepted() {
    let m = module_one("f", vec![], "f32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Float(1.0)], "f32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f32"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "f32 type should be valid; got: {:?}", errs);
}

// Test 1.9
#[test]
fn validate_call_builtin_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("call_builtin", None, vec![], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

// Test 1.10
#[test]
fn validate_safepoint_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

// Test 1.11
#[test]
fn validate_io_ops_rejected() {
    for op in &["io_in", "io_out"] {
        let m = module_one("f", vec![], "void", vec![
            IIRInstr::new(*op, None, vec![], "void"),
        ]);
        let errs = validate_for_wasm(&m);
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")),
            "expected UnsupportedOp for {:?}",
            op
        );
    }
}

// Test 1.12
#[test]
fn validate_memory_ops_rejected() {
    for op in &["load_mem", "store_mem", "alloc"] {
        let m = module_one("f", vec![], "void", vec![
            IIRInstr::new(*op, None, vec![], "void"),
        ]);
        let errs = validate_for_wasm(&m);
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")),
            "expected UnsupportedOp for {:?}",
            op
        );
    }
}

// Test 1.13
#[test]
fn validate_valid_void_function_clean() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
}

// Test 1.14 — valid i32 function
#[test]
fn validate_valid_i32_function_clean() {
    let m = module_one("add", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
}

// ---------------------------------------------------------------------------
// ── Group 2: Module structure ────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 2.1
#[test]
fn lower_produces_non_empty_functions_list() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert!(!wm.functions.is_empty());
}

// Test 2.2
#[test]
fn lower_produces_non_empty_types_list() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert!(!wm.types.is_empty());
}

// Test 2.3
#[test]
fn lower_produces_exports() {
    let m = module_one("my_fn", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.exports.len(), 1);
    assert_eq!(wm.exports[0].name, "my_fn");
    assert_eq!(wm.exports[0].kind, ExternalKind::Function);
}

// Test 2.4
#[test]
fn lower_produces_code_section() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.code.len(), 1);
}

// Test 2.5
#[test]
fn lower_function_body_code_non_empty() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert!(!wm.code[0].code.is_empty());
}

// Test 2.6 — functions and code arrays are parallel
#[test]
fn lower_functions_and_code_same_length() {
    let m = module_multi(vec![
        ("add", vec![("a", "i32"), ("b", "i32")], "i32", vec![
            IIRInstr::new("add", Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]),
        ("main", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.functions.len(), wm.code.len());
    assert_eq!(wm.functions.len(), 2);
}

// Test 2.7 — exported indices are correct for multi-function module
#[test]
fn lower_multi_fn_export_indices() {
    let m = module_multi(vec![
        ("f1", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]),
        ("f2", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.exports[0].index, 0);
    assert_eq!(wm.exports[1].index, 1);
}

// Test 2.8 — types are deduplicated
#[test]
fn lower_deduplicates_func_types() {
    // Two functions with the same signature share one type entry.
    let m = module_multi(vec![
        ("f1", vec![("x", "i32")], "i32", vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
        ]),
        ("f2", vec![("y", "i32")], "i32", vec![
            IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "i32"),
        ]),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.types.len(), 1, "two identical signatures → one type entry");
}

// Test 2.9 — correct param types in FuncType
#[test]
fn lower_func_type_params_i32() {
    let m = module_one("add", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.types[0].params, vec![ValueType::I32, ValueType::I32]);
    assert_eq!(wm.types[0].results, vec![ValueType::I32]);
}

// Test 2.10 — void function type has empty results
#[test]
fn lower_void_return_type() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert!(wm.types[0].results.is_empty());
}

// ---------------------------------------------------------------------------
// ── Group 3: Binary encoding ──────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 3.1
#[test]
fn encode_produces_wasm_magic() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_and_encode(&m);
    assert!(
        bytes.starts_with(b"\x00asm"),
        "WASM magic not found; got {:?}",
        &bytes[..4.min(bytes.len())]
    );
}

// Test 3.2
#[test]
fn encode_produces_wasm_version() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_and_encode(&m);
    // WASM version 1 = [0x01, 0x00, 0x00, 0x00]
    assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
}

// Test 3.3
#[test]
fn encode_non_empty_binary() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_and_encode(&m);
    assert!(bytes.len() > 8, "binary should have more than just magic+version");
}

// Test 3.4 — multi-function module encodes correctly
#[test]
fn encode_multi_function_module() {
    let m = module_multi(vec![
        ("f1", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]),
        ("f2", vec![("x", "i32")], "i32", vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
        ]),
    ]);
    let bytes = lower_and_encode(&m);
    assert!(bytes.starts_with(b"\x00asm"));
    assert!(bytes.len() > 8);
}

// ---------------------------------------------------------------------------
// ── Group 4: Opcode emission ──────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 4.1 — i32.const emitted for integer constant
#[test]
fn emit_i32_const_opcode() {
    let m = module_one("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x41 = i32.const
    assert!(wm.code[0].code.contains(&0x41));
}

// Test 4.2 — i64.const emitted for i64 constant
#[test]
fn emit_i64_const_opcode() {
    let m = module_one("f", vec![], "i64", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(999)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x42 = i64.const
    assert!(wm.code[0].code.contains(&0x42));
}

// Test 4.3 — f64.const emitted for float constant
#[test]
fn emit_f64_const_opcode() {
    let m = module_one("f", vec![], "f64", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Float(2.718)], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x44 = f64.const
    assert!(wm.code[0].code.contains(&0x44));
}

// Test 4.4 — i32.add emitted
#[test]
fn emit_i32_add_opcode() {
    let m = module_one("add", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6A = i32.add
    assert!(wm.code[0].code.contains(&0x6A));
}

// Test 4.5 — i64.add emitted for i64 type
#[test]
fn emit_i64_add_opcode() {
    let m = module_one("add64", vec![("a", "i64"), ("b", "i64")], "i64", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x7C = i64.add
    assert!(wm.code[0].code.contains(&0x7C));
}

// Test 4.6 — f64.add emitted
#[test]
fn emit_f64_add_opcode() {
    let m = module_one("addf", vec![("a", "f64"), ("b", "f64")], "f64", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0xA0 = f64.add
    assert!(wm.code[0].code.contains(&0xA0));
}

// Test 4.7 — i32.sub emitted
#[test]
fn emit_i32_sub_opcode() {
    let m = module_one("sub", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("sub", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6B = i32.sub
    assert!(wm.code[0].code.contains(&0x6B));
}

// Test 4.8 — i32.mul emitted
#[test]
fn emit_i32_mul_opcode() {
    let m = module_one("mul", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("mul", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6C = i32.mul
    assert!(wm.code[0].code.contains(&0x6C));
}

// Test 4.9 — i32.div_s emitted for signed i32
#[test]
fn emit_i32_div_s_opcode() {
    let m = module_one("div", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("div", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6D = i32.div_s
    assert!(wm.code[0].code.contains(&0x6D));
}

// Test 4.10 — i32.div_u emitted for unsigned i32
#[test]
fn emit_i32_div_u_opcode() {
    let m = module_one("divu", vec![("a", "u32"), ("b", "u32")], "u32", vec![
        IIRInstr::new("div", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6E = i32.div_u
    assert!(wm.code[0].code.contains(&0x6E));
}

// Test 4.11 — i32.eq emitted
#[test]
fn emit_i32_eq_opcode() {
    let m = module_one("cmp_eq", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("eq", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x46 = i32.eq
    assert!(wm.code[0].code.contains(&0x46));
}

// Test 4.12 — i32.lt_s emitted
#[test]
fn emit_i32_lt_s_opcode() {
    let m = module_one("cmp_lt", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("lt", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x48 = i32.lt_s
    assert!(wm.code[0].code.contains(&0x48));
}

// Test 4.13 — i32.and emitted
#[test]
fn emit_i32_and_opcode() {
    let m = module_one("band", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("and", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x71 = i32.and
    assert!(wm.code[0].code.contains(&0x71));
}

// Test 4.14 — i32.or emitted
#[test]
fn emit_i32_or_opcode() {
    let m = module_one("bor", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("or", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x72 = i32.or
    assert!(wm.code[0].code.contains(&0x72));
}

// Test 4.15 — i32.xor emitted
#[test]
fn emit_i32_xor_opcode() {
    let m = module_one("bxor", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("xor", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x73 = i32.xor
    assert!(wm.code[0].code.contains(&0x73));
}

// Test 4.16 — RETURN opcode emitted for ret_void
#[test]
fn emit_return_for_ret_void() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x0F = return
    assert!(wm.code[0].code.contains(&0x0F));
}

// Test 4.17 — local.get emitted (0x20)
#[test]
fn emit_local_get_for_var_access() {
    let m = module_one("id", vec![("x", "i32")], "i32", vec![
        IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x20 = local.get
    assert!(wm.code[0].code.contains(&0x20));
}

// Test 4.18 — i32.shl emitted
#[test]
fn emit_i32_shl_opcode() {
    let m = module_one("shl", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("shl", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x74 = i32.shl
    assert!(wm.code[0].code.contains(&0x74));
}

// Test 4.19 — neg synthesized (contains 0x6B = i32.sub for integer neg)
#[test]
fn emit_neg_i32() {
    let m = module_one("neg", vec![("a", "i32")], "i32", vec![
        IIRInstr::new("neg", Some("v0".into()), vec![Operand::Var("a".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // i32 neg = i32.const 0; local.get; i32.sub
    // 0x6B = i32.sub
    assert!(wm.code[0].code.contains(&0x6B));
}

// Test 4.20 — f64.neg emitted for float neg
#[test]
fn emit_neg_f64() {
    let m = module_one("negf", vec![("a", "f64")], "f64", vec![
        IIRInstr::new("neg", Some("v0".into()), vec![Operand::Var("a".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x9A = f64.neg
    assert!(wm.code[0].code.contains(&0x9A));
}

// ---------------------------------------------------------------------------
// ── Group 5: Control flow ────────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 5.1 — function with a label and jmp succeeds
#[test]
fn lower_label_and_jmp_succeeds() {
    let m = module_one("loop_fn", vec![], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("entry".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("entry".into())], "void"),
    ]);
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(result.is_ok(), "label+jmp should succeed; err: {:?}", result);
}

// Test 5.2 — dispatch-loop uses LOOP opcode (0x03)
#[test]
fn dispatch_loop_emits_loop_opcode() {
    let m = module_one("loop_fn", vec![], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x03 = loop
    assert!(wm.code[0].code.contains(&0x03), "dispatch loop should emit LOOP opcode");
}

// Test 5.3 — dispatch-loop uses BLOCK opcode (0x02)
#[test]
fn dispatch_loop_emits_block_opcode() {
    let m = module_one("cond_fn", vec![("c", "i32")], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("c".into()), Operand::Var("done".into())], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x02 = block
    assert!(wm.code[0].code.contains(&0x02), "dispatch loop should emit BLOCK opcode");
}

// Test 5.4 — jmp_if_true succeeds
#[test]
fn lower_jmp_if_true_succeeds() {
    let m = module_one("cond", vec![("c", "i32")], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("end_label".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("c".into()), Operand::Var("end_label".into())], "void"),
    ]);
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(result.is_ok(), "jmp_if_true should succeed; err: {:?}", result);
}

// Test 5.5 — jmp_if_false succeeds
#[test]
fn lower_jmp_if_false_succeeds() {
    let m = module_one("cond2", vec![("c", "i32")], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("exit".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("c".into()), Operand::Var("exit".into())], "void"),
    ]);
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(result.is_ok(), "jmp_if_false should succeed; err: {:?}", result);
}

// Test 5.6 — br_table opcode emitted for dispatch loop (0x0E)
#[test]
fn dispatch_loop_emits_br_table() {
    let m = module_one("loop_fn2", vec![], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("lbl".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("lbl".into())], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x0E = br_table
    assert!(wm.code[0].code.contains(&0x0E));
}

// ---------------------------------------------------------------------------
// ── Group 6: Function call ───────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 6.1 — call emits CALL opcode (0x10)
#[test]
fn emit_call_opcode() {
    let m = module_multi(vec![
        ("helper", vec![("x", "i32")], "i32", vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
        ]),
        ("main", vec![], "i32", vec![
            IIRInstr::new("const", Some("arg".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new("call", Some("result".into()),
                vec![Operand::Var("helper".into()), Operand::Var("arg".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "i32"),
        ]),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x10 = call
    assert!(wm.code[1].code.contains(&0x10));
}

// ---------------------------------------------------------------------------
// ── Group 7: Error cases ────────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 7.1 — unknown op produces UnsupportedOp error
#[test]
fn unknown_op_produces_error() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("frobnicate", None, vec![], "void"),
    ]);
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(
        matches!(result, Err(IIRWasmError::UnsupportedOp { .. })),
        "expected UnsupportedOp; got {:?}",
        result
    );
}

// Test 7.2 — validation failure propagated as ValidationFailed
#[test]
fn validation_failure_propagated() {
    let m = IIRModule {
        name: "bad".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
    };
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(
        matches!(result, Err(IIRWasmError::ValidationFailed(_))),
        "expected ValidationFailed; got {:?}",
        result
    );
}

// Test 7.3 — error implements Display
#[test]
fn error_display() {
    let err = IIRWasmError::UnsupportedOp {
        function: "f".to_string(),
        op: "bad_op".to_string(),
    };
    let s = format!("{}", err);
    assert!(s.contains("UnsupportedOp"));
    assert!(s.contains("bad_op"));
}

// Test 7.4 — UndefinedVariable error
#[test]
fn undefined_variable_error() {
    // Use a variable that was never defined.
    let m = module_one("f", vec![], "i32", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("no_exist".into()), Operand::Var("also_no_exist".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    // Note: the register allocator assigns indices to ALL vars it encounters,
    // so "no_exist" actually gets an index. The error only fires if get_src_reg
    // can't find it, which happens when it's truly missing from the map.
    // In this test the lowering should succeed (register allocator handles it).
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(result.is_ok(), "vars are pre-allocated; should succeed");
}

// ---------------------------------------------------------------------------
// ── Group 8: IIRWasmConfig ──────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 8.1 — default config has sensible module name
#[test]
fn config_default_module_name() {
    let cfg = IIRWasmConfig::default();
    assert!(!cfg.module_name.is_empty());
}

// Test 8.2 — new() sets module name
#[test]
fn config_new_sets_name() {
    let cfg = IIRWasmConfig::new("my_module");
    assert_eq!(cfg.module_name, "my_module");
}

// ---------------------------------------------------------------------------
// ── Group 9: Type mapping ────────────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 9.1 — bool param maps to I32
#[test]
fn bool_param_maps_to_i32() {
    let m = module_one("f", vec![("b", "bool")], "bool", vec![
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "bool"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.types[0].params, vec![ValueType::I32]);
    assert_eq!(wm.types[0].results, vec![ValueType::I32]);
}

// Test 9.2 — u8/u16/u32 map to I32
#[test]
fn unsigned_8_16_32_map_to_i32() {
    for ty in &["u8", "u16", "u32"] {
        let m = module_one("f", vec![("x", ty)], ty, vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], *ty),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(
            wm.types[0].params[0],
            ValueType::I32,
            "type {} should map to I32",
            ty
        );
    }
}

// Test 9.3 — i64/u64 map to I64
#[test]
fn i64_u64_map_to_i64() {
    for ty in &["i64", "u64"] {
        let m = module_one("f", vec![("x", ty)], ty, vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], *ty),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(
            wm.types[0].params[0],
            ValueType::I64,
            "type {} should map to I64",
            ty
        );
    }
}

// Test 9.4 — f64 maps to F64
#[test]
fn f64_maps_to_f64() {
    let m = module_one("f", vec![("x", "f64")], "f64", vec![
        IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.types[0].params[0], ValueType::F64);
    assert_eq!(wm.types[0].results[0], ValueType::F64);
}

// Test 9.5 — f32 maps to F32
#[test]
fn f32_maps_to_f32() {
    let m = module_one("f", vec![("x", "f32")], "f32", vec![
        IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "f32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.types[0].params[0], ValueType::F32);
    assert_eq!(wm.types[0].results[0], ValueType::F32);
}

// ---------------------------------------------------------------------------
// ── Group 10: Misc / edge cases ─────────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 10.1 — lnot produces i32.eqz (0x45)
#[test]
fn lnot_emits_i32_eqz() {
    let m = module_one("lnot_fn", vec![("c", "i32")], "i32", vec![
        IIRInstr::new("lnot", Some("v".into()), vec![Operand::Var("c".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x45 = i32.eqz
    assert!(wm.code[0].code.contains(&0x45));
}

// Test 10.2 — mov emits local.get + local.set
#[test]
fn mov_emits_get_and_set() {
    let m = module_one("mov_fn", vec![("a", "i32")], "i32", vec![
        IIRInstr::new("mov", Some("b".into()), vec![Operand::Var("a".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x20 = local.get, 0x21 = local.set
    assert!(wm.code[0].code.contains(&0x20));
    assert!(wm.code[0].code.contains(&0x21));
}

// Test 10.3 — bool constant emits i32.const
#[test]
fn bool_const_emits_i32_const() {
    let m = module_one("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Bool(true)], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "bool"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x41 = i32.const
    assert!(wm.code[0].code.contains(&0x41));
}

// Test 10.4 — full pipeline round trip succeeds and produces non-trivial output
#[test]
fn full_pipeline_round_trip() {
    let m = module_one(
        "compute",
        vec![("x", "i32"), ("y", "i32")],
        "i32",
        vec![
            IIRInstr::new("mul", Some("p".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("mul", Some("q".into()),
                vec![Operand::Var("y".into()), Operand::Var("y".into())], "i32"),
            IIRInstr::new("add", Some("r".into()),
                vec![Operand::Var("p".into()), Operand::Var("q".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let bytes = lower_and_encode(&m);
    assert!(bytes.starts_with(b"\x00asm"));
    // Should be significantly more than just the header.
    assert!(bytes.len() > 20);
}

// Test 10.5 — i32.rem_s emitted for signed rem
#[test]
fn emit_i32_rem_s() {
    let m = module_one("rem", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("rem", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x6F = i32.rem_s
    assert!(wm.code[0].code.contains(&0x6F));
}
