//! Owned programs distinguish actual output from a crash after printing it.
//! `saved` avoids the conventional `result` exit-value behavior deliberately
//! supported by the general matrix. Each source needs real execution.
use super::*;
mod return_observer;

pub(super) fn observe_engine_return(backend: Backend,source: &'static str,value: Option<&vm_core::value::Value>) {
    return_observer::observe(backend,source,value);
}

#[test]
fn native18_seventeen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 17); output(saved) end","16"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 17); output(saved) end","-18"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 17); output(saved) end","17"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 17); output(saved) end","-17"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 17); output(saved) end","18"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 17); output(saved) end","-16"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 16)); output(saved) end","16"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 16); output(saved) end","18"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 17) else ((entier(sign(rightvalue())) + 1) + 16))); output(saved) end","CL16"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 17) else ((entier(sign(rightvalue())) + 1) + 16))); output(saved) end","CR18"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but seventeen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"seventeen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"seventeen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("seventeen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("seventeen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"seventeen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn native18_eighteen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 18); output(saved) end","17"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 18); output(saved) end","-19"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 18); output(saved) end","18"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 18); output(saved) end","-18"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 18); output(saved) end","19"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 18); output(saved) end","-17"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 17)); output(saved) end","17"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 17); output(saved) end","19"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 18) else ((entier(sign(rightvalue())) + 1) + 17))); output(saved) end","CL17"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 18) else ((entier(sign(rightvalue())) + 1) + 17))); output(saved) end","CR19"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but eighteen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"eighteen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"eighteen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("eighteen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("eighteen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"eighteen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn sixteen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 16); output(saved) end","15"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 16); output(saved) end","-17"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 16); output(saved) end","16"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 16); output(saved) end","-16"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 16); output(saved) end","17"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 16); output(saved) end","-15"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 15)); output(saved) end","15"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 15); output(saved) end","17"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 16) else ((entier(sign(rightvalue())) + 1) + 15))); output(saved) end","CL15"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 16) else ((entier(sign(rightvalue())) + 1) + 15))); output(saved) end","CR17"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but sixteen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"sixteen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"sixteen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("sixteen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("sixteen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"sixteen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn fifteen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 15); output(saved) end","14"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 15); output(saved) end","-16"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 15); output(saved) end","15"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 15); output(saved) end","-15"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 15); output(saved) end","16"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 15); output(saved) end","-14"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 14)); output(saved) end","14"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 14); output(saved) end","16"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 15) else ((entier(sign(rightvalue())) + 1) + 14))); output(saved) end","CL14"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 15) else ((entier(sign(rightvalue())) + 1) + 14))); output(saved) end","CR16"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but fifteen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"fifteen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"fifteen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("fifteen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("fifteen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"fifteen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn fourteen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 14); output(saved) end","13"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 14); output(saved) end","-15"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 14); output(saved) end","14"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 14); output(saved) end","-14"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 14); output(saved) end","15"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 14); output(saved) end","-13"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 13)); output(saved) end","13"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 13); output(saved) end","15"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 14) else ((entier(sign(rightvalue())) + 1) + 13))); output(saved) end","CL13"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 14) else ((entier(sign(rightvalue())) + 1) + 13))); output(saved) end","CR15"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but fourteen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"fourteen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"fourteen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("fourteen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("fourteen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"fourteen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn thirteen_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 13); output(saved) end","12"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 13); output(saved) end","-14"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 13); output(saved) end","13"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 13); output(saved) end","-13"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 13); output(saved) end","14"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 13); output(saved) end","-12"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 12)); output(saved) end","12"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 12); output(saved) end","14"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 13) else ((entier(sign(rightvalue())) + 1) + 12))); output(saved) end","CL12"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 13) else ((entier(sign(rightvalue())) + 1) + 12))); output(saved) end","CR14"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but thirteen-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"thirteen-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"thirteen-unit{case} {backend:?}");},RunResult::Trapped=>panic!("thirteen-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("thirteen-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"thirteen-unit case{case} needs actual backend execution");
    }
}

