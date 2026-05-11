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
    codegen::IIRJvmCodeGenerator, lower_iir_to_jvm, validate_for_jvm, IIRJvmConfig, IIRJvmError,
    JvmClassFile,
};

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

/// The `"str"` type hint is not supported (no string arithmetic in v1).
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
    };
    let errors = validate_for_jvm(&module);
    assert!(
        !errors.iter().any(|e| e.contains("UnsupportedType")),
        "f32 should be accepted, got errors: {:?}",
        errors
    );
}

/// Float constant operands are accepted (unlike the BEAM backend).
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

/// We target Java 8 (major version 52).
#[test]
fn lower_version_is_java8() {
    let module = module_with(void_fn("main"));
    let class = lower(&module);
    assert_eq!(class.version.major, 52);
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
