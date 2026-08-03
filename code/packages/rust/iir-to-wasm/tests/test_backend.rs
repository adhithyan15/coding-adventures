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
use wasm_types::{ExternalKind, ImportTypeInfo, ValueType};

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
        exports: vec![],
        imports: vec![],
    }
}

/// Build a multi-function module.
// The tuple shape is a compact test-fixture descriptor (name, params, ret, body);
// factoring it into named types would only add boilerplate to the tests.
#[allow(clippy::type_complexity)]
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
// 3.14 is arbitrary float test input; not an approximation of PI.
#[allow(clippy::approx_constant)]
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
    // io_in is still unsupported (raw byte-level input is not wired to WASM).
    // io_out is now SUPPORTED (LANG32) — it maps to call $__print_i64.
    {
        let op = &"io_in";
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

// Test 1.11b (LANG32)
#[test]
fn validate_io_out_accepted() {
    // io_out is accepted by the WASM validator since LANG32.
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("io_out", None, vec![Operand::Var("v".into())], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(
        errs.iter().all(|e| !e.contains("UnsupportedOp")),
        "io_out should be accepted (LANG32); got: {:?}",
        errs
    );
}

// Test 1.12
#[test]
fn validate_memory_ops_accepted() {
    // After the BF→WASM lowering, `load_mem` and `store_mem` are
    // accepted — they lower to `i32.load8_u` and `i32.store8` over the
    // module's linear memory (a single 1-page WASM memory injected when
    // any memory op is used).  See iir-to-wasm/src/lower.rs.
    //
    // Note: validate.rs's per-instruction errors are emitted independently,
    // so we tolerate other errors (e.g. UndefinedVariable) here — only the
    // `UnsupportedOp` check is relevant for the memory-op promotion.
    for op in &["load_mem", "store_mem"] {
        let m = module_one("f", vec![], "void", vec![
            IIRInstr::new(*op, None, vec![], "u8"),
        ]);
        let errs = validate_for_wasm(&m);
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "{:?} should no longer be UnsupportedOp; errs: {:?}",
            op, errs
        );
    }
    // `alloc` with a non-ref type hint is still rejected — GC ops require a
    // supported ref<...> type.
    {
        let m = module_one("f", vec![], "void", vec![
            IIRInstr::new("alloc", None, vec![], "i32"),
        ]);
        let errs = validate_for_wasm(&m);
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")),
            "expected UnsupportedOp for alloc with i32 type"
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
// 2.718 is arbitrary float test input; not an approximation of E.
#[allow(clippy::approx_constant)]
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

// Test 4.6b — f64 `mul`/`div` and ordered/equality comparisons select the
// `f64.*` opcodes (LANG-FULL E3 — locks the op selection that lets ALGOL 60
// reals run on WASM; the typed-local model already carries an `f64` variable in
// an `F64` local, so no slot rework was needed unlike the LLVM backend).
#[test]
fn emit_f64_mul_div_opcodes() {
    let mul = module_one("mulf", vec![("a", "f64"), ("b", "f64")], "f64", vec![
        IIRInstr::new("mul", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&mul, &IIRWasmConfig::default()).unwrap();
    assert!(wm.code[0].code.contains(&0xA2), "expected f64.mul (0xA2)");

    let div = module_one("divf", vec![("a", "f64"), ("b", "f64")], "f64", vec![
        IIRInstr::new("div", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
    ]);
    let wm = lower_iir_to_wasm(&div, &IIRWasmConfig::default()).unwrap();
    assert!(wm.code[0].code.contains(&0xA3), "expected f64.div (0xA3)");
}

#[test]
fn emit_f64_comparison_opcodes() {
    // f64 equality → f64.eq (0x61).
    let eq = module_one("eqf", vec![("a", "f64"), ("b", "f64")], "i64", vec![
        IIRInstr::new("cmp_eq", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ]);
    let wm = lower_iir_to_wasm(&eq, &IIRWasmConfig::default()).unwrap();
    assert!(wm.code[0].code.contains(&0x61), "expected f64.eq (0x61)");

    // f64 ordered `<` → f64.lt (0x63).
    let lt = module_one("ltf", vec![("a", "f64"), ("b", "f64")], "i64", vec![
        IIRInstr::new("cmp_lt", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ]);
    let wm = lower_iir_to_wasm(&lt, &IIRWasmConfig::default()).unwrap();
    assert!(wm.code[0].code.contains(&0x63), "expected f64.lt (0x63)");
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

// Test 4.10 — unsigned narrow div emits the UNSIGNED i64 div (LANG-FULL E2).
// Narrow unsigned types ride the i64 register model, so a `u32` divide is
// `i64.div_u` (0x80, not the old `i32.div_u` 0x6E) followed by the u32 wrap
// mask — keeping it unsigned and operand-width-agnostic over i64 slots.
#[test]
fn emit_i32_div_u_opcode() {
    let m = module_one("divu", vec![("a", "u32"), ("b", "u32")], "u32", vec![
        IIRInstr::new("div", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // 0x80 = i64.div_u
    assert!(wm.code[0].code.contains(&0x80), "u32 div → i64.div_u");
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
        exports: vec![],
        imports: vec![],
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

// Test 9.2 — u4/u8/u16/u32 map to I64 (LANG-FULL E2). Narrow unsigned types ride
// the i64 register model so their arithmetic never meets a width-mismatched
// i64-slot operand (e.g. a const/let); the value is masked to width after each
// op instead. (Was I32 before E2's compute-wide-and-mask rework.)
#[test]
fn unsigned_8_16_32_map_to_i32() {
    for ty in &["u4", "u8", "u16", "u32"] {
        let m = module_one("f", vec![("x", ty)], ty, vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], *ty),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(
            wm.types[0].params[0],
            ValueType::I64,
            "type {} should map to I64 (i64 register model)",
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

// ---------------------------------------------------------------------------
// ── Group 11: WasmGC heap ops ───────────────────────────────────────────────
// ---------------------------------------------------------------------------
//
// Phase 2: WasmGC struct types for LispyPair (car/cdr cons cell).
//
// These tests verify that the IIR GC ops (`alloc`, `field_load`,
// `field_store`, `is_null`, `const ref<LispyPair>`) lower correctly to
// WasmGC bytecode.

// Test 11.1 — alloc ref<LispyPair> emits ref.null none (0xD0 0x0F)
#[test]
fn gc_alloc_emits_ref_null_none() {
    let m = module_one("make_nil", vec![], "void", vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // ref.null none = [0xD0, 0x0F]
    let code = &wm.code[0].code;
    assert!(code.windows(2).any(|w| w == [0xD0, 0x0F]),
        "alloc should emit ref.null none (0xD0 0x0F); code: {:?}", code);
}

// Test 11.2 — field_load index 0 (car) emits struct.get prefix (0xFB 0x02)
#[test]
fn gc_field_load_car_emits_struct_get() {
    let m = module_one("car", vec![("p", "ref<LispyPair>")], "void", vec![
        IIRInstr::new(
            "field_load",
            Some("h".into()),
            vec![Operand::Var("p".into()), Operand::Int(0)],
            "ref<LispyPair>",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    let code = &wm.code[0].code;
    // struct.get prefix = 0xFB 0x02
    assert!(code.windows(2).any(|w| w == [0xFB, 0x02]),
        "field_load should emit struct.get (0xFB 0x02); code: {:?}", code);
}

// Test 11.3 — field_load index 1 (cdr) emits struct.get with field=1
#[test]
fn gc_field_load_cdr_emits_struct_get() {
    let m = module_one("cdr", vec![("p", "ref<LispyPair>")], "void", vec![
        IIRInstr::new(
            "field_load",
            Some("t".into()),
            vec![Operand::Var("p".into()), Operand::Int(1)],
            "ref<LispyPair>",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    let code = &wm.code[0].code;
    // struct.get prefix = 0xFB 0x02 then type_idx LEB then field=1
    assert!(code.windows(2).any(|w| w == [0xFB, 0x02]),
        "field_load (cdr) should emit struct.get (0xFB 0x02); code: {:?}", code);
    // The field index 1 should appear somewhere after the 0xFB 0x02 prefix.
    assert!(code.contains(&0x01), "field index 1 should be present; code: {:?}", code);
}

// Test 11.4 — field_store emits struct.set prefix (0xFB 0x04)
#[test]
fn gc_field_store_emits_struct_set() {
    let m = module_one("set_car", vec![("p", "ref<LispyPair>"), ("v", "ref<LispyPair>")], "void", vec![
        IIRInstr::new(
            "field_store",
            None,
            vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("v".into())],
            "ref<LispyPair>",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    let code = &wm.code[0].code;
    // struct.set prefix = 0xFB 0x04
    assert!(code.windows(2).any(|w| w == [0xFB, 0x04]),
        "field_store should emit struct.set (0xFB 0x04); code: {:?}", code);
}

// Test 11.5 — is_null emits ref.is_null (0xD1)
#[test]
fn gc_is_null_emits_ref_is_null() {
    let m = module_one("nullp", vec![("p", "ref<LispyPair>")], "i32", vec![
        IIRInstr::new(
            "is_null",
            Some("b".into()),
            vec![Operand::Var("p".into())],
            "bool",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    let code = &wm.code[0].code;
    // ref.is_null = 0xD1
    assert!(code.contains(&0xD1),
        "is_null should emit ref.is_null (0xD1); code: {:?}", code);
}

// Test 11.6 — const ref<LispyPair> (nil) emits ref.null none
#[test]
fn gc_const_ref_nil_emits_ref_null() {
    let m = module_one("nil_fn", vec![], "void", vec![
        // const with no sources and ref<LispyPair> type = nil
        IIRInstr::new("const", Some("n".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    let code = &wm.code[0].code;
    // ref.null none = [0xD0, 0x0F]
    assert!(code.windows(2).any(|w| w == [0xD0, 0x0F]),
        "const ref<LispyPair> should emit ref.null none; code: {:?}", code);
}

// Test 11.7 — module with LispyPair ops registers a struct_type
#[test]
fn gc_module_has_struct_type() {
    let m = module_one("make_nil", vec![], "void", vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wm.struct_types.len(), 1,
        "module with ref<LispyPair> should have 1 struct type");
    // The struct type should have 2 fields.
    assert_eq!(wm.struct_types[0].fields.len(), 2,
        "LispyPair should have 2 fields ($head and $tail)");
}

// Test 11.8 — module without heap ops has no struct_types
#[test]
fn gc_module_without_heap_ops_has_no_struct_types() {
    let m = module_one("add", vec![("a", "i32"), ("b", "i32")], "i32", vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert!(wm.struct_types.is_empty(),
        "pure arithmetic module should have no struct types");
}

// Test 11.9 — alloc with unsupported type ref<Other> is rejected by validator
#[test]
fn gc_alloc_unsupported_type_rejected() {
    let m = module_one("f", vec![], "void", vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<Other>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(
        errs.iter().any(|e| e.contains("UnsupportedType")),
        "ref<Other> should be rejected; got: {:?}",
        errs
    );
}

// Test 11.10 — module with both integer arithmetic and heap ops compiles
#[test]
fn gc_mixed_arithmetic_and_heap_ops() {
    let m = module_multi(vec![
        // Pure arithmetic function.
        ("add_i32", vec![("a", "i32"), ("b", "i32")], "i32", vec![
            IIRInstr::new("add", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ]),
        // GC heap function.
        ("alloc_pair", vec![], "void", vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]),
    ]);
    let bytes = lower_and_encode(&m);
    assert!(bytes.starts_with(b"\x00asm"), "mixed module should encode to valid WASM");
    assert!(bytes.len() > 8);
}

// Test 11.11 — field_load and field_store round-trip in the same function
#[test]
fn gc_field_load_and_store_in_same_function() {
    let m = module_one(
        "copy_head",
        vec![("src", "ref<LispyPair>"), ("dst", "ref<LispyPair>")],
        "void",
        vec![
            // Load head from src.
            IIRInstr::new(
                "field_load",
                Some("h".into()),
                vec![Operand::Var("src".into()), Operand::Int(0)],
                "ref<LispyPair>",
            ),
            // Store head into dst.
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("dst".into()), Operand::Int(0), Operand::Var("h".into())],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
    assert!(result.is_ok(), "field_load + field_store should succeed; err: {:?}", result);
    let wm = result.unwrap();
    let code = &wm.code[0].code;
    // Both struct.get (0xFB 0x02) and struct.set (0xFB 0x04) should appear.
    assert!(code.windows(2).any(|w| w == [0xFB, 0x02]), "struct.get not found");
    assert!(code.windows(2).any(|w| w == [0xFB, 0x04]), "struct.set not found");
}

// Test 11.12 — full GC pipeline encodes to valid WASM bytes
#[test]
fn gc_full_pipeline_encodes_to_valid_wasm() {
    let m = module_one(
        "cons",
        vec![("head", "ref<LispyPair>"), ("tail", "ref<LispyPair>")],
        "void",
        vec![
            // Allocate a new pair.
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
            // Set head field.
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("head".into())],
                "ref<LispyPair>",
            ),
            // Set tail field.
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("p".into()), Operand::Int(1), Operand::Var("tail".into())],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_and_encode(&m);
    assert!(bytes.starts_with(b"\x00asm"), "GC module should start with WASM magic");
    // Should be larger than just header + empty section.
    assert!(bytes.len() > 16);
}

// Test 11.13 — ref<LispyPair> param produces Anyref in FuncType (not I32)
#[test]
fn gc_ref_param_maps_to_anyref_func_type() {
    use wasm_types::ValueType;
    let m = module_one("id_pair", vec![("p", "ref<LispyPair>")], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    // The function type should have Anyref as the param type.
    assert_eq!(wm.types[0].params, vec![ValueType::Anyref],
        "ref<LispyPair> param should map to Anyref in FuncType");
}

// ===========================================================================
// LANG35 — ClosureOpcode validator tests
// ===========================================================================
//
// The WASM backend does not yet support `alloc_closure` / `call_closure`.
// Full WASM closure lowering (WasmGC function references + call_indirect) is
// deferred to a future LANG spec.
//
// Instead of silently emitting a confusing `UntypedInstruction` error (which
// fires because `"closure"` and `"any"` are not concrete WASM value types),
// the validator now returns a specific `ClosureOpcode` error with an
// actionable message: apply iir-builtin-lowering Phase 4 to downgrade to the
// `call_builtin` form before lowering to WASM.
//
// Tests 12.1–12.3 verify this behaviour.

// Test 12.1 — alloc_closure returns ClosureOpcode error, not UntypedInstruction
#[test]
fn lang35_alloc_closure_closure_opcode_error() {
    let m = module_one(
        "make_closure",
        vec![],
        "closure",
        vec![
            IIRInstr::new(
                "alloc_closure",
                Some("cl".into()),
                vec![Operand::Str("__lambda_0".into())],
                "closure",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("cl".into())], "closure"),
        ],
    );
    let errs = validate_for_wasm(&m);
    assert!(
        !errs.is_empty(),
        "alloc_closure must produce a validation error in the WASM backend"
    );
    assert!(
        errs.iter().any(|e| e.contains("ClosureOpcode")),
        "error must contain \"ClosureOpcode\"; got: {errs:?}"
    );
}

// Test 12.2 — call_closure returns ClosureOpcode error, not UntypedInstruction
#[test]
fn lang35_call_closure_closure_opcode_error() {
    let m = module_one(
        "apply_it",
        vec![("h", "i64"), ("a", "i64")],
        "i64",
        vec![
            IIRInstr::new(
                "call_closure",
                Some("r".into()),
                vec![Operand::Var("h".into()), Operand::Var("a".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let errs = validate_for_wasm(&m);
    assert!(
        !errs.is_empty(),
        "call_closure must produce a validation error in the WASM backend"
    );
    assert!(
        errs.iter().any(|e| e.contains("ClosureOpcode")),
        "error must contain \"ClosureOpcode\"; got: {errs:?}"
    );
}

// Test 12.3 — ClosureOpcode error text does not mention UntypedInstruction
//
// This confirms the LANG35 diagnostic improvement: the message is specific to
// closure opcodes, not a confusing false-positive from the type-hint check.
#[test]
fn lang35_closure_opcode_error_not_untyped() {
    let m = module_one(
        "make_closure",
        vec![],
        "closure",
        vec![
            IIRInstr::new(
                "alloc_closure",
                Some("cl".into()),
                vec![Operand::Str("__lambda_0".into())],
                "closure",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("cl".into())], "closure"),
        ],
    );
    let errs = validate_for_wasm(&m);
    // The error must be the actionable ClosureOpcode message.
    assert!(
        errs.iter().any(|e| e.contains("ClosureOpcode")),
        "expected ClosureOpcode error; got: {errs:?}"
    );
    // It must NOT be the confusing generic UntypedInstruction error.
    assert!(
        !errs.iter().any(|e| e.contains("UntypedInstruction")),
        "error must not say UntypedInstruction for closure ops; got: {errs:?}"
    );
}

// ===========================================================================
// G1 — cmp_* opcode lowerings (BASIC / Nib / Oct emit `cmp_lt` etc.).
//
// The lower step pre-G1 only matched the bare `eq | ne | lt | le | gt | ge`
// shape (the form Twig historically emitted).  Languages that prefix with
// `cmp_` would lower to `UnsupportedOp` even though the validator accepted
// them.  G1 extends the match to strip the prefix.
// ===========================================================================

/// Helper: build a module where `main(a: i64, b: i64) -> i64` runs `cmp_<op>`
/// over its parameters and returns the result.  The dest's type at the IIR
/// level is `bool` because that's how the BASIC/Nib/Oct frontends emit it,
/// but the underlying wasm opcode produces i32, which we widen to i64 with a
/// final `i64.extend_i32_u` step modeled by a separate `mov`-style helper
/// in real frontends.  For this unit test we keep the return type `i32` so
/// no widening is needed.
fn cmp_i64_module(op: &str) -> IIRModule {
    module_one("main", vec![("a", "i64"), ("b", "i64")], "i32", vec![
        IIRInstr::new(op, Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
    ])
}

#[test]
fn g1_cmp_eq_i64_lowers_to_wasm() {
    let m = cmp_i64_module("cmp_eq");
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "validator accepted cmp_eq before G1; got {errs:?}");
    let _wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("G1: cmp_eq must lower; pre-G1 this would have returned UnsupportedOp");
}

#[test]
fn g1_cmp_ne_i64_lowers_to_wasm() {
    let _ = lower_iir_to_wasm(&cmp_i64_module("cmp_ne"), &IIRWasmConfig::default())
        .expect("G1: cmp_ne must lower");
}

#[test]
fn g1_cmp_lt_i64_lowers_to_wasm() {
    let _ = lower_iir_to_wasm(&cmp_i64_module("cmp_lt"), &IIRWasmConfig::default())
        .expect("G1: cmp_lt must lower");
}

#[test]
fn g1_cmp_le_i64_lowers_to_wasm() {
    let _ = lower_iir_to_wasm(&cmp_i64_module("cmp_le"), &IIRWasmConfig::default())
        .expect("G1: cmp_le must lower");
}

#[test]
fn g1_cmp_gt_i64_lowers_to_wasm() {
    let _ = lower_iir_to_wasm(&cmp_i64_module("cmp_gt"), &IIRWasmConfig::default())
        .expect("G1: cmp_gt must lower");
}

#[test]
fn g1_cmp_ge_i64_lowers_to_wasm() {
    let _ = lower_iir_to_wasm(&cmp_i64_module("cmp_ge"), &IIRWasmConfig::default())
        .expect("G1: cmp_ge must lower");
}

/// Backwards-compatibility: Twig's existing bare `eq` / `lt` / etc.  must
/// still lower.  The prefix-stripper passes the bare form through unchanged.
#[test]
fn g1_bare_eq_still_lowers_to_wasm() {
    let m = module_one("main", vec![("a", "i64"), ("b", "i64")], "i32", vec![
        IIRInstr::new("eq", Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
    ]);
    let _ = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("G1: bare `eq` (Twig form) must still lower");
}

// ===========================================================================
// G2 — `call_builtin "print_i64"` reuses the `env.__print_i64` host import.
//
// BASIC's `PRINT` lowers to `call_builtin "print_i64"`.  Pre-G2 this was
// rejected by the validator because `print_i64` wasn't in
// `CALL_BUILTIN_SUPPORTED_NAMES`.  G2 adds it; the import is the SAME
// one the `io_out` opcode injects (`env.__print_i64`), so a module that
// already uses `io_out` and one that uses `print_i64` exclusively both
// produce the same single import.
// ===========================================================================

#[test]
fn g2_call_builtin_print_i64_validator_accepts() {
    let m = module_one("main", vec![("x", "i64")], "void", vec![
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("print_i64".into()), Operand::Var("x".into())],
            "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(),
        "G2: validator must accept call_builtin print_i64; got {errs:?}");
}

#[test]
fn g2_call_builtin_print_i64_lowers_to_wasm_bytes() {
    let m = module_one("main", vec![("x", "i64")], "void", vec![
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("print_i64".into()), Operand::Var("x".into())],
            "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("G2: lower must succeed for call_builtin print_i64");
    let bytes = encode_module(&wm).expect("encode");
    assert!(bytes.len() >= 8);
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D], "wasm magic prefix");
}

#[test]
fn g2_call_builtin_print_i64_injects_host_import() {
    let m = module_one("main", vec![("x", "i64")], "void", vec![
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("print_i64".into()), Operand::Var("x".into())],
            "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("lower");
    let has_print = wm.imports.iter().any(|i|
        i.module_name == "env" && i.name == "__print_i64");
    assert!(has_print,
        "G2: print_i64 builtin must inject the env.__print_i64 host import; got imports {:?}",
        wm.imports.iter().map(|i| (&i.module_name, &i.name)).collect::<Vec<_>>());
}

#[test]
fn g2_unknown_builtin_still_rejected() {
    // Defense-in-depth: a builtin name not in the whitelist must still
    // be rejected (so G2 didn't accidentally widen the gate).
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("does_not_exist".into())],
            "void"),
    ]);
    let errs = validate_for_wasm(&m);
    assert!(!errs.is_empty(),
        "G2: unknown builtin must still be rejected; got no errors");
}

#[test]
fn e4_string_print_lowers_to_data_memory_and_host_import() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new(
            "str_const",
            Some("s".into()),
            vec![Operand::Str("HELLO".into())],
            "str",
        ),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);

    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("E4: str_const + print_str should lower");
    let import = wm
        .imports
        .iter()
        .find(|i| i.module_name == "env" && i.name == "__print_str")
        .expect("E4: print_str must inject env.__print_str");
    let ImportTypeInfo::Function(type_idx) = import.type_info else {
        panic!("E4: __print_str import should be a function");
    };
    assert_eq!(wm.types[type_idx as usize].params, vec![ValueType::I32, ValueType::I32]);
    assert_eq!(wm.types[type_idx as usize].results, Vec::<ValueType>::new());
    assert_eq!(wm.memories.len(), 1, "E4: string bytes live in linear memory");
    assert_eq!(wm.data.len(), 1, "E4: literal should be emitted as one data segment");
    assert_eq!(wm.data[0].data, b"HELLO");
    assert!(
        wm.code[0].code.contains(&0x10),
        "E4: function body should call env.__print_str"
    );

    let bytes = encode_module(&wm).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D], "wasm magic prefix");
}

#[test]
fn e4_string_concat_len_lowers_to_literal_length() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new(
            "str_const",
            Some("a".into()),
            vec![Operand::Str("AB".into())],
            "str",
        ),
        IIRInstr::new(
            "str_const",
            Some("b".into()),
            vec![Operand::Str("CDE".into())],
            "str",
        ),
        IIRInstr::new("str_concat", Some("s".into()), vec![
            Operand::Var("a".into()),
            Operand::Var("b".into()),
        ], "str"),
        IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
    ]);

    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "E4: str_concat + str_len should validate: {errs:?}");
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("E4: str_concat + str_len should lower");
    assert_eq!(
        wm.data[0].data,
        b"ABCDEABCDE",
        "E4: string data should include both literals plus the concatenated literal"
    );
    assert!(
        wm.code[0].code.windows(2).any(|w| w == [0x42, 0x05]),
        "E4: str_len over literal concat should emit i64.const 5"
    );
}

#[test]
fn e4_string_cmp_lowers_to_literal_ordering() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new(
            "str_const",
            Some("a".into()),
            vec![Operand::Str("ALPHA".into())],
            "str",
        ),
        IIRInstr::new(
            "str_const",
            Some("b".into()),
            vec![Operand::Str("BETA".into())],
            "str",
        ),
        IIRInstr::new("str_cmp", Some("ord".into()), vec![
            Operand::Var("a".into()),
            Operand::Var("b".into()),
        ], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("ord".into())], "i64"),
    ]);

    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "E4: str_cmp should validate: {errs:?}");
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("E4: str_cmp should lower");
    assert!(
        wm.code[0].code.windows(2).any(|w| w == [0x42, 0x7f]),
        "E4: str_cmp over ALPHA/BETA should emit i64.const -1"
    );
}

#[test]
fn e4_string_slice_index_lowers_to_literal_byte_load() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new(
            "str_const",
            Some("s".into()),
            vec![Operand::Str("ABCDE".into())],
            "str",
        ),
        IIRInstr::new("const", Some("start".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("const", Some("end".into()), vec![Operand::Int(4)], "i64"),
        IIRInstr::new(
            "str_slice",
            Some("sub".into()),
            vec![
                Operand::Var("s".into()),
                Operand::Var("start".into()),
                Operand::Var("end".into()),
            ],
            "str",
        ),
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new(
            "str_index",
            Some("b".into()),
            vec![Operand::Var("sub".into()), Operand::Var("i".into())],
            "i64",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
    ]);

    let errs = validate_for_wasm(&m);
    assert!(
        errs.is_empty(),
        "E4: str_slice + str_index should validate: {errs:?}"
    );
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("E4: str_slice + str_index should lower");
    assert_eq!(
        wm.data[0].data, b"ABCDEBCD",
        "E4: string data should contain the source and sliced literal"
    );
    assert!(
        wm.code[0].code.contains(&0x2D),
        "E4: str_index over a slice should still emit i32.load8_u"
    );
}

#[test]
fn e4_string_index_lowers_to_literal_byte_load() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new(
            "str_const",
            Some("s".into()),
            vec![Operand::Str("ABC".into())],
            "str",
        ),
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("str_index", Some("b".into()), vec![
            Operand::Var("s".into()),
            Operand::Var("i".into()),
        ], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
    ]);

    let errs = validate_for_wasm(&m);
    assert!(errs.is_empty(), "E4: str_index should validate: {errs:?}");
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("E4: str_index should lower");
    assert_eq!(wm.data[0].data, b"ABC", "E4: string data should contain ABC");
    assert!(
        wm.code[0].code.contains(&0x2D),
        "E4: str_index should emit i32.load8_u"
    );
    assert!(
        wm.code[0].code.contains(&0xAD),
        "E4: i64 str_index result should zero-extend the loaded byte"
    );
}

#[test]
fn e4_string_print_coexists_with_putchar_newline_import() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new(
            "str_const",
            Some("s".into()),
            vec![Operand::Str("HELLO".into())],
            "str",
        ),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        IIRInstr::new("const", Some("nl".into()), vec![Operand::Int(10)], "i64"),
        IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("putchar".into()), Operand::Var("nl".into())],
            "void",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);

    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default())
        .expect("E4: string print plus putchar should lower");
    assert!(
        wm.imports.iter().any(|i| i.module_name == "env" && i.name == "__print_str"),
        "expected env.__print_str import"
    );
    assert!(
        wm.imports.iter().any(|i| i.module_name == "env" && i.name == "putchar"),
        "expected env.putchar import"
    );
    assert_eq!(wm.data[0].data, b"HELLO");
}

// ---------------------------------------------------------------------------
// ── Group: WasmGC i31ref box / unbox (LANG77 / McCarthy L3b-3a) ────────────
//
// The boxing primitives the uniform-anyref lisp value model needs: a lisp
// integer atom is boxed into an `i31ref` (a WasmGC tagged 31-bit integer
// reference) so it can live in a cons cell's `anyref` field, and unboxed back
// to a machine `i32` at the numeric boundary.
//
// Verified at the opcode-byte level (the repo has no WasmGC runtime/validator
// to execute the module — see the lang-aot wasm CHANGELOG):
//   ref.i31   = 0xFB 0x1C   (GcInstruction::I31New)
//   i31.get_s = 0xFB 0x1D   (GcInstruction::I31GetS)
// ---------------------------------------------------------------------------

/// True iff `needle` appears as a contiguous subsequence of `haystack`.
fn contains_subseq(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn box_unbox_round_trip_lowers_and_emits_i31_opcodes() {
    // fn main() -> i32 { unbox(box(const 7)) }
    let m = module_one(
        "main",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("box", Some("b".into()), vec![Operand::Var("v".into())], "ref<any>"),
            IIRInstr::new("unbox", Some("u".into()), vec![Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("u".into())], "i32"),
        ],
    );
    // lower_and_encode runs validation internally — its success proves box/unbox
    // are now accepted (no longer in UNSUPPORTED_OPS).
    let bytes = lower_and_encode(&m);
    assert!(
        contains_subseq(&bytes, &[0xFB, 0x1C]),
        "box must emit ref.i31 (0xFB 0x1C)",
    );
    assert!(
        contains_subseq(&bytes, &[0xFB, 0x1D]),
        "unbox must emit i31.get_s (0xFB 0x1D)",
    );
}

#[test]
fn box_and_unbox_are_no_longer_rejected_by_validation() {
    for (op, dest_ty) in [("box", "ref<any>"), ("unbox", "i32")] {
        let m = module_one(
            "main",
            vec![],
            "void",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
                IIRInstr::new(op, Some("d".into()), vec![Operand::Var("v".into())], dest_ty),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        );
        let errs = validate_for_wasm(&m);
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedOp")),
            "{op} must not be an UnsupportedOp anymore; got {errs:?}",
        );
    }
}

#[test]
fn box_without_dest_is_rejected() {
    let m = module_one(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
            IIRInstr::new("box", None, vec![Operand::Var("v".into())], "ref<any>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    assert!(lower_iir_to_wasm(&m, &IIRWasmConfig::default()).is_err());
}

// ---------------------------------------------------------------------------
// ── Group N: byte-tape ops + i64 conversions (LANG-MATRIX LM-W Brainfuck) ──
// ---------------------------------------------------------------------------
//
// `lower_brainfuck_for_aot` widens Brainfuck's cell/pointer registers to i64
// and rewrites the tape into `alloc_bytes` / `load_byte` / `store_byte`. These
// tests cover the wasm lowering of those ops + the i64↔i32 conversions they and
// the i64 loop guard need. Opcode bytes asserted: i32.load8_u=0x2D,
// i32.store8=0x3A, i32.wrap_i64=0xA7, i64.extend_i32_u=0xAD, i64.eqz=0x50.

/// `alloc_bytes`/`load_byte`/`store_byte` lower (the module validates, lowers,
/// and encodes) and pull in a linear memory + the byte/conversion opcodes.
#[test]
fn byte_tape_ops_lower_with_memory_and_conversions() {
    // A tape round-trip with i64 registers (the widened BF value model):
    //   const tape_size = 8 (i64)
    //   alloc_bytes tape <- tape_size       ; base offset 0
    //   const idx = 0 (i64)
    //   const val = 65 (i64)
    //   store_byte tape, idx, val           ; mem[0] = 65
    //   load_byte got <- tape, idx          ; got = 65 (zero-extended to i64)
    //   ret got
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("tape_size".into()), vec![Operand::Int(8)], "i64"),
        IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("tape_size".into())], "i64"),
        IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("const", Some("val".into()), vec![Operand::Int(65)], "i64"),
        IIRInstr::new("store_byte", None, vec![
            Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("val".into()),
        ], "i64"),
        IIRInstr::new("load_byte", Some("got".into()), vec![
            Operand::Var("tape".into()), Operand::Var("idx".into()),
        ], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i64"),
    ]);

    // Validates (no UnsupportedOp for the new ops).
    let errs = validate_for_wasm(&m);
    assert!(
        errs.iter().all(|e| !e.contains("UnsupportedOp")),
        "byte-tape ops must not be UnsupportedOp; errs: {:?}", errs
    );

    // Lowers, and the module carries a linear memory for the tape.
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    assert!(!wm.memories.is_empty(), "byte-tape ops must add a linear memory");

    // Encodes; the byte stream carries the memory ops + i64↔i32 conversions.
    let bytes = encode_module(&wm).expect("encoding failed");
    assert!(bytes.contains(&0x2Du8), "expected i32.load8_u (0x2D) for load_byte");
    assert!(bytes.contains(&0x3Au8), "expected i32.store8 (0x3A) for store_byte");
    assert!(bytes.contains(&0xA7u8), "expected i32.wrap_i64 (0xA7) narrowing an i64 addr/val");
    assert!(bytes.contains(&0xADu8), "expected i64.extend_i32_u (0xAD) widening the loaded byte");
}

/// `store_byte` with a dest is rejected — it produces no value.
#[test]
fn store_byte_with_dest_is_rejected() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("t".into()), vec![Operand::Int(8)], "i64"),
        IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("t".into())], "i64"),
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("store_byte", Some("oops".into()), vec![
            Operand::Var("tape".into()), Operand::Var("i".into()), Operand::Var("v".into()),
        ], "i64"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    assert!(
        lower_iir_to_wasm(&m, &IIRWasmConfig::default()).is_err(),
        "store_byte must not carry a dest"
    );
}

/// An i64 comparison result is widened to i64 (so it matches its i64-declared
/// local), and an i64 loop guard branches via `i64.eqz` — the dual fix that
/// keeps the module well-typed once Brainfuck's cells became i64.
#[test]
fn i64_condition_uses_i64_eqz_and_widened_cmp() {
    // c = (a == b)  with i64 operands → i64-declared `c`; then loop on it.
    //   label L
    //   c = cmp_eq a, b        ; i32 result widened to i64 (c is i64)
    //   jmp_if_false c, End     ; i64.eqz (not i32.eqz)
    //   jmp L
    //   label End
    //   ret a
    let m = module_one("main", vec![("a", "i64"), ("b", "i64")], "i64", vec![
        IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
        IIRInstr::new("cmp_eq", Some("c".into()), vec![
            Operand::Var("a".into()), Operand::Var("b".into()),
        ], "i64"),
        IIRInstr::new("jmp_if_false", None, vec![
            Operand::Var("c".into()), Operand::Var("End".into()),
        ], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("L".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("End".into())], "void"),
        IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wm).expect("encoding failed");
    // i64.eqz (0x50) for the i64 guard; i64.extend_i32_u (0xAD) widening the
    // i32 comparison boolean to the i64-declared `c`.
    assert!(bytes.contains(&0x50u8), "expected i64.eqz (0x50) for an i64 loop guard");
    assert!(bytes.contains(&0xADu8), "expected i64.extend_i32_u (0xAD) widening the i64 cmp result");
}

// ---------------------------------------------------------------------------
// ── Group 12: E4-dyn (E4d-3) runtime (branch-selected) strings ─────────────
// ---------------------------------------------------------------------------
//
// A string variable assigned by `str_const` in more than one basic block is
// chosen by control flow, so the compiler cannot fold it to one literal. Such a
// variable is promoted to a runtime **handle** = the i32 offset of a
// length-prefixed block `[i32 len (LE)][bytes]` in linear memory. `str_const`
// stores that offset; `print_str` reads the length back with `i32.load` (0x28)
// and passes `handle + 4` + that length to `env.__print_str(ptr, len)`. This is
// the WASM sibling of iir-to-llvm's E4d-2 `inttoptr` + `load` runtime path.

// Test 12.1 — a branch-selected string lowers to a runtime handle + i32.load.
#[test]
fn e4dyn_branch_selected_string_emits_runtime_handle_and_load() {
    // main():
    //   cond = 1
    //   if !cond goto Lelse
    //   Lthen: A = "HI"; goto Ldone
    //   Lelse: A = "LO"
    //   Ldone: print_str A
    //   ret_void
    // `A` is the dest of `str_const` in the two branch blocks, so it is promoted.
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("Lelse".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("Lthen".into())], "void"),
        IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("HI".into())], "str"),
        IIRInstr::new("jmp", None, vec![Operand::Var("Ldone".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("Lelse".into())], "void"),
        IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("LO".into())], "str"),
        IIRInstr::new("label", None, vec![Operand::Var("Ldone".into())], "void"),
        IIRInstr::new("print_str", None, vec![Operand::Var("A".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");

    // The runtime read: print_str of a promoted var reads the block length with
    // i32.load (0x28). The literal fast path never emits a load.
    assert!(
        wm.code[0].code.contains(&0x28),
        "print_str of a branch-selected string must read its length with i32.load (0x28)"
    );

    // The data segment carries a length-prefixed block for each distinct
    // literal: `[len=2 (i32 LE)][b'H', b'I']` and `[len=2][b'L', b'O']`.
    let data = &wm.data[0].data;
    let hi_block = [2u8, 0, 0, 0, b'H', b'I'];
    let lo_block = [2u8, 0, 0, 0, b'L', b'O'];
    assert!(
        data.windows(hi_block.len()).any(|w| w == hi_block),
        "data segment must contain the length-prefixed \"HI\" runtime block"
    );
    assert!(
        data.windows(lo_block.len()).any(|w| w == lo_block),
        "data segment must contain the length-prefixed \"LO\" runtime block"
    );

    // Encoding still succeeds (well-formed module).
    encode_module(&wm).expect("encoding failed");
}

// Test 12.2 — a single-block string keeps the folded literal fast path.
#[test]
fn e4dyn_single_block_string_keeps_literal_fast_path() {
    // `s` is assigned once (one block), so it stays a compile-time literal: the
    // length is folded, no runtime i32.load is emitted, and the data segment
    // holds only the raw bytes (no 4-byte length prefix).
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    assert!(
        !wm.code[0].code.contains(&0x28),
        "single-assignment string must fold to a compile-time length (no i32.load)"
    );
    assert_eq!(
        wm.data[0].data, b"HI",
        "fast-path string must not add a length-prefixed runtime block"
    );
}

// Test 12.3 — a straight-line reassignment stays on the fast path (matches the
// E4d-2 rule: promotion needs *distinct basic blocks*, not just two writes).
#[test]
fn e4dyn_straight_line_reassignment_is_not_promoted() {
    // `s := "OK"; s := "NO"; print s` — two writes, one block. The last-writer
    // literal tracking is exactly right, so no runtime handle and no i32.load.
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("OK".into())], "str"),
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("NO".into())], "str"),
        IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    assert!(
        !wm.code[0].code.contains(&0x28),
        "a straight-line reassignment must not be promoted to a runtime handle"
    );
}

// ===========================================================================
// E4-dyn (E4d-3b): a runtime string as a function RETURN VALUE / call result
// ===========================================================================

/// An ALGOL `string procedure` lowers to a function returning `str` — carried
/// as an i32 **handle** — that the caller prints. `str` boundaries type as i32,
/// and `print_str` of a *call result* (which has no compile-time literal entry)
/// reads the length from the `[i32 len][bytes]` block header at run time
/// (`i32.load`, opcode 0x28).
#[test]
fn e4dyn_wasm_string_procedure_return_and_call_result_print() {
    // pick(n) -> str : if n > 0 then "HI" else "LO"  (branch-selected → runtime)
    let pick = (
        "pick",
        vec![("n", "i64")],
        "str",
        vec![
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str(String::new())], "str"),
            IIRInstr::new("const", Some("c0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("cmp_gt", Some("t".into()),
                vec![Operand::Var("n".into()), Operand::Var("c0".into())], "i64"),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("t".into()), Operand::Var("Lelse".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("jmp", None, vec![Operand::Var("Ldone".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("Lelse".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("LO".into())], "str"),
            IIRInstr::new("label", None, vec![Operand::Var("Ldone".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("pick".into())], "str"),
        ],
    );
    // main() : print(pick(1))
    let main = (
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("call", Some("r".into()),
                vec![Operand::Var("pick".into()), Operand::Var("one".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("r".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
        ],
    );
    let m = module_multi(vec![pick, main]);

    // Validation accepts `str` on `call` and `ret`.
    assert!(validate_for_wasm(&m).is_empty(), "str on call/ret must validate: {:?}", validate_for_wasm(&m));

    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");

    // `pick`'s function type returns i32 (a str handle).
    let pick_idx = wm.exports.iter().position(|e| e.name == "pick").expect("pick exported");
    let pick_type = wm.functions[pick_idx];
    assert_eq!(wm.types[pick_type as usize].results, vec![ValueType::I32],
        "string procedure returns an i32 handle");

    // The print of the call result uses the runtime path: i32.load (0x28).
    let main_code = &wm.code[wm.exports.iter().position(|e| e.name == "main").unwrap()].code;
    assert!(main_code.contains(&0x28),
        "print of a call-result runtime string must read its length with i32.load (0x28)");

    // Well-formed module.
    encode_module(&wm).expect("encoding failed");
}

/// E4-dyn: BASIC string `INPUT A$` lowers `call_builtin "input_str"` (a `str`
/// result) to a call to the imported `env.__input_str`, and the module declares
/// that import. The `str` result + `mov` must pass the validator's str-type gate,
/// and the encoded module must reference the import by name.
#[test]
fn input_str_lowers_and_declares_env_import() {
    let m = module_one("main", vec![], "i64", vec![
        IIRInstr::new("call_builtin", Some("t".into()),
            vec![Operand::Var("input_str".into())], "str"),
        IIRInstr::new("mov", Some("s".into()), vec![Operand::Var("t".into())], "str"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
    ]);
    assert!(validate_for_wasm(&m).is_empty(),
        "str call_builtin \"input_str\" + str mov must validate for WASM");
    let bytes = lower_and_encode(&m);
    // The import name "__input_str" appears verbatim in the wasm import section.
    let needle = b"__input_str";
    assert!(
        bytes.windows(needle.len()).any(|w| w == needle),
        "encoded module must declare the env.__input_str import"
    );
}

/// Twig GC completion round (Part 3): a module that bump-allocates (here, a
/// single `alloc_array`) must declare linear memory with room to actually
/// grow — the confirmed bug this round fixes was `Limits { min: 1, max:
/// Some(1) }` hardcoded on every memory-using module, with no `memory.grow`
/// call anywhere in the emitted bytecode, so any program allocating past the
/// first 64 KiB page had no path forward. `$__ensure_capacity` (called from
/// every bump-allocation site) now emits real `memory.grow`; this asserts the
/// *other* half of the fix — the module's own declared `max` must actually
/// permit that growth up to the WASM spec's absolute ceiling.
#[test]
fn alloc_array_module_declares_growable_memory_not_capped_at_one_page() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("const", Some("count".into()), vec![Operand::Int(3)], "i64"),
        IIRInstr::new("alloc_array", Some("h".into()), vec![Operand::Var("count".into())], "array<i64>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    let mem = wm.memories.first().expect("alloc_array module must declare linear memory");
    assert_eq!(mem.limits.min, 1, "still starts at a single 64 KiB page");
    assert_eq!(
        mem.limits.max,
        Some(65536),
        "max must allow growth to the WASM spec ceiling, not stay capped at 1 page"
    );
    encode_module(&wm).expect("encoding failed");
}

/// The companion codegen-shape check: an `alloc_array` module must actually
/// emit `memory.grow` (0x40) and `memory.size` (0x3F) somewhere in its code —
/// not just declare a growable memory section. Both opcodes are followed by
/// the reserved memory-index byte `0x00` (this backend only ever declares one
/// memory), so the two-byte pairs are unambiguous needles.
#[test]
fn alloc_array_emits_memory_grow_and_memory_size_opcodes() {
    let m = module_one("main", vec![], "void", vec![
        IIRInstr::new("const", Some("count".into()), vec![Operand::Int(3)], "i64"),
        IIRInstr::new("alloc_array", Some("h".into()), vec![Operand::Var("count".into())], "array<i64>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    let ensure_capacity_code = wm.code.last().expect("$__ensure_capacity body appended").code.as_slice();
    assert!(
        ensure_capacity_code.windows(2).any(|w| w == [0x3F, 0x00]),
        "$__ensure_capacity must call memory.size"
    );
    assert!(
        ensure_capacity_code.windows(2).any(|w| w == [0x40, 0x00]),
        "$__ensure_capacity must call memory.grow"
    );
}
