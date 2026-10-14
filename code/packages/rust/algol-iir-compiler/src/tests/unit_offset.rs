//! Public compiler oracles for isolating the one-unit outer-entier slice.
use super::*;

#[test]
fn nested_offsets_and_shifted_narrow_range_proofs_have_exact_typed_refusals() {
    let mut failures = Vec::new();
    for (name, expression) in [
        ("nested-right", "entier((sign(pick()) + 1) + 1)"),
        ("nested-left", "entier(1 + (1 + sign(pick())))"),
        ("entier-wrapped", "entier(entier(sign(pick()) + 1) + 1)"),
        ("abs-wrapped", "entier(abs(sign(pick()) + 1) + 1)"),
        ("shifted-triple-exp", "entier(ln(exp(exp(exp(sign(pick()) + 1)))))"),
        ("flat-two", "entier(sign(pick()) + 1 + 1)"),
        ("non-unit", "entier(sign(pick()) + 2)"),
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        match compile_source(&source, "unit_offset_refusal") {
            Err(CompileError::Type(message)) if message.contains("cannot print a real value") => {}
            Ok(_) => failures.push((name, "unexpectedly accepted".to_owned())),
            Err(error) => failures.push((name, format!("wrong refusal: {error:?}"))),
        }
    }
    assert!(failures.is_empty(), "exact Type/formatter refusal failures: {failures:?}");
}

#[test]
fn flat_units_and_original_zero_exp_keep_real_formatter_admission() {
    for expression in [
        "entier(sign(pick()) + 1)",
        "entier(sign(pick()) - 1)",
        "entier(1 + sign(pick()))",
        "entier(1 - sign(pick()))",
        "entier((entier(sign(pick())) + 1))",
        "entier(-(sign(pick()) + 1))",
        "entier(ln(exp(exp(exp(sign(pick()) + 0)))))",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        let module = compile_source(&source, "unit_offset_preservation").unwrap_or_else(|e| panic!("{source}: {e}"));
        let main = module.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op=="call" && i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")), "{source}");
    }
}

#[test]
fn variable_terms_and_overridden_sign_keep_exact_typed_refusals() {
    for source in [
        "begin real procedure pick; pick := 3.5; real unit, saved; unit := 1.0; saved := entier(sign(pick()) + unit); output(saved) end",
        "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 1); output(saved) end",
    ] {
        assert!(matches!(compile_source(source,"unit_offset_identity"),Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "{source}");
    }
}
