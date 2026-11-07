//! # Warrant Type Checker Integration Tests (VER04d / VER-P007)

use coding_adventures_warrant_grammar::binding::{
    validate_bindings, BindingManifest, ErrorNormalization, OpBinding,
};
use coding_adventures_warrant_grammar::checker::{
    check_warrant, check_warrant_with_limits, CheckerLimits, DiagnosticCode, Type,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use std::collections::BTreeMap;

fn parse_and_check(
    src: &str,
) -> Result<
    coding_adventures_warrant_grammar::checker::CheckedWarrantModule,
    coding_adventures_warrant_grammar::checker::CheckerError,
> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admission");
    let tokens = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let ast = parse_document(&tokens, &doc, ParserLimits::default()).expect("parser");
    check_warrant(&ast)
}

#[test]
fn test_valid_ct_compare_pilot() {
    let src = r#"warrant ct_compare
  error SelectError = CT_SELECT_LENGTH_MISMATCH
  model same(a: Bytes, b: Bytes) -> Bool = a == b
  op eq(a: Bytes, b: Bytes) -> Bool
  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool
  op select_bytes(a: Bytes, b: Bytes, choice: Bool) -> Result<Bytes, SelectError> fresh
  op eq_u64(a: U64, b: U64) -> Bool

  assume platform:
    none
  assume environment:
    none

  labels eq:
    a.contents secret
    a.length public
    b.contents secret
    b.length public

  labels eq_fixed:
    a.contents secret
    b.contents secret

  labels select_bytes:
    a.contents secret
    a.length public
    b.contents secret
    b.length public
    choice secret

  labels eq_u64:
    a secret
    b secret

  example empty_equal:
    eq(b"", b"") == true
  example select_left:
    select_bytes(b"00", b"ff", true) == Ok(b"00")
  example select_length_error:
    select_bytes(b"", b"00", false) == Err(SelectError.CT_SELECT_LENGTH_MISMATCH)
  example high_bit:
    eq_u64(9223372036854775808u64, 0u64) == false
  example fixed_empty:
    eq_fixed<0>(b"", b"") == true

  claim equal_agrees:
    forall a: Bytes, b: Bytes. eq(a, b) == same(a, b)

  input select_bytes.a:
    hostile
    rejects len(a) != len(b) with SelectError.CT_SELECT_LENGTH_MISMATCH
    ensures no_panic, terminates

  bound eq:
    steps <= 64 * len(a) + 512
    heap <= 2 * len(a)
    depth <= 32
"#;
    let checked = parse_and_check(src).expect("valid pilot module should check cleanly");
    assert_eq!(checked.module_name, "ct_compare");
    assert_eq!(checked.operations.len(), 4);
    assert_eq!(checked.models.len(), 1);
    assert_eq!(checked.errors.len(), 1);
    assert_eq!(checked.examples.len(), 5);
    assert_eq!(checked.claims.len(), 1);

    // Verify valid bindings
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "eq".to_string(),
        OpBinding {
            export: "ct_eq".to_string(),
            static_params: vec![],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["always_true".to_string()],
        },
    );
    bindings.insert(
        "eq_fixed".to_string(),
        OpBinding {
            export: "ct_eq_fixed".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["always_false".to_string()],
        },
    );
    bindings.insert(
        "select_bytes".to_string(),
        OpBinding {
            export: "ct_select_bytes".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: Some(ErrorNormalization {
                kind: "panic_to_error".to_string(),
                error_case: "CT_SELECT_LENGTH_MISMATCH".to_string(),
            }),
            negative_controls: vec!["corrupt_choice".to_string()],
        },
    );
    bindings.insert(
        "eq_u64".to_string(),
        OpBinding {
            export: "ct_eq_u64".to_string(),
            static_params: vec![],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["always_zero".to_string()],
        },
    );

    let manifest = BindingManifest {
        package: "coding_adventures_ct_compare".to_string(),
        revision: "origin/main".to_string(),
        bindings,
    };

    validate_bindings(&checked, &manifest).expect("valid bindings should pass");
}

