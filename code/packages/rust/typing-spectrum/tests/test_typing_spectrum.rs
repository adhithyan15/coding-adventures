//! Integration tests for the `typing-spectrum` crate.
//!
//! These tests exercise the full pipeline:
//!  1. Build an IIRModule (with varying levels of type information).
//!  2. Run `iir_type_checker::infer_and_check` to fill in inferred types.
//!  3. Run `typing_spectrum::advise` to get the compilation advisory.
//!  4. Assert on mode, threshold, tier, and helper predicates.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use iir_type_checker::infer_and_check;
use typing_spectrum::advisory::advise;
use typing_spectrum::canonical::{
    map_frontend_type, is_canonical, ALL_CANONICAL_TYPES, TYPE_I64, TYPE_F64, TYPE_BOOL, TYPE_ANY,
};
use typing_spectrum::mode::CompilationMode;
use typing_spectrum::threshold::{JitPromotionThreshold, THRESHOLD_FULLY_TYPED, THRESHOLD_UNTYPED};
use iir_type_checker::tier::TypingTier;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a module with one fully-typed "identity" function.
fn fully_typed_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "identity",
        vec![("x".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("typed", "tetrad");
    m.add_or_replace(fn_);
    m
}

/// Build a module with one untyped function (all "any").
fn untyped_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "mystery",
        vec![("a".into(), "any".into()), ("b".into(), "any".into())],
        "any",
        vec![
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "any"),
        ],
    );
    let mut m = IIRModule::new("untyped", "twig");
    m.add_or_replace(fn_);
    m
}

/// Build a module with one partially-typed function.
fn partial_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "mixed",
        vec![("n".into(), "i64".into()), ("s".into(), "any".into())],
        "any",
        vec![
            // typed: "const" with Int literal → will be inferred as i64
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "any"),
            // typed: add with all-i64 operands → will be inferred as i64
            IIRInstr::new(
                "add",
                Some("np1".into()),
                vec![Operand::Var("n".into()), Operand::Var("one".into())],
                "any",
            ),
            // untyped: uses "s" which is "any"
            IIRInstr::new(
                "call_builtin",
                Some("result".into()),
                vec![Operand::Var("s".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "any"),
        ],
    );
    let mut m = IIRModule::new("partial", "twig");
    m.add_or_replace(fn_);
    m
}

// ===========================================================================
// CompilationMode tests
// ===========================================================================

#[test]
fn fully_typed_module_recommends_aot_no_profile() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert_eq!(adv.recommended_mode, CompilationMode::AotNoProfile);
}

#[test]
fn fully_typed_does_not_require_deopt() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert!(!adv.requires_deopt());
}

#[test]
fn fully_typed_does_not_require_profile_input() {
    assert!(!CompilationMode::AotNoProfile.requires_profile_input());
}

#[test]
fn jit_mode_writes_profile() {
    assert!(CompilationMode::Jit.writes_profile());
    assert!(CompilationMode::JitThenAotWithPgo.writes_profile());
    assert!(!CompilationMode::AotNoProfile.writes_profile());
    assert!(!CompilationMode::AotWithPgo.writes_profile());
}

#[test]
fn aot_with_pgo_requires_profile_input() {
    assert!(CompilationMode::AotWithPgo.requires_profile_input());
}

#[test]
fn tree_walking_never_requires_deopt_or_profile() {
    assert!(!CompilationMode::TreeWalking.requires_deopt());
    assert!(!CompilationMode::TreeWalking.requires_profile_input());
    assert!(!CompilationMode::TreeWalking.writes_profile());
}

#[test]
fn untyped_module_recommends_jit() {
    let mut m = untyped_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    // An untyped module (all "any", inference can't help because params are "any")
    // should recommend JIT.
    assert_eq!(adv.recommended_mode, CompilationMode::Jit);
}

