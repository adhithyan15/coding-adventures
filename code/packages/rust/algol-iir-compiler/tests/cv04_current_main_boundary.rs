//! Integration boundary committed before current-main conflict resolution.
//! Main extends finite sign offsets through21; this preserves old17/18 cases
//! and tests the new22 boundary without relaxing malformed-actual controls.
use algol_iir_compiler::{compile_source, CompileError};

fn source(offset: usize) -> String {
    format!("begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := if choose() then relay(entier(entier(sign(left)) + {offset})) else relay(entier(entier(sign(right)) + {offset})); output(result) end")
}

#[test]
fn current_main_finite_twenty_one_units_preserves_seventeen_and_eighteen() {
    for units in [17, 18, 19, 20, 21] {
        compile_source(&source(units), "integration")
            .unwrap_or_else(|error| panic!("guarded {units}-unit source must compile: {error:?}"));
        println!("CV04_MAIN_NATIVE_ACCEPT {units}");
    }
}

#[test]
fn current_main_twenty_two_units_still_refuses_at_typed_output_boundary() {
    let error = compile_source(&source(24), "integration").unwrap_err();
    assert!(
        matches!(&error, CompileError::Type(message) if message == "standard output procedure \"output\" cannot print a real value"),
        "{error:?}"
    );
    println!("CV04_MAIN_NATIVE_REFUSE 24");
}

#[test]
fn rounded_f64_intermediate_never_grants_an_exact_integer_bound() {
    let source = source(21).replace("+ 21", "+ ((9007199254740992 + 1) - 9007199254740992 + 21)");
    let error = compile_source(&source, "integration").unwrap_err();
    assert!(
        matches!(&error, CompileError::Type(message) if message == "standard output procedure \"output\" cannot print a real value"),
        "{error:?}"
    );
    println!("CV04_MAIN_NATIVE_REFUSE rounded-intermediate");
}


// Keep the old22 declaration/consumer verbatim through source(22); its legacy
// negative test name remains above while its quantitative boundary becomes24.
#[test]
fn current_cap23_preserves_original_twenty_two_and_new_twenty_three_integration_sources() {
    use algol_iir_compiler::Operand;
    for units in [22,23] {
        let text=source(units);
        let module=compile_source(&text,"integration").unwrap_or_else(|error|panic!("{text}: {error:?}"));
        let instructions=&module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i|i.op=="call" && i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1,"{text}");
        assert_eq!(instructions.iter().filter(|i|i.op=="call_builtin" && i.srcs.first().and_then(Operand::as_var)==Some("print_i64")).count(),0,"{text}");
        println!("CV04_MAIN_NATIVE_EXACT_ADMITTED {units}");
    }
}
