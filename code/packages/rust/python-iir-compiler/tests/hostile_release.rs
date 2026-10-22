//! LANG84 C5 hostile-input probes for the release path, run in a child
//! process.
//!
//! A stack overflow is an abort, not a panic: it would take the test harness
//! down with it and look like a crash of the whole suite. So the probes run
//! in a child copy of this test binary, and the parent only checks that the
//! child exited cleanly. Each probe must end in `Ok` or a typed error.

use python_iir_compiler::{compile_source_at, run_module, FrontendBudgets, FrontendError};

const CHILD_ENV: &str = "PYTHON_IIR_HOSTILE_PROBE_CHILD";

fn compile(source: &str) -> Result<interpreter_ir::IIRModule, FrontendError> {
    compile_source_at(source, None, "t", &FrontendBudgets::default())
}

#[test]
fn hostile_probe_child() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    // Nesting far past the parser's depth cap is a Budget error.
    let deep = format!("print({}1{})", "(".repeat(5_000), ")".repeat(5_000));
    assert!(matches!(compile(&deep), Err(FrontendError::Budget { .. })));

    // The deepest nesting the parser admits is walked without trouble.
    let mut admitted = 0;
    for depth in 1..200 {
        let source = format!("print({}1{})", "(".repeat(depth), ")".repeat(depth));
        match compile(&source) {
            Ok(_) => admitted = depth,
            Err(FrontendError::Budget { .. }) => break,
            Err(other) => panic!("depth {depth}: {other}"),
        }
    }
    assert!(admitted >= 15, "{admitted}");
    let unary = format!("print({}1)", "- ".repeat(400));
    assert!(matches!(compile(&unary), Ok(_) | Err(FrontendError::Budget { .. })));

    // A long flat chain is a loop, not recursion, and computes the right sum.
    let chain = format!("print(1{})", " + 1".repeat(7_000));
    let module = compile(&chain).expect("a 7,001-term sum is within budget");
    assert_eq!(run_module(module).unwrap().stdout, "7001\n");

    // Thousands of statements, then one past the token budget.
    assert!(compile(&"print(1)\n".repeat(3_000)).is_ok());
    assert!(matches!(compile(&"print(1)\n".repeat(5_000)), Err(FrontendError::Budget { .. })));

    // A 60,000-digit literal is refused without being echoed back whole.
    let huge = format!("print({})", "9".repeat(60_000));
    let error = compile(&huge).unwrap_err();
    assert!(error.to_string().len() < 300, "{}", error.to_string().len());
    let zeros = format!("print({}1)", "0".repeat(30_000));
    assert!(compile(&zeros).is_err());

    // Unterminated constructs and junk are ordinary errors.
    for junk in ["print(1 + ", "print((((", "\u{0}\u{1}", "€€€", "print(1) \\", "print 1 //", "\t\tprint(1)"] {
        assert!(compile(junk).is_err(), "{junk:?}");
    }
}

#[test]
fn hostile_probes_end_in_typed_results_in_a_child_process() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hostile_probe_child", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hostile probes crashed or failed: {:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}