#[test]
fn twelve_unit_offsets_preserve_exact_stdout_normal_exit_and_selected_order() {
    for (case,(source,expected)) in [
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 12); output(saved) end","11"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) - 12); output(saved) end","-13"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) + 12); output(saved) end","12"),
        ("begin real procedure pick; pick := 0.0; real saved; saved := entier(sign(pick()) - 12); output(saved) end","-12"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 12); output(saved) end","13"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) - 12); output(saved) end","-11"),
        ("begin real procedure pick; pick := -2.25; real saved; saved := entier(abs((sign(pick()) + 1) + 11)); output(saved) end","11"),
        ("begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 11); output(saved) end","13"),
        ("begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 12) else ((entier(sign(rightvalue())) + 1) + 11))); output(saved) end","CL11"),
        ("begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 12) else ((entier(sign(rightvalue())) + 1) + 11))); output(saved) end","CR13"),
    ].into_iter().enumerate() {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but twelve-unit case{case} did not execute");continue;};
            match &result {RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"twelve-unit{case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"twelve-unit{case} {backend:?}");},RunResult::Trapped=>panic!("twelve-unit{case} {backend:?} trapped")}
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("twelve-unit EXECUTED case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"twelve-unit case{case} needs actual backend execution");
    }
}

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

/// The integration boundary needs the same Exit0 and evaluation-order contract
/// as the owned lower offsets. A backend returning output and then trapping is
/// a failed cell; an unavailable toolchain is never counted as execution.
#[test]
fn current_main_nineteen_through_twenty_one_normal_exit() {
    // Prog and its expected stdout deliberately borrow immutable fixture text.
    // Materialize these24 bounded source pairs instead of leaking dynamic text
    // or weakening the shared harness's static lifetime and execution contract.
    const CASES:&[(usize,usize,&str,&str)]=&[
        (19,0,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 19); output(saved) end","P18"),
        (19,1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 19); output(saved) end","P-20"),
        (19,2,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 19); output(saved) end","P19"),
        (19,3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 19); output(saved) end","P-19"),
        (19,4,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 19); output(saved) end","P20"),
        (19,5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 19); output(saved) end","P-18"),
        (19,6,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 19) else ((entier(sign(rightvalue())) + 1) + 18))); output(saved) end","CL18"),
        (19,7,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 19) else ((entier(sign(rightvalue())) + 1) + 18))); output(saved) end","CR20"),
        (20,0,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 20); output(saved) end","P19"),
        (20,1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 20); output(saved) end","P-21"),
        (20,2,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 20); output(saved) end","P20"),
        (20,3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 20); output(saved) end","P-20"),
        (20,4,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 20); output(saved) end","P21"),
        (20,5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 20); output(saved) end","P-19"),
        (20,6,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 20) else ((entier(sign(rightvalue())) + 1) + 19))); output(saved) end","CL19"),
        (20,7,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 20) else ((entier(sign(rightvalue())) + 1) + 19))); output(saved) end","CR21"),
        (21,0,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 21); output(saved) end","P20"),
        (21,1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 21); output(saved) end","P-22"),
        (21,2,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 21); output(saved) end","P21"),
        (21,3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 21); output(saved) end","P-21"),
        (21,4,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 21); output(saved) end","P22"),
        (21,5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 21); output(saved) end","P-20"),
        (21,6,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 21) else ((entier(sign(rightvalue())) + 1) + 20))); output(saved) end","CL20"),
        (21,7,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 21) else ((entier(sign(rightvalue())) + 1) + 20))); output(saved) end","CR22"),
    ];
    for &(offset,case,source,expected) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but offset={offset} case={case} did not execute");continue;};
            match &result {
                RunResult::Completed{code,stdout}=>{assert_eq!(*code,Some(0),"offset={offset} case={case} {backend:?}: {stdout:?}");assert_eq!(stdout.trim(),expected,"offset={offset} case={case} {backend:?}");},
                RunResult::Trapped=>panic!("offset={offset} case={case} {backend:?} trapped")
            }
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("current-main EXECUTED offset={offset} case={case} backend={backend:?} expected={expected} Exit0");
        }
        assert!(completed>0,"offset={offset} case={case} needs actual backend execution");
    }
}


