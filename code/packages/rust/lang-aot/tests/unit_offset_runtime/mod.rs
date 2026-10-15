//! Owned programs distinguish actual output from a crash after printing it.
//! `saved` avoids the conventional `result` exit-value behavior deliberately
//! supported by the general matrix. Each source needs real execution.
use super::*;

#[test]
fn eleven_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 11); output(saved) end","10"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 11); output(saved) end","-12"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 11); output(saved) end","11"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 11); output(saved) end","-11"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 11); output(saved) end","12"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 11); output(saved) end","-10"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 10)); output(saved) end","10"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 10); output(saved) end","12"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 11) else ((entier(sign(rightvalue())) + 1) + 10))); output(saved) end","CL10"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 11) else ((entier(sign(rightvalue())) + 1) + 10))); output(saved) end","CR12"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but eleven-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"eleven-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"eleven-unit{case} {backend:?}");},RunResult::Trapped=>panic!("eleven-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("eleven-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"eleven-unit case{case} needs actual backend execution");
    }
}

#[test]
fn ten_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 10); output(saved) end","9"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 10); output(saved) end","-11"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 10); output(saved) end","10"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 10); output(saved) end","-10"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 10); output(saved) end","11"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 10); output(saved) end","-9"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 9)); output(saved) end","9"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 9); output(saved) end","11"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 10) else ((entier(sign(rightvalue())) + 1) + 9))); output(saved) end","CL9"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 10) else ((entier(sign(rightvalue())) + 1) + 9))); output(saved) end","CR11"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but ten-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"ten-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"ten-unit{case} {backend:?}");},RunResult::Trapped=>panic!("ten-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("ten-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"ten-unit case{case} needs actual backend execution");
    }
}

#[test]
fn nine_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 9); output(saved) end","8"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 9); output(saved) end","-10"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 9); output(saved) end","9"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 9); output(saved) end","-9"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 9); output(saved) end","10"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 9); output(saved) end","-8"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 8)); output(saved) end","8"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 8); output(saved) end","10"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 9) else ((entier(sign(rightvalue())) + 1) + 8))); output(saved) end","CL8"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 9) else ((entier(sign(rightvalue())) + 1) + 8))); output(saved) end","CR10"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but nine-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"nine-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"nine-unit{case} {backend:?}");},RunResult::Trapped=>panic!("nine-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("nine-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"nine-unit case{case} needs actual backend execution");
    }
}

#[test]
fn eight_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 8); output(saved) end","7"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 8); output(saved) end","-9"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 8); output(saved) end","8"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 8); output(saved) end","-8"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 8); output(saved) end","9"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 8); output(saved) end","-7"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 7)); output(saved) end","7"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 7); output(saved) end","9"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 8) else ((entier(sign(rightvalue())) + 1) + 7))); output(saved) end","CL7"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 8) else ((entier(sign(rightvalue())) + 1) + 7))); output(saved) end","CR9"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but eight-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"eight-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"eight-unit{case} {backend:?}");},RunResult::Trapped=>panic!("eight-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("eight-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"eight-unit case{case} needs actual backend execution");
    }
}

#[test]
fn seven_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 7); output(saved) end","6"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 7); output(saved) end","-8"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 7); output(saved) end","7"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 7); output(saved) end","-7"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 7); output(saved) end","8"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 7); output(saved) end","-6"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 6)); output(saved) end","6"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 6); output(saved) end","8"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 7) else ((entier(sign(rightvalue())) + 1) + 6))); output(saved) end","CL6"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 7) else ((entier(sign(rightvalue())) + 1) + 6))); output(saved) end","CR8"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but seven-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"seven-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"seven-unit{case} {backend:?}");},RunResult::Trapped=>panic!("seven-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("seven-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"seven-unit case{case} needs actual backend execution");
    }
}

#[test]
fn six_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 6); output(saved) end","5"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 6); output(saved) end","-7"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 6); output(saved) end","6"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 6); output(saved) end","-6"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 6); output(saved) end","7"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 6); output(saved) end","-5"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 5)); output(saved) end","5"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 5); output(saved) end","7"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 6) else ((entier(sign(rightvalue())) + 1) + 5))); output(saved) end","CL5"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 6) else ((entier(sign(rightvalue())) + 1) + 5))); output(saved) end","CR7"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but six-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"six-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"six-unit{case} {backend:?}");},RunResult::Trapped=>panic!("six-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("six-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"six-unit case{case} needs actual backend execution");
    }
}

