//! Tests precede the four dedicated cap32 edits. Old source assertions stay exact.
//! Complete original occurrence identities and independent mathematical oracles
//! come from the reviewed native32 precontract; duplicates remain distinct rows.
use super::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum ExpectedMain {
    Float(u64),
    Int(i64),
}

// A compiled cap31 refusal is collected instead of stopping the first row:
// the actual red log identifies every attempted whole program. Green paths
// inspect the real formatter call, execute the VM and compare complete bytes.
fn run_native32_cases(cases: &[(&str, &str, &str, ExpectedMain)]) {
    let mut refused = Vec::new();
    for &(id, source, expected_stdout, expected_main) in cases {
        let mut module = match compile_source(source, "native32_exact") {
            Ok(module) => module,
            Err(CompileError::Type(message)) if message.contains("cannot print a real value") => {
                eprintln!("native32-exact REFUSED occurrence={id}: {message}");
                refused.push(id);
                continue;
            }
            Err(error) => panic!("{id}: unexpected source failure: {error:?}: {source}"),
        };
        let main = module.get_function("main").expect("source main");
        assert_eq!(main.instructions.iter().filter(|i| i.op == "call"
            && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1,
            "{id}: real formatter: {source}");
        assert_eq!(main.instructions.iter().filter(|i| i.op == "call_builtin"
            && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0,
            "{id}: no integer formatter substitution: {source}");
        let output = Arc::new(Mutex::new(Vec::<u8>::new()));
        let mut vm = VMCore::new();
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("putchar", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let byte = u8::try_from(args[0].as_i64().expect("formatter integer")).expect("formatter byte");
            let mut bytes = captured.lock().expect("formatter buffer");
            assert!(bytes.len() < 32, "bounded declared formatter output");
            bytes.push(byte);
            Ok(Value::Null)
        });
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("print_i64", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let text = args[0].as_i64().expect("formatter integer").to_string();
            let mut bytes = captured.lock().expect("formatter buffer");
            assert!(bytes.len().checked_add(text.len()).is_some_and(|n| n <= 32),
                "bounded declared formatter output");
            bytes.extend_from_slice(text.as_bytes());
            Ok(Value::Null)
        });
        // P and every numeric formatter byte share one ordered capture.
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("print_str", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let text = args[0].as_str().expect("literal string effect");
            let mut bytes = captured.lock().expect("formatter buffer");
            assert!(bytes.len().checked_add(text.len()).is_some_and(|n| n <= 32),
                "bounded declared formatter output");
            bytes.extend_from_slice(text.as_bytes());
            Ok(Value::Null)
        });
        let actual = vm.execute(&mut module, "main", &[])
            .unwrap_or_else(|error| panic!("{id}: {source}: {error:?}"));
        match (actual, expected_main) {
            (Some(Value::Float(actual)), ExpectedMain::Float(bits)) =>
                assert_eq!(actual.to_bits(), bits, "{id}: exact Float main: {source}"),
            (Some(Value::Int(actual)), ExpectedMain::Int(expected)) =>
                assert_eq!(actual, expected, "{id}: exact Int main: {source}"),
            (actual, ExpectedMain::Float(bits)) =>
                panic!("{id}: typed main {actual:?}; expected Float bits {bits:016x}"),
            (actual, ExpectedMain::Int(expected)) =>
                panic!("{id}: typed main {actual:?}; expected Int {expected}"),
        }
        assert_eq!(*output.lock().expect("formatter buffer"), expected_stdout.as_bytes(),
            "{id}: complete ordered formatter/effect bytes: {source}");
        eprintln!("native32-exact COMPLETE occurrence={id}");
    }
    eprintln!("native32-exact ATTEMPTED={} REFUSED={}", cases.len(), refused.len());
    assert!(refused.is_empty(), "native32 complete programs remain refused: {refused:?}");
}