/// Exact cap23 admission sources supplement the unchanged saved-only19..21
/// matrix. Each marker follows both the status and full output assertions.
#[test]
fn current_cap23_exact_sources_preserve_stdout_and_completed_exit_contract() {
    const CASES:&[(usize,&str,&str,Option<i64>)]=&[
        (1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 22); output(saved) end","P21",None),
        (2,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 22); output(saved) end","P-23",None),
        (3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 22); output(saved) end","P22",None),
        (4,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 22); output(saved) end","P-22",None),
        (5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 22); output(saved) end","P23",None),
        (6,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 22); output(saved) end","P-21",None),
        (7,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 22) else ((entier(sign(rightvalue())) + 1) + 21))); output(saved) end","CL21",None),
        (8,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 22) else ((entier(sign(rightvalue())) + 1) + 21))); output(saved) end","CR23",None),
        (9,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 23); output(saved) end","P22",None),
        (10,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 23); output(saved) end","P-24",None),
        (11,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 23); output(saved) end","P23",None),
        (12,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 23); output(saved) end","P-23",None),
        (13,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 23); output(saved) end","P24",None),
        (14,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 23); output(saved) end","P-22",None),
        (15,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 23) else ((entier(sign(rightvalue())) + 1) + 22))); output(saved) end","CL22",None),
        (16,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 23) else ((entier(sign(rightvalue())) + 1) + 22))); output(saved) end","CR24",None),
        (17,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 22); output(result) end","21",Some(21)),
        (18,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 21 + 1); output(result) end","21",Some(21)),
        (19,"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 21) + 1); output(result) end","21",Some(21)),
        (20,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 11) + 11)); output(result) end","21",Some(21)),
        (21,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 23)) else relay(entier(entier(sign(right)) + 23)); output(result) end","24",Some(24)),
        (22,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 22 + 1)) else relay(entier(entier(sign(right)) + 1 + 22)); output(result) end","24",Some(24)),
        (23,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 23); output(result) end","22",Some(22)),
        (24,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 22 + 1); output(result) end","22",Some(22)),
        (25,"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 22) + 1); output(result) end","22",Some(22)),
        (26,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 11) + 12)); output(result) end","22",Some(22)),
        (27,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 22) else ((entier(sign(right)) + 11) + 11))); output(result) end","23",Some(23)),
        (28,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 23) else ((entier(sign(right)) + 11) + 12))); output(result) end","24",Some(24)),
        (29,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := if choose() then relay(entier(entier(sign(left)) + 22)) else relay(entier(entier(sign(right)) + 22)); output(result) end","21",Some(21)),
    ];
    for &(case,source,expected,real_return) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let observation=real_return.filter(|_|matches!(backend,Vm|Jit))
                .map(|_|return_observer::Observation::arm(source,backend,case));
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but cap23 case={case} did not execute");continue;};
            let (actual_code,typed_bits)=match &result {
                RunResult::Completed{code,stdout}=>{
                    let expected_code=match (real_return,backend) {
                        (Some(_),Vm|Jit)=>None,
                        (Some(value),Wasm|Clr)=>Some((value as i32)&0xff),
                        _=>Some(0),
                    };
                    assert_eq!(*code,expected_code,"cap23 case={case} {backend:?}: {stdout:?}");
                    assert_eq!(stdout.trim(),expected,"cap23 case={case} {backend:?}");
                    let bits=observation.as_ref().map(|guard|guard.assert_float(real_return.expect("real guard") as f64));
                    (*code,bits)
                },
                RunResult::Trapped=>panic!("cap23 case={case} {backend:?} trapped")
            };
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("cap23-runtime EXECUTED case={case} backend={backend:?} expected={expected} code={actual_code:?} typed_bits={typed_bits:?}");
        }
        assert!(completed>0,"cap23 case={case} needs actual backend execution");
    }
}


