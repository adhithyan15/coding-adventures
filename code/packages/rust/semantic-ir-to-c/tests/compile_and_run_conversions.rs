//! Execution proof for SIR26 `Expr::Convert` rendering — build a module whose
//! `main` prints `convert(to, IntLit(v))`, emit C, compile it with a real
//! gcc/clang-style compiler, run it, and assert stdout.
//!
//! Compiler discovery mirrors `compile_and_run.rs`: `SIR_CC` first, then
//! `cc`/`clang`/`gcc` on `PATH`; when none is present every case **skips**.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, IntSpec, IntWidth, Metadata,
    Module, Overflow, Span, CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        let ok = Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_run(cc: &str, module: &Module) -> String {
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_conv_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(cc)
        .arg("-std=c99")
        .arg("-o")
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn C compiler");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let run = Command::new(&exe).output().expect("run emitted program");
    assert!(
        run.status.success(),
        "run failed (exit {:?})",
        run.status.code()
    );
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&exe);
    String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
}

/// A module whose `main` does `puts(convert(to, IntLit(v)))`.
fn convert_module(v: i64, to: IntSpec) -> Module {
    let conv = Expr::Convert {
        value: Box::new(Expr::IntLit {
            value: v,
            span: Span::synthetic(),
        }),
        to,
        span: Span::synthetic(),
    };
    let puts = Expr::BuiltinCall {
        name: "puts".into(),
        args: vec![conv],
        effects: EffectSet::PURE,
        span: Span::synthetic(),
    };
    Module {
        name: "prog".into(),
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
                stmts: vec![],
                value: puts,
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

fn sized(w: IntWidth, signed: bool) -> IntSpec {
    IntSpec::sized(w, signed, Overflow::Wrap)
}

#[test]
fn convert_cases_compile_and_run() {
    let Some(cc) = find_cc() else {
        eprintln!("skip: no C compiler (set SIR_CC or install cc/clang/gcc)");
        return;
    };
    // (value, target, expected stdout) — the canonical wraparound/truncation set.
    let cases: &[(i64, IntSpec, &str)] = &[
        (300, sized(IntWidth::W8, false), "44\n"), // (uint8_t)300
        (200, sized(IntWidth::W8, true), "-56\n"), // (int8_t)200
        (-1, sized(IntWidth::W32, false), "4294967295\n"), // (uint32_t)-1
        (4_000_000_000, sized(IntWidth::W32, true), "-294967296\n"), // (int32_t)4e9
        (65_535, sized(IntWidth::W16, false), "65535\n"), // fits u16
        (70_000, sized(IntWidth::W16, false), "4464\n"), // 70000 mod 65536
        (4_000_000_000, IntSpec::arbitrary(), "4000000000\n"), // identity widen
    ];
    for (v, to, expected) in cases {
        let got = compile_and_run(&cc, &convert_module(*v, *to));
        assert_eq!(&got, expected, "convert({v}, {to:?})");
    }
}

#[test]
fn convert_emit_shape_uses_the_runtime_helper() {
    let src = semantic_ir_to_c::compile(&convert_module(300, sized(IntWidth::W8, false)))
        .unwrap()
        .source;
    // uint8 → _sir_convert(<value>, 8, 0); the runtime helper is present.
    assert!(
        src.contains("_sir_convert(_sir_int(300LL), 8, 0)"),
        "emit:\n{src}"
    );
    assert!(
        src.contains("int64_t _sir_mask_to("),
        "runtime helper present"
    );
    // An arbitrary-width target is the identity (no _sir_convert wrapper).
    let arb = semantic_ir_to_c::compile(&convert_module(5, IntSpec::arbitrary()))
        .unwrap()
        .source;
    assert!(
        arb.contains("_sir_puts(1, _sir_int(5LL))"),
        "arbitrary is identity:\n{arb}"
    );
}
