//! Executes the checked-in consumers for every stdlib library that previously
//! lacked a worked query. These are runnable artifacts, not documentation-only
//! examples: each expected answer and its provenance cross the CLI boundary.

use std::path::PathBuf;
use std::process::Command;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data")
        .canonicalize()
        .expect("spec data root must exist")
}

#[test]
fn stdlib_worked_queries_execute_with_provenance() {
    let cases: &[(&str, &[&str])] = &[
        (
            "adj-formula-stdlib/arithmetic/arithmetic.query.adj",
            &["\"name\":\"sum\"", "\"value\":12", "Sum.html"],
        ),
        (
            "adj-formula-stdlib/arithmetic/average.query.adj",
            &[
                "\"name\":\"mean_two\"",
                "\"value\":5",
                "ArithmeticMean.html",
            ],
        ),
        (
            "adj-formula-stdlib/arithmetic/percent.query.adj",
            &["\"name\":\"percent\"", "\"value\":75", "Percent.html"],
        ),
        (
            "adj-formula-stdlib/arithmetic/ratio.query.adj",
            &["\"name\":\"ratio\"", "\"value\":0.75", "Ratio.html"],
        ),
        (
            "adj-formula-stdlib/clinical/bmi.query.adj",
            &["\"name\":\"bmi\"", "22.857142857142858", "who.int"],
        ),
        (
            "adj-formula-stdlib/clinical/bmi_category.query.adj",
            &["\"hypothesis\":\"obese\"", "0.9999910000809994", "who.int"],
        ),
        (
            "adj-formula-stdlib/cockcroft_gault.query.adj",
            &["\"name\":\"cockcroft_gault\"", "\"value\":80", "NBK555956"],
        ),
        (
            "mycin-2026/recall/anemia-recall.query.adj",
            &["\"Class\":\"microcytic\"", "NBK560876"],
        ),
        (
            "mycin-2026/recall/coag-recall.query.adj",
            &["\"Factor\":\"factor_viii\"", "\"trust\":\"authoritative\""],
        ),
        (
            "mycin-2026/recall/endocrine-recall.query.adj",
            &[
                "\"Gland\":\"pancreatic_beta_cells\"",
                "\"trust\":\"authoritative\"",
            ],
        ),
        (
            "mycin-2026/recall/iem-recall.query.adj",
            &[
                "\"Enzyme\":\"hexosaminidase_a\"",
                "\"trust\":\"authoritative\"",
            ],
        ),
        (
            "mycin-2026/recall/vitamin-recall.query.adj",
            &["\"Disease\":\"beriberi\"", "\"trust\":\"authoritative\""],
        ),
    ];

    for (relative, expected) in cases {
        let program = data_root().join(relative);
        let output = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
            .arg(&program)
            .output()
            .unwrap_or_else(|error| panic!("run {}: {error}", program.display()));
        let stdout = String::from_utf8(output.stdout).expect("CLI output must be UTF-8");

        assert!(
            output.status.success(),
            "{} exited non-zero: {stdout}",
            program.display()
        );
        assert!(
            !stdout.contains("\"error\""),
            "{} returned an ADJ error: {stdout}",
            program.display()
        );
        for provenance_field in ["\"source\":", "\"locator\":", "\"trust\":\"authoritative\""] {
            assert!(
                stdout.contains(provenance_field),
                "{} omitted provenance field {provenance_field:?}: {stdout}",
                program.display()
            );
        }
        for needle in *expected {
            assert!(
                stdout.contains(needle),
                "{} did not contain {needle:?}: {stdout}",
                program.display()
            );
        }
    }
}