#[test]
fn cap23_real_return_observer_rejects_invalid_captures_and_cleans_up() {
    use vm_core::value::Value;
    use std::panic::{catch_unwind,AssertUnwindSafe};
    // A missing, absent, wrong-typed, wrong-valued or duplicate return cannot
    // satisfy typed observation merely because stdout/code have the right shape.
    for (value,visits) in [(None,0),(None,1),(Some(Value::Int(21)),1),
        (Some(Value::Float(22.0)),1),(Some(Value::Float(21.0)),2)] {
        let observation=return_observer::Observation::arm("probe",Vm,17);
        for _ in 0..visits {return_observer::observe(Vm,"probe",value.as_ref());}
        assert!(catch_unwind(AssertUnwindSafe(||observation.assert_float(21.0))).is_err());
        drop(observation);
    }
    for (backend,source) in [(Jit,"probe"),(Vm,"stale-source")] {
        let observation=return_observer::Observation::arm("probe",Vm,17);
        assert!(catch_unwind(AssertUnwindSafe(||return_observer::observe(backend,source,Some(&Value::Float(21.0))))).is_err());
        drop(observation);
    }
    // Unarmed generic runs are inert; every valid guard starts fresh, and panic
    // unwinding drops its state instead of making it available to another case.
    return_observer::observe(Vm,"unarmed",Some(&Value::Float(99.0)));
    assert!(catch_unwind(|| {
        let _observation=return_observer::Observation::arm("probe",Vm,17);
        return_observer::observe(Vm,"probe",Some(&Value::Float(21.0)));
        panic!("simulated failed cell");
    }).is_err());
    for backend in [Vm,Jit] {
        let observation=return_observer::Observation::arm("fresh",backend,18);
        return_observer::observe(backend,"fresh",Some(&Value::Float(24.0)));
        assert_eq!(observation.assert_float(24.0),24.0_f64.to_bits());
    }
}

/// Dedicated24 cases supplement, rather than replace, all prior29 exact programs.
/// Runtime tuples require real completion, exact effect/output and backend-specific
/// typed-return/status observations; unavailable backends are never counted.
#[test]
fn current_cap24_exact_sources_preserve_stdout_and_completed_exit_contract() {
    const CASES:&[(usize,&str,&str,Option<i64>)]=&[
        (1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 24); output(saved) end","P23",None),
        (2,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 24); output(saved) end","P-25",None),
        (3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 24); output(saved) end","P24",None),
        (4,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 24); output(saved) end","P-24",None),
        (5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 24); output(saved) end","P25",None),
        (6,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 24); output(saved) end","P-23",None),
        (7,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 24) else ((entier(sign(rightvalue())) + 12) + 12))); output(saved) end","CL23",None),
        (8,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 24) else ((entier(sign(rightvalue())) + 12) + 12))); output(saved) end","CR25",None),
        (9,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 24)) else relay(entier(entier(sign(right)) + 24)); output(result) end","25",Some(25)),
        (10,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 23 + 1)) else relay(entier(entier(sign(right)) + 1 + 23)); output(result) end","25",Some(25)),
        (11,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 24); output(result) end","23",Some(23)),
        (12,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 23 + 1); output(result) end","23",Some(23)),
        (13,"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 23) + 1); output(result) end","23",Some(23)),
        (14,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 12) + 12)); output(result) end","23",Some(23)),
        (15,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 24) else ((entier(sign(right)) + 12) + 12))); output(result) end","25",Some(25)),
    ];
    for &(case,source,expected,real_return) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let observation=real_return.filter(|_|matches!(backend,Vm|Jit))
                .map(|_|return_observer::Observation::arm(source,backend,case));
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but cap24 case={case} did not execute");continue;};
            let (actual_code,typed_bits)=match &result {
                RunResult::Completed{code,stdout}=>{
                    let expected_code=match (real_return,backend) {
                        (Some(_),Vm|Jit)=>None,
                        (Some(value),Wasm|Clr)=>Some((value as i32)&0xff),
                        _=>Some(0),
                    };
                    assert_eq!(*code,expected_code,"cap24 case={case} {backend:?}: {stdout:?}");
                    assert_eq!(stdout.trim(),expected,"cap24 case={case} {backend:?}");
                    let bits=observation.as_ref().map(|guard|guard.assert_float(real_return.expect("real guard") as f64));
                    (*code,bits)
                },
                RunResult::Trapped=>panic!("cap24 case={case} {backend:?} trapped")
            };
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("cap24-runtime EXECUTED case={case} backend={backend:?} expected={expected} code={actual_code:?} typed_bits={typed_bits:?}");
        }
        assert!(completed>0,"cap24 case={case} needs actual backend execution");
    }
}