#[test]
fn native32_exact_original_quantitative_occurrences() {
    run_native32_cases(&[
        (r#"old-26"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"old-27"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 31 + 1)) else relay(entier(entier(sign(right)) + 1 + 31)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"old-29"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"old-30"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 31 + 1)) else relay(entier(entier(sign(right)) + 1 + 31)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"old-32"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-33"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-34"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-35"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-36"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-47"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-48"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 11) + 21); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-49"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-50"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-51"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-76"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-77"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-78"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-82"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-83"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-84"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-85"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-86"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-87"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-88"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-89"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-90"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-102"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-103"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-104"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-105"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-106"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-117"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-118"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-119"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-123"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-124"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-125"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-126"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-127"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-128"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-129"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-130"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-131"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-142"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-143"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-144"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-145"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-146"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-157"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-158"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-159"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-160"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-161"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-172"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-173"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-174"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-178"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-179"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-180"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-181"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-182"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-183"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-184"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-185"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-186"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-197"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-198"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-199"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-200"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-201"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-212"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-213"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-214"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-215"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-216"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-227"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-228"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-229"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-230"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-231"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-242"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-243"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-244"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-248"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-249"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-250"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-251"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-252"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-253"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-254"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-255"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-256"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-267"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-268"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-269"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-270"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-271"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-282"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-283"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-284"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-288"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-289"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-290"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-291"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-292"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-293"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-294"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-295"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-296"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-307"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-308"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-309"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-310"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-311"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-323"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-324"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-325"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-326"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-336"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"old-337"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"0"#, ExpectedMain::Int(0)),
        (r#"old-339"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end"#, r#"2"#, ExpectedMain::Int(0)),
        (r#"old-340"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-341"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-342"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-343"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-344"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-346"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 8 + 24); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"old-347"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"main-additive-0"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 32); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"main-additive-1"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 31 + 1); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"main-additive-2"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 31) + 1); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"main-additive-3"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 12) + 20)); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"canonical_twenty_four_units_stay_outside_the_dedicated_budget-0"#, r#"begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"canonical_twenty_four_units_stay_outside_the_dedicated_budget-1"#, r#"begin real procedure pick; pick := -2.25; real saved; saved := entier((sign(pick()) + 12) + 20); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"canonical_twenty_four_units_stay_outside_the_dedicated_budget-2"#, r#"begin real procedure pick; pick := -2.25; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"canonical_twenty_four_units_stay_outside_the_dedicated_budget-3"#, r#"begin real procedure pick; pick := -2.25; real saved; saved := entier(abs(sign(pick()) + 12) + 20); output(saved) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"runtime-compile25"#, r#"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"P31"#, ExpectedMain::Int(0)),
        (r#"integration24"#, r#"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"native25-raw-1"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"native25-raw-2"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 31 + 1)) else relay(entier(entier(sign(right)) + 1 + 31)); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"native25-raw-4"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 32); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"native25-raw-5"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 31 + 1); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"native25-raw-6"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 31) + 1); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"native25-raw-7"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 13) + 19)); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
        (r#"native31-guard-0"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"native31-guard-1"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 + 1); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"native31-guard-2"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 31) + 1); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"native31-guard-3"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
        (r#"native31-guard-4"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end"#, r#"33"#, ExpectedMain::Int(0)),
    ]);
}

#[test]
fn native32_exact_missing_incoming_positives() {
    run_native32_cases(&[
        (r#"raw-positive-1"#, r#"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 32) else ((entier(sign(right)) + 16) + 16))); output(result) end"#, r#"33"#, ExpectedMain::Float(0x4040800000000000)),
        (r#"raw-positive-2"#, r#"begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; output(relay(entier(entier(sign(pick())) + 32))) end"#, r#"31"#, ExpectedMain::Int(0)),
        (r#"raw-positive-3"#, r#"begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; output(relay(entier(abs((sign(pick()) + 16) + 16)))) end"#, r#"31"#, ExpectedMain::Int(0)),
    ]);
}

#[test]
fn native32_exact_raw_abs_frame() {
    run_native32_cases(&[
        (r#"raw-missing-abs16-plus16"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 16) + 16)); output(result) end"#, r#"31"#, ExpectedMain::Float(0x403f000000000000)),
    ]);
}

#[test]
fn native32_canonical_real_read_once() {
    run_native32_cases(&[
        (r#"canonical-real32-read-once"#, r#"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 32.0); output(saved) end"#, r#"P31"#, ExpectedMain::Int(0)),
    ]);
}

// These controls check actual compiler refusal, including precision and structure.
#[test]
fn native32_rounding_shared_and_structural_refusals_remain_exact() {
    for (id, source) in [
        (r#"guard-0"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 33); output(saved) end"#),
        (r#"guard-1"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32 + 1); output(saved) end"#),
        (r#"guard-2"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 32) + 1); output(saved) end"#),
        (r#"guard-3"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 32) + 1)); output(saved) end"#),
        (r#"guard-4"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 32) + 1)); output(saved) end"#),
        (r#"guard-5"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 33 - 33); output(saved) end"#),
        (r#"guard-6"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32 - 32); output(saved) end"#),
        (r#"guard-7"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992 + 32)); output(saved) end"#),
        (r#"guard-8"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(((9007199254740992 + 1) - 9007199254740992 + 32) + sign(pick())); output(saved) end"#),
        (r#"guard-9"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(sign(pick()) + 32)))); output(saved) end"#),
        (r#"guard-10"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((if true then (sign(pick()) + 32) else (sign(pick()) + 32)) + 1); output(saved) end"#),
        (r#"guard-11"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier((if true then (sign(pick()) + 31) else (sign(pick()) + 31)) + 1); output(saved) end"#),
        (r#"guard-12"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 33.0); output(saved) end"#),
        (r#"guard-13"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 33); output(saved) end"#),
        (r#"guard-14"#, r#"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) * 32); output(saved) end"#),
        (r#"raw-structural31-plus1"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := relay(entier((if choose() then (entier(sign(left)) + 31) else (entier(sign(right)) + 31)) + 1)); output(result) end"#),
        (r#"raw-structural32-plus1"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := relay(entier((if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 32)) + 1)); output(result) end"#),
        (r#"raw-abs16-plus17"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 16) + 17)); output(result) end"#),
        (r#"raw-replacement-0"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 33)) else relay(entier(entier(sign(right)) + 33)); output(result) end"#),
        (r#"raw-replacement-1"#, r#"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32 + 1)) else relay(entier(entier(sign(right)) + 1 + 32)); output(result) end"#),
        (r#"raw-replacement-3"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 33); output(result) end"#),
        (r#"raw-replacement-4"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 32 + 1); output(result) end"#),
        (r#"raw-replacement-5"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 32) + 1); output(result) end"#),
        (r#"raw-replacement-6"#, r#"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 16) + 17)); output(result) end"#),
    ] {
        assert!(matches!(compile_source(source, "native32_guard"),
            Err(CompileError::Type(message)) if message.contains("cannot print a real value")),
            "{id}: exact typed formatter refusal: {source}");
        eprintln!("native32-guard REFUSED occurrence={id}");
    }
}