#[test]
fn test_w0201_duplicate_symbols() {
    // Model duplicate
    let src1 = r#"warrant dup_test
  model foo(x: Int) -> Int = x
  model foo(x: Int) -> Int = x
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Error duplicate
    let src2 = r#"warrant dup_test
  error E = A
  error E = B
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Error constructor duplicate within enum
    let src3 = r#"warrant dup_test
  error E = A | A
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Op duplicate
    let src4 = r#"warrant dup_test
  op f() -> Bool
  op f() -> Bool
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Duplicate test name across example and claim
    let src5 = r#"warrant dup_test
  example t1:
    true == true
  claim t1:
    true == true
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Duplicate parameter
    let src6 = r#"warrant dup_param
  model add(x: Int, x: Int) -> Int = x + x
"#;
    assert_eq!(
        parse_and_check(src6).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Duplicate labels
    let src7 = r#"warrant dup_contract
  op f(x: Int) -> Int
  labels f:
    x secret
  labels f:
    x public
"#;
    assert_eq!(
        parse_and_check(src7).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Duplicate bounds
    let src8 = r#"warrant dup_contract
  op f(x: Int) -> Int
  bound f:
    steps <= 10
  bound f:
    steps <= 20
"#;
    assert_eq!(
        parse_and_check(src8).unwrap_err().code,
        DiagnosticCode::W0201
    );

    // Duplicate input
    let src9 = r#"warrant dup_contract
  op f(x: Int) -> Int
  input f.x:
    hostile
    ensures no_panic
  input f.x:
    hostile
    ensures terminates
"#;
    assert_eq!(
        parse_and_check(src9).unwrap_err().code,
        DiagnosticCode::W0201
    );
}

#[test]
fn test_w0202_unbound_identifiers() {
    let src1 = r#"warrant unbound_var
  model test(x: Int) -> Int = y + 1
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0202
    );

    let src2 = r#"warrant unbound_type
  model test(x: NonExistentType) -> Int = 1
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0202
    );

    let src3 = r#"warrant unbound_ctor
  error MyError = CASE_A
  example ex:
    MyError.CASE_B == MyError.CASE_A
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0202
    );

    let src4 = r#"warrant unbound_fixed_param
  model test(b: Bytes<N>) -> Int = 1
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0202
    );

    let src5 = r#"warrant unbound_result_err
  model test() -> Result<Int, UnknownErr> = Ok(1)
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0202
    );
}

#[test]
fn test_w0203_cycles_and_shadowing() {
    // 2-cycle alias
    let src1 = r#"warrant cycle_alias
  model A = B
  model B = A
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0203
    );

    // 3-cycle alias
    let src2 = r#"warrant cycle_alias
  model A = B
  model B = C
  model C = A
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0203
    );

    // Direct model recursion
    let src3 = r#"warrant cycle_models
  model f(x: Int) -> Int = f(x)
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0203
    );

    // Mutual model recursion
    let src4 = r#"warrant cycle_models
  model f(x: Int) -> Int = g(x)
  model g(x: Int) -> Int = f(x)
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0203
    );

    // Shadowed quantifier binder
    let src5 = r#"warrant shadow_test
  claim test:
    forall x: Int. (forall x: Int. x == x)
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0203
    );
}