// No backend tuple is invented for this unsupported25-unit output boundary.
#[test]
fn current_cap24_unsupported25_remains_a_compile_refusal() {
    let source = "begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 33); output(saved) end";
    let error = lang_aot::compile_source_to_iir(Language::Algol60, source, "native24_refusal").expect_err("unsupported26 is a compile refusal");
    assert!(format!("{error:?}").contains("cannot print a real value"), "{source}: {error:?}");
    eprintln!("cap24-runtime COMPILE_REFUSED units=33");
}

/// Dedicated25 cases supplement the complete retained44-program module.
/// Runtime tuples require real completion, exact effect/output and backend-specific
/// typed-return/status observations; unavailable backends are never counted.
#[test]
fn current_cap25_exact_sources_preserve_stdout_and_completed_exit_contract() {
    const CASES:&[(usize,&str,&str,Option<i64>)]=&[
        (1,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 25); output(saved) end","P24",None),
        (2,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) - 25); output(saved) end","P-26",None),
        (3,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) + 25); output(saved) end","P25",None),
        (4,"begin real procedure pick; begin output('P'); pick := 0.0 end; real saved; saved := entier(sign(pick()) - 25); output(saved) end","P-25",None),
        (5,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) + 25); output(saved) end","P26",None),
        (6,"begin real procedure pick; begin output('P'); pick := 3.5 end; real saved; saved := entier(sign(pick()) - 25); output(saved) end","P-24",None),
        (7,"begin boolean procedure choose; begin output('C'); choose := true end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 25) else ((entier(sign(rightvalue())) + 12) + 13))); output(saved) end","CL24",None),
        (8,"begin boolean procedure choose; begin output('C'); choose := false end; real procedure leftvalue; begin output('L'); leftvalue := -2.25 end; real procedure rightvalue; begin output('R'); rightvalue := 3.5 end; real procedure relay(x); real x; relay := x; real saved; saved := relay(entier(if choose() then (entier(sign(leftvalue())) + 25) else ((entier(sign(rightvalue())) + 12) + 13))); output(saved) end","CR26",None),
        (9,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 25)) else relay(entier(entier(sign(right)) + 25)); output(result) end","26",Some(26)),
        (10,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 24 + 1)) else relay(entier(entier(sign(right)) + 1 + 24)); output(result) end","26",Some(26)),
        (11,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 25); output(result) end","24",Some(24)),
        (12,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 24 + 1); output(result) end","24",Some(24)),
        (13,"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 24) + 1); output(result) end","24",Some(24)),
        (14,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 12) + 13)); output(result) end","24",Some(24)),
        (15,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 25) else ((entier(sign(right)) + 12) + 13))); output(result) end","26",Some(26)),
    ];
    for &(case,source,expected,real_return) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let observation=real_return.filter(|_|matches!(backend,Vm|Jit))
                .map(|_|return_observer::Observation::arm(source,backend,case));
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but cap25 case={case} did not execute");continue;};
            let (actual_code,typed_bits)=match &result {
                RunResult::Completed{code,stdout}=>{
                    let expected_code=match (real_return,backend) {
                        (Some(_),Vm|Jit)=>None,
                        (Some(value),Wasm|Clr)=>Some((value as i32)&0xff),
                        _=>Some(0),
                    };
                    assert_eq!(*code,expected_code,"cap25 case={case} {backend:?}: {stdout:?}");
                    assert_eq!(stdout.trim(),expected,"cap25 case={case} {backend:?}");
                    let bits=observation.as_ref().map(|guard|guard.assert_float(real_return.expect("real guard") as f64));
                    (*code,bits)
                },
                RunResult::Trapped=>panic!("cap25 case={case} {backend:?} trapped")
            };
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("cap25-runtime EXECUTED case={case} backend={backend:?} expected={expected} code={actual_code:?} typed_bits={typed_bits:?}");
        }
        assert!(completed>0,"cap25 case={case} needs actual backend execution");
    }
}