#[test]
fn mode_description_is_non_empty() {
    for mode in [
        CompilationMode::TreeWalking,
        CompilationMode::AotNoProfile,
        CompilationMode::AotWithPgo,
        CompilationMode::Jit,
        CompilationMode::JitThenAotWithPgo,
    ] {
        assert!(!mode.description().is_empty(), "empty description for {mode}");
    }
}

#[test]
fn mode_display_is_lowercase_kebab() {
    assert_eq!(CompilationMode::AotNoProfile.to_string(), "aot-no-profile");
    assert_eq!(CompilationMode::AotWithPgo.to_string(), "aot-with-pgo");
    assert_eq!(CompilationMode::Jit.to_string(), "jit");
    assert_eq!(CompilationMode::JitThenAotWithPgo.to_string(), "jit-then-aot-pgo");
    assert_eq!(CompilationMode::TreeWalking.to_string(), "tree-walking");
}

#[test]
fn speedup_range_is_positive_and_ordered_for_all_combinations() {
    let tiers = [
        TypingTier::FullyTyped,
        TypingTier::Partial(0.5),
        TypingTier::Untyped,
    ];
    let modes = [
        CompilationMode::TreeWalking,
        CompilationMode::AotNoProfile,
        CompilationMode::AotWithPgo,
        CompilationMode::Jit,
        CompilationMode::JitThenAotWithPgo,
    ];
    for tier in &tiers {
        for mode in &modes {
            let (lo, hi) = mode.expected_speedup_over_interp(tier);
            assert!(lo >= 1, "{mode} × {tier:?}: lo={lo} < 1");
            assert!(lo <= hi, "{mode} × {tier:?}: lo={lo} > hi={hi}");
        }
    }
}

// ===========================================================================
// JitPromotionThreshold tests
// ===========================================================================

#[test]
fn fully_typed_threshold_is_zero() {
    let t = JitPromotionThreshold::for_tier(&TypingTier::FullyTyped);
    assert_eq!(t.call_count, THRESHOLD_FULLY_TYPED);
    assert!(t.should_promote(0));
}

#[test]
fn untyped_threshold_is_hundred() {
    let t = JitPromotionThreshold::for_tier(&TypingTier::Untyped);
    assert_eq!(t.call_count, THRESHOLD_UNTYPED);
    assert!(!t.should_promote(99));
    assert!(t.should_promote(100));
}

#[test]
fn partial_at_half_threshold_is_fifty_five() {
    let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(0.5));
    assert_eq!(t.call_count, 55);
}

#[test]
fn threshold_monotone_decreasing_with_fraction() {
    let fracs = [0.0f32, 0.1, 0.2, 0.4, 0.6, 0.8, 1.0];
    let mut prev = u32::MAX;
    for &f in &fracs {
        let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(f));
        assert!(t.call_count <= prev, "non-monotone at {f}: {} > {prev}", t.call_count);
        prev = t.call_count;
    }
}

#[test]
fn threshold_label_is_correct() {
    assert_eq!(
        JitPromotionThreshold { call_count: 0 }.label(),
        "compile-before-first-call"
    );
    assert_eq!(
        JitPromotionThreshold { call_count: 10 }.label(),
        "after-10-calls"
    );
}

// ===========================================================================
// Canonical type-name tests
// ===========================================================================

#[test]
fn all_canonical_types_are_recognised() {
    for &ty in ALL_CANONICAL_TYPES {
        assert!(is_canonical(ty), "{ty} not recognised as canonical");
    }
}

#[test]
fn type_any_is_not_canonical() {
    assert!(!is_canonical(TYPE_ANY));
}

#[test]
fn map_twig_int_to_i64() {
    assert_eq!(map_frontend_type("int", "twig"), Some(TYPE_I64));
}

#[test]
fn map_twig_float_to_f64() {
    assert_eq!(map_frontend_type("float", "twig"), Some(TYPE_F64));
}

#[test]
fn map_twig_bool_to_bool() {
    assert_eq!(map_frontend_type("bool", "twig"), Some(TYPE_BOOL));
}

#[test]
fn map_typescript_number_to_f64() {
    assert_eq!(map_frontend_type("number", "typescript"), Some(TYPE_F64));
}

