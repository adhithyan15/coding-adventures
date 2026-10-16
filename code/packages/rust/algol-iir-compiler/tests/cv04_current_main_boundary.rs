//! Integration boundary committed before current-main conflict resolution.
//! Main extends finite sign offsets through21; this preserves old17/18 cases
//! and tests the new22 boundary without relaxing malformed-actual controls.
use algol_iir_compiler::compile_source;

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
fn current_main_twenty_two_units_still_refuses_before_lowering() {
    let error = compile_source(&source(22), "integration").unwrap_err();
    assert!(error.to_string().contains("unsupported"), "{error:?}");
    println!("CV04_MAIN_NATIVE_REFUSE 22");
}