/// Exact current-main26..31 sources and bounded effect/branch companions extend
/// the retained59 programs. Float VM/JIT returns use the existing observer;
/// saved-only Int0 cases retain completed status/output checks here and exact
/// typed native-VM unit coverage separately. Missing backends stay uncounted.
#[test]
fn current_cap31_exact_sources_preserve_stdout_and_completed_exit_contract() {
    const CASES:&[(usize,&str,&str,Option<i64>)]=&[
        (1,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 26) else ((entier(sign(right)) + 13) + 13))); output(result) end","27",Some(27)),
        (2,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 27) else ((entier(sign(right)) + 13) + 14))); output(result) end","28",Some(28)),
        (3,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 28) else ((entier(sign(right)) + 14) + 14))); output(result) end","29",Some(29)),
        (4,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 29) else ((entier(sign(right)) + 14) + 15))); output(result) end","30",Some(30)),
        (5,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 30) else ((entier(sign(right)) + 15) + 15))); output(result) end","31",Some(31)),
        (6,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 31) else ((entier(sign(right)) + 15) + 16))); output(result) end","32",Some(32)),
        (7,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 26); output(saved) end","P25",None),
        (8,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 31); output(saved) end","P30",None),
        (9,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 31) else (entier(sign(right)) + 1)); output(saved) end","30",None),
        (10,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 31)); output(saved) end","0",None),
        (11,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 31) else (entier(sign(right)) + 1)); output(saved) end","2",None),
        (12,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 31)); output(saved) end","32",None),
        (13,"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 23); output(saved) end","32",None),
        (14,"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 30); output(saved) end","32",None),
        (15,"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 30 - 1); output(saved) end","30",None),
    ];
    for &(case,source,expected,real_return) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let observation=real_return.filter(|_|matches!(backend,Vm|Jit))
                .map(|_|return_observer::Observation::arm(source,backend,case));
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but cap31 case={case} did not execute");continue;};
            let (actual_code,typed_bits)=match &result {
                RunResult::Completed{code,stdout}=>{
                    let expected_code=match (real_return,backend) {
                        (Some(_),Vm|Jit)=>None,
                        (Some(value),Wasm|Clr)=>Some((value as i32)&0xff),
                        _=>Some(0),
                    };
                    assert_eq!(*code,expected_code,"cap31 case={case} {backend:?}: {stdout:?}");
                    assert_eq!(stdout.trim(),expected,"cap31 case={case} {backend:?}");
                    let bits=observation.as_ref().map(|guard|guard.assert_float(real_return.expect("real guard") as f64));
                    (*code,bits)
                },
                RunResult::Trapped=>panic!("cap31 case={case} {backend:?} trapped")
            };
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("cap31-runtime EXECUTED case={case} backend={backend:?} expected={expected} code={actual_code:?} typed_bits={typed_bits:?}");
        }
        assert!(completed>0,"cap31 case={case} needs actual backend execution");
    }
}

