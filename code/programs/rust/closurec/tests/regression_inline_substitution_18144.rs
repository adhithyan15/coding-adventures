//! Regression test for #18144: ADVANCED inline substitution soundness:
//! - Closure-deferred reads of parameter identifiers must not be inlined when arguments are non-literal.
//! - Parameter capture in nested closures is safe to inline only when arguments are literals.
//! - Computed class method keys must be substituted in enclosing scope.
//! - ReturnStatement and other statements inside nested closures must be substituted.
//! - Nested declarations shadowing parameters must be preserved (not substituted).
//! - Parameters reassigned inside nested closures must not be inlined.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

const PROGRAMS: &[&str] = &[
    // 1. Closure-deferred read of argument variable:
    // If inlined, `f` would read `a` after `a = 2`, printing 2 instead of 1.
    "function make(x){return function(){return x}}var a=1;var f=make(a);a=2;console.log(f())",

    // 2. Arrow closure-deferred read:
    "function make(x){return ()=>x}var a=1;var f=make(a);a=2;console.log(f())",

    // 3. Class method deferred read:
    "function make(x){return class{m(){return x}}}var a=1;var c=make(a);a=2;console.log(new c().m())",

    // 4. Computed class method key:
    // If not substituted, `k` remains free and throws ReferenceError.
    "function make(k){return class{[k](){return 42}}}var c=make(\"foo\");console.log(new c().foo())",

    // 5. Nested closure return arm substitution:
    // ReturnStatement was previously omitted in `substitute_in_stmt`.
    "function make(x){return function(){return x}}console.log(make(42)())",

    // 6. Nested closure local variable shadowing:
    // Inner declaration must not be substituted.
    "function make(x){return function(){var x=99;return x}}console.log(make(1)())",
    "function make(x){return function(){let x=99;return x}}console.log(make(1)())",
    "function make(x){return function(){const x=99;return x}}console.log(make(1)())",

    // 7. Nested catch parameter shadowing:
    "function make(x){return function(){try{throw 77}catch(x){return x}}}console.log(make(1)())",

    // 8. Nested for-of binding shadowing:
    "function make(x){return function(){for(let x of [88]){return x}}}console.log(make(1)())",

    // 9. Reassignment of parameter inside nested closure:
    // If inlined with literal argument `1`, produces invalid `1 = 2` assignment.
    "function make(x){return function(){x=2;return x}}var f1=make(1);var f2=make(1);console.log(f1(),f2())",

    // 10. Void statement inlining with closure-deferred read:
    "var a=1;var fn;function t(x){fn=function(){return x}}t(a);a=2;console.log(fn())",
];

fn compile(source: &str, level: &str) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "closurec-inline-subst-18144-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir(&dir).expect("fresh scratch dir");
    let path = dir.join("input.js");
    std::fs::write(&path, format!("{source}\n")).expect("write input");
    let out = Command::new(BINARY)
        .args(["--compilation_level", level, "--js"])
        .arg(&path)
        .output()
        .expect("run closurec");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{source:?} at {level}: exit {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .expect("closurec output is UTF-8")
        .trim_end_matches('\n')
        .to_string()
}

fn node_stdout(code: &str) -> Option<String> {
    let out = Command::new("node").args(["-e", code]).output().ok()?;
    assert!(
        out.status.success(),
        "node -e {code:?} failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn compiled_output_prints_what_the_source_prints() {
    for source in PROGRAMS {
        let Some(source_says) = node_stdout(source) else {
            eprintln!("node not on PATH; skipping runtime comparison");
            return;
        };
        for level in ["SIMPLE", "ADVANCED"] {
            let compiled = compile(source, level);
            let compiled_says = node_stdout(&compiled).expect("node ran a moment ago");
            assert_eq!(
                compiled_says, source_says,
                "{source:?} at {level}: {compiled}"
            );
        }
    }
}
