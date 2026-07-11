//! End-to-end test for the FL-3 elementary-arithmetic primitive library through
//! the built CLI binary: a consumer `import`s the SHIPPED `arithmetic.adj`
//! formula library, binds two operands from its own `observe`d facts, and applies
//! one of the four cited primitive formulas (sum/difference/product/quotient). For
//! each, the CLI must compute the value on the CPU and render the applied formula's
//! citation in the `derived` section — the auditable answer, zero math by the model.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped elementary-arithmetic library, resolved from this
/// crate's manifest dir so the test is location-independent.
fn shipped_arithmetic_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/arithmetic/arithmetic.adj")
        .canonicalize()
        .expect("shipped arithmetic.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_prim_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Run one primitive: copy the shipped library next to a consumer that binds the
/// two operands and applies `call`, then assert the derived value AND that the
/// applied formula carries its cited provenance (mathworld locator + trust tier).
fn check_primitive(tag: &str, obs_one: &str, obs_two: &str, call: &str, name: &str, value: &str) {
    let dir = scratch(tag);
    let lib = std::fs::read_to_string(shipped_arithmetic_lib()).unwrap();
    std::fs::write(dir.join("arithmetic.adj"), lib).unwrap();
    let consumer = format!(
        "import \"arithmetic.adj\"\n\
         observe {obs_one}\n\
         observe {obs_two}\n\
         ? {call}\n"
    );
    std::fs::write(dir.join("case.adj"), consumer).unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains(&format!("\"name\":\"{name}\"")) && s.contains(&format!("\"value\":{value}")),
        "{name} computed to {value}: {s}"
    );
    // The applied primitive carries its cited definition + trust tier — auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\"") && s.contains("mathworld.wolfram.com"),
        "{name} carries its cited provenance: {s}"
    );
}

#[test]
fn sum_primitive_binds_and_computes_with_citation() {
    // "…3 apples… 4 apples…" → the model binds the two addends; the engine adds them.
    check_primitive(
        "sum",
        "addend_one(3)",
        "addend_two(4)",
        "sum(addend_one, addend_two)",
        "sum",
        "7",
    );
}

#[test]
fn difference_primitive_binds_and_computes_with_citation() {
    check_primitive(
        "difference",
        "minuend(10)",
        "subtrahend(3)",
        "difference(minuend, subtrahend)",
        "difference",
        "7",
    );
}

#[test]
fn product_primitive_binds_and_computes_with_citation() {
    check_primitive(
        "product",
        "factor_one(6)",
        "factor_two(7)",
        "product(factor_one, factor_two)",
        "product",
        "42",
    );
}

#[test]
fn quotient_primitive_binds_and_computes_with_citation() {
    check_primitive(
        "quotient",
        "dividend(20)",
        "divisor(4)",
        "quotient(dividend, divisor)",
        "quotient",
        "5",
    );
}