/// All38 distinct sources in the native32 closed occurrence set are new.
/// Preserve the old74 programs; require ordered output, exact completion
/// contracts and VM/JIT Float bits. Native/LLVM typed-main ABI remains open.
#[test]
fn current_cap32_exact_sources_preserve_stdout_and_completed_exit_contract() {
    const CASES:&[(usize,&str,&str,Option<i64>)]=&[
        (1,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end","33",Some(33)),
        (2,"begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 31 + 1)) else relay(entier(entier(sign(right)) + 1 + 31)); output(result) end","33",Some(33)),
        (3,"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end","33",None),
        (4,"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end","33",None),
        (5,"begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end","33",None),
        (6,"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end","33",None),
        (7,"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end","33",None),
        (8,"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 11) + 21); output(saved) end","33",None),
        (9,"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 - 1); output(saved) end","31",None),
        (10,"begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 31) + 1)); output(saved) end","33",None),
        (11,"begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 31) + 1)); output(saved) end","33",None),
        (12,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end","31",None),
        (13,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end","0",None),
        (14,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end","2",None),
        (15,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end","33",None),
        (16,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end","31",None),
        (17,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end","0",None),
        (18,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 32) else (entier(sign(right)) + 1)); output(saved) end","2",None),
        (19,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 32)); output(saved) end","33",None),
        (20,"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 8 + 24); output(saved) end","33",None),
        (21,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 32); output(result) end","31",Some(31)),
        (22,"begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 31 + 1); output(result) end","31",Some(31)),
        (23,"begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 31) + 1); output(result) end","31",Some(31)),
        (24,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 12) + 20)); output(result) end","31",Some(31)),
        (25,"begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 32); output(saved) end","31",None),
        (26,"begin real procedure pick; pick := -2.25; real saved; saved := entier((sign(pick()) + 12) + 20); output(saved) end","31",None),
        (27,"begin real procedure pick; pick := -2.25; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end","31",None),
        (28,"begin real procedure pick; pick := -2.25; real saved; saved := entier(abs(sign(pick()) + 12) + 20); output(saved) end","31",None),
        (29,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 32); output(saved) end","P31",None),
        (30,"begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := if choose() then relay(entier(entier(sign(left)) + 32)) else relay(entier(entier(sign(right)) + 32)); output(result) end","31",Some(31)),
        (31,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 13) + 19)); output(result) end","31",Some(31)),
        (32,"begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 31 + 1); output(saved) end","33",None),
        (33,"begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 31) + 1); output(saved) end","33",None),
        (34,"begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 16) + 16)); output(result) end","31",Some(31)),
        (35,"begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 32.0); output(saved) end","P31",None),
        (36,"begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := relay(entier(if choose() then (entier(sign(left)) + 32) else ((entier(sign(right)) + 16) + 16))); output(result) end","33",Some(33)),
        (37,"begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; output(relay(entier(entier(sign(pick())) + 32))) end","31",None),
        (38,"begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; output(relay(entier(abs((sign(pick()) + 16) + 16)))) end","31",None),
    ];
    for &(case,source,expected,real_return) in CASES {
        let program=Prog{lang:Language::Algol60,ext:"alg",src:source,expect:Expect::Stdout(expected),backends:&[NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit]};
        let mut completed=0;
        for backend in [NativeAot,Llvm,Wasm,Jvm,Clr,Vm,Jit] {
            let available=toolchain_available(backend);
            let observation=real_return.filter(|_|matches!(backend,Vm|Jit))
                .map(|_|return_observer::Observation::arm(source,backend,case));
            let Some(result)=run(backend,&program) else {assert!(!available,"{backend:?} present but cap32 case={case} did not execute");continue;};
            let (actual_code,typed_bits)=match &result {
                RunResult::Completed{code,stdout}=>{
                    let expected_code=match (real_return,backend) {
                        (Some(_),Vm|Jit)=>None,
                        (Some(value),Wasm|Clr)=>Some((value as i32)&0xff),
                        _=>Some(0),
                    };
                    assert_eq!(*code,expected_code,"cap32 case={case} {backend:?}: {stdout:?}");
                    assert_eq!(stdout.trim(),expected,"cap32 case={case} {backend:?}");
                    let bits=observation.as_ref().map(|guard|guard.assert_float(real_return.expect("real guard") as f64));
                    (*code,bits)
                },
                RunResult::Trapped=>panic!("cap32 case={case} {backend:?} trapped")
            };
            assert_cell(backend,&program,result);completed+=1;
            eprintln!("cap32-runtime EXECUTED case={case} backend={backend:?} expected={expected} code={actual_code:?} typed_bits={typed_bits:?}");
        }
        assert!(completed>0,"cap32 case={case} needs actual backend execution");
    }
}
