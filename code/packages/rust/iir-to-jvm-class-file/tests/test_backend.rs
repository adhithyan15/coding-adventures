//! Integration tests for `iir-to-jvm-class-file`.
//!
//! These tests exercise the full pipeline: validate → lower → inspect
//! the resulting `JvmClassFile`.  They are intentionally analogous to the
//! BEAM backend tests so the two backends can be cross-checked.
//!
//! # Organisation
//!
//! Tests are grouped by theme:
//!
//! 1.  Validation errors
//! 2.  Config and constructor
//! 3.  Successful lowering — basic structure checks
//! 4.  Instruction-level bytecode coverage (non-empty code)
//! 5.  Multi-function modules
//! 6.  Float support
//! 7.  Comparison synthesis
//! 8.  Control flow (labels, jumps)
//! 9.  Call instruction
//! 10. Codegen adapter

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_jvm_class_file::{
    codegen::IIRJvmCodeGenerator, lower_iir_to_jvm, serialize_jvm_class_file, validate_for_jvm,
    IIRJvmConfig, IIRJvmError, JvmClassFile,
};
use jvm_class_file::{JvmConstantPoolEntry, JvmMethodInfo};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a module containing a single function.
fn module_with(func: IIRFunction) -> IIRModule {
    let name = func.name.clone();
    IIRModule {
        name: "test".into(),
        functions: vec![func],
        entry_point: Some(name),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

/// Convenience: lower a module and unwrap, panicking with a message on error.
fn lower(module: &IIRModule) -> JvmClassFile {
    lower_iir_to_jvm(module, &IIRJvmConfig::new("TestClass"))
        .expect("lowering should succeed")
}

/// Return a simple `ret_void` void function.
fn void_fn(name: &str) -> IIRFunction {
    IIRFunction::new(
        name,
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    )
}

/// Return an `add` function: `fn add(a: i32, b: i32) -> i32`.
fn add_fn() -> IIRFunction {
    IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    )
}

// ===========================================================================
// 1. Validation errors
// ===========================================================================

/// An empty module (no functions) must be rejected.
#[test]
fn validate_rejects_empty_module() {
    let module = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errors = validate_for_jvm(&module);
    assert!(!errors.is_empty(), "empty module should produce errors");
    assert!(errors[0].contains("EmptyModule"));
}

/// A function with no instructions must be rejected.
#[test]
fn validate_rejects_empty_function() {
    let func = IIRFunction::new("main", vec![], "void", vec![]);
    let errors = validate_for_jvm(&module_with(func));
    assert!(!errors.is_empty());
    assert!(errors[0].contains("EmptyFunction"));
}

/// The `"any"` type hint must be rejected (unresolved type).
#[test]
fn validate_rejects_any_type_hint() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new(
            "add",
            Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "any",
        )],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UntypedInstruction")));
}

/// The `"polymorphic"` type hint must be rejected (mega-morphic = not specialisable).
#[test]
fn validate_rejects_polymorphic_type_hint() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("add", Some("v".into()), vec![], "polymorphic")],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UntypedInstruction")));
}

/// The `"str"` type hint is not supported for value-producing arithmetic ops
/// like `const` (no string arithmetic in v1).
#[test]
fn validate_rejects_str_type() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("const", Some("v".into()), vec![], "str")],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType")));
}

/// E4-dyn: a `str` VALUE (a `java.lang.String`) flows through a `call` (a `str`
/// return / call result) and a `ret` (a `str`-returning method) — an ALGOL
/// `string procedure`'s returned runtime string. Those two ops accept `str`.
#[test]
fn validate_accepts_str_on_call_and_ret() {
    // id(s: str) -> str { ret str s }
    let id = IIRFunction::new(
        "id",
        vec![("s".into(), "str".into())],
        "str",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "str")],
    );
    // main() { r = call id(s0); ... }  (str call result)
    let main = IIRFunction::new(
        "main",
        vec![("s0".into(), "str".into())],
        "void",
        vec![
            IIRInstr::new("call", Some("r".into()),
                vec![Operand::Var("id".into()), Operand::Var("s0".into())], "str"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = IIRModule {
        name: "strproc".into(),
        functions: vec![id, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "str on call/ret must validate; got: {errors:?}");
}

/// Reference types `ref<T>` are not supported.
#[test]
fn validate_rejects_ref_type() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("const", Some("v".into()), vec![], "ref<i32>")],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType")));
}

/// The `call_builtin` opcode is rejected.
#[test]
fn validate_rejects_call_builtin() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("call_builtin", None, vec![], "void")],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")));
}

/// The `safepoint` opcode is rejected.
#[test]
fn validate_rejects_safepoint() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("safepoint", None, vec![], "void")],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")));
}

/// Float type hints are accepted (unlike the BEAM backend).
#[test]
fn validate_accepts_f32_type() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    // The module itself is clean — we just want to confirm a separate f32
    // instruction wouldn't be blocked.
    let f32_func = IIRFunction::new(
        "floatfn",
        vec![("x".into(), "f32".into())],
        "f32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "f32")],
    );
    let module = IIRModule {
        name: "test".into(),
        functions: vec![func, f32_func],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errors = validate_for_jvm(&module);
    assert!(
        !errors.iter().any(|e| e.contains("UnsupportedType")),
        "f32 should be accepted, got errors: {:?}",
        errors
    );
}

/// Float constant operands are accepted (unlike the BEAM backend).
// `3.14` is an arbitrary float operand payload, not an approximation of PI.
#[allow(clippy::approx_constant)]
#[test]
fn validate_accepts_float_const_operand() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "const",
                Some("v".into()),
                vec![Operand::Float(3.14)],
                "f64",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        !errors.iter().any(|e| e.contains("Float") || e.contains("UnsupportedType")),
        "float const should be allowed, got: {:?}",
        errors
    );
}

// ===========================================================================
// 2. Config and constructor
// ===========================================================================

/// Default config should use `"IIRModule"` as the class name.
#[test]
fn config_default_class_name() {
    let cfg = IIRJvmConfig::default();
    assert_eq!(cfg.class_name, "IIRModule");
}

/// `IIRJvmConfig::new` should accept any string.
#[test]
fn config_new_sets_class_name() {
    let cfg = IIRJvmConfig::new("com/example/Foo");
    assert_eq!(cfg.class_name, "com/example/Foo");
}

// ===========================================================================
// 3. Successful lowering — basic structure
// ===========================================================================

/// A simple void function should lower successfully.
#[test]
fn lower_void_fn_succeeds() {
    let module = module_with(void_fn("main"));
    let result = lower_iir_to_jvm(&module, &IIRJvmConfig::new("Test"));
    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
}

/// The class name in the output should match the config.
#[test]
fn lower_class_name_from_config() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.this_class_name, "TestClass");
}

/// The super class should always be `java/lang/Object`.
#[test]
fn lower_super_class_is_object() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.super_class_name, "java/lang/Object");
}

/// We target Java 5 (major version 49) to avoid the mandatory StackMapTable
/// attribute required by the Java 7+ verifier for branching methods.
#[test]
fn lower_version_is_java5() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.version.major, 49);
    assert_eq!(class.version.minor, 0);
}

/// One function in → one method out.
#[test]
fn lower_one_function_produces_one_method() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.methods.len(), 1);
}

/// Method name should match the function name.
#[test]
fn lower_method_name_matches_function_name() {
    let module = module_with(void_fn("myFunc"));
    let class = lower(&module);
    assert_eq!(class.methods[0].name, "myFunc");
}

/// Method should be public and static.
#[test]
fn lower_method_is_public_static() {
    use jvm_class_file::{ACC_PUBLIC, ACC_STATIC};
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    let flags = class.methods[0].access_flags;
    assert!(flags & ACC_PUBLIC != 0, "should be public");
    assert!(flags & ACC_STATIC != 0, "should be static");
}

/// Every method must have a non-empty Code attribute.
#[test]
fn lower_method_has_non_empty_code() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    let code = class.methods[0].code_attribute().unwrap();
    assert!(!code.code.is_empty(), "bytecode should not be empty");
}

/// An empty module should produce a ValidationFailed error.
#[test]
fn lower_empty_module_returns_error() {
    let module = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let result = lower_iir_to_jvm(&module, &IIRJvmConfig::default());
    assert!(matches!(result, Err(IIRJvmError::ValidationFailed(_))));
}

/// Void descriptor: `() -> void` should produce `"()V"`.
#[test]
fn lower_method_descriptor_void() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.methods[0].descriptor, "()V");
}

/// `fn add(a: i32, b: i32) -> i32` should produce descriptor `"(II)I"`.
#[test]
fn lower_method_descriptor_int_params() {
    let module = module_with(add_fn());
    let class = lower(&module);
    assert_eq!(class.methods[0].descriptor, "(II)I");
}

// ===========================================================================
// 4. Instruction-level bytecode coverage
// ===========================================================================

