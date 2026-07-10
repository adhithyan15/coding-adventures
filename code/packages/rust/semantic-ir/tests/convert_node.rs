//! Tests for the SIR26 `Expr::Convert` node: text format, feature gating, and
//! the validator's observation of `Conversions` + the SIR21 type-implied
//! features of the target type.

use semantic_ir::effects::EffectSet;
use semantic_ir::{
    print_expr, validate, Block, Expr, Feature, FeatureManifest, Function, IntSpec, IntWidth,
    Metadata, Module, Overflow, Span, CURRENT_SIR_VERSION,
};

fn u8_wrap() -> IntSpec {
    IntSpec::sized(IntWidth::W8, false, Overflow::Wrap)
}

fn convert(to: IntSpec, inner: Expr) -> Expr {
    Expr::Convert {
        value: Box::new(inner),
        to,
        span: Span::synthetic(),
    }
}

fn int(v: i64) -> Expr {
    Expr::IntLit {
        value: v,
        span: Span::synthetic(),
    }
}

/// A module whose `main` returns `body`, with the given manifest features.
fn module_returning(body: Expr, features: &[Feature]) -> Module {
    Module {
        name: "prog".into(),
        manifest: FeatureManifest::from_features(features),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: body,
                span: Span::synthetic(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: Span::synthetic(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: Span::synthetic(),
    }
}

#[test]
fn prints_as_convert_with_intspec_target() {
    let e = convert(u8_wrap(), int(300));
    assert_eq!(print_expr(&e), "(convert (int u8 wrap) (int 300))");
}

#[test]
fn arbitrary_target_prints_bare_int() {
    // A widen into the unbounded integer prints its target as bare `int`.
    let e = convert(IntSpec::arbitrary(), int(5));
    assert_eq!(print_expr(&e), "(convert int (int 5))");
}

#[test]
fn convert_requires_conversions_feature() {
    // Undeclared: the validator observes `Conversions` but the manifest does
    // not declare it → error.
    let m = module_returning(convert(u8_wrap(), int(300)), &[]);
    let r = validate(&m);
    assert!(
        !r.is_ok(),
        "a Convert node with no manifest feature must fail"
    );
    assert!(
        r.issues.iter().any(|i| i.message.contains("conversions")),
        "the diagnostic should name the missing `conversions` feature: {:?}",
        r.issues
    );
}

#[test]
fn convert_validates_with_full_manifest() {
    // A u8-wrap target implies Conversions + SizedIntegers + Unsigned +
    // WrappingArithmetic; declaring all of them validates.
    let m = module_returning(
        convert(u8_wrap(), int(300)),
        &[
            Feature::Conversions,
            Feature::SizedIntegers,
            Feature::Unsigned,
            Feature::WrappingArithmetic,
        ],
    );
    let r = validate(&m);
    assert!(r.is_ok(), "should validate: {:?}", r.issues);
}

#[test]
fn signed_target_does_not_observe_unsigned() {
    // An i32-wrap target implies Conversions + SizedIntegers + Wrapping, but
    // NOT Unsigned — so declaring those three (without Unsigned) validates.
    let i32_wrap = IntSpec::sized(IntWidth::W32, true, Overflow::Wrap);
    let m = module_returning(
        convert(i32_wrap, int(5)),
        &[
            Feature::Conversions,
            Feature::SizedIntegers,
            Feature::WrappingArithmetic,
        ],
    );
    assert!(validate(&m).is_ok());
}

#[test]
fn conversions_feature_name_round_trips() {
    assert_eq!(Feature::Conversions.name(), "conversions");
    assert_eq!(
        Feature::from_name("conversions"),
        Some(Feature::Conversions)
    );
}
