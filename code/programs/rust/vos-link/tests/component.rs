//! `check-component` (VOS-039a, VOS04 section 5.6): one passing fixture crate, then one
//! deliberate break at a time.  Every negative control asserts the finding's own code AND its
//! own text, and that nothing else fired, because a control that passes for the wrong reason is
//! a defect of the gate (VER04h section 6).

mod common;
use common::*;

use std::process::Command;

use vos_link::component::{build_lines, tokenize};
use vos_link::{check_component, Outcome};

const PROD: &str = "vos-fix-comp";
const ANY: &str = "vos-fix-comp-any";

/// The BUILD of a crate that satisfies every line, in the order of section 5.2.
fn build_text(c: &str, windows: bool) -> String {
    let vl = "../../../programs/rust/vos-link/Cargo.toml";
    let mut s = format!(
        "# comment lines are skipped\n\
cargo test -p {c} --lib --test behaviour -- --nocapture\n\
cargo test -p {c} --features ghost --lib --test behaviour\n\
cargo test -p {c} --doc\n\
cargo run -q -p vos-shape-check -- .\n\
cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .\n\
cargo run -q --manifest-path {vl} -- check-doc .\n\
cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check {c}\n"
    );
    if windows {
        s += &format!("set WARRANT_EVIDENCE_OUT=evidence/test-manifest.json&& cargo test -q -p {c} --features ghost --test warrant_evidence\n");
        s += &format!("set WARRANT_EVIDENCE_FAULT=field-zero&& set WARRANT_EVIDENCE_FILTER=/exhaustive/x&& cargo test -q -p {c} --features ghost --test warrant_evidence >\"%TEMP%\\n.txt\" 2>NUL & findstr /B /C:\"not ok \" \"%TEMP%\\n.txt\" >NUL && (findstr /C:\"Bail out!\" \"%TEMP%\\n.txt\" >NUL && exit /b 1 || exit /b 0)\n");
    } else {
        s += &format!("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {c} --features ghost --test warrant_evidence\n");
        s += &format!("WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/x cargo test -q -p {c} --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{{n=1}} /Bail out!/{{b=1}} END{{exit !(n&&!b)}}'\n");
        s += &format!("if [ \"$(uname)\" = \"Linux\" ]; then cargo tarpaulin -p {c} --features ghost --lib --test behaviour --ignore-tests --engine llvm --include-files '{c}/src/*' --fail-under 95 --skip-clean; fi\n");
    }
    s += &format!("cargo run -q --manifest-path {vl} -- check-component .\n");
    s
}