/// The `ret_void` instruction should produce non-empty bytecode.
#[test]
fn bytecode_ret_void_non_empty() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// An `add` instruction should produce non-empty bytecode.
#[test]
fn bytecode_add_non_empty() {
    let module = module_with(add_fn());
    let class = lower(&module);
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `sub` instruction.
#[test]
fn bytecode_sub_non_empty() {
    let func = IIRFunction::new(
        "sub",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "sub",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `const` instruction with an integer operand.
#[test]
fn bytecode_const_int_non_empty() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `neg` instruction.
#[test]
fn bytecode_neg_non_empty() {
    let func = IIRFunction::new(
        "negate",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("neg", Some("r".into()), vec![Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// An `and` instruction.
#[test]
fn bytecode_and_non_empty() {
    let func = IIRFunction::new(
        "band",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "and",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// An `or` instruction.
#[test]
fn bytecode_or_non_empty() {
    let func = IIRFunction::new(
        "bor",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "or",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `shl` instruction.
#[test]
fn bytecode_shl_non_empty() {
    let func = IIRFunction::new(
        "shift",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "shl",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `not` instruction.
#[test]
fn bytecode_not_non_empty() {
    let func = IIRFunction::new(
        "bnot",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("not", Some("r".into()), vec![Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `div` instruction.
#[test]
fn bytecode_div_non_empty() {
    let func = IIRFunction::new(
        "divide",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "div",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// A `mod` (remainder) instruction.
#[test]
fn bytecode_mod_non_empty() {
    let func = IIRFunction::new(
        "modulo",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "mod",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

// ===========================================================================
// 5. Multi-function modules
// ===========================================================================

/// Two functions produce two methods in the right order.
#[test]
fn lower_two_functions_two_methods() {
    let module = IIRModule {
        name: "multi".into(),
        functions: vec![void_fn("foo"), void_fn("bar")],
        entry_point: Some("foo".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = lower(&module);
    assert_eq!(class.methods.len(), 2);
}

/// Method names should match function names in order.
#[test]
fn lower_two_functions_method_names() {
    let module = IIRModule {
        name: "multi".into(),
        functions: vec![void_fn("alpha"), void_fn("beta")],
        entry_point: Some("alpha".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = lower(&module);
    assert_eq!(class.methods[0].name, "alpha");
    assert_eq!(class.methods[1].name, "beta");
}

/// Three functions produce exactly three methods.
#[test]
fn lower_three_functions_three_methods() {
    let module = IIRModule {
        name: "tri".into(),
        functions: vec![void_fn("a"), void_fn("b"), void_fn("c")],
        entry_point: Some("a".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = lower(&module);
    assert_eq!(class.methods.len(), 3);
}

/// Each method in a multi-function module must have non-empty code.
#[test]
fn lower_multi_fn_all_methods_have_code() {
    let module = IIRModule {
        name: "multi".into(),
        functions: vec![void_fn("x"), void_fn("y")],
        entry_point: Some("x".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = lower(&module);
    for method in &class.methods {
        let code = method.code_attribute().unwrap();
        assert!(!code.code.is_empty(), "method {} has empty code", method.name);
    }
}

// ===========================================================================
// 6. Float support
// ===========================================================================

/// A function with f32 parameters and return type should lower successfully.
#[test]
fn lower_f32_function_ok() {
    let func = IIRFunction::new(
        "fadd",
        vec![("a".into(), "f32".into()), ("b".into(), "f32".into())],
        "f32",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "f32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f32"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("FloatTest"));
    assert!(result.is_ok(), "f32 function should lower ok: {:?}", result.err());
}

/// f32 method descriptor should be `"(FF)F"`.
#[test]
fn lower_f32_method_descriptor() {
    let func = IIRFunction::new(
        "fadd",
        vec![("a".into(), "f32".into()), ("b".into(), "f32".into())],
        "f32",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "f32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f32"),
        ],
    );
    let class = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("FloatTest")).unwrap();
    assert_eq!(class.methods[0].descriptor, "(FF)F");
}

/// A function with f64 parameters and return type should lower successfully.
#[test]
fn lower_f64_function_ok() {
    let func = IIRFunction::new(
        "dadd",
        vec![("a".into(), "f64".into()), ("b".into(), "f64".into())],
        "f64",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "f64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("DoubleTest"));
    assert!(result.is_ok(), "f64 function should lower ok: {:?}", result.err());
}

/// f64 method descriptor should be `"(DD)D"`.
#[test]
fn lower_f64_method_descriptor() {
    let func = IIRFunction::new(
        "dadd",
        vec![("a".into(), "f64".into()), ("b".into(), "f64".into())],
        "f64",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "f64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ],
    );
    let class = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("DoubleTest")).unwrap();
    assert_eq!(class.methods[0].descriptor, "(DD)D");
}

/// A `const` with float operand and f64 type should lower.
// `3.14` is an arbitrary float operand payload, not an approximation of PI.
#[allow(clippy::approx_constant)]
#[test]
fn lower_const_float_operand_ok() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "float const should lower ok: {:?}", result.err());
}

// ===========================================================================
// 7. Comparison synthesis
// ===========================================================================

/// `cmp_eq` should produce non-empty bytecode.
#[test]
fn bytecode_cmp_eq_non_empty() {
    let func = IIRFunction::new(
        "eq",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "cmp_eq",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// `cmp_ne` should produce non-empty bytecode.
#[test]
fn bytecode_cmp_ne_non_empty() {
    let func = IIRFunction::new(
        "ne",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "cmp_ne",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// `cmp_lt` should produce non-empty bytecode.
#[test]
fn bytecode_cmp_lt_non_empty() {
    let func = IIRFunction::new(
        "lt",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "cmp_lt",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

/// `cmp_gt` should produce non-empty bytecode.
#[test]
fn bytecode_cmp_gt_non_empty() {
    let func = IIRFunction::new(
        "gt",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "cmp_gt",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let class = lower(&module_with(func));
    assert!(!class.methods[0].code_attribute().unwrap().code.is_empty());
}

// ===========================================================================
// 8. Control flow
// ===========================================================================

/// A function with a `label` instruction should lower correctly.
#[test]
fn lower_label_instruction_ok() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "{:?}", result.err());
}

/// A function with a `jmp` to a label should lower correctly.
#[test]
fn lower_jmp_to_label_ok() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("exit".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("exit".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "jmp should succeed: {:?}", result.err());
}

/// A backward jump (loop) should lower correctly.
#[test]
fn lower_backward_jump_ok() {
    let func = IIRFunction::new(
        "infinite_loop",
        vec![],
        "void",
        vec![
            IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
            IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "backward jump should succeed: {:?}", result.err());
}

/// A `jmp_if_true` with a boolean condition variable.
#[test]
fn lower_jmp_if_true_ok() {
    let func = IIRFunction::new(
        "conditional",
        vec![("cond".into(), "i32".into())],
        "void",
        vec![
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("done".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "jmp_if_true should succeed: {:?}", result.err());
}

/// A `jmp_if_false` with a boolean condition variable.
#[test]
fn lower_jmp_if_false_ok() {
    let func = IIRFunction::new(
        "conditional_false",
        vec![("cond".into(), "i32".into())],
        "void",
        vec![
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("done".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "jmp_if_false should succeed: {:?}", result.err());
}

// ===========================================================================
// 9. Call instruction
// ===========================================================================

/// A `call` to another function in the same module should lower to Ok.
#[test]
fn lower_call_same_module_ok() {
    let callee = void_fn("helper");
    let caller = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "call",
                None,
                vec![Operand::Var("helper".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = IIRModule {
        name: "call_test".into(),
        functions: vec![callee, caller],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let result = lower_iir_to_jvm(&module, &IIRJvmConfig::new("CallTest"));
    assert!(result.is_ok(), "call should succeed: {:?}", result.err());
}

/// A `call` should produce non-empty bytecode in the caller.
#[test]
fn bytecode_call_non_empty() {
    let callee = void_fn("helper");
    let caller = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = IIRModule {
        name: "call_test".into(),
        functions: vec![callee, caller],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("CallTest")).unwrap();
    // Find the "main" method (index 1)
    let main_method = class.methods.iter().find(|m| m.name == "main").unwrap();
    assert!(!main_method.code_attribute().unwrap().code.is_empty());
}

// ===========================================================================
// 10. Codegen adapter
// ===========================================================================

/// `IIRJvmCodeGenerator::name()` should return `"iir-jvm"`.
#[test]
fn codegen_name_is_iir_jvm() {
    let gen = IIRJvmCodeGenerator::default_class();
    assert_eq!(gen.name(), "iir-jvm");
}

/// `IIRJvmCodeGenerator::validate()` returns empty on a valid module.
#[test]
fn codegen_validate_empty_on_valid() {
    let gen = IIRJvmCodeGenerator::default_class();
    let module = module_with(void_fn("main"));
    assert!(gen.validate(&module).is_empty());
}

/// `IIRJvmCodeGenerator::validate()` returns errors on an empty module.
#[test]
fn codegen_validate_errors_on_empty_module() {
    let gen = IIRJvmCodeGenerator::default_class();
    let empty = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert!(!gen.validate(&empty).is_empty());
}

/// `IIRJvmCodeGenerator::generate()` returns a class file with the right class name.
#[test]
fn codegen_generate_class_name() {
    let gen = IIRJvmCodeGenerator::new("GenClass");
    let module = module_with(void_fn("main"));
    let class = gen.generate(&module);
    assert_eq!(class.this_class_name, "GenClass");
}

/// `IIRJvmCodeGenerator::generate()` produces methods for all functions.
#[test]
fn codegen_generate_method_count() {
    let gen = IIRJvmCodeGenerator::new("Multi");
    let module = IIRModule {
        name: "multi".into(),
        functions: vec![void_fn("a"), void_fn("b"), void_fn("c")],
        entry_point: Some("a".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let class = gen.generate(&module);
    assert_eq!(class.methods.len(), 3);
}

/// `IIRJvmCodeGenerator` is `Send + Sync`.
#[test]
fn codegen_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<IIRJvmCodeGenerator>();
}

/// Long (i64) function lowers successfully.
#[test]
fn lower_long_fn_ok() {
    let func = IIRFunction::new(
        "ladd",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("LongTest"));
    assert!(result.is_ok(), "long add should lower ok: {:?}", result.err());
}

/// i64 method descriptor should be `"(JJ)J"`.
#[test]
fn lower_long_method_descriptor() {
    let func = IIRFunction::new(
        "ladd",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let class = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("LongTest")).unwrap();
    assert_eq!(class.methods[0].descriptor, "(JJ)J");
}

/// `type_assert` is silently erased — the module still lowers ok.
#[test]
fn lower_type_assert_is_nop() {
    let func = IIRFunction::new(
        "main",
        vec![("x".into(), "i32".into())],
        "void",
        vec![
            IIRInstr::new(
                "type_assert",
                None,
                vec![Operand::Var("x".into())],
                "i32",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "type_assert should be a nop: {:?}", result.err());
}

/// A `const` with bool operand should lower to Ok.
#[test]
fn lower_const_bool_ok() {
    let func = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("t".into()), vec![Operand::Bool(true)], "bool"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::default());
    assert!(result.is_ok(), "bool const should lower ok: {:?}", result.err());
}

// ===========================================================================
// 11. Heap ops — Object[] cons cells (Phase 2)
// ===========================================================================
//
// The JVM backend in Phase 2 supports Lispy cons cells via `Object[]` arrays.
// No Java class definitions are needed — the JVM GC manages them.
//
// Cons cell layout:
//   Object[] pair = new Object[2];
//   pair[0] = head;   // car
//   pair[1] = tail;   // cdr
//
// nil  ↔  null
// cons ↔  alloc ref<LispyPair> + field_store[0] + field_store[1]
// car  ↔  field_load index 0
// cdr  ↔  field_load index 1
// null? ↔ is_null

/// `alloc ref<LispyPair>` + two `field_store` instructions (the cons pattern)
/// must produce valid class bytes without panic.
#[test]
fn heap_cons_cell_alloc_and_field_stores_ok() {
    // Build: fn cons(head: ref<LispyPair>, tail: ref<LispyPair>) -> ref<LispyPair>
    //   %pair = alloc ref<LispyPair>
    //   field_store %pair, 0, %head
    //   field_store %pair, 1, %tail
    //   ret %pair
    let func = IIRFunction::new(
        "mk_pair",
        vec![
            ("head".into(), "ref<LispyPair>".into()),
            ("tail".into(), "ref<LispyPair>".into()),
        ],
        "ref<LispyPair>",
        vec![
            IIRInstr::new("alloc", Some("pair".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("pair".into()),
                    Operand::Int(0),
                    Operand::Var("head".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("pair".into()),
                    Operand::Int(1),
                    Operand::Var("tail".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("pair".into())], "ref<LispyPair>"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("PairClass"));
    assert!(result.is_ok(), "cons pattern should lower ok: {:?}", result.err());
    // The bytecode must be non-empty and the method must exist.
    let class = result.unwrap();
    let method = class.methods.iter().find(|m| m.name == "mk_pair").unwrap();
    assert!(!method.code_attribute().unwrap().code.is_empty());
}

/// `field_load` at index 0 (car) compiles without error.
#[test]
fn heap_field_load_car_ok() {
    // fn car(pair: ref<LispyPair>) -> ref<LispyPair>
    //   %h = field_load %pair, 0
    //   ret %h
    let func = IIRFunction::new(
        "car",
        vec![("pair".into(), "ref<LispyPair>".into())],
        "ref<LispyPair>",
        vec![
            IIRInstr::new(
                "field_load",
                Some("h".into()),
                vec![Operand::Var("pair".into()), Operand::Int(0)],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "ref<LispyPair>"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("CarTest"));
    assert!(result.is_ok(), "car (field_load 0) should lower ok: {:?}", result.err());
}

/// `field_load` at index 1 (cdr) compiles without error.
#[test]
fn heap_field_load_cdr_ok() {
    // fn cdr(pair: ref<LispyPair>) -> ref<LispyPair>
    //   %t = field_load %pair, 1
    //   ret %t
    let func = IIRFunction::new(
        "cdr",
        vec![("pair".into(), "ref<LispyPair>".into())],
        "ref<LispyPair>",
        vec![
            IIRInstr::new(
                "field_load",
                Some("t".into()),
                vec![Operand::Var("pair".into()), Operand::Int(1)],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("t".into())], "ref<LispyPair>"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("CdrTest"));
    assert!(result.is_ok(), "cdr (field_load 1) should lower ok: {:?}", result.err());
}

/// `is_null` compiles and produces non-empty bytecode.
#[test]
fn heap_is_null_ok() {
    // fn null_check(pair: ref<LispyPair>) -> bool
    //   %r = is_null %pair
    //   ret %r
    let func = IIRFunction::new(
        "null_check",
        vec![("pair".into(), "ref<LispyPair>".into())],
        "bool",
        vec![
            IIRInstr::new(
                "is_null",
                Some("r".into()),
                vec![Operand::Var("pair".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "bool"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("NullTest"));
    assert!(result.is_ok(), "is_null should lower ok: {:?}", result.err());
    let class = result.unwrap();
    let method = class.methods.iter().find(|m| m.name == "null_check").unwrap();
    assert!(!method.code_attribute().unwrap().code.is_empty());
}

/// `const ref<LispyPair>` (nil) compiles to aconst_null + astore.
#[test]
fn heap_const_nil_ok() {
    // fn make_nil() -> ref<LispyPair>
    //   %n = const ref<LispyPair>   ; nil
    //   ret %n
    let func = IIRFunction::new(
        "make_nil",
        vec![],
        "ref<LispyPair>",
        vec![
            IIRInstr::new(
                "const",
                Some("n".into()),
                vec![Operand::Int(0)], // Int(0) signals nil — value ignored
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "ref<LispyPair>"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("NilTest"));
    assert!(result.is_ok(), "const nil should lower ok: {:?}", result.err());
    // Verify the bytecode contains ACONST_NULL (0x01).
    let class = result.unwrap();
    let code = class.methods[0].code_attribute().unwrap().code.clone();
    assert!(
        code.contains(&0x01u8),
        "bytecode should contain ACONST_NULL (0x01), got: {:?}",
        code
    );
}

/// `alloc` with a non-LispyPair `ref<…>` type is rejected by the validator,
/// so `lower_iir_to_jvm` should return a `ValidationFailed` error.
#[test]
fn heap_alloc_wrong_ref_type_rejected() {
    let func = IIRFunction::new(
        "bad_alloc",
        vec![],
        "void",
        vec![
            IIRInstr::new("alloc", Some("x".into()), vec![], "ref<SomeOtherType>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("BadAlloc"));
    assert!(
        matches!(result, Err(IIRJvmError::ValidationFailed(_))),
        "alloc ref<SomeOtherType> should produce ValidationFailed, got: {:?}",
        result
    );
}

/// Hand-crafted `length` function: alloc pair, car, cdr, is_null, conditional branch.
///
/// This is a realistic Lispy-style recursive length skeleton (not fully
/// recursive — just one level to exercise all four heap ops in a single
/// function body without needing recursion in IIR).
#[test]
fn heap_length_like_function_ok() {
    // Simulates:
    //   fn length_step(pair: ref<LispyPair>) -> i32
    //     %is_end = is_null %pair         ; is this nil?
    //     jmp_if_true %is_end, done       ; yes: return 0
    //     %head = field_load %pair, 0     ; car
    //     %tail = field_load %pair, 1     ; cdr
    //     %is_tail_nil = is_null %tail
    //     %one = const 1 : i32
    //     jmp done
    //     label done
    //     ret %one
    let func = IIRFunction::new(
        "length_step",
        vec![("pair".into(), "ref<LispyPair>".into())],
        "i32",
        vec![
            IIRInstr::new(
                "is_null",
                Some("is_end".into()),
                vec![Operand::Var("pair".into())],
                "bool",
            ),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("is_end".into()), Operand::Var("done".into())],
                "void",
            ),
            IIRInstr::new(
                "field_load",
                Some("head".into()),
                vec![Operand::Var("pair".into()), Operand::Int(0)],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "field_load",
                Some("tail".into()),
                vec![Operand::Var("pair".into()), Operand::Int(1)],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "is_null",
                Some("is_tail_nil".into()),
                vec![Operand::Var("tail".into())],
                "bool",
            ),
            IIRInstr::new(
                "const",
                Some("one".into()),
                vec![Operand::Int(1)],
                "i32",
            ),
            IIRInstr::new("jmp", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("one".into())], "i32"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("LengthTest"));
    assert!(result.is_ok(), "length-like function should lower ok: {:?}", result.err());
    let class = result.unwrap();
    let method = class.methods.iter().find(|m| m.name == "length_step").unwrap();
    assert!(!method.code_attribute().unwrap().code.is_empty());
}

/// A bare `field_store` (not immediately preceded by `alloc`) compiles on its own.
///
/// This covers the case where a pair was allocated earlier and we update a
/// field in a later instruction.
#[test]
fn heap_bare_field_store_ok() {
    // fn update_head(pair: ref<LispyPair>, new_head: ref<LispyPair>) -> void
    //   field_store %pair, 0, %new_head
    //   ret_void
    let func = IIRFunction::new(
        "update_head",
        vec![
            ("pair".into(), "ref<LispyPair>".into()),
            ("new_head".into(), "ref<LispyPair>".into()),
        ],
        "void",
        vec![
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("pair".into()),
                    Operand::Int(0),
                    Operand::Var("new_head".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("UpdateTest"));
    assert!(result.is_ok(), "bare field_store should lower ok: {:?}", result.err());
}

/// Multiple alloc+field_store sequences in one function.
///
/// Each sequence must produce independent Object[] arrays.  Both pairs are
/// fully initialised in the same function body.
#[test]
fn heap_two_cons_sequences_in_one_function_ok() {
    // fn two_pairs(a: ref<LispyPair>, b: ref<LispyPair>, nil: ref<LispyPair>)
    //     -> ref<LispyPair>
    //   %p1 = alloc ref<LispyPair>
    //   field_store %p1, 0, %a
    //   field_store %p1, 1, %nil
    //   %p2 = alloc ref<LispyPair>
    //   field_store %p2, 0, %b
    //   field_store %p2, 1, %p1
    //   ret %p2
    let func = IIRFunction::new(
        "two_pairs",
        vec![
            ("a".into(), "ref<LispyPair>".into()),
            ("b".into(), "ref<LispyPair>".into()),
            ("nil_ref".into(), "ref<LispyPair>".into()),
        ],
        "ref<LispyPair>",
        vec![
            IIRInstr::new("alloc", Some("p1".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("p1".into()),
                    Operand::Int(0),
                    Operand::Var("a".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("p1".into()),
                    Operand::Int(1),
                    Operand::Var("nil_ref".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new("alloc", Some("p2".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("p2".into()),
                    Operand::Int(0),
                    Operand::Var("b".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("p2".into()),
                    Operand::Int(1),
                    Operand::Var("p1".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("p2".into())], "ref<LispyPair>"),
        ],
    );
    let result = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("TwoPairsTest"));
    assert!(result.is_ok(), "two cons sequences should lower ok: {:?}", result.err());
    let class = result.unwrap();
    let method = class.methods.iter().find(|m| m.name == "two_pairs").unwrap();
    // Each cons sequence emits ~12 bytes (iconst_2 + anewarray + dup + iconst_0
    // + aload + aastore + dup + iconst_1 + aload + aastore + astore), so the
    // combined bytecode should be well over 20 bytes.
    assert!(
        method.code_attribute().unwrap().code.len() > 20,
        "two cons sequences should produce > 20 bytes of bytecode"
    );
}

/// Validates that the generated class file bytes parse back correctly
/// (round-trip: lower → serialize → parse).
#[test]
fn heap_alloc_roundtrip_parse() {
    use jvm_class_file::{build_minimal_class_file, parse_class_file, BuildMinimalClassFileParams};

    // First lower to get a JvmClassFile, then re-encode it as bytes and parse.
    let func = IIRFunction::new(
        "cons_test",
        vec![
            ("head".into(), "ref<LispyPair>".into()),
            ("tail".into(), "ref<LispyPair>".into()),
        ],
        "ref<LispyPair>",
        vec![
            IIRInstr::new("alloc", Some("pair".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("pair".into()),
                    Operand::Int(0),
                    Operand::Var("head".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("pair".into()),
                    Operand::Int(1),
                    Operand::Var("tail".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("pair".into())], "ref<LispyPair>"),
        ],
    );
    let class = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("RoundTripClass"))
        .expect("lowering should succeed");

    // Extract the code bytes from the generated method and build a minimal
    // class file around them so we can use the existing parser.
    let method = &class.methods[0];
    let code_attr = method.code_attribute().unwrap();
    let bytes = build_minimal_class_file(BuildMinimalClassFileParams {
        class_name: "RoundTripClass".into(),
        method_name: "cons_test".into(),
        descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;".into(),
        code: code_attr.code.clone(),
        max_stack: code_attr.max_stack,
        max_locals: code_attr.max_locals,
        ..Default::default()
    })
    .expect("build_minimal_class_file should succeed");

    // Parse back — should not produce any error.
    let parsed = parse_class_file(&bytes).expect("parsed class should be valid");
    assert_eq!(parsed.this_class_name, "RoundTripClass");
    assert_eq!(parsed.methods.len(), 1);
    assert!(!parsed.methods[0].code_attribute().unwrap().code.is_empty());
}

/// Validates that `is_null` bytecode has the expected opcode structure.
///
/// The canonical sequence is:
///   aload N    (0x19 N)                   2 bytes
///   ifnull +7  (0xC6 0x00 0x07)           3 bytes
///   iconst_0   (0x03)                     1 byte
///   goto +4    (0xA7 0x00 0x04)           3 bytes
///   iconst_1   (0x04)                     1 byte
///   istore N   (0x36 N)                   2 bytes
///                                total = 12 bytes (for single-byte slot)
#[test]
fn heap_is_null_bytecode_structure() {
    let func = IIRFunction::new(
        "is_nil",
        vec![("x".into(), "ref<LispyPair>".into())],
        "bool",
        vec![
            IIRInstr::new(
                "is_null",
                Some("r".into()),
                vec![Operand::Var("x".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "bool"),
        ],
    );
    let class = lower_iir_to_jvm(&module_with(func), &IIRJvmConfig::new("IsNilTest"))
        .expect("is_null should lower ok");
    let code = &class.methods[0].code_attribute().unwrap().code;

    // The sequence must contain IFNULL (0xC6) and GOTO (0xA7).
    assert!(
        code.contains(&0xC6u8),
        "bytecode should contain IFNULL (0xC6), got: {:?}",
        code
    );
    assert!(
        code.contains(&0xA7u8),
        "bytecode should contain GOTO (0xA7), got: {:?}",
        code
    );
}

/// Verifies that `const ref<LispyPair>` validates successfully (Phase 2).
#[test]
fn heap_nil_const_validates_ok() {
    let func = IIRFunction::new(
        "nil_fn",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "const",
                Some("n".into()),
                vec![Operand::Int(0)],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        errors.is_empty(),
        "const ref<LispyPair> should validate ok, got: {:?}",
        errors
    );
}

/// Verifies that `alloc ref<LispyPair>` validates successfully (Phase 2).
#[test]
fn heap_alloc_validates_ok() {
    let func = IIRFunction::new(
        "alloc_fn",
        vec![],
        "void",
        vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        errors.is_empty(),
        "alloc ref<LispyPair> should validate ok, got: {:?}",
        errors
    );
}

// ===========================================================================
// LANG36 — JVM closure lowering tests
// ===========================================================================
//
// LANG36 promotes the JVM backend from "reject alloc_closure/call_closure
// with ClosureOpcode" (LANG35) to fully lowering closures via a long[]-based
// dispatch-table approach:
//
//   closure layout: long[] { fn_dispatch_idx, cap0_as_long, cap1_as_long, … }
//   dispatch:       __callClosure(long[] closure, long[] args) -> long
//
// The LANG35 rejection tests below are replaced by acceptance tests.

// ---------------------------------------------------------------------------
// Helper: two-function module with alloc_closure + call_closure
//
//   fn __adder(cx: i64, y: i64) -> i64  { r = cx + y; ret r }
//   fn make_and_call(x: i64, y: i64) -> i64 {
//       cl  = alloc_closure("__adder", x) : "closure"
//       res = call_closure(cl, y)         : "any"
//       ret res
//   }
// ---------------------------------------------------------------------------
fn make_closure_module() -> IIRModule {
    let adder = IIRFunction::new(
        "__adder",
        vec![("cx".into(), "i64".into()), ("y".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("cx".into()), Operand::Var("y".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let caller = IIRFunction::new(
        "make_and_call",
        vec![("x".into(), "i64".into()), ("y".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "alloc_closure",
                Some("cl".into()),
                vec![Operand::Str("__adder".into()), Operand::Var("x".into())],
                "closure",
            ),
            IIRInstr::new(
                "call_closure",
                Some("res".into()),
                vec![Operand::Var("cl".into()), Operand::Var("y".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("res".into())], "i64"),
        ],
    );
    IIRModule {
        name: "closure_test".into(),
        functions: vec![adder, caller],
        entry_point: Some("make_and_call".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

// ---------------------------------------------------------------------------
// Validator tests
// ---------------------------------------------------------------------------

/// `alloc_closure` with integer captures is now ACCEPTED by the JVM validator
/// (LANG36: closures are lowered to long[] dispatch tables).
#[test]
fn lang36_alloc_closure_accepted_by_jvm_validator() {
    let func = IIRFunction::new(
        "make_closure",
        vec![("x".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "alloc_closure",
                Some("cl".into()),
                vec![Operand::Str("__lambda_0".into()), Operand::Var("x".into())],
                "closure",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        errors.is_empty(),
        "alloc_closure with i64 capture must be accepted since LANG36; got: {errors:?}"
    );
}

/// `call_closure` with `\"any\"` type hint is now ACCEPTED by the JVM validator.
#[test]
fn lang36_call_closure_accepted_by_jvm_validator() {
    let func = IIRFunction::new(
        "apply_it",
        vec![("h".into(), "i64".into()), ("a".into(), "i64".into())],
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
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        errors.is_empty(),
        "call_closure must be accepted since LANG36; got: {errors:?}"
    );
}

/// `alloc_closure` with `f32`/`f64` captures is still REJECTED with a
/// `ClosureOpcode` error — float captures are deferred to LANG38.
#[test]
fn lang36_float_closure_still_rejected() {
    let func = IIRFunction::new(
        "make_float_closure",
        vec![("x".into(), "f32".into())],
        "i64",
        vec![
            IIRInstr::new(
                "alloc_closure",
                Some("cl".into()),
                vec![Operand::Str("__lambda_0".into()), Operand::Var("x".into())],
                "closure",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(func));
    assert!(
        !errors.is_empty(),
        "alloc_closure with f32 capture must be rejected; got no errors"
    );
    assert!(
        errors.iter().any(|e| e.contains("ClosureOpcode")),
        "error must contain \"ClosureOpcode\"; got: {errors:?}"
    );
    assert!(
        !errors.iter().any(|e| e.contains("UntypedInstruction")),
        "float-capture rejection must NOT say UntypedInstruction; got: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Lowering tests
// ---------------------------------------------------------------------------

/// `alloc_closure` emits `NEWARRAY` (0xBC) in the caller's bytecode.
#[test]
fn lang36_alloc_closure_emits_newarray() {
    let module = make_closure_module();
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureTest"))
        .expect("closure module should lower ok");
    let caller = class
        .methods
        .iter()
        .find(|m| m.name == "make_and_call")
        .expect("make_and_call method should exist");
    let code = &caller.code_attribute().unwrap().code;
    assert!(
        code.contains(&0xBCu8),
        "NEWARRAY (0xBC) must appear in make_and_call bytecode; got: {code:?}"
    );
}

/// `alloc_closure` emits `LASTORE` (0x50) to store captures in the array.
#[test]
fn lang36_alloc_closure_emits_lastore() {
    let module = make_closure_module();
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureTest"))
        .expect("closure module should lower ok");
    let caller = class
        .methods
        .iter()
        .find(|m| m.name == "make_and_call")
        .expect("make_and_call method should exist");
    let code = &caller.code_attribute().unwrap().code;
    assert!(
        code.contains(&0x50u8),
        "LASTORE (0x50) must appear in make_and_call bytecode; got: {code:?}"
    );
}

/// `call_closure` emits `INVOKESTATIC` (0xB8) pointing to `__callClosure`.
#[test]
fn lang36_call_closure_emits_invokestatic_dispatch() {
    let module = make_closure_module();
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureTest"))
        .expect("closure module should lower ok");
    let caller = class
        .methods
        .iter()
        .find(|m| m.name == "make_and_call")
        .expect("make_and_call method should exist");
    let code = &caller.code_attribute().unwrap().code;
    const INVOKESTATIC: u8 = 0xB8;
    assert!(
        code.contains(&INVOKESTATIC),
        "INVOKESTATIC (0xB8) must appear in make_and_call bytecode; got: {code:?}"
    );
}

/// A module with `alloc_closure` gets a synthetic `__callClosure` static method.
#[test]
fn lang36_dispatch_method_generated() {
    let module = make_closure_module();
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureTest"))
        .expect("closure module should lower ok");
    let dispatch = class.methods.iter().find(|m| m.name == "__callClosure");
    assert!(
        dispatch.is_some(),
        "__callClosure synthetic method must be generated; methods: {:?}",
        class.methods.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    // Descriptor must be ([J[J)J  (two long[] args, returns long).
    let desc = &dispatch.unwrap().descriptor;
    assert_eq!(
        desc, "([J[J)J",
        "__callClosure descriptor must be ([J[J)J, got: {desc:?}"
    );
}

/// The `__callClosure` dispatch method contains `LCMP` (0x94).
///
/// `LCMP` compares two longs; it is used to test whether `closure[0]` matches
/// the expected dispatch index for each function branch.
#[test]
fn lang36_dispatch_method_contains_lcmp() {
    let module = make_closure_module();
    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureTest"))
        .expect("closure module should lower ok");
    let dispatch = class
        .methods
        .iter()
        .find(|m| m.name == "__callClosure")
        .expect("__callClosure must exist");
    let code = &dispatch.code_attribute().unwrap().code;
    assert!(
        code.contains(&0x94u8),
        "LCMP (0x94) must appear in __callClosure bytecode; got: {code:?}"
    );
}

// ---------------------------------------------------------------------------
// Real-JVM round-trip (LANG36 equivalent of BEAM test_66)
// ---------------------------------------------------------------------------
//
// Gated by `java_available()`: if no JVM is on the PATH, the test is a no-op.
//
// Strategy:
//   1. Lower the closure module to JvmClassFile.
//   2. Extend the constant pool with System.out and PrintStream.println refs.
//   3. Inject a synthetic `main` method that calls make_and_call(3, 4) and
//      prints the result with System.out.println.
//   4. Serialize to bytes and write ClosureAdder.class to a temp directory.
//   5. Run `java -cp <tmpdir> ClosureAdder` and assert stdout == "7".

/// Returns `true` if `java` is on the PATH and returns exit code 0.
fn java_available() -> bool {
    std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Append a CP entry to a `Vec<Option<JvmConstantPoolEntry>>` and return
/// its 1-based index (0 is always the reserved phantom slot).
fn cp_append(cp: &mut Vec<Option<JvmConstantPoolEntry>>, entry: JvmConstantPoolEntry) -> u16 {
    cp.push(Some(entry));
    (cp.len() - 1) as u16
}

/// Scan a CP vec for a `Methodref` whose resolved class, name, and descriptor
/// all match.  Returns the 1-based index, or 0 if not found.
fn find_methodref_in_cp(
    cp: &[Option<JvmConstantPoolEntry>],
    class_name: &str,
    method_name: &str,
    descriptor: &str,
) -> u16 {
    // Collect UTF-8 → index map once.
    let utf8_idx = |s: &str| -> u16 {
        cp.iter().enumerate().find_map(|(i, e)| {
            if let Some(JvmConstantPoolEntry::Utf8(v)) = e {
                if v == s { Some(i as u16) } else { None }
            } else {
                None
            }
        })
        .unwrap_or(0)
    };
    let class_utf8 = utf8_idx(class_name);
    let class_idx = cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::Class { name_index }) = e {
            if *name_index == class_utf8 { Some(i as u16) } else { None }
        } else {
            None
        }
    })
    .unwrap_or(0);

    let name_u8 = utf8_idx(method_name);
    let desc_u8 = utf8_idx(descriptor);
    let nat_idx = cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::NameAndType { name_index, descriptor_index }) = e {
            if *name_index == name_u8 && *descriptor_index == desc_u8 {
                Some(i as u16)
            } else {
                None
            }
        } else {
            None
        }
    })
    .unwrap_or(0);

    cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::Methodref { class_index, name_and_type_index }) = e {
            if *class_index == class_idx && *name_and_type_index == nat_idx {
                Some(i as u16)
            } else {
                None
            }
        } else {
            None
        }
    })
    .unwrap_or(0)
}

/// End-to-end JVM round-trip: compile a closure-adder module, write the
/// `.class` file, run `java`, and assert the output is `"7"`.
///
/// make_and_call(3, 4) == __adder(3, 4) == 3 + 4 == 7.
#[test]
fn lang36_real_jvm_closure_adder() {
    if !java_available() {
        return; // skip gracefully if java is not on the PATH
    }

    // ── Build and lower the closure module ───────────────────────────────────
    let module = make_closure_module();
    let mut class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("ClosureAdder"))
        .expect("closure module should lower ok");

    // ── Extend the CP with System.out and PrintStream.println ─────────────────
    //
    // The lowering pass does not add these because no user function uses them.
    // We inject them here so the synthetic `main` method can call println.
    {
        let cp = &mut class.constant_pool;

        // java/lang/System → Class
        let sys_utf8  = cp_append(cp, JvmConstantPoolEntry::Utf8("java/lang/System".into()));
        let sys_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: sys_utf8 });

        // out : Ljava/io/PrintStream;  →  Fieldref
        let out_utf8      = cp_append(cp, JvmConstantPoolEntry::Utf8("out".into()));
        let ps_desc_utf8  = cp_append(cp, JvmConstantPoolEntry::Utf8("Ljava/io/PrintStream;".into()));
        let out_nat       = cp_append(cp, JvmConstantPoolEntry::NameAndType {
            name_index: out_utf8,
            descriptor_index: ps_desc_utf8,
        });
        let out_fieldref  = cp_append(cp, JvmConstantPoolEntry::Fieldref {
            class_index: sys_class,
            name_and_type_index: out_nat,
        });

        // java/io/PrintStream  →  Class
        let ps_utf8   = cp_append(cp, JvmConstantPoolEntry::Utf8("java/io/PrintStream".into()));
        let ps_class  = cp_append(cp, JvmConstantPoolEntry::Class { name_index: ps_utf8 });

        // println(J)V  →  Methodref
        let println_utf8      = cp_append(cp, JvmConstantPoolEntry::Utf8("println".into()));
        let println_desc_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("(J)V".into()));
        let println_nat       = cp_append(cp, JvmConstantPoolEntry::NameAndType {
            name_index: println_utf8,
            descriptor_index: println_desc_utf8,
        });
        let println_ref = cp_append(cp, JvmConstantPoolEntry::Methodref {
            class_index: ps_class,
            name_and_type_index: println_nat,
        });

        // UTF-8 entries for the main method's name and descriptor
        // (serialize_method looks these up by string value).
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("main".into()));
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("([Ljava/lang/String;)V".into()));

        // Store the fieldref and methodref indices for the bytecode below.
        // We stash them in local bindings so the borrow on `cp` ends here.
        let _ = cp; // end the mutable borrow — we need immutable later

        // ── Find make_and_call Methodref CP index ─────────────────────────────
        let mac_ref = find_methodref_in_cp(
            &class.constant_pool,
            "ClosureAdder",
            "make_and_call",
            "(JJ)J",
        );
        assert_ne!(mac_ref, 0, "make_and_call Methodref must be in CP");

        // ── Build main bytecode ───────────────────────────────────────────────
        //
        // We push System.out FIRST so it sits below the long result on the
        // operand stack — `invokevirtual println(J)V` expects the receiver
        // (PrintStream) below the long argument.  This avoids any
        // lstore/lload round-trip and sidesteps the 0x3F (lstore_0) vs
        // 0x40 (lstore_1) confusion.
        //
        //   getstatic     (0xB2 hi lo)   push System.out
        //   bipush 3      (0x10 0x03)    push int 3
        //   i2l           (0x85)         widen to long
        //   bipush 4      (0x10 0x04)    push int 4
        //   i2l           (0x85)         widen to long
        //   invokestatic  (0xB8 hi lo)   make_and_call(JJ)J  → stack: [out, 7L]
        //   invokevirtual (0xB6 hi lo)   PrintStream.println(J)V
        //   return        (0xB1)
        let [mac_hi, mac_lo] = mac_ref.to_be_bytes();
        let [out_hi, out_lo] = out_fieldref.to_be_bytes();
        let [pln_hi, pln_lo] = println_ref.to_be_bytes();

        let main_code = vec![
            0xB2, out_hi, out_lo,   // getstatic System.out
            0x10, 0x03,             // bipush 3
            0x85,                   // i2l
            0x10, 0x04,             // bipush 4
            0x85,                   // i2l
            0xB8, mac_hi, mac_lo,   // invokestatic make_and_call
            0xB6, pln_hi, pln_lo,   // invokevirtual println(J)V
            0xB1,                   // return
        ];

        // ── Inject main method into the class ────────────────────────────────
        use jvm_class_file::{ACC_PUBLIC, ACC_STATIC, JvmCodeAttribute, JvmMethodAttribute};
        let main_method = JvmMethodInfo {
            access_flags: ACC_PUBLIC | ACC_STATIC,
            name: "main".into(),
            descriptor: "([Ljava/lang/String;)V".into(),
            attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
                name: "Code".into(),
                max_stack: 5,  // System.out + 2×long (bipush→long takes 2 slots each)
                max_locals: 1, // slot 0 = String[] args only
                code: main_code,
                nested_attributes: vec![],
            })],
        };
        class.methods.push(main_method);
    }

    // ── Serialize to bytes ────────────────────────────────────────────────────
    let bytes = serialize_jvm_class_file(&class);

    // ── Write to temp directory and run ──────────────────────────────────────
    let tmp_dir = std::env::temp_dir().join("lang36_real_jvm_test");
    std::fs::create_dir_all(&tmp_dir).expect("create temp dir for JVM test");
    let class_file = tmp_dir.join("ClosureAdder.class");
    std::fs::write(&class_file, &bytes).expect("write ClosureAdder.class");

    // ── Run `java -Xverify:none -cp <dir> ClosureAdder` ──────────────────────
    //
    // `-Xverify:none` bypasses the bytecode verifier's StackMapTable check.
    // StackMapTable generation requires a full dataflow analysis pass that
    // LANG36 does not yet implement — that is deferred to LANG39.  The flag is
    // deprecated in Java 13+ but still honoured in Java 21; we accept the
    // stderr deprecation warning since our assertion is on stdout only.
    let output = std::process::Command::new("java")
        .arg("-Xverify:none")
        .arg("-cp")
        .arg(&tmp_dir)
        .arg("ClosureAdder")
        .output()
        .expect("java command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "7",
        "expected output \"7\", got {:?}; stderr: {:?}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
}

// ===========================================================================
// G3 — call_builtin "print_i64" → invokestatic env/BasicRuntime.println(J)V
// ===========================================================================
//
// Tests in this section verify the JVM counterpart to iir-to-wasm v0.8.0's
// env.__print_i64 host import.  The wasm backend routes BASIC's PRINT to a
// host function via wasm imports; the JVM backend routes it to a host
// class via invokestatic.  Both let BASIC's PRINT statement reach real
// backend bytecode without the backend itself owning a stdout.
//
// Reference: code/specs/MULTILANG-BACKEND-PLAN.md (item G3).
//
// Convention picked here:
//   * Class:      env/BasicRuntime
//   * Method:     println
//   * Descriptor: (J)V    — takes one long, returns void
//
// Rationale for a dedicated class (vs. reusing env/BFRuntime): BASIC's I/O
// model is line/value oriented, Brainfuck's is byte-stream oriented.  Two
// classes lets a JVM launcher stub or provide either one independently.

/// Build a function that calls `print_i64` once with a local of type i64.
///
/// IIR layout:
///   ```
///   fn print_42() -> void {
///     v = const 42 : i64
///     call_builtin print_i64(v)
///     ret_void
///   }
///   ```
///
/// We pick this minimal shape so the test exercises the validator and the
/// lowerer end-to-end, without any unrelated control flow.
fn print_i64_module() -> IIRModule {
    let f = IIRFunction::new(
        "print_42",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "const",
                Some("v".into()),
                vec![Operand::Int(42)],
                "i64",
            ),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![
                    Operand::Var("print_i64".into()),
                    Operand::Var("v".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    module_with(f)
}

/// Validator must accept `call_builtin "print_i64"` after G3.
///
/// Before G3 the validator's `CALL_BUILTIN_SUPPORTED_NAMES` only contained
/// `["putchar", "getchar"]`; this test would have produced an UnsupportedOp
/// error.  After G3 it must validate cleanly.
#[test]
fn g3_validator_accepts_print_i64() {
    let module = print_i64_module();
    let errors = validate_for_jvm(&module);
    assert!(
        errors.is_empty(),
        "validate should accept call_builtin \"print_i64\" after G3; got: {:?}",
        errors
    );
}

/// Validator still rejects unknown builtin names — defence in depth that the
/// whitelist did not accidentally widen.
#[test]
fn g3_validator_still_rejects_unknown_builtin() {
    let f = IIRFunction::new(
        "boom",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("definitely_not_a_real_builtin".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_jvm(&module_with(f));
    assert!(
        errors.iter().any(|e| e.contains("UnsupportedOp")),
        "unknown call_builtin name should still produce UnsupportedOp; got: {:?}",
        errors
    );
}

/// Lowering a `print_i64` call emits an INVOKESTATIC (0xB8) byte in the
/// generated bytecode.  This is the marker opcode that confirms we routed
/// the builtin through the host-class path rather than (e.g.) silently
/// dropping it.
#[test]
fn g3_lowers_print_i64_to_invokestatic() {
    let module = print_i64_module();
    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "print_42")
        .expect("print_42 method must exist");
    let code = &method.code_attribute().unwrap().code;
    const INVOKESTATIC: u8 = 0xB8;
    assert!(
        code.contains(&INVOKESTATIC),
        "INVOKESTATIC (0xB8) must appear in print_42 bytecode after lowering print_i64; got: {:?}",
        code
    );
}

/// The constant pool of the lowered class must contain a Methodref whose
/// resolved (class, name, descriptor) is exactly the G3 convention:
/// `env/BasicRuntime.println(J)V`.
///
/// We scan via `resolve_methodref` over every CP slot to avoid coupling the
/// test to the exact index ordering — only the *presence* of the right
/// triple matters for round-tripping the class file on a JVM launcher.
#[test]
fn g3_constant_pool_has_basicruntime_println_methodref() {
    let module = print_i64_module();
    let class = lower(&module);

    let mut found = false;
    for (idx, entry) in class.constant_pool.iter().enumerate() {
        if matches!(entry, Some(JvmConstantPoolEntry::Methodref { .. })) {
            if let Ok(mref) = class.resolve_methodref(idx as u16) {
                if mref.class_name == "env/BasicRuntime"
                    && mref.name == "println"
                    && mref.descriptor == "(J)V"
                {
                    found = true;
                    break;
                }
            }
        }
    }
    assert!(
        found,
        "constant pool must contain Methodref env/BasicRuntime.println(J)V; \
         entries: {:?}",
        class
            .constant_pool
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Some(JvmConstantPoolEntry::Methodref { .. }) =>
                    class.resolve_methodref(i as u16).ok(),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
}

/// The lowered class file serializes to bytes that begin with the JVM
/// magic number `0xCAFEBABE`.  This is a smoke-level check that the
/// G3 path produces a structurally valid `.class` blob, not just a
/// happy `JvmClassFile` struct.
#[test]
fn g3_print_i64_class_serializes_with_cafebabe_magic() {
    let module = print_i64_module();
    let class = lower(&module);
    let bytes = serialize_jvm_class_file(&class);
    assert!(bytes.len() >= 4, "class file must have at least 4 bytes");
    assert_eq!(
        &bytes[0..4],
        &[0xCA, 0xFE, 0xBA, 0xBE],
        "serialized class must start with CAFEBABE magic; got: {:02X?}",
        &bytes[0..4]
    );
}

// ===========================================================================
// LANG-FULL E4 — string literal output foothold
// ===========================================================================

fn string_print_module() -> IIRModule {
    let f = IIRFunction::new(
        "print_hello",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    module_with(f)
}

#[test]
fn e4_string_print_lowers_to_ldc_and_printstream_print() {
    let module = string_print_module();
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal print should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "print_hello")
        .expect("print_hello method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.iter().any(|b| *b == 0x12 || *b == 0x13),
        "str_const must load a CONSTANT_String with ldc/ldc_w; got: {:?}",
        code
    );
    assert!(code.contains(&0xB2), "print_str must getstatic System.out; got: {:?}", code);
    assert!(code.contains(&0xB6), "print_str must invokevirtual PrintStream.print; got: {:?}", code);

    let has_hello_string = class.constant_pool.iter().any(|entry| {
        let Some(JvmConstantPoolEntry::String { string_index }) = entry else {
            return false;
        };
        matches!(
            class.constant_pool.get(*string_index as usize),
            Some(Some(JvmConstantPoolEntry::Utf8(s))) if s == "HELLO"
        )
    });
    assert!(has_hello_string, "constant pool must contain CONSTANT_String HELLO");

    let print_ref = find_methodref_in_cp(
        &class.constant_pool,
        "java/io/PrintStream",
        "print",
        "(Ljava/lang/String;)V",
    );
    assert_ne!(print_ref, 0, "constant pool must contain PrintStream.print(String)");
}

#[test]
fn e4_string_len_lowers_to_string_length() {
    let f = IIRFunction::new(
        "len_hello",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal len should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "len_hello")
        .expect("len_hello method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.contains(&0xB6),
        "str_len must invokevirtual java/lang/String.length; got: {:?}",
        code
    );
    assert!(
        code.contains(&0x85),
        "i64 str_len result must widen String.length()I with I2L; got: {:?}",
        code
    );

    let length_ref = find_methodref_in_cp(&class.constant_pool, "java/lang/String", "length", "()I");
    assert_ne!(length_ref, 0, "constant pool must contain java/lang/String.length()I");
}

#[test]
fn e4_string_index_lowers_to_string_char_at() {
    let f = IIRFunction::new(
        "index_abc",
        vec![],
        "i64",
        vec![
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
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal index should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "index_abc")
        .expect("index_abc method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.contains(&0xB6),
        "str_index must invokevirtual java/lang/String.charAt; got: {:?}",
        code
    );
    assert!(
        code.contains(&0x85),
        "i64 str_index result must widen String.charAt(I)C with I2L; got: {:?}",
        code
    );

    let char_at_ref = find_methodref_in_cp(&class.constant_pool, "java/lang/String", "charAt", "(I)C");
    assert_ne!(char_at_ref, 0, "constant pool must contain java/lang/String.charAt(I)C");
}

#[test]
fn e4_string_concat_len_lowers_to_string_concat_and_length() {
    let f = IIRFunction::new(
        "concat_len",
        vec![],
        "i64",
        vec![
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
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal concat len should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "concat_len")
        .expect("concat_len method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.iter().filter(|&&b| b == 0xB6).count() >= 2,
        "str_concat + str_len should use invokevirtual for concat and length; got: {:?}",
        code
    );

    let concat_ref = find_methodref_in_cp(
        &class.constant_pool,
        "java/lang/String",
        "concat",
        "(Ljava/lang/String;)Ljava/lang/String;",
    );
    assert_ne!(concat_ref, 0, "constant pool must contain java/lang/String.concat(String)");
    let length_ref = find_methodref_in_cp(&class.constant_pool, "java/lang/String", "length", "()I");
    assert_ne!(length_ref, 0, "constant pool must contain java/lang/String.length()I");
}

#[test]
fn e4_string_slice_index_lowers_to_string_substring_and_char_at() {
    let f = IIRFunction::new(
        "slice_index",
        vec![],
        "i64",
        vec![
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
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(
        errors.is_empty(),
        "string literal slice/index should validate: {:?}",
        errors
    );

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "slice_index")
        .expect("slice_index method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.iter().filter(|&&b| b == 0xB6).count() >= 2,
        "str_slice + str_index should use invokevirtual for substring and charAt; got: {:?}",
        code
    );
    assert!(
        code.iter().filter(|&&b| b == 0x88).count() >= 3,
        "i64 start/end/index operands should narrow with L2I; got: {:?}",
        code
    );

    let substring_ref = find_methodref_in_cp(
        &class.constant_pool,
        "java/lang/String",
        "substring",
        "(II)Ljava/lang/String;",
    );
    assert_ne!(
        substring_ref, 0,
        "constant pool must contain java/lang/String.substring(II)"
    );
    let char_at_ref =
        find_methodref_in_cp(&class.constant_pool, "java/lang/String", "charAt", "(I)C");
    assert_ne!(
        char_at_ref, 0,
        "constant pool must contain java/lang/String.charAt(I)C"
    );
}

#[test]
fn e4_string_eq_lowers_to_string_equals() {
    let f = IIRFunction::new(
        "eq_hello",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "str_const",
                Some("a".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("b".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("str_eq", Some("ok".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "i64"),
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal eq should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "eq_hello")
        .expect("eq_hello method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.contains(&0xB6),
        "str_eq must invokevirtual java/lang/String.equals; got: {:?}",
        code
    );
    assert!(
        code.contains(&0x85),
        "i64 str_eq result must widen boolean int with I2L; got: {:?}",
        code
    );

    let equals_ref = find_methodref_in_cp(
        &class.constant_pool,
        "java/lang/String",
        "equals",
        "(Ljava/lang/Object;)Z",
    );
    assert_ne!(equals_ref, 0, "constant pool must contain java/lang/String.equals(Object)");
}

#[test]
fn e4_string_cmp_lowers_to_compare_to_and_signum() {
    let f = IIRFunction::new(
        "cmp_hello",
        vec![],
        "i64",
        vec![
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
        ],
    );
    let module = module_with(f);
    let errors = validate_for_jvm(&module);
    assert!(errors.is_empty(), "string literal cmp should validate: {:?}", errors);

    let class = lower(&module);
    let method = class
        .methods
        .iter()
        .find(|m| m.name == "cmp_hello")
        .expect("cmp_hello method must exist");
    let code = &method.code_attribute().unwrap().code;

    assert!(
        code.contains(&0xB6),
        "str_cmp must invokevirtual java/lang/String.compareTo; got: {:?}",
        code
    );
    assert!(
        code.contains(&0xB8),
        "str_cmp must invokestatic java/lang/Integer.signum; got: {:?}",
        code
    );
    assert!(
        code.contains(&0x85),
        "i64 str_cmp result must widen signum int with I2L; got: {:?}",
        code
    );

    let compare_ref = find_methodref_in_cp(
        &class.constant_pool,
        "java/lang/String",
        "compareTo",
        "(Ljava/lang/String;)I",
    );
    assert_ne!(compare_ref, 0, "constant pool must contain java/lang/String.compareTo(String)");
    let signum_ref =
        find_methodref_in_cp(&class.constant_pool, "java/lang/Integer", "signum", "(I)I");
    assert_ne!(
        signum_ref, 0,
        "constant pool must contain java/lang/Integer.signum(int)"
    );
}

/// McCarthy W3b: `box` lowers to `Integer.valueOf(I)` (invokestatic 0xB8) and
/// `unbox` to `checkcast` (0xC0) + `Integer.intValue()` (invokevirtual 0xB6).
#[test]
fn mccarthy_w3b_box_unbox_lower_to_integer() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("box", Some("b".into()), vec![Operand::Var("a".into())], "ref<any>"),
            IIRInstr::new("unbox", Some("c".into()), vec![Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
        ],
    );
    let class = lower(&module_with(f));
    let code = &class
        .methods
        .iter()
        .find(|m| m.name == "main")
        .unwrap()
        .code_attribute()
        .unwrap()
        .code;
    assert!(code.contains(&0xB8u8), "box → invokestatic Integer.valueOf");
    assert!(code.contains(&0xC0u8), "unbox → checkcast Integer");
    assert!(code.contains(&0xB6u8), "unbox → invokevirtual intValue");
}

/// McCarthy W4: the lisp predicates lower to JVM bytecode — `pair?` →
/// `instanceof` (0xC1), `not` → `ixor` (0x82), `equal?` → `checkcast` (0xC0) +
/// `invokevirtual intValue` (0xB6) + `if_icmpne` (0xA0, via emit_int_compare).
#[test]
fn mccarthy_w4_predicates_lower() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new("box", Some("ab".into()), vec![Operand::Var("a".into())], "ref<any>"),
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![Operand::Var("pair?".into()), Operand::Var("ab".into())],
                "bool",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("n".into()),
                vec![Operand::Var("not".into()), Operand::Var("p".into())],
                "bool",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("e".into()),
                vec![Operand::Var("equal?".into()), Operand::Var("ab".into()), Operand::Var("ab".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("e".into())], "i32"),
        ],
    );
    let class = lower(&module_with(f));
    let code = &class
        .methods
        .iter()
        .find(|m| m.name == "main")
        .unwrap()
        .code_attribute()
        .unwrap()
        .code;
    assert!(code.contains(&0xC1u8), "pair? → instanceof");
    assert!(code.contains(&0x82u8), "not → ixor");
    assert!(code.contains(&0xA0u8), "equal? → if_icmpne (compare-to-bool)");
}

/// McCarthy W5a: a large `int` const (beyond ±32767 — e.g. an interned symbol id
/// ≥ 2²⁹) lowers via `ldc`/`ldc_w` + a `CONSTANT_Integer` pool entry, NOT the old
/// invalid `ldc 0` placeholder (which crashed real JVMs at `constantTag`).
#[test]
fn mccarthy_w5a_large_int_const_uses_constant_pool() {
    let big = 536_870_912i64; // 2^29 — a symbol id; needs ldc.
    let f = IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(big)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let class = lower(&module_with(f));
    // A CONSTANT_Integer(2^29) entry must exist in the pool.
    assert!(
        class.constant_pool.iter().any(|e| matches!(
            e,
            Some(jvm_class_file::JvmConstantPoolEntry::Integer(v)) if *v == big as i32
        )),
        "large const must add a CONSTANT_Integer pool entry"
    );
    let code = &class.methods.iter().find(|m| m.name == "main").unwrap()
        .code_attribute().unwrap().code;
    // ldc (0x12) or ldc_w (0x13) — and never a bare `ldc 0` (index 0 reserved).
    let ldc_pos = code.iter().position(|&b| b == 0x12 || b == 0x13).expect("ldc/ldc_w emitted");
    assert_ne!(code[ldc_pos + 1], 0, "ldc index must not be the reserved slot 0");
}

// ---------------------------------------------------------------------------
// Byte-tape ops + i64 conditions (LANG-MATRIX LM-J Brainfuck)
// ---------------------------------------------------------------------------
//
// `lower_brainfuck_for_aot` rewrites Brainfuck's tape into `alloc_bytes` /
// `load_byte` / `store_byte` over a static `env/BFRuntime.__tape : [B`. These
// tests cover that lowering + the i64↔i32 conversions for the (optionally
// widened) value model. Opcode bytes: GETSTATIC=0xB2, BALOAD=0x33,
// BASTORE=0x54, IAND=0x7E, L2I=0x88, I2L=0x85, LCMP=0x94.

fn code_bytes(class: &JvmClassFile) -> Vec<u8> {
    class.methods.iter()
        .find(|m| m.name == "main")
        .and_then(|m| m.code_attribute())
        .map(|c| c.code.clone())
        .expect("main method code")
}

/// `alloc_bytes` emits no bytecode (the JVM tape is a pre-allocated static
/// field), and i32 `load_byte`/`store_byte` lower to `getstatic __tape` +
/// `baload`/`bastore` (the unsigned-byte tape access), masking the loaded byte.
#[test]
fn byte_tape_ops_i32_lower_to_static_tape_access() {
    let f = IIRFunction::new("main", vec![], "i32", vec![
        IIRInstr::new("const", Some("size".into()), vec![Operand::Int(30_000)], "i32"),
        IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i32"),
        IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i32"),
        IIRInstr::new("const", Some("val".into()), vec![Operand::Int(65)], "i32"),
        IIRInstr::new("store_byte", None, vec![
            Operand::Var("tape".into()), Operand::Var("idx".into()), Operand::Var("val".into()),
        ], "i32"),
        IIRInstr::new("load_byte", Some("got".into()), vec![
            Operand::Var("tape".into()), Operand::Var("idx".into()),
        ], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i32"),
    ]);
    let class = lower(&module_with(f));
    let code = code_bytes(&class);
    assert!(code.contains(&0xB2), "expected GETSTATIC (0xB2) for the static __tape field");
    assert!(code.contains(&0x33), "expected BALOAD (0x33) for load_byte");
    assert!(code.contains(&0x54), "expected BASTORE (0x54) for store_byte");
    assert!(code.contains(&0x7E), "expected IAND (0x7E) masking the loaded byte to u8");
    // alloc_bytes itself adds no opcodes — no allocation primitive needed.
    assert!(!code.contains(&0xBC), "alloc_bytes must not emit a `newarray` (0xBC)");
}

/// i64 `load_byte`/`store_byte` (the widened BF value model) narrow the index /
/// value with `l2i` for the array op and widen the loaded byte with `i2l`.
#[test]
fn byte_tape_ops_i64_convert_at_the_boundary() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("size".into()), vec![Operand::Int(8)], "i64"),
        IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("size".into())], "i64"),
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
    let class = lower(&module_with(f));
    let code = code_bytes(&class);
    assert!(code.contains(&0x88), "expected L2I (0x88) narrowing an i64 index/value for the array op");
    assert!(code.contains(&0x85), "expected I2L (0x85) widening the loaded byte to the i64 dest");
}

/// `store_byte` with a dest is rejected — it produces no value.
#[test]
fn store_byte_with_dest_is_rejected() {
    let f = IIRFunction::new("main", vec![], "i32", vec![
        IIRInstr::new("const", Some("t".into()), vec![Operand::Int(8)], "i32"),
        IIRInstr::new("alloc_bytes", Some("tape".into()), vec![Operand::Var("t".into())], "i32"),
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i32"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("store_byte", Some("oops".into()), vec![
            Operand::Var("tape".into()), Operand::Var("i".into()), Operand::Var("v".into()),
        ], "i32"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    assert!(
        lower_iir_to_jvm(&module_with(f), &IIRJvmConfig::new("TestClass")).is_err(),
        "store_byte must not carry a dest"
    );
}

/// An i64 loop guard branches via `lcmp` (compare-to-0 → int) before `ifeq`,
/// not a bare `iload` (which would read only one slot of the two-slot long).
#[test]
fn i64_loop_guard_branches_via_lcmp() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
        IIRInstr::new("const", Some("c".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("jmp_if_false", None, vec![
            Operand::Var("c".into()), Operand::Var("End".into()),
        ], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("L".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("End".into())], "void"),
        IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    let class = lower(&module_with(f));
    let code = code_bytes(&class);
    assert!(code.contains(&0x94), "expected LCMP (0x94) reducing the i64 guard to an int before ifeq");
}

// ---------------------------------------------------------------------------
// E2 (LANG-FULL): narrow-width arithmetic emits a width mask
//
// JVM `int` arithmetic (iadd/imul/…) wraps mod-2³², so u32/i32 are already
// correct.  The smaller widths (u4/u8/u16) get an explicit
// `iconst/sipush/ldc <mask>; iand` after the op.  These tests assert the
// lowering injects that mask (the executed cross-backend proof lands in the
// integration PR via lang-aot's real-`java` run_jvm).
// ---------------------------------------------------------------------------

/// `main` = `const 200; const 100; <op> [ty]; ret [ty]`.
fn e2_binop_fn(op: &str, ty: &str) -> IIRFunction {
    IIRFunction::new(
        "main",
        vec![],
        ty,
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], ty),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], ty),
            IIRInstr::new(op, Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], ty),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], ty),
        ],
    )
}

fn has_seq(code: &[u8], seq: &[u8]) -> bool {
    code.windows(seq.len()).any(|w| w == seq)
}

// LANG-FULL E2: narrow unsigned types use the JVM `int` model, so the width mask
// is `sipush 0x00FF; iand` → bytes [0x11, 0x00, 0xFF, 0x7E]. (`SIPUSH` 0x11,
// then the 2-byte short, then `IAND` 0x7E.) `LADD`/`LRETURN` (the long opcodes)
// must NOT appear for a narrow op.
const U8_MASK_SEQ: [u8; 4] = [0x11, 0x00, 0xFF, 0x7E];
const LADD: u8 = 0x61;
const LRETURN: u8 = 0xAD;

#[test]
fn e2_u8_add_emits_iand_width_mask() {
    let class = lower(&module_with(e2_binop_fn("add", "u8")));
    let code = code_bytes(&class);
    assert!(has_seq(&code, &U8_MASK_SEQ),
        "u8 `add` must emit `sipush 255; iand` (int model) to wrap mod-256");
    assert!(!code.contains(&LADD), "u8 `add` is an int op, not a long `ladd`");
}

#[test]
fn e2_u8_not_and_shl_emit_width_mask() {
    // `not` (synthesised as int XOR -1) and a left shift both need the byte mask.
    let not_fn = IIRFunction::new("main", vec![], "u8", vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0)], "u8"),
        IIRInstr::new("not", Some("c".into()), vec![Operand::Var("a".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "u8"),
    ]);
    let class = lower(&module_with(not_fn));
    assert!(has_seq(&code_bytes(&class), &U8_MASK_SEQ), "u8 `not` must mask to a byte");

    let shl_class = lower(&module_with(e2_binop_fn("shl", "u8")));
    assert!(has_seq(&code_bytes(&shl_class), &U8_MASK_SEQ), "u8 `shl` must mask to a byte");
}

#[test]
fn e2_i64_and_u32_add_have_no_byte_mask() {
    // i64 uses the long opcodes; u32 wraps natively via the 32-bit int op — so
    // neither emits the `sipush 255; iand` byte mask.
    for ty in ["i64", "u32"] {
        let class = lower(&module_with(e2_binop_fn("add", ty)));
        assert!(!has_seq(&code_bytes(&class), &U8_MASK_SEQ),
            "{ty} `add` must not emit a byte-width mask");
    }
}

/// LANG-FULL E2 regression: the shape a real frontend emits AFTER
/// `lang_aot::concretize_scalar_any_for_jvm` runs — it narrows a scalar
/// module's `i64`→`i32` (the jvm-simulator is 32-bit and the entry must
/// `ireturn`), leaving the narrow-unsigned op alone. So the JVM backend sees
/// `const i32; const i32; add u8; ret i32`. The narrow op must use the **int**
/// model (`iadd` + `iand`, NOT `ladd`/`land`) so it is operand-consistent with
/// the concretized i32 consts, and the method must `ireturn` (not `lreturn`).
/// A v0.13.0 attempt to type the narrow op `long` produced unverifiable
/// bytecode here (`istore` consts feeding an `lmul`, `lreturn` from an `int`
/// method) — this test guards against that regression. (`200u8+100u8` → `44`.)
#[test]
fn e2_concretized_u8_shape_is_all_int() {
    let f = IIRFunction::new("main", vec![], "i32", vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "i32"),
        IIRInstr::new("add", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    assert!(has_seq(&code, &U8_MASK_SEQ), "u8 add over i32 operands masks with `sipush 255; iand`");
    assert!(!code.contains(&LADD), "must be an int `iadd`, not a long `ladd`");
    assert!(!code.contains(&LRETURN), "must `ireturn` (int method), not `lreturn`");
}

/// BA-JVM-1 regression: a comparison over **`i64`** operands (BASIC keeps the
/// long value model — it prints, so it skips the scalar `concretize`-to-i32
/// pass) feeding a `jmp_if_false`. The comparison result is a 0/1 bool stored
/// with `istore`, so its slot must be `int`; a later `jmp_if_false` must read it
/// with `iload; ifeq`, NOT the long guard `lload; lconst_0; lcmp; ifeq`. Before
/// the fix, `build_type_map` typed the cmp dest `Long` (from its `i64`
/// *operand*-width hint), so it was `istore`d as int but `lload`ed as long → the
/// JVM verifier rejected "Accessing value from uninitialized register pair".
#[test]
fn ba_jvm_1_i64_cmp_into_jmp_if_uses_int_guard() {
    const LCONST_0: u8 = 0x09;
    const LCMP: u8 = 0x94;
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i64"),
        // i64-operand comparison → 0/1 bool result
        IIRInstr::new("cmp_lt", Some("cond".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("done".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
        IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    // The buggy guard loads the bool as a long and compares to 0L
    // (`lconst_0; lcmp`). The fixed guard is `iload; ifeq` (no lconst_0/lcmp on
    // the cond). The cmp_lt over i64 operands DOES use `lcmp` to compare a and b,
    // but never preceded by `lconst_0` — that pairing is unique to the bad guard.
    assert!(!has_seq(&code, &[LCONST_0, LCMP]),
        "the bool cond must be read with the int guard (iload; ifeq), not `lload; lconst_0; lcmp`");
}

/// Oct `&&`/`||` on the JVM (BA-JVM-1 follow-through): a `mov` from an `int`
/// (bool) comparison result into a `long`-typed accumulator must widen with
/// `i2l` before `lstore`, else the long slot's second half is left
/// uninitialized and a later `lload` trips the verifier ("uninitialized register
/// pair"). Oct's short-circuit keeps the i64 value model (it `out`-prints, so it
/// skips the scalar concretize-to-i32 pass), so its accumulator is `long` while
/// the comparison results are `int`.
#[test]
fn mov_int_bool_into_long_accumulator_widens_with_i2l() {
    const I2L: u8 = 0x85;
    const ISTORE: u8 = 0x36;
    // `acc` is read as a long (jmp_if_false guard over an i64-typed var), so its
    // slot is Long; it is assigned the int bool result of a comparison.
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i64"),
        IIRInstr::new("cmp_eq", Some("cond".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        // accumulator typed i64 (Long slot), assigned the int bool result
        IIRInstr::new("mov", Some("acc".into()), vec![Operand::Var("cond".into())], "i64"),
        // read `acc` as a long guard — forces its slot to Long
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("acc".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    // The mov must widen the int bool to long (`i2l`) — not `istore` it into the
    // long `acc` slot (which would leave slot+1 uninitialized).
    assert!(code.contains(&I2L), "mov of int bool into a long accumulator must `i2l`-widen");
    // And the cond (int) is `istore`d while acc (long) is `lstore`d — confirm the
    // int store of the comparison result is still present (it is the i2l source).
    assert!(code.contains(&ISTORE), "the int comparison result is istore'd before the widening mov");
}

// ===========================================================================
// f64 (double) support — LANG-FULL enabler E3
// ===========================================================================

/// A non-0/1 `f64` constant must be loaded with `ldc2_w` pointing at a real
/// `CONSTANT_Double` pool entry — not the old placeholder index `#0` (the
/// unused phantom slot), which made the class fail JVM verification.
#[test]
fn f64_constant_uses_real_double_pool_entry() {
    const LDC2_W: u8 = 0x14;
    let f = IIRFunction::new("main", vec![], "f64", vec![
        IIRInstr::new("const", Some("r".into()), vec![Operand::Float(2.5)], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
    ]);
    let class = lower(&module_with(f));
    // A CONSTANT_Double(2.5) entry exists in the pool.
    assert!(class.constant_pool.iter().any(|e|
        matches!(e, Some(JvmConstantPoolEntry::Double(v)) if (*v - 2.5).abs() < 1e-12)),
        "expected a CONSTANT_Double(2.5) pool entry");
    let code = code_bytes(&class);
    // ldc2_w is emitted, and its 2-byte operand is NOT 0x0000 (the phantom slot).
    let pos = code.iter().position(|&b| b == LDC2_W).expect("ldc2_w for the f64 const");
    let idx = u16::from_be_bytes([code[pos + 1], code[pos + 2]]);
    assert_ne!(idx, 0, "ldc2_w must reference a real CP index, not the phantom #0");
}

/// `0.0d` and `1.0d` keep their 1-byte short forms (`dconst_0`/`dconst_1`) —
/// no pool entry needed.
#[test]
fn f64_zero_and_one_use_short_forms() {
    const DCONST_0: u8 = 0x0E;
    const DCONST_1: u8 = 0x0F;
    let f = IIRFunction::new("main", vec![], "f64", vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Float(0.0)], "f64"),
        IIRInstr::new("const", Some("o".into()), vec![Operand::Float(1.0)], "f64"),
        IIRInstr::new("add", Some("s".into()),
            vec![Operand::Var("z".into()), Operand::Var("o".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "f64"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    assert!(code.contains(&DCONST_0), "0.0d should use dconst_0");
    assert!(code.contains(&DCONST_1), "1.0d should use dconst_1");
}

/// An `f64` comparison lowers to `dcmpl`/`dcmpg` + a unary branch — NOT the
/// integer `if_icmp*` path, which would `iload` a two-slot double as a single
/// int and fail verification.
#[test]
fn f64_comparison_uses_dcmp_not_if_icmp() {
    const DCMPL: u8 = 0x97;
    const DCMPG: u8 = 0x98;
    const IF_ICMPNE: u8 = 0xA0;
    // r := 7.0; if r < 4.0 ... — a real ordered comparison.
    let lt = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("r".into()), vec![Operand::Float(7.0)], "f64"),
        IIRInstr::new("const", Some("four".into()), vec![Operand::Float(4.0)], "f64"),
        IIRInstr::new("cmp_lt", Some("c".into()),
            vec![Operand::Var("r".into()), Operand::Var("four".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
    ]);
    let code = code_bytes(&lower(&module_with(lt)));
    assert!(code.contains(&DCMPL) || code.contains(&DCMPG),
        "f64 comparison must use dcmpl/dcmpg");
    assert!(!code.contains(&IF_ICMPNE),
        "f64 comparison must NOT fall back to the integer if_icmp path");
}

/// `>` / `>=` over reals use `dcmpg` (so a NaN operand makes them false),
/// matching javac's convention.
#[test]
fn f64_greater_uses_dcmpg() {
    const DCMPG: u8 = 0x98;
    let gt = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Float(7.0)], "f64"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Float(4.0)], "f64"),
        IIRInstr::new("cmp_gt", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
    ]);
    let code = code_bytes(&lower(&module_with(gt)));
    assert!(code.contains(&DCMPG), "cmp_gt over f64 should use dcmpg");
}

// ===========================================================================
// LANG-FULL E6 (layer 1) — typed module globals (static fields)
// ===========================================================================

/// Build the E6 proof module: `compute()` seeds a global, a *separate* `bump()`
/// reads/increments/writes it, and `compute` returns it ⇒ 42. Entry is named
/// `compute` so the test launcher's `main(String[])` doesn't collide with it.
fn e6_globals_module() -> IIRModule {
    let mut m = IIRModule::new("Main", "Main");
    m.add_or_replace(IIRFunction::new(
        "compute",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("seed".into()), vec![Operand::Int(41)], "i64"),
            IIRInstr::new("global_store", None, vec![Operand::Str("g".into()), Operand::Var("seed".into())], "void"),
            IIRInstr::new("call", Some("res".into()), vec![Operand::Var("bump".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("res".into())], "i64"),
        ],
    ));
    m.add_or_replace(IIRFunction::new(
        "bump",
        vec![],
        "i64",
        vec![
            IIRInstr::new("global_load", Some("cur".into()), vec![Operand::Str("g".into())], "i64"),
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("add", Some("nxt".into()), vec![Operand::Var("cur".into()), Operand::Var("one".into())], "i64"),
            IIRInstr::new("global_store", None, vec![Operand::Str("g".into()), Operand::Var("nxt".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("nxt".into())], "i64"),
        ],
    ));
    m
}

/// The lowered class declares a `public static long G_0` field and `bump`
/// reads/writes it via `getstatic`/`putstatic`.
#[test]
fn e6_global_lowers_to_static_field_and_getstatic_putstatic() {
    let class = lower_iir_to_jvm(&e6_globals_module(), &IIRJvmConfig { class_name: "Main".into() })
        .expect("E6 globals should lower");
    // A single static long field G_0.
    assert_eq!(class.fields.len(), 1, "one global ⇒ one static field");
    assert_eq!(class.fields[0].name, "G_0");
    assert_eq!(class.fields[0].descriptor, "J");
    // bump's bytecode contains getstatic (0xB2) and putstatic (0xB3).
    let bump = class.methods.iter().find(|m| m.name == "bump").expect("bump method");
    let code = &bump.code_attribute().expect("bump has Code").code;
    assert!(code.contains(&0xB2), "bump should getstatic the global");
    assert!(code.contains(&0xB3), "bump should putstatic the global");
}

/// Regression: a `global_load` into an **i32** dest must narrow the 64-bit field
/// with `l2i` (0xB2 getstatic J pushes a long; `istore` of a long is a verifier
/// type error). An `integer` ALGOL program concretised to i32 hit exactly this —
/// the matrix proof caught it; this guards it without `-Xverify:none`.
#[test]
fn e6_global_load_into_i32_dest_narrows_with_l2i() {
    let mut m = IIRModule::new("Main", "Main");
    m.add_or_replace(IIRFunction::new(
        "f",
        vec![],
        "i32",
        vec![
            // store an i32, then load it back into an i32 dest.
            IIRInstr::new("const", Some("seed".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("global_store", None, vec![Operand::Str("g".into()), Operand::Var("seed".into())], "void"),
            IIRInstr::new("global_load", Some("v".into()), vec![Operand::Str("g".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    ));
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig { class_name: "Main".into() })
        .expect("lowers");
    let f = class.methods.iter().find(|m| m.name == "f").expect("f");
    let code = &f.code_attribute().expect("Code").code;
    assert!(code.contains(&0x88), "global_load into an i32 dest must emit l2i (0x88)");
    // and the store side widens an i32 value with i2l (0x85).
    assert!(code.contains(&0x85), "global_store of an i32 value must emit i2l (0x85)");
}

/// End-to-end on real `java`: the cross-function global program prints 42.
/// Skipped if `java` is unavailable.
#[test]
fn e6_global_runs_on_real_java() {
    if !java_available() {
        eprintln!("java not available — skipping e6_global_runs_on_real_java");
        return;
    }
    let mut class = lower_iir_to_jvm(&e6_globals_module(), &IIRJvmConfig { class_name: "Main".into() })
        .expect("lower");

    // Append the CP entries the launcher needs (System.out + println(J)V), then
    // inject a `main(String[])` that calls Main.compute()J and prints it.
    {
        use jvm_class_file::JvmConstantPoolEntry as E;
        let cp = &mut class.constant_pool;
        let sys_utf8 = cp_append(cp, E::Utf8("java/lang/System".into()));
        let sys_class = cp_append(cp, E::Class { name_index: sys_utf8 });
        let out_utf8 = cp_append(cp, E::Utf8("out".into()));
        let ps_desc = cp_append(cp, E::Utf8("Ljava/io/PrintStream;".into()));
        let out_nat = cp_append(cp, E::NameAndType { name_index: out_utf8, descriptor_index: ps_desc });
        let out_fieldref = cp_append(cp, E::Fieldref { class_index: sys_class, name_and_type_index: out_nat });
        let ps_utf8 = cp_append(cp, E::Utf8("java/io/PrintStream".into()));
        let ps_class = cp_append(cp, E::Class { name_index: ps_utf8 });
        let pln_utf8 = cp_append(cp, E::Utf8("println".into()));
        let pln_desc = cp_append(cp, E::Utf8("(J)V".into()));
        let pln_nat = cp_append(cp, E::NameAndType { name_index: pln_utf8, descriptor_index: pln_desc });
        let println_ref = cp_append(cp, E::Methodref { class_index: ps_class, name_and_type_index: pln_nat });
        let _ = cp_append(cp, E::Utf8("main".into()));
        let _ = cp_append(cp, E::Utf8("([Ljava/lang/String;)V".into()));

        let compute_ref = find_methodref_in_cp(&class.constant_pool, "Main", "compute", "()J");
        assert_ne!(compute_ref, 0, "Main.compute Methodref must be in CP");

        let [out_hi, out_lo] = out_fieldref.to_be_bytes();
        let [cmp_hi, cmp_lo] = compute_ref.to_be_bytes();
        let [pln_hi, pln_lo] = println_ref.to_be_bytes();
        // getstatic System.out; invokestatic Main.compute()J; invokevirtual println(J)V; return
        let main_code = vec![
            0xB2, out_hi, out_lo,
            0xB8, cmp_hi, cmp_lo,
            0xB6, pln_hi, pln_lo,
            0xB1,
        ];
        use jvm_class_file::{ACC_PUBLIC, ACC_STATIC, JvmCodeAttribute, JvmMethodAttribute};
        class.methods.push(JvmMethodInfo {
            access_flags: ACC_PUBLIC | ACC_STATIC,
            name: "main".into(),
            descriptor: "([Ljava/lang/String;)V".into(),
            attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
                name: "Code".into(),
                max_stack: 4, // System.out (1) + long result (2)
                max_locals: 1,
                code: main_code,
                nested_attributes: vec![],
            })],
        });
    }

    let bytes = serialize_jvm_class_file(&class);
    let tmp = std::env::temp_dir().join(format!("e6_jvm_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(stdout.trim(), "42",
        "expected 42, got {stdout:?}; stderr: {:?}", String::from_utf8_lossy(&out.stderr));
}

// ===========================================================================
// LANG-FULL E8 — numeric conversions (int ⇄ real)
//
// Three IIR ops lower to JVM primitive-conversion opcodes:
//   int_to_real      → i2d (0x87) / l2d (0x8A)   [widen, exact]
//   real_to_int_trunc → d2i (0x8E) / d2l (0x8F)  [truncate toward zero]
//   real_to_int_floor → invokestatic Math.floor(D)D ; d2i/d2l  [round to −∞]
// The width (int vs long form) follows the operand's value model, not the
// type_hint — see the dual-value-model note in lower.rs.
// ===========================================================================

const I2D: u8 = 0x87;
const L2D: u8 = 0x8A;
const D2I: u8 = 0x8E;
const D2L: u8 = 0x8F;

/// `int_to_real` over an `i64` (Long) source widens with `l2d`; the round-trip
/// `real_to_int_trunc` back to `i64` narrows with `d2l` (truncate toward zero).
#[test]
fn e8_int_to_real_and_trunc_use_l2d_and_d2l() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(45)], "i64"),
        IIRInstr::new("int_to_real", Some("r".into()), vec![Operand::Var("i".into())], "f64"),
        IIRInstr::new("real_to_int_trunc", Some("o".into()), vec![Operand::Var("r".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("o".into())], "i64"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    assert!(code.contains(&L2D), "int_to_real over an i64 source must widen with l2d");
    assert!(code.contains(&D2L), "real_to_int_trunc into an i64 dest must narrow with d2l");
    // It must NOT route through the float opcodes for an integer source/dest.
    assert!(!code.contains(&I2D), "an i64 source uses l2d, not i2d");
    assert!(!code.contains(&D2I), "an i64 dest uses d2l, not d2i");
}

/// `int_to_real` over an `i32` (Int) source widens with `i2d`, and
/// `real_to_int_trunc` into an `i32` dest narrows with `d2i`.
#[test]
fn e8_int_to_real_and_trunc_over_i32_use_i2d_and_d2i() {
    let f = IIRFunction::new("main", vec![], "i32", vec![
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(45)], "i32"),
        IIRInstr::new("int_to_real", Some("r".into()), vec![Operand::Var("i".into())], "f64"),
        IIRInstr::new("real_to_int_trunc", Some("o".into()), vec![Operand::Var("r".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("o".into())], "i32"),
    ]);
    let code = code_bytes(&lower(&module_with(f)));
    assert!(code.contains(&I2D), "int_to_real over an i32 source must widen with i2d");
    assert!(code.contains(&D2I), "real_to_int_trunc into an i32 dest must narrow with d2i");
}

/// `real_to_int_floor` has no single opcode: it calls `java/lang/Math.floor(D)D`
/// (round toward −∞, still a double) and then `d2l`/`d2i` to land in the integer
/// model. Assert both the Math.floor methodref and the narrowing opcode appear.
#[test]
fn e8_real_to_int_floor_calls_math_floor_then_narrows() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("const", Some("x".into()), vec![Operand::Float(-2.7)], "f64"),
        IIRInstr::new("real_to_int_floor", Some("o".into()), vec![Operand::Var("x".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("o".into())], "i64"),
    ]);
    let class = lower(&module_with(f));
    assert_ne!(
        find_methodref_in_cp(&class.constant_pool, "java/lang/Math", "floor", "(D)D"), 0,
        "real_to_int_floor must add a Methodref to java/lang/Math.floor(D)D");
    assert!(code_bytes(&class).contains(&D2L),
        "after Math.floor the floored double is narrowed to i64 with d2l");
}

/// End-to-end on real `java`: `floor(int_to_real(45) − 2.7)` ⇒ `floor(42.3)` ⇒ 42.
/// This exercises all three conversion ops in one program — int_to_real (l2d),
/// an f64 subtraction (dsub), and real_to_int_floor (Math.floor + d2l) — and
/// matches the LLVM/WASM/VM matrix cell value of 42. Skipped if `java` is absent.
#[test]
fn e8_conversions_round_trip_runs_on_real_java() {
    if !java_available() {
        eprintln!("java not available — skipping e8_conversions_round_trip_runs_on_real_java");
        return;
    }
    // compute()J: floor(int_to_real(45) − 2.7) = 42.
    let mut m = IIRModule::new("Main", "Main");
    m.add_or_replace(IIRFunction::new("compute", vec![], "i64", vec![
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(45)], "i64"),
        IIRInstr::new("int_to_real", Some("r".into()), vec![Operand::Var("i".into())], "f64"),
        IIRInstr::new("const", Some("c".into()), vec![Operand::Float(2.7)], "f64"),
        IIRInstr::new("sub", Some("d".into()),
            vec![Operand::Var("r".into()), Operand::Var("c".into())], "f64"),
        IIRInstr::new("real_to_int_floor", Some("o".into()), vec![Operand::Var("d".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("o".into())], "i64"),
    ]));
    let mut class = lower_iir_to_jvm(&m, &IIRJvmConfig { class_name: "Main".into() })
        .expect("lower");

    // Inject a `main(String[])` that prints Main.compute()J (same launcher shape
    // as e6_global_runs_on_real_java).
    {
        use jvm_class_file::JvmConstantPoolEntry as E;
        let cp = &mut class.constant_pool;
        let sys_utf8 = cp_append(cp, E::Utf8("java/lang/System".into()));
        let sys_class = cp_append(cp, E::Class { name_index: sys_utf8 });
        let out_utf8 = cp_append(cp, E::Utf8("out".into()));
        let ps_desc = cp_append(cp, E::Utf8("Ljava/io/PrintStream;".into()));
        let out_nat = cp_append(cp, E::NameAndType { name_index: out_utf8, descriptor_index: ps_desc });
        let out_fieldref = cp_append(cp, E::Fieldref { class_index: sys_class, name_and_type_index: out_nat });
        let ps_utf8 = cp_append(cp, E::Utf8("java/io/PrintStream".into()));
        let ps_class = cp_append(cp, E::Class { name_index: ps_utf8 });
        let pln_utf8 = cp_append(cp, E::Utf8("println".into()));
        let pln_desc = cp_append(cp, E::Utf8("(J)V".into()));
        let pln_nat = cp_append(cp, E::NameAndType { name_index: pln_utf8, descriptor_index: pln_desc });
        let println_ref = cp_append(cp, E::Methodref { class_index: ps_class, name_and_type_index: pln_nat });
        let _ = cp_append(cp, E::Utf8("main".into()));
        let _ = cp_append(cp, E::Utf8("([Ljava/lang/String;)V".into()));

        let compute_ref = find_methodref_in_cp(&class.constant_pool, "Main", "compute", "()J");
        assert_ne!(compute_ref, 0, "Main.compute Methodref must be in CP");

        let [out_hi, out_lo] = out_fieldref.to_be_bytes();
        let [cmp_hi, cmp_lo] = compute_ref.to_be_bytes();
        let [pln_hi, pln_lo] = println_ref.to_be_bytes();
        let main_code = vec![
            0xB2, out_hi, out_lo, // getstatic System.out
            0xB8, cmp_hi, cmp_lo, // invokestatic Main.compute()J
            0xB6, pln_hi, pln_lo, // invokevirtual println(J)V
            0xB1,                 // return
        ];
        use jvm_class_file::{ACC_PUBLIC, ACC_STATIC, JvmCodeAttribute, JvmMethodAttribute};
        class.methods.push(JvmMethodInfo {
            access_flags: ACC_PUBLIC | ACC_STATIC,
            name: "main".into(),
            descriptor: "([Ljava/lang/String;)V".into(),
            attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
                name: "Code".into(),
                max_stack: 4,
                max_locals: 1,
                code: main_code,
                nested_attributes: vec![],
            })],
        });
    }

    let bytes = serialize_jvm_class_file(&class);
    let tmp = std::env::temp_dir().join(format!("e8_jvm_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir");
    std::fs::write(tmp.join("Main.class"), &bytes).expect("write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&tmp).arg("Main")
        .output().expect("run java");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(&tmp);
    assert_eq!(stdout.trim(), "42",
        "expected 42, got {stdout:?}; stderr: {:?}", String::from_utf8_lossy(&out.stderr));
}