#[test]
fn test_w0301_type_and_match_errors() {
    // Model calls op
    let src1 = r#"warrant op_call_test
  op real_op(x: Int) -> Int
  labels real_op:
    x secret
  model impure_model(x: Int) -> Int = real_op(x)
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Return type mismatch
    let src2 = r#"warrant ret_mismatch
  model test(x: Int) -> Bool = x + 1
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Binary arithmetic mismatch (Int + Bool)
    let src3 = r#"warrant bin_mismatch
  model test(x: Int, b: Bool) -> Int = x + b
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Binary comparison mismatch (Bool < Bool)
    let src4 = r#"warrant cmp_mismatch
  example ex:
    true < false
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Equality mismatch (Int == Bool)
    let src5 = r#"warrant eq_mismatch
  example ex:
    1 == true
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Logical op mismatch (Int and Bool)
    let src6 = r#"warrant log_mismatch
  example ex:
    1 and true
"#;
    assert_eq!(
        parse_and_check(src6).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Non-exhaustive Option match
    let src7 = r#"warrant match_test
  model unwrap(opt: Option<Int>) -> Int = match opt with Some(x) -> x
"#;
    assert_eq!(
        parse_and_check(src7).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Non-exhaustive Result match
    let src8 = r#"warrant match_test
  error E = A
  model unwrap(r: Result<Int, E>) -> Int = match r with Ok(x) -> x
"#;
    assert_eq!(
        parse_and_check(src8).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Incompatible match arms (Int vs Bool)
    let src9 = r#"warrant match_test
  model unwrap(opt: Option<Int>) -> Int = match opt with None -> true | Some(x) -> x
"#;
    assert_eq!(
        parse_and_check(src9).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Match on non-Option non-Result
    let src10 = r#"warrant match_test
  model test(x: Int) -> Int = match x with None -> 0 | Some(y) -> y
"#;
    assert_eq!(
        parse_and_check(src10).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Unary minus on Bool
    let src11 = r#"warrant un_mismatch
  model test(b: Bool) -> Int = -b
"#;
    assert_eq!(
        parse_and_check(src11).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Unary not on Int
    let src12 = r#"warrant un_mismatch
  model test(x: Int) -> Bool = not x
"#;
    assert_eq!(
        parse_and_check(src12).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Incompatible conditional branches
    let src13 = r#"warrant if_mismatch
  model test(b: Bool) -> Int = if b then 1 else false
"#;
    assert_eq!(
        parse_and_check(src13).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Conditional with non-Bool condition
    let src14 = r#"warrant if_mismatch
  model test(x: Int) -> Int = if x then 1 else 2
"#;
    assert_eq!(
        parse_and_check(src14).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Empty list without contextual type
    let src15 = r#"warrant list_mismatch
  example ex:
    [] == []
"#;
    assert_eq!(
        parse_and_check(src15).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // List with mismatched element types
    let src16 = r#"warrant list_mismatch
  example ex:
    [1, true] == [1, true]
"#;
    assert_eq!(
        parse_and_check(src16).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Quantifier refinement non-Bool
    let src17 = r#"warrant quant_mismatch
  claim cl:
    forall x: Int | x + 1. x == x
"#;
    assert_eq!(
        parse_and_check(src17).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Claim body non-Bool
    let src18 = r#"warrant quant_mismatch
  claim cl:
    forall x: Int. x + 1
"#;
    assert_eq!(
        parse_and_check(src18).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Example body non-Bool
    let src19 = r#"warrant ex_mismatch
  example ex:
    1 + 1
"#;
    assert_eq!(
        parse_and_check(src19).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Invalid bound expression (non-numeric)
    let src20 = r#"warrant bound_err
  op f(x: Int) -> Int
  labels f:
    x secret
  bound f:
    steps <= true
"#;
    assert_eq!(
        parse_and_check(src20).unwrap_err().code,
        DiagnosticCode::W0301
    );
}

#[test]
fn test_w0401_contract_errors() {
    // Missing secrecy label
    let src1 = r#"warrant labels_missing
  op test_op(a: Bytes, b: Bytes) -> Bool
  labels test_op:
    a.contents secret
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Unknown op in labels
    let src2 = r#"warrant labels_unknown
  labels nonexistent_op:
    a secret
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Unknown param in labels
    let src3 = r#"warrant labels_param_unknown
  op test_op(a: Bytes) -> Bool
  labels test_op:
    nonexistent secret
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Invalid projection in labels
    let src4 = r#"warrant labels_proj
  op test_op(a: Bytes) -> Bool
  labels test_op:
    a.invalid_proj secret
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Unknown op in bounds
    let src5 = r#"warrant bound_unknown
  bound nonexistent_op:
    steps <= 10
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Invalid bound metric
    let src6 = r#"warrant bound_metric
  op test_op(a: Bytes) -> Bool
  labels test_op:
    a secret
  bound test_op:
    cycles <= 100
"#;
    assert_eq!(
        parse_and_check(src6).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Invalid input target (more than 2 segments)
    let src7 = r#"warrant input_err
  op test_op(a: Bytes) -> Bool
  labels test_op:
    a secret
  input test_op.a.extra:
    hostile
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src7).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Unknown param in input
    let src8 = r#"warrant input_err
  op test_op(a: Bytes) -> Bool
  labels test_op:
    a secret
  input test_op.nonexistent:
    hostile
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src8).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Rejection error on op not returning Result
    let src9 = r#"warrant input_err
  error E = A
  op test_op(a: Bytes) -> Bool
  labels test_op:
    a secret
  input test_op.a:
    hostile
    rejects true with E.A
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src9).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Rejection error type mismatch
    let src10 = r#"warrant input_err
  error ExpectedError = A
  error OtherError = B
  op test_op(a: Bytes) -> Result<Bytes, ExpectedError>
  labels test_op:
    a secret
  input test_op.a:
    hostile
    rejects true with OtherError.B
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src10).unwrap_err().code,
        DiagnosticCode::W0401
    );
}

#[test]
fn test_w0501_and_w0502_binding_errors() {
    let src = r#"warrant bind_test
  error E = A
  op op_a(a: Bytes) -> Bytes fresh
  op op_b<N: Nat>(a: Bytes<N>) -> Result<Bytes, E>
  labels op_a:
    a secret
  labels op_b:
    a secret
"#;
    let checked = parse_and_check(src).unwrap();

    // W0501: Missing binding for op_b
    let mut b1 = BTreeMap::new();
    b1.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m1 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b1,
    };
    assert_eq!(
        validate_bindings(&checked, &m1).unwrap_err().code,
        DiagnosticCode::W0501
    );

    // W0501: Unknown op in manifest
    let mut b2 = BTreeMap::new();
    b2.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    b2.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    b2.insert(
        "unknown_op".to_string(),
        OpBinding {
            export: "unknown".to_string(),
            static_params: vec![],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m2 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b2,
    };
    assert_eq!(
        validate_bindings(&checked, &m2).unwrap_err().code,
        DiagnosticCode::W0501
    );

    // W0501: Invalid static param count
    let mut b3 = BTreeMap::new();
    b3.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    b3.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec![], // Missing N!
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m3 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b3,
    };
    assert_eq!(
        validate_bindings(&checked, &m3).unwrap_err().code,
        DiagnosticCode::W0501
    );

    // W0501: Error normalization on non-Result op
    let mut b4 = BTreeMap::new();
    b4.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: Some(ErrorNormalization {
                kind: "panic_to_error".to_string(),
                error_case: "A".to_string(),
            }),
            negative_controls: vec!["c1".to_string()],
        },
    );
    b4.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m4 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b4,
    };
    assert_eq!(
        validate_bindings(&checked, &m4).unwrap_err().code,
        DiagnosticCode::W0501
    );

    // W0501: Error normalization with invalid variant
    let mut b5 = BTreeMap::new();
    b5.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    b5.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: Some(ErrorNormalization {
                kind: "panic_to_error".to_string(),
                error_case: "NONEXISTENT_VARIANT".to_string(),
            }),
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m5 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b5,
    };
    assert_eq!(
        validate_bindings(&checked, &m5).unwrap_err().code,
        DiagnosticCode::W0501
    );

    // W0502: Fresh requirement missing
    let mut b6 = BTreeMap::new();
    b6.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: false, // Must be true!
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    b6.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m6 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b6,
    };
    assert_eq!(
        validate_bindings(&checked, &m6).unwrap_err().code,
        DiagnosticCode::W0502
    );

    // W0502: Negative controls empty
    let mut b7 = BTreeMap::new();
    b7.insert(
        "op_a".to_string(),
        OpBinding {
            export: "op_a".to_string(),
            static_params: vec![],
            fresh: true,
            error_normalization: None,
            negative_controls: vec![], // Empty!
        },
    );
    b7.insert(
        "op_b".to_string(),
        OpBinding {
            export: "op_b".to_string(),
            static_params: vec!["N".to_string()],
            fresh: false,
            error_normalization: None,
            negative_controls: vec!["c1".to_string()],
        },
    );
    let m7 = BindingManifest {
        package: "p".to_string(),
        revision: "r".to_string(),
        bindings: b7,
    };
    assert_eq!(
        validate_bindings(&checked, &m7).unwrap_err().code,
        DiagnosticCode::W0502
    );
}

#[test]
fn test_builtins_typecheck() {
    let src = r#"warrant builtins_test
  model test_len(s: Bytes) -> Nat = len(s)
  model test_len_list(l: List<Int>) -> Nat = len(l)
  model test_at(s: Bytes, i: Nat) -> Option<U8> = at(s, i)
  model test_at_list(l: List<Int>, i: Nat) -> Option<Int> = at(l, i)
  model test_concat(a: Bytes, b: Bytes) -> Bytes = concat(a, b)
  model test_concat_fixed(a: Bytes<2>, b: Bytes<3>) -> Bytes<5> = concat(a, b)
  model test_fixed(b: Bytes) -> Option<Bytes<4>> = fixed<4>(b)
  model test_div(a: Int, b: Int) -> Option<Int> = div(a, b)
  model test_match(opt: Option<Int>) -> Int = match opt with None -> 0 | Some(x) -> x
  model test_some(x: Int) -> Option<Int> = Some(x)
  model test_none() -> Option<Int> = None
"#;
    let checked = parse_and_check(src).expect("builtins should typecheck");
    assert_eq!(checked.models.len(), 11);
}

#[test]
fn test_numeric_types_and_operators() {
    let src = r#"warrant num_test
  model test_u8(a: U8, b: U8) -> U8 = a + b - a * b
  model test_u8_cmp(a: U8, b: U8) -> Bool = a < b and a <= b and a > b and a >= b
  model test_u64(a: U64, b: U64) -> U64 = a + b - a * b
  model test_u64_cmp(a: U64, b: U64) -> Bool = a < b and a <= b and a > b and a >= b
  model test_int_cmp(a: Int, b: Nat) -> Bool = a < b and a <= b and a > b and a >= b
  model test_neg(x: Nat) -> Int = -x
  model test_logic(a: Bool, b: Bool) -> Bool = (a and b) or (a => b)
"#;
    let checked = parse_and_check(src).expect("numeric types should check");
    assert_eq!(checked.models.len(), 7);
}

#[test]
fn test_w0602_work_budget_exhaustion() {
    let src = r#"warrant work_test
  model f(x: Int) -> Int = x + 1 + 2 + 3 + 4
"#;
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let ast = parse_document(&tokens, &doc, ParserLimits::default()).unwrap();
    let limits = CheckerLimits {
        max_work: 2, // Tiny work limit
        max_depth: 64,
    };
    let err = check_warrant_with_limits(&ast, limits).unwrap_err();
    assert_eq!(err.code, DiagnosticCode::W0602);
}

#[test]
fn test_w0602_depth_budget_exhaustion() {
    let src = r#"warrant depth_test
  model f(x: Int) -> Int = ((((x + 1) + 2) + 3) + 4)
"#;
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
    let tokens = capture_layout(&doc, LayoutLimits::default()).unwrap();
    let ast = parse_document(&tokens, &doc, ParserLimits::default()).unwrap();
    let limits = CheckerLimits {
        max_work: 1_000_000,
        max_depth: 1, // Depth 1 will fail on nested expression
    };
    let err = check_warrant_with_limits(&ast, limits).unwrap_err();
    assert_eq!(err.code, DiagnosticCode::W0602);
}

#[test]
fn test_subtyping_nat_to_int_and_fixed_to_bytes() {
    let nat_ty = Type::Nat;
    let int_ty = Type::Int;
    assert!(nat_ty.is_assignable_to(&int_ty));
    assert!(!int_ty.is_assignable_to(&nat_ty));

    let fixed_ty = Type::FixedBytes(16);
    let bytes_ty = Type::Bytes;
    assert!(fixed_ty.is_assignable_to(&bytes_ty));
    assert!(!bytes_ty.is_assignable_to(&fixed_ty));

    let common = Type::common_type(&nat_ty, &int_ty).unwrap();
    assert_eq!(common, Type::Int);

    let common_bytes = Type::common_type(&fixed_ty, &bytes_ty).unwrap();
    assert_eq!(common_bytes, Type::Bytes);

    let fixed_param_ty = Type::FixedBytesParam("N".to_string());
    assert!(fixed_param_ty.is_assignable_to(&bytes_ty));

    let opt_nat = Type::Option(Box::new(Type::Nat));
    let opt_int = Type::Option(Box::new(Type::Int));
    assert!(opt_nat.is_assignable_to(&opt_int));

    let res_nat = Type::Result(Box::new(Type::Nat), "E".to_string());
    let res_int = Type::Result(Box::new(Type::Int), "E".to_string());
    assert!(res_nat.is_assignable_to(&res_int));

    assert_eq!(Type::common_type(&opt_nat, &opt_int).unwrap(), opt_int);
    assert_eq!(Type::common_type(&res_nat, &res_int).unwrap(), res_int);

    // Display names
    assert_eq!(Type::Bool.display_name(), "Bool");
    assert_eq!(Type::U8.display_name(), "U8");
    assert_eq!(Type::U64.display_name(), "U64");
    assert_eq!(Type::List(Box::new(Type::Int)).display_name(), "List<Int>");
    assert_eq!(fixed_param_ty.display_name(), "Bytes<N>");
    assert_eq!(
        Type::ErrorVariant("E.A".to_string()).display_name(),
        "ErrorVariant(E.A)"
    );
}

#[test]
fn test_builtins_and_calls_edge_cases() {
    // Model call wrong arity
    let src1 = r#"warrant call_err
  model f(x: Int) -> Int = x
  example ex:
    f(1, 2) == 1
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Model call wrong arg type
    let src2 = r#"warrant call_err
  model f(x: Int) -> Int = x
  example ex:
    f(true) == 1
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Op call wrong arity
    let src3 = r#"warrant call_err
  op f(x: Int) -> Int
  labels f:
    x secret
  example ex:
    f(1, 2) == 1
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Op call wrong static params arity
    let src4 = r#"warrant call_err
  op f<N: Nat>(x: Bytes<N>) -> Int
  labels f:
    x secret
  example ex:
    f(b"") == 1
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Op call wrong arg type
    let src5 = r#"warrant call_err
  op f(x: Int) -> Int
  labels f:
    x secret
  example ex:
    f(true) == 1
"#;
    assert_eq!(
        parse_and_check(src5).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Some wrong arity
    let src6 = r#"warrant some_err
  example ex:
    Some(1, 2) == None
"#;
    assert_eq!(
        parse_and_check(src6).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // None without expected Option
    let src7 = r#"warrant none_err
  example ex:
    None == 1
"#;
    assert_eq!(
        parse_and_check(src7).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Ok wrong arity
    let src8 = r#"warrant ok_err
  example ex:
    Ok(1, 2) == Ok(1)
"#;
    assert_eq!(
        parse_and_check(src8).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Err wrong arity
    let src9 = r#"warrant err_err
  example ex:
    Err(1, 2) == Ok(1)
"#;
    assert_eq!(
        parse_and_check(src9).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Err with non ErrorVariant
    let src10 = r#"warrant err_err
  example ex:
    Err(42) == Ok(1)
"#;
    assert_eq!(
        parse_and_check(src10).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // len wrong arity
    let src11 = r#"warrant len_err
  example ex:
    len(b"", b"") == 0
"#;
    assert_eq!(
        parse_and_check(src11).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // len on non-sequence
    let src12 = r#"warrant len_err
  example ex:
    len(42) == 0
"#;
    assert_eq!(
        parse_and_check(src12).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // at wrong arity
    let src13 = r#"warrant at_err
  example ex:
    at(b"") == None
"#;
    assert_eq!(
        parse_and_check(src13).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // at non-sequence
    let src14 = r#"warrant at_err
  example ex:
    at(42, 0) == None
"#;
    assert_eq!(
        parse_and_check(src14).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // at non-int index
    let src15 = r#"warrant at_err
  example ex:
    at(b"", true) == None
"#;
    assert_eq!(
        parse_and_check(src15).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // concat wrong arity
    let src16 = r#"warrant concat_err
  example ex:
    concat(b"") == b""
"#;
    assert_eq!(
        parse_and_check(src16).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // concat non-sequences
    let src17 = r#"warrant concat_err
  example ex:
    concat(1, 2) == 3
"#;
    assert_eq!(
        parse_and_check(src17).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // fixed wrong width count
    let src18 = r#"warrant fixed_err
  example ex:
    fixed<1, 2>(b"") == None
"#;
    assert_eq!(
        parse_and_check(src18).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // fixed wrong arity
    let src19 = r#"warrant fixed_err
  example ex:
    fixed<4>(b"", b"") == None
"#;
    assert_eq!(
        parse_and_check(src19).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // fixed non-bytes
    let src20 = r#"warrant fixed_err
  example ex:
    fixed<4>(42) == None
"#;
    assert_eq!(
        parse_and_check(src20).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // div wrong arity
    let src21 = r#"warrant div_err
  example ex:
    div(1) == None
"#;
    assert_eq!(
        parse_and_check(src21).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // div non-int
    let src22 = r#"warrant div_err
  example ex:
    div(true, 1) == None
"#;
    assert_eq!(
        parse_and_check(src22).unwrap_err().code,
        DiagnosticCode::W0301
    );
}

#[test]
fn test_contracts_and_types_edge_cases() {
    // Input rejection condition non-Bool
    let src1 = r#"warrant input_err
  error E = A
  op test_op(a: Bytes) -> Result<Bytes, E>
  labels test_op:
    a secret
  input test_op.a:
    hostile
    rejects 1 + 1 with E.A
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src1).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Input rejection error not 2 segments
    let src2 = r#"warrant input_err
  error E = A
  op test_op(a: Bytes) -> Result<Bytes, E>
  labels test_op:
    a secret
  input test_op.a:
    hostile
    rejects true with E.A.EXTRA
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src2).unwrap_err().code,
        DiagnosticCode::W0401
    );

    // Input rejection error variant unbound
    let src3 = r#"warrant input_err
  error E = A
  op test_op(a: Bytes) -> Result<Bytes, E>
  labels test_op:
    a secret
  input test_op.a:
    hostile
    rejects true with E.NONEXISTENT
    ensures no_panic
"#;
    assert_eq!(
        parse_and_check(src3).unwrap_err().code,
        DiagnosticCode::W0202
    );

    // Environment assumption non-Bool
    let src4 = r#"warrant env_err
  assume environment:
    foo: 1 + 1
"#;
    assert_eq!(
        parse_and_check(src4).unwrap_err().code,
        DiagnosticCode::W0301
    );

    // Type aliases with List, Option, Result
    let src5 = r#"warrant alias_ok
  error E = A
  model IntList = List<Int>
  model OptBytes = Option<Bytes>
  model Res = Result<Int, E>
  model use_aliases(l: IntList, o: OptBytes, r: Res) -> Int = 1
"#;
    let checked = parse_and_check(src5).expect("aliases should check cleanly");
    assert_eq!(checked.type_aliases.len(), 3);
    assert_eq!(checked.models.len(), 1);
}

#[test]
fn test_display_and_diagnostics_coverage() {
    let codes = [
        DiagnosticCode::W0201,
        DiagnosticCode::W0202,
        DiagnosticCode::W0203,
        DiagnosticCode::W0301,
        DiagnosticCode::W0401,
        DiagnosticCode::W0402,
        DiagnosticCode::W0501,
        DiagnosticCode::W0502,
        DiagnosticCode::W0602,
    ];
    for c in codes {
        let s = format!("{c}");
        assert!(!s.is_empty());
    }

    let err = coding_adventures_warrant_grammar::checker::CheckerError {
        code: DiagnosticCode::W0201,
        kind: coding_adventures_warrant_grammar::checker::CheckerErrorKind::WorkBudgetExceeded,
        span: (0, 1),
    };
    let s = format!("{err}");
    assert!(s.contains("W0201"));
}