#[test]
fn five_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 5); output(saved) end","4"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 5); output(saved) end","-6"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 5); output(saved) end","5"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 5); output(saved) end","-5"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 5); output(saved) end","6"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 5); output(saved) end","-4"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 4)); output(saved) end","4"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 4); output(saved) end","6"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 5) else ((entier(sign(rightvalue())) + 1) + 4))); output(saved) end","CL4"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 5) else ((entier(sign(rightvalue())) + 1) + 4))); output(saved) end","CR6"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but five-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"five-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"five-unit{case} {backend:?}");},RunResult::Trapped=>panic!("five-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("five-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"five-unit case{case} needs actual backend execution");
    }
}

#[test]
fn four_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 4); output(saved) end","3"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 4); output(saved) end","-5"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 4); output(saved) end","4"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 4); output(saved) end","-4"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 4); output(saved) end","5"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 4); output(saved) end","-3"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 3)); output(saved) end","3"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 3); output(saved) end","5"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 4) else ((entier(sign(rightvalue())) + 1) + 3))); output(saved) end","CL3"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 4) else ((entier(sign(rightvalue())) + 1) + 3))); output(saved) end","CR5"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but four-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"four-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"four-unit{case} {backend:?}");},RunResult::Trapped=>panic!("four-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("four-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"four-unit case{case} needs actual backend execution");
    }
}

#[test]
fn three_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 3); output(saved) end","2"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 3); output(saved) end","-4"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 3); output(saved) end","3"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 3); output(saved) end","-3"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 3); output(saved) end","4"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 3); output(saved) end","-2"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 2)); output(saved) end","2"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 2); output(saved) end","4"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 3) else ((entier(sign(rightvalue())) + 1) + 2))); output(saved) end","CL2"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 3) else ((entier(sign(rightvalue())) + 1) + 2))); output(saved) end","CR4"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but three-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"three-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"three-unit{case} {backend:?}");},RunResult::Trapped=>panic!("three-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("three-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"three-unit case{case} needs actual backend execution");
    }
}

#[test]
fn two_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 2); output(saved) end","1"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 2); output(saved) end","-3"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 2); output(saved) end","2"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 2); output(saved) end","-2"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 2); output(saved) end","3"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 2); output(saved) end","-1"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 1)); output(saved) end","1"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 1); output(saved) end","3"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 2) else ((entier(sign(rightvalue())) + 1) + 1))); output(saved) end","CL1"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 2) else ((entier(sign(rightvalue())) + 1) + 1))); output(saved) end","CR3"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but two-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"two-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"two-unit{case} {backend:?}");},RunResult::Trapped=>panic!("two-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("two-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"two-unit case{case} needs actual backend execution");
    }
}

#[test]
fn identity_conditional_evaluates_condition_and_selected_branch_once() {
    for (condition, source, expected) in [
        ("true", "begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 1) else (entier(sign(rightvalue())) + 1))); output(saved) end", "CL0"),
        ("false", "begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 1) else (entier(sign(rightvalue())) + 1))); output(saved) end", "CR2"),
    ] {
        let program = Prog { lang: Language::Algol60, ext: "alg", src: source,
            expect: Expect::Stdout(expected), backends: &[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] };
        let mut completed = 0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available = toolchain_available(backend);
            let Some(result) = run(backend, &program) else {
                assert!(!available, "{backend:?} present but identity order source did not execute");
                continue;
            };
            match &result {
                RunResult::Completed { code, stdout } => {
                    assert_eq!(*code, Some(0), "{backend:?} {condition}: {stdout:?}");
                    assert_eq!(stdout.trim(), expected, "exact ordered single-evaluation markers: {backend:?}");
                }
                RunResult::Trapped => panic!("{backend:?} identity order source trapped"),
            }
            assert_cell(backend, &program, result);
            completed += 1;
            eprintln!("identity-order EXECUTED condition={condition} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed > 0, "identity order source requires actual execution");
    }
}

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