fn manifest_text(claims: &[&str], property: i64, fuzz: i64, cases: i64, negctl_ok: bool) -> String {
    let mut t = Vec::new();
    t.push(
        r#"{"cases":1,"claim":"","id":"m/example/e","kind":"example","op":"o","verdict":"pass"}"#
            .to_string(),
    );
    for c in claims {
        t.push(format!(r#"{{"cases":{cases},"claim":"{c}","id":"m/exhaustive/{c}","kind":"exhaustive","verdict":"pass"}}"#));
        t.push(format!(r#"{{"cases":{property},"claim":"{c}","id":"m/property/{c}","kind":"property","verdict":"pass"}}"#));
        t.push(format!(r#"{{"claim":"{c}","id":"m/fuzz/{c}","iterations":{fuzz},"kind":"fuzz","verdict":"pass"}}"#));
    }
    for tier in ["example", "exhaustive", "property", "fuzz"] {
        let v = if negctl_ok { "pass" } else { "fail" };
        t.push(format!(r#"{{"claim":"","fault":"field-zero","id":"m/negctl/{tier}/op/field-zero","kind":"negctl","op":"op","target":"{tier}","verdict":"{v}"}}"#));
    }
    format!(
        r#"{{"schema":"warrant-test-evidence-v1","tests":[{}]}}"#,
        t.join(",")
    )
}

fn vos_toml(kind: &str) -> String {
    format!("class = \"K\"\nkind = \"{kind}\"\ncontract = \"vos-sched-contract\"\nobligations = [\"safety\"]\nevidence-features = [\"ghost\"]\ncoverage-floor = 95\n")
}

fn cargo_toml(c: &str, publish: &str) -> String {
    format!("[package]\nname = \"{c}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{publish}\n[lib]\npath = \"src/lib.rs\"\n")
}

/// A repository holding one crate that passes.
fn good(label: &str, name: &str, kind: &str) -> Repo {
    let r = Repo::new(label);
    let d = format!("code/packages/rust/{name}");
    r.write(&format!("{d}/BUILD"), build_text(name, false));
    r.write(&format!("{d}/BUILD_windows"), build_text(name, true));
    r.write(&format!("{d}/README.md"), "# x\n");
    r.write(&format!("{d}/CHANGELOG.md"), "# x\n");
    let publish = if kind == "any" { "publish = false" } else { "" };
    r.write(&format!("{d}/Cargo.toml"), cargo_toml(name, publish));
    r.write(&format!("{d}/vos.toml"), vos_toml(kind));
    r.write(&format!("{d}/required_capabilities.json"), "{}\n");
    r.write(&format!("{d}/warrant/impl.part"), "-- x\n");
    r.write(&format!("{d}/evidence.bindings"), "package x\n");
    r.write(
        &format!("{d}/evidence/test-manifest.json"),
        manifest_text(&["c1", "c2"], 4096, 5000, 10, true),
    );
    r.write(
        &format!("{d}/tests/generated/warrant_evidence.rs"),
        "// x\n",
    );
    r.write(&format!("code/specs/{name}.warrant"), "warrant x v1\n");
    r.write(
        "code/programs/go/build-tool/repository_inputs.go",
        format!("var rows = []row{{\n\tvosImplementationRow(\"vos-other\"),\n\tvosImplementationRow(\"{name}\"),\n}}\n"),
    );
    r
}

fn run(r: &Repo, name: &str) -> Outcome {
    check_component(&r.pkg(name), None).expect("a verdict")
}

/// Exactly one finding, with this code and this text inside it.
fn only(o: &Outcome, code: &str, text: &str) {
    assert_eq!(o.findings.len(), 1, "{}", show(o));
    assert_eq!(o.findings[0].code, code, "{}", show(o));
    assert!(
        o.findings[0].message.contains(text),
        "wanted {text:?} in {}",
        show(o)
    );
}

/// This finding is among the findings.
fn has(o: &Outcome, code: &str, text: &str) {
    assert!(
        o.findings
            .iter()
            .any(|f| f.code == code && f.message.contains(text)),
        "wanted {code} {text:?} in {}",
        show(o)
    );
}

fn crate_file(name: &str, rel: &str) -> String {
    format!("code/packages/rust/{name}/{rel}")
}

// --- the passing fixtures -----------------------------------------------------------------

#[test]
fn a_complete_production_crate_passes() {
    let r = good("cc-pass-prod", PROD, "production");
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(o.header.contains("kind production"));
}

#[test]
fn a_complete_any_crate_passes_and_a_reference_crate_is_in_scope() {
    let r = good("cc-pass-any", ANY, "any");
    assert!(run(&r, ANY).findings.is_empty());
    let r = good("cc-pass-ref", PROD, "reference");
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn changelog_d_meta_satisfies_the_changelog_rule() {
    let r = good("cc-chlog", PROD, "production");
    std::fs::remove_file(r.path(&crate_file(PROD, "CHANGELOG.md"))).unwrap();
    only(
        &run(&r, PROD),
        "CC02",
        "neither CHANGELOG.md nor CHANGELOG.d/_meta.md",
    );
    r.write(&crate_file(PROD, "CHANGELOG.d/_meta.md"), "# x\n");
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC01: scope --------------------------------------------------------------------------

#[test]
fn a_contract_crate_is_out_of_scope_and_says_so() {
    let r = good("cc01-contract", PROD, "production");
    r.write(&crate_file(PROD, "vos.toml"), "kind = \"contract\"\n");
    only(
        &run(&r, PROD),
        "CC01",
        "kind `contract` is not an implementation kind",
    );
}

#[test]
fn a_crate_with_no_kind_a_bad_vos_toml_or_none_is_cc01() {
    let r = good("cc01-nokind", PROD, "production");
    r.write(&crate_file(PROD, "vos.toml"), "class = \"H\"\n");
    only(&run(&r, PROD), "CC01", "kind `(none)`");
    r.write(&crate_file(PROD, "vos.toml"), "kind = \"nonsense\"\n");
    only(&run(&r, PROD), "CC01", "vos.toml:1");
    std::fs::remove_file(r.path(&crate_file(PROD, "vos.toml"))).unwrap();
    only(&run(&r, PROD), "CC01", "vos.toml: not found");
}

// --- CC02: the file set -------------------------------------------------------------------

#[test]
fn each_missing_required_file_is_its_own_finding() {
    for rel in [
        "BUILD",
        "BUILD_windows",
        "README.md",
        "Cargo.toml",
        "required_capabilities.json",
        "warrant/impl.part",
        "evidence.bindings",
        "evidence/test-manifest.json",
        "tests/generated/warrant_evidence.rs",
    ] {
        let r = good("cc02", PROD, "production");
        std::fs::remove_file(r.path(&crate_file(PROD, rel))).unwrap();
        only(
            &run(&r, PROD),
            "CC02",
            &format!("file set: {rel} is missing"),
        );
    }
}

#[test]
fn a_missing_generated_document_is_cc02() {
    let r = good("cc02-doc", PROD, "production");
    std::fs::remove_file(r.path(&format!("code/specs/{PROD}.warrant"))).unwrap();
    only(
        &run(&r, PROD),
        "CC02",
        "the generated document code/specs/vos-fix-comp.warrant is missing",
    );
}

// --- CC03: a missing BUILD line fails, in BUILD and in BUILD_windows ------------------------

/// Remove every line of `build` containing `needle`.
fn drop_lines(text: &str, needle: &str) -> String {
    let kept: Vec<&str> = text.lines().filter(|l| !l.contains(needle)).collect();
    assert!(kept.len() < text.lines().count(), "no line has {needle:?}");
    kept.join("\n") + "\n"
}

#[test]
fn each_missing_build_line_fails_in_both_build_files() {
    // (a needle that exactly one line has, the line of VOS04 5.2 it is, the text the finding must carry)
    let cases = [
        (
            "--lib --test behaviour -- --nocapture",
            "line 1",
            "unit tests without the ghost face",
        ),
        (
            "test -p vos-fix-comp --features ghost --lib",
            "line 1",
            "unit tests with the evidence feature",
        ),
        ("--doc", "line 1", "doc tests"),
        ("vos-shape-check", "line 2", "the shape check"),
        (
            "ca_capability_analyzer",
            "line 3",
            "the capability analyzer",
        ),
        ("-- check-doc .", "line 4", "the document check"),
        ("gen_evidence", "line 5", "harness drift"),
    ];
    for file in ["BUILD", "BUILD_windows"] {
        for (needle, line, what) in cases {
            let r = good("cc03", PROD, "production");
            let p = crate_file(PROD, file);
            r.write(&p, drop_lines(&r.read(&p), needle));
            let o = run(&r, PROD);
            only(
                &o,
                "CC03",
                &format!("{file}: {line} of VOS04 5.2 ({what}) is missing"),
            );
        }
    }
}

#[test]
fn a_missing_generated_suite_line_is_line_six() {
    for file in ["BUILD", "BUILD_windows"] {
        let r = good("cc03-six", PROD, "production");
        let p = crate_file(PROD, file);
        r.write(&p, drop_lines(&r.read(&p), "WARRANT_EVIDENCE_OUT="));
        let o = run(&r, PROD);
        // The check-component line then has no run to follow: that is not a second finding.
        only(&o, "CC03", &format!("{file}: line 6 of VOS04 5.2"));
    }
}

#[test]
fn missing_sabotage_lines_are_line_nine() {
    for file in ["BUILD", "BUILD_windows"] {
        let r = good("cc03-nine", PROD, "production");
        let p = crate_file(PROD, file);
        r.write(&p, drop_lines(&r.read(&p), "WARRANT_EVIDENCE_FAULT="));
        only(
            &run(&r, PROD),
            "CC03",
            &format!("{file}: line 9 of VOS04 5.2 (sabotage"),
        );
    }
}

#[test]
fn a_crate_name_that_only_starts_like_ours_does_not_satisfy_a_line() {
    // `-p vos-fix-comp-extra` must not satisfy `-p vos-fix-comp` (token boundaries).
    let r = good("cc03-tok", PROD, "production");
    let p = crate_file(PROD, "BUILD");
    r.edit(
        &p,
        &format!("cargo test -p {PROD} --doc"),
        &format!("cargo test -p {PROD}-extra --doc"),
    );
    only(&run(&r, PROD), "CC03", "doc tests");
}

#[test]
fn the_manifest_run_may_be_the_tolerance_script() {
    let r = good("cc03-script", PROD, "production");
    let p = crate_file(PROD, "BUILD");
    r.write(&p, drop_lines(&r.read(&p), "WARRANT_EVIDENCE_OUT="));
    r.edit(
        &p,
        "WARRANT_EVIDENCE_FAULT",
        "sh warrant/check_evidence.sh\nWARRANT_EVIDENCE_FAULT",
    );
    // The script does not run the suite yet: line 6 is still missing.
    r.write(&crate_file(PROD, "warrant/check_evidence.sh"), "echo hi\n");
    only(&run(&r, PROD), "CC03", "line 6 of VOS04 5.2");
    r.write(
        &crate_file(PROD, "warrant/check_evidence.sh"),
        format!("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {PROD} --features ghost --test warrant_evidence\n"),
    );
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC04: the guards ---------------------------------------------------------------------

#[test]
fn a_sabotage_line_that_is_not_bail_out_aware_fails_with_its_line_number() {
    let r = good("cc04-bail", PROD, "production");
    let p = crate_file(PROD, "BUILD");
    r.edit(
        &p,
        "awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'",
        "grep -q '^not ok '",
    );
    let o = run(&r, PROD);
    only(
        &o,
        "CC04",
        "BUILD:10: a sabotage line is not bail-out-aware",
    );
}

#[test]
fn the_negctl_checker_script_counts_as_a_guard() {
    let r = good("cc04-negctl", PROD, "production");
    let p = crate_file(PROD, "BUILD");
    r.edit(
        &p,
        "awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'",
        "&& true",
    );
    r.edit(
        &p,
        "WARRANT_EVIDENCE_FAULT=field-zero",
        "python3 code/scripts/warrant_negctl.py WARRANT_EVIDENCE_FAULT=field-zero",
    );
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn check_component_before_the_generated_suite_reads_a_stale_manifest() {
    for file in ["BUILD", "BUILD_windows"] {
        let r = good("cc04-order", PROD, "production");
        let p = crate_file(PROD, file);
        let text = r.read(&p);
        let cc = text.lines().last().unwrap().to_string();
        let mut lines: Vec<&str> = text.lines().collect();
        lines.pop();
        let at = lines.iter().position(|l| l.contains("check-doc")).unwrap();
        lines.insert(at + 1, &cc);
        r.write(&p, lines.join("\n") + "\n");
        only(
            &run(&r, PROD),
            "CC04",
            &format!(
                "{file}:{}: check-component runs before the generated suite",
                at + 2
            ),
        );
    }
}

#[test]
fn a_missing_check_component_line_is_cc03() {
    for file in ["BUILD", "BUILD_windows"] {
        let r = good("cc03-cc", PROD, "production");
        let p = crate_file(PROD, file);
        r.write(&p, drop_lines(&r.read(&p), "check-component"));
        only(
            &run(&r, PROD),
            "CC03",
            &format!("{file}: the check-component line is missing"),
        );
    }
}

// --- CC05: the build-tool row -------------------------------------------------------------

#[test]
fn a_missing_build_tool_row_fails() {
    let r = good("cc05", PROD, "production");
    r.write(
        "code/programs/go/build-tool/repository_inputs.go",
        "var rows = []row{\n\tvosImplementationRow(\"vos-other\"),\n}\n",
    );
    only(
        &run(&r, PROD),
        "CC05",
        "has no line `vosImplementationRow(\"vos-fix-comp\"),`",
    );
}

#[test]
fn a_commented_out_or_lookalike_row_does_not_count() {
    for body in [
        "// vosImplementationRow(\"vos-fix-comp\"),\n",
        "\tvosImplementationRow(\"vos-fix-comp-2\"),\n",
        "\tvosImplementationRow(\"vos-fix-comp\") // no comma, not a row\n",
    ] {
        let r = good("cc05-look", PROD, "production");
        r.write("code/programs/go/build-tool/repository_inputs.go", body);
        only(&run(&r, PROD), "CC05", "build-tool row");
    }
}

#[test]
fn a_missing_build_tool_file_is_cc05() {
    let r = good("cc05-file", PROD, "production");
    std::fs::remove_file(r.path("code/programs/go/build-tool/repository_inputs.go")).unwrap();
    only(&run(&r, PROD), "CC05", "repository_inputs.go not found");
}

// --- CC06: VOS-D082 -----------------------------------------------------------------------

#[test]
fn an_any_crate_without_publish_false_fails() {
    let r = good("cc06", ANY, "any");
    for cargo in [
        cargo_toml(ANY, ""),
        cargo_toml(ANY, "publish = true"),
        cargo_toml(ANY, "# publish = false"),
    ] {
        r.write(&crate_file(ANY, "Cargo.toml"), cargo);
        only(&run(&r, ANY), "CC06", "must say `publish = false`");
    }
}

#[test]
fn publish_false_must_be_in_the_package_table() {
    let r = good("cc06-tbl", ANY, "any");
    r.write(
        &crate_file(ANY, "Cargo.toml"),
        format!("[package]\nname = \"{ANY}\"\nversion = \"0.1.0\"\n[lib]\npublish = false\n"),
    );
    only(&run(&r, ANY), "CC06", "publish = false");
}

#[test]
fn an_unreadable_cargo_toml_is_cc06() {
    let r = good("cc06-bad", ANY, "any");
    r.write(&crate_file(ANY, "Cargo.toml"), "[package\nname = \n");
    let o = run(&r, ANY);
    has(&o, "CC06", "Cargo.toml:");
}

#[test]
fn a_production_crate_may_publish() {
    // The policy of a production crate is the release gate's (VOS-D082): not this rule.
    let r = good("cc06-prod", PROD, "production");
    r.write(
        &crate_file(PROD, "Cargo.toml"),
        cargo_toml(PROD, "publish = true"),
    );
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC07: the coverage gate --------------------------------------------------------------

#[test]
fn a_missing_floor_fails() {
    let r = good("cc07-none", PROD, "production");
    r.write(
        &crate_file(PROD, "vos.toml"),
        "kind = \"production\"\nevidence-features = [\"ghost\"]\n",
    );
    only(&run(&r, PROD), "CC07", "declares no `coverage-floor`");
}

#[test]
fn a_floor_below_the_repository_minimum_fails() {
    let r = good("cc07-low", PROD, "production");
    r.edit(
        &crate_file(PROD, "vos.toml"),
        "coverage-floor = 95",
        "coverage-floor = 94",
    );
    // The BUILD line still says 95, so the mismatch is a second finding; both are real.
    let o = run(&r, PROD);
    has(
        &o,
        "CC07",
        "coverage-floor = 94 is below the repository minimum 95",
    );
    has(&o, "CC07", "--fail-under 94");
}

#[test]
fn a_build_gate_that_differs_from_the_floor_fails() {
    let r = good("cc07-diff", PROD, "production");
    r.edit(
        &crate_file(PROD, "vos.toml"),
        "coverage-floor = 95",
        "coverage-floor = 98",
    );
    only(&run(&r, PROD), "CC07", "--fail-under 98");
    // A higher floor is fine once the gate agrees.
    r.edit(
        &crate_file(PROD, "BUILD"),
        "--fail-under 95",
        "--fail-under 98",
    );
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn each_broken_part_of_the_coverage_line_fails() {
    for (from, to) in [
        ("--include-files 'vos-fix-comp/src/*'", ""),
        (
            "--include-files 'vos-fix-comp/src/*'",
            "--include-files 'other/src/*'",
        ),
        ("--engine llvm", "--engine ptrace"),
        ("--ignore-tests", ""),
        (
            "--features ghost --lib --test behaviour --ignore-tests",
            "--lib --test behaviour --ignore-tests",
        ),
        ("tarpaulin -p vos-fix-comp", "tarpaulin -p other"),
        ("if [ \"$(uname)\" = \"Linux\" ]; then ", "if true; then "),
        ("--fail-under 95", ""),
    ] {
        let r = good("cc07-line", PROD, "production");
        r.edit(&crate_file(PROD, "BUILD"), from, to);
        only(
            &run(&r, PROD),
            "CC07",
            "coverage: BUILD has no Linux line `cargo tarpaulin",
        );
    }
}

#[test]
fn a_missing_coverage_line_fails() {
    let r = good("cc07-gone", PROD, "production");
    let p = crate_file(PROD, "BUILD");
    r.write(&p, drop_lines(&r.read(&p), "tarpaulin"));
    only(&run(&r, PROD), "CC07", "coverage: BUILD has no Linux line");
}

#[test]
fn the_windows_build_does_not_need_the_coverage_line() {
    // tarpaulin is a Linux gate; the fixture's BUILD_windows has none and passes.
    let r = good("cc07-win", PROD, "production");
    assert!(!r
        .read(&crate_file(PROD, "BUILD_windows"))
        .contains("tarpaulin"));
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC08: the manifest guards ------------------------------------------------------------

fn with_manifest(label: &str, text: String) -> Outcome {
    let r = good(label, PROD, "production");
    r.write(&crate_file(PROD, "evidence/test-manifest.json"), text);
    run(&r, PROD)
}

#[test]
fn an_empty_manifest_fails_line_six() {
    let o = with_manifest(
        "cc08-empty",
        r#"{"schema":"warrant-test-evidence-v1","tests":[]}"#.into(),
    );
    only(
        &o,
        "CC08",
        "`tests` is empty: no test ran (line 6, none ran = fail)",
    );
}

#[test]
fn a_claim_with_zero_exhaustive_cases_fails_line_six() {
    let o = with_manifest("cc08-zero", manifest_text(&["c1"], 4096, 5000, 0, true));
    only(&o, "CC08", "claim \"c1\" ran zero exhaustive cases (line 6");
}

#[test]
fn fewer_than_4096_property_cases_fails_line_seven() {
    let o = with_manifest("cc08-prop", manifest_text(&["c1"], 4095, 5000, 10, true));
    only(
        &o,
        "CC08",
        "claim \"c1\" ran 4095 property cases, fewer than 4096 (line 7)",
    );
}

#[test]
fn fewer_than_5000_fuzz_iterations_fails_line_eight() {
    let o = with_manifest("cc08-fuzz", manifest_text(&["c1"], 4096, 4999, 10, true));
    only(
        &o,
        "CC08",
        "claim \"c1\" ran 4999 fuzz iterations, fewer than 5000 (line 8)",
    );
}

#[test]
fn a_claim_missing_a_tier_fails() {
    let m = manifest_text(&["c1", "c2"], 4096, 5000, 10, true).replace(
        r#"{"cases":4096,"claim":"c2","id":"m/property/c2","kind":"property","verdict":"pass"},"#,
        "",
    );
    only(
        &with_manifest("cc08-tier", m),
        "CC08",
        "claim \"c2\" has no property entry (line 7",
    );
}

#[test]
fn a_tier_without_a_negative_control_fails_line_nine() {
    let m = manifest_text(&["c1"], 4096, 5000, 10, true)
        .replace(r#"{"claim":"","fault":"field-zero","id":"m/negctl/fuzz/op/field-zero","kind":"negctl","op":"op","target":"fuzz","verdict":"pass"}"#, r#"{"cases":1,"claim":"","id":"m/example/e2","kind":"example","verdict":"pass"}"#);
    only(
        &with_manifest("cc08-neg", m),
        "CC08",
        "no negative control ran in the fuzz tier (line 9",
    );
}

#[test]
fn a_failing_control_fails_unless_the_tolerance_script_names_it() {
    let r = good("cc08-fail", PROD, "production");
    let p = crate_file(PROD, "evidence/test-manifest.json");
    r.write(&p, manifest_text(&["c1"], 4096, 5000, 10, false));
    let o = run(&r, PROD);
    assert_eq!(o.findings.len(), 4, "{}", show(&o));
    has(
        &o,
        "CC08",
        "test \"m/negctl/exhaustive/op/field-zero\" has verdict \"fail\", not pass",
    );
    // A script that names every failing id excuses them (VOS-D065, VOS-D098) ...
    r.write(
        &crate_file(PROD, "warrant/check_evidence.sh"),
        "known='m/negctl/example/op/field-zero m/negctl/exhaustive/op/field-zero m/negctl/property/op/field-zero m/negctl/fuzz/op/field-zero'\n",
    );
    assert!(run(&r, PROD).findings.is_empty());
    // ... but never a failing test that is not a control.
    let bad = manifest_text(&["c1"], 4096, 5000, 10, true).replace(
        r#"{"cases":10,"claim":"c1","id":"m/exhaustive/c1","kind":"exhaustive","verdict":"pass"}"#,
        r#"{"cases":10,"claim":"c1","id":"m/exhaustive/c1","kind":"exhaustive","verdict":"fail"}"#,
    );
    r.write(&p, bad);
    only(
        &run(&r, PROD),
        "CC08",
        "test \"m/exhaustive/c1\" has verdict \"fail\"",
    );
}

#[test]
fn a_manifest_that_is_not_the_generators_fails() {
    only(
        &with_manifest("cc08-json", "{".into()),
        "CC08",
        "manifest: not JSON",
    );
    only(
        &with_manifest("cc08-arr", "[]".into()),
        "CC08",
        "the top level is not an object",
    );
    only(
        &with_manifest(
            "cc08-notests",
            r#"{"schema":"warrant-test-evidence-v1"}"#.into(),
        ),
        "CC08",
        "no `tests` array",
    );
    let o = with_manifest(
        "cc08-schema",
        manifest_text(&["c1"], 4096, 5000, 10, true).replace("evidence-v1", "evidence-v2"),
    );
    only(&o, "CC08", "`schema` is not `warrant-test-evidence-v1`");
    let o = with_manifest(
        "cc08-malformed",
        manifest_text(&["c1"], 4096, 5000, 10, true)
            .replace(r#""tests":["#, r#""tests":[5,{"kind":"x"},"#),
    );
    only(&o, "CC08", "2 entries lack a string kind, id or verdict");
}

#[test]
fn a_manifest_with_no_claims_fails() {
    let m = r#"{"schema":"warrant-test-evidence-v1","tests":[{"cases":1,"claim":"","id":"m/example/e","kind":"example","verdict":"pass"}]}"#;
    let o = with_manifest("cc08-noclaims", m.into());
    has(
        &o,
        "CC08",
        "no claim has an exhaustive, property or fuzz entry",
    );
}

// --- the BUILD reader itself --------------------------------------------------------------

#[test]
fn tokenize_splits_at_shell_separators_and_drops_quotes() {
    assert_eq!(
        tokenize("set A=1&& cargo test -p x; (a|b) 'q/*' \"d\""),
        ["set", "A=1", "cargo", "test", "-p", "x", "a", "b", "q/*", "d"]
    );
    assert!(tokenize("   ").is_empty());
}

#[test]
fn build_lines_skips_blanks_and_comments_and_numbers_from_one() {
    let ls = build_lines("# c\n\n  cargo test  \nREM x\nrem y\n  # c2\ncargo run\n");
    assert_eq!(ls.len(), 2);
    assert_eq!((ls[0].number, ls[0].text.as_str()), (3, "cargo test"));
    assert_eq!(ls[1].number, 7);
}

// --- the command line ---------------------------------------------------------------------

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vos-link"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn the_command_line_exits_zero_one_and_two() {
    let r = good("cc-cli", PROD, "production");
    let dir = r.pkg(PROD).to_string_lossy().into_owned();
    let ok = cli(&["check-component", &dir]);
    let text = String::from_utf8_lossy(&ok.stdout).into_owned();
    assert_eq!(ok.status.code(), Some(0), "{text}");
    assert!(
        text.contains("vos-link check-component: crate vos-fix-comp, kind production"),
        "{text}"
    );
    assert!(text.contains("result: PASS, 0 findings"), "{text}");

    std::fs::remove_file(r.path(&crate_file(PROD, "README.md"))).unwrap();
    let bad = cli(&["check-component", &dir, "--root", &r.root.to_string_lossy()]);
    let text = String::from_utf8_lossy(&bad.stdout).into_owned();
    assert_eq!(bad.status.code(), Some(1), "{text}");
    assert!(
        text.contains("CC02 file set: README.md is missing"),
        "{text}"
    );
    assert!(text.contains("result: FAIL, 1 findings"), "{text}");

    let none = cli(&["check-component", "/no/such/crate-dir"]);
    assert_eq!(none.status.code(), Some(2));
    let usage = cli(&["check-component"]);
    assert_eq!(usage.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&usage.stderr).contains("no crate directory"));
}

#[test]
fn a_symlinked_file_is_no_verdict_not_a_pass() {
    #[cfg(unix)]
    {
        let r = good("cc-sym", PROD, "production");
        let p = r.path(&crate_file(PROD, "README.md"));
        std::fs::remove_file(&p).unwrap();
        std::os::unix::fs::symlink(r.path(&crate_file(PROD, "BUILD")), &p).unwrap();
        let e = check_component(&r.pkg(PROD), None).unwrap_err();
        assert!(e.to_string().contains("is a symlink"), "{e}");
    }
}

#[test]
fn an_oversized_build_file_is_no_verdict() {
    let r = good("cc-big", PROD, "production");
    r.write(&crate_file(PROD, "BUILD"), "x".repeat(300 * 1024));
    let e = check_component(&r.pkg(PROD), None).unwrap_err();
    assert!(e.to_string().contains("over the"), "{e}");
}
