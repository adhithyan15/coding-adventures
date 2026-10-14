//! Owned programs distinguish actual output from a crash after printing it.
//! `saved` avoids the conventional `result` exit-value behavior deliberately
//! supported by the general matrix. Each source needs real execution.
use super::*;

#[test]
fn single_unit_offsets_preserve_exact_stdout_and_normal_exit() {
    for (case, (source, expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 1); output(saved) end", "0"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 1); output(saved) end", "-2"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 1); output(saved) end", "1"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 1); output(saved) end", "-1"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 1); output(saved) end", "2"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 1); output(saved) end", "0"),
        ("begin real procedure pick; pick := 3.5; real local, saved; local := pick(); saved := entier(1 - sign(local)); output(saved) end", "0"),
        ("begin real procedure pick; pick := 3.5; real local, saved; local := pick(); saved := entier(-(sign(local) + 1)); output(saved) end", "-2"),
        ("begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := if choose() then relay(entier(entier(sign(left)) + 1)) else relay(entier(entier(sign(right)) + 1)); output(saved) end", "0"),
        ("begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := if choose() then relay(entier(entier(sign(left)) + 1)) else relay(entier(entier(sign(right)) + 1)); output(saved) end", "2"),
        ("begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := if choose() then relay(entier(entier(sign(left)) - 1)) else relay(entier(entier(sign(right)) - 1)); output(saved) end", "-2"),
        ("begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := if choose() then relay(entier(entier(sign(left)) - 1)) else relay(entier(entier(sign(right)) - 1)); output(saved) end", "0"),
        ("begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 1))); output(saved) end", "0"),
        ("begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 1))); output(saved) end", "2"),
    ].into_iter().enumerate() {
        let program = Prog { lang: Language::Algol60, ext: "alg", src: source,
            expect: Expect::Stdout(expected), backends: &[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] };
        let mut completed = 0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available = toolchain_available(backend);
            let Some(result) = run(backend,&program) else {
                assert!(!available,"{backend:?} present but unit-offset source {case} did not execute");
                continue;
            };
            match &result {
                RunResult::Completed { code, stdout } => assert_eq!(*code,Some(0),
                    "unit-offset {case} {backend:?} must exit normally; stdout={stdout:?}; source={source}"),
                RunResult::Trapped => panic!("unit-offset {case} {backend:?} trapped: {source}"),
            }
            assert_cell(backend,&program,result);
            completed += 1;
            eprintln!("unit-offset EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"unit-offset source {case} needs at least one actual backend execution");
    }
}