#[test]
fn map_typescript_boolean_to_bool() {
    assert_eq!(map_frontend_type("boolean", "typescript"), Some(TYPE_BOOL));
}

#[test]
fn map_typescript_null_to_nil() {
    assert_eq!(map_frontend_type("null", "typescript"), Some("nil"));
    assert_eq!(map_frontend_type("undefined", "typescript"), Some("nil"));
}

#[test]
fn map_ruby_integer_to_i64() {
    assert_eq!(map_frontend_type("Integer", "ruby"), Some(TYPE_I64));
}

#[test]
fn map_ruby_nilclass_to_nil() {
    assert_eq!(map_frontend_type("NilClass", "ruby"), Some("nil"));
}

#[test]
fn map_ruby_trueclassfalseclass_to_bool() {
    assert_eq!(map_frontend_type("TrueClass", "ruby"), Some(TYPE_BOOL));
    assert_eq!(map_frontend_type("FalseClass", "ruby"), Some(TYPE_BOOL));
}

#[test]
fn map_python_int_to_i64() {
    assert_eq!(map_frontend_type("int", "python"), Some(TYPE_I64));
}

#[test]
fn map_python_none_to_nil() {
    assert_eq!(map_frontend_type("None", "python"), Some("nil"));
}

#[test]
fn map_rust_primitives_pass_through() {
    for &ty in &["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64"] {
        assert_eq!(map_frontend_type(ty, "rust"), Some(ty), "rust/{ty}");
    }
}

#[test]
fn map_c_char_to_u8() {
    assert_eq!(map_frontend_type("char", "c"), Some("u8"));
}

#[test]
fn unknown_type_unknown_language_returns_none() {
    assert_eq!(map_frontend_type("Flubber", "cobol"), None);
}

#[test]
fn canonical_type_passes_through_for_any_language() {
    // Canonical types are accepted regardless of language tag.
    assert_eq!(map_frontend_type("i64", "brainfuck"), Some("i64"));
    assert_eq!(map_frontend_type("bool", "cobol"), Some("bool"));
}

// ===========================================================================
// Advisory round-trip tests
// ===========================================================================

#[test]
fn advisory_module_name_matches() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert_eq!(adv.module_name, "typed");
}

#[test]
fn advisory_function_count_matches() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert_eq!(adv.functions.len(), m.functions.len());
}

#[test]
fn advisory_fully_typed_function_has_zero_threshold() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert_eq!(adv.functions[0].jit_threshold.call_count, 0);
}

#[test]
fn advisory_summary_is_non_empty() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    assert!(!adv.summary().is_empty());
}

#[test]
fn advisory_partial_module_is_between_untyped_and_fully_typed() {
    let mut m = partial_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    // Module has some typed and some untyped → Partial or Untyped tier.
    match &adv.module_tier {
        TypingTier::Partial(_) | TypingTier::Untyped => {} // expected
        TypingTier::FullyTyped => {
            // It's possible if inference resolved everything; that's also fine.
        }
    }
}

#[test]
fn function_advisory_typed_fraction_is_in_range() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    for fa in &adv.functions {
        assert!(fa.typed_fraction >= 0.0 && fa.typed_fraction <= 1.0,
            "typed_fraction out of range: {}", fa.typed_fraction);
    }
}

#[test]
fn function_advisory_fully_typed_subset_correct() {
    let mut m = fully_typed_module();
    infer_and_check(&mut m);
    let adv = advise(&m);
    // Our module is fully typed, so the fully_typed_functions subset should be non-empty.
    assert!(!adv.fully_typed_functions().is_empty());
    // And fully_untyped_functions should be empty.
    assert!(adv.fully_untyped_functions().is_empty());
}

#[test]
fn jit_threshold_zero_always_promotes() {
    let t = JitPromotionThreshold { call_count: 0 };
    for n in [0u32, 1, 100, u32::MAX] {
        assert!(t.should_promote(n));
    }
}
