//! Dev helper: emit a self-contained C program that prints several
//! `Expr::Convert` results, for portability checking on clang/gcc/MSVC.

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, IntSpec, IntWidth, Metadata,
    Module, Overflow, Span, Stmt, CURRENT_SIR_VERSION,
};

fn conv(v: i64, w: IntWidth, signed: bool) -> Expr {
    Expr::Convert {
        value: Box::new(Expr::IntLit {
            value: v,
            span: Span::synthetic(),
        }),
        to: IntSpec::sized(w, signed, Overflow::Wrap),
        span: Span::synthetic(),
    }
}

fn puts(e: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args: vec![e],
            effects: EffectSet::PURE,
            span: Span::synthetic(),
        },
        span: Span::synthetic(),
    }
}

fn main() {
    let stmts = vec![
        puts(conv(300, IntWidth::W8, false)),           // 44
        puts(conv(200, IntWidth::W8, true)),            // -56
        puts(conv(70_000, IntWidth::W16, false)),       // 4464
        puts(conv(-1, IntWidth::W32, false)),           // 4294967295
        puts(conv(4_000_000_000, IntWidth::W32, true)), // -294967296
    ];
    let module = Module {
        name: "conv".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Conversions,
            Feature::SizedIntegers,
            Feature::Unsigned,
            Feature::WrappingArithmetic,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::NilLit {
                    span: Span::synthetic(),
                },
                span: Span::synthetic(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: Span::synthetic(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: Span::synthetic(),
    };
    print!("{}", semantic_ir_to_c::compile(&module).unwrap().source);
}
