//! Formula-only verification through the real `adj-verify` binary.
//!
//! A computed answer has three independent obligations: CPU math must replay,
//! the formula must quote its source bytes, and every numeric input must quote
//! the bytes from which it was decomposed. This fixture exercises all three.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

const DOC: &str = "Formula: total is a plus b. Input a is 7. Input b is 5.";
const NESTED_DOC: &str = "Outer: ratio is quotient. Inner: quotient is dividend divided by divisor. Input n is 9. Input d is 3.";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjverify_formula_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn offset(doc: &str, quote: &str) -> usize {
    doc.find(quote)
        .expect("fixture quote must occur in source bytes")
}

fn write_program(dir: &Path, doc: &str) -> PathBuf {
    let hash = ContentHash::of(doc.as_bytes()).as_hex().to_string();
    let snapshots = dir.join("snapshots");
    std::fs::create_dir_all(&snapshots).unwrap();
    std::fs::write(snapshots.join(&hash), doc).unwrap();

    let source = format!(
        "formulabook demo {{\n\
             formula total(a, b) = a + b\n\
                 quote \"total is a plus b\" at {} snapshot \"{}\"\n\
                 source \"formula fixture\"\n\
                 locator \"cas://formula-source\"\n\
                 trust authoritative\n\
         }}\n\
         observe a(7)\n\
             quote \"Input a is 7\" at {} snapshot \"{}\"\n\
             source \"input fixture\"\n\
             locator \"cas://input\"\n\
             trust authoritative\n\
         observe b(5)\n\
             quote \"Input b is 5\" at {} snapshot \"{}\"\n\
             source \"input fixture\"\n\
             locator \"cas://input\"\n\
             trust authoritative\n\
         ? total(a, b)\n",
        offset(DOC, "total is a plus b"),
        hash,
        offset(DOC, "Input a is 7"),
        hash,
        offset(DOC, "Input b is 5"),
        hash,
    );
    let program = dir.join("case.adj");
    std::fs::write(&program, source).unwrap();
    program
}

fn write_nested_program(dir: &Path, doc: &str) -> PathBuf {
    let hash = ContentHash::of(doc.as_bytes()).as_hex().to_string();
    let snapshots = dir.join("snapshots");
    std::fs::create_dir_all(&snapshots).unwrap();
    std::fs::write(snapshots.join(&hash), doc).unwrap();
    let source = format!(
        "formulabook nested {{\n\
             formula quotient(dividend, divisor) = dividend / divisor\n\
                 quote \"quotient is dividend divided by divisor\" at {} snapshot \"{}\"\n\
                 source \"inner fixture\" trust authoritative\n\
             formula ratio(numerator, denominator) = quotient(numerator, denominator)\n\
                 quote \"ratio is quotient\" at {} snapshot \"{}\"\n\
                 source \"outer fixture\" trust authoritative\n\
         }}\n\
         observe n(9)\n\
             quote \"Input n is 9\" at {} snapshot \"{}\"\n\
             source \"input fixture\" trust authoritative\n\
         observe d(3)\n\
             quote \"Input d is 3\" at {} snapshot \"{}\"\n\
             source \"input fixture\" trust authoritative\n\
         ? ratio(n, d)\n",
        offset(NESTED_DOC, "quotient is dividend divided by divisor"),
        hash,
        offset(NESTED_DOC, "ratio is quotient"),
        hash,
        offset(NESTED_DOC, "Input n is 9"),
        hash,
        offset(NESTED_DOC, "Input d is 3"),
        hash,
    );
    let program = dir.join("nested.adj");
    std::fs::write(&program, source).unwrap();
    program
}

fn verify(program: &Path, snapshots: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(snapshots)
        .arg(program)
        .output()
        .expect("run adj-verify");
    (
        output.status.success(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

#[test]
fn formula_math_and_all_three_quotes_are_fully_verified() {
    let dir = scratch("pass");
    let program = write_program(&dir, DOC);
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(ok, "fully grounded formula must verify: {output}");
    assert!(output.contains("\"verified\":true"), "{output}");
    assert!(output.contains("\"fully_verified\":true"), "{output}");
    assert!(output.contains("\"computations\":1"), "{output}");
    assert!(output.contains("\"computations_rechecked\":1"), "{output}");
    assert!(
        output.contains("\"computations_fully_verified\":1"),
        "{output}"
    );
    assert!(output.contains("\"quotes_verified\":3"), "{output}");
    assert!(
        output.contains("\"input_quotes\":[{\"fact_id\":0"),
        "{output}"
    );
}

#[test]
fn changed_formula_source_bytes_fail_closed() {
    let dir = scratch("drift");
    let drifted = "Formula: total is a plus c. Input a is 7. Input b is 5.";
    let program = write_program(&dir, drifted);
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(
        !ok,
        "a formula quote absent from its snapshot must fail: {output}"
    );
    assert!(output.contains("\"verified\":false"), "{output}");
    assert!(output.contains("\"fully_verified\":false"), "{output}");
    assert!(output.contains("\"pass\":\"formula_quote\""), "{output}");
    assert!(output.contains("\"status\":\"quote_missing\""), "{output}");
}

#[test]
fn changed_input_source_bytes_fail_closed() {
    let dir = scratch("input_drift");
    let drifted = "Formula: total is a plus b. Input a is 8. Input b is 5.";
    let program = write_program(&dir, drifted);
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(!ok, "an input absent from its snapshot must fail: {output}");
    assert!(output.contains("\"verified\":false"), "{output}");
    assert!(output.contains("\"fully_verified\":false"), "{output}");
    assert!(output.contains("\"pass\":\"input_quote\""), "{output}");
    assert!(output.contains("\"status\":\"quote_missing\""), "{output}");
}

#[test]
fn a_computation_without_a_question_cannot_receive_the_strongest_verdict() {
    let dir = scratch("no_query");
    let program = write_program(&dir, DOC);
    let source = std::fs::read_to_string(&program).unwrap();
    std::fs::write(&program, source.replace("? total(a, b)\n", "")).unwrap();
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(ok, "the computation itself still replays: {output}");
    assert!(output.contains("\"verified\":true"), "{output}");
    assert!(output.contains("\"fully_verified\":false"), "{output}");
    assert!(output.contains("\"query_computations\":0"), "{output}");
}

#[test]
fn nested_formula_sources_are_each_byte_verified() {
    let dir = scratch("nested");
    let program = write_nested_program(&dir, NESTED_DOC);
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(ok, "nested formula chain must replay: {output}");
    assert!(output.contains("\"fully_verified\":true"), "{output}");
    assert!(output.contains("\"quotes_verified\":4"), "{output}");
}

#[test]
fn drifted_nested_formula_source_fails_closed() {
    let dir = scratch("nested_drift");
    let drifted = NESTED_DOC.replace("dividend divided", "dividend timesxx");
    let program = write_nested_program(&dir, &drifted);
    let (ok, output) = verify(&program, &dir.join("snapshots"));

    assert!(!ok, "a nested formula quote must not be optional: {output}");
    assert!(output.contains("\"pass\":\"formula_quote\""), "{output}");
    assert!(output.contains("\"fully_verified\":false"), "{output}");
}
