//! CK11 and CC12 (VOS-040c, VOS31 section 10): the kit's side of the host model-SAT lane.
//!
//! `vos-link` cannot call the lane (the Cargo edge runs the other way), so CK11 checks FILES:
//! the BUILD line, the committed `warrant/lemmas.sat`, and "none ran = fail".  Every control
//! below breaks exactly one thing in a crate that otherwise passes, and asserts the finding's
//! code AND its text AND that nothing else fired (a control that passes for the wrong reason is
//! a defect of the gate).

mod common;
use common::*;

use vos_link::build_gen::{canonical_contract_build, MARKER, SAT_LINE};
use vos_link::sat_lane::{judge_sat_table, SAT_LADDER, SAT_TRIVIAL_LABEL};
use vos_link::{check_component, check_contract, gen_build, gen_contract, Outcome};
use vos_manifest::digest::sha256_hex;
use vos_shape_check::manifest::SatLane;

const C: &str = "vos-fixture-contract";
const CLAIMS: [&str; 2] = ["put_then_mem", "ins_range"];
const ROWS_GO: &str = "package main\n\nvar repositoryInputConsumers = []repositoryInputConsumer{\n\tvosImplementationRow(\"vos-other\"),\n\tvosContractRow(\"vos-fixture-contract\"),\n}\n";

fn pkg_path(rel: &str) -> String {
    format!("code/packages/rust/{C}/{rel}")
}

fn chk(r: &Repo) -> Outcome {
    check_contract(&r.pkg(C), None).expect("a verdict")
}

fn only(o: &Outcome, code: &str, text: &str) {
    assert_eq!(o.findings.len(), 1, "{}", show(o));
    assert_eq!(o.findings[0].code, code, "{}", show(o));
    assert!(
        o.findings[0].message.contains(text),
        "wanted {text:?} in {}",
        show(o)
    );
}

/// A table for `claims` with every row counted (or every row trivial).
fn table(claims: &[&str], c_sha: &str, l_sha: &str, label: &str) -> String {
    let mut s = format!(
        "vos-sat 1\ncontract-part {c_sha}\nlemmas-part {l_sha}\nroute-budget conflicts 50000 check-work 2147483648\n"
    );
    let sha = "ab".repeat(32);
    for c in claims {
        for n in SAT_LADDER {
            s.push_str(&format!(
                "lemma {c} n {n} label {label} and 10 vars 20 clauses 30 cnf {sha} verdict unsat collapsed no control sat premise witnessed\n"
            ));
        }
    }
    s.push_str("end\n");
    s
}

const OK_LABEL: &str = "lemma_family_member";

fn shas(r: &Repo) -> (String, String) {
    (
        sha256_hex(r.read(&pkg_path("warrant/contract.part")).as_bytes()),
        sha256_hex(r.read(&pkg_path("warrant/lemmas.part")).as_bytes()),
    )
}

/// A complete toy contract crate that opts in to the lane, with a good table and the generated
/// BUILD files.
fn opted_in(label: &str) -> Repo {
    let r = Repo::empty(label);
    r.add_contract_fixture(C);
    let vt = r.read(&pkg_path("vos.toml"));
    r.write(&pkg_path("vos.toml"), format!("{vt}sat = \"model\"\n"));
    let o = gen_contract(&r.pkg(C), None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
    r.write(&pkg_path("README.md"), "# x\n");
    r.write(&pkg_path("CHANGELOG.md"), "# x\n");
    r.write(
        &pkg_path("Cargo.toml"),
        "[package]\nname = \"vos-fixture-contract\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    r.write(&pkg_path("required_capabilities.json"), "{}\n");
    // src/lib.rs stays the fixture version: its enums are the CK10 variant rows of the table.
    r.write("code/programs/go/build-tool/repository_inputs.go", ROWS_GO);
    let o = gen_build(&r.pkg(C), None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
    let (c, l) = shas(&r);
    r.write(
        &pkg_path("warrant/lemmas.sat"),
        table(&CLAIMS, &c, &l, OK_LABEL),
    );
    r
}

fn put_table(r: &Repo, text: &str) {
    r.write(&pkg_path("warrant/lemmas.sat"), text);
}

// --- the generator ---------------------------------------------------------------------------

#[test]
fn gen_build_writes_the_lane_line_between_check_contract_and_check_component() {
    let r = opted_in("ck11-gen");
    let build = r.read(&pkg_path("BUILD"));
    assert_eq!(build, r.read(&pkg_path("BUILD_windows")));
    assert_eq!(build, canonical_contract_build(C, Some(SatLane::Model)));
    let lines: Vec<&str> = build.lines().collect();
    let at = |pred: &dyn Fn(&str) -> bool| lines.iter().position(|l| pred(l)).unwrap();
    let sat = at(&|l| l == SAT_LINE);
    assert_eq!(
        at(&|l| l.ends_with("-- check-contract .")) + 2,
        sat,
        "a comment line in between"
    );
    assert_eq!(at(&|l| l.ends_with("-- check-component .")), sat + 1);
    assert!(SAT_LINE.contains("--release") && SAT_LINE.ends_with("-- check ."));
    // Without the key, the region is the one written before VOS-040c.
    let plain = canonical_contract_build(C, None);
    assert!(!plain.contains("vos-model-sat"));
    assert_eq!(
        plain.lines().count() + 2,
        build.lines().count(),
        "exactly a comment and the line are added"
    );
    // Idempotent.
    let again = gen_build(&r.pkg(C), None).unwrap();
    assert_eq!(
        again.action.as_deref(),
        Some("unchanged BUILD, BUILD_windows")
    );
}

// --- CK11 -----------------------------------------------------------------------------------------

#[test]
fn an_opted_in_crate_with_a_good_table_passes_both_checks() {
    let r = opted_in("ck11-pass");
    let o = chk(&r);
    assert!(o.findings.is_empty(), "{}", show(&o));
    let o = check_component(&r.pkg(C), None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
}

#[test]
fn ck11_the_lane_line_missing_from_either_build_file() {
    for name in ["BUILD", "BUILD_windows"] {
        let r = opted_in("ck11-noline");
        let text = r
            .read(&pkg_path(name))
            .replace(&format!("{SAT_LINE}\n"), "");
        r.write(&pkg_path(name), text);
        let o = chk(&r);
        only(&o, "CK11", &format!("{name}: vos.toml says"));
        // CC03 of check-component also names the file, byte for byte.
        let o = check_component(&r.pkg(C), None).expect("a verdict");
        assert!(o.findings.iter().any(|f| f.code == "CC03"), "{}", show(&o));
    }
}

#[test]
fn ck11_a_lane_line_after_the_marker_does_not_count() {
    let r = opted_in("ck11-aftermarker");
    let text = r
        .read(&pkg_path("BUILD"))
        .replace(&format!("{SAT_LINE}\n"), "");
    r.write(&pkg_path("BUILD"), format!("{text}{SAT_LINE}\n"));
    assert!(text.contains(MARKER));
    only(&chk(&r), "CK11", "BUILD: vos.toml says");
}

#[test]
fn ck11_an_absent_table_is_a_failure_not_a_skip() {
    let r = opted_in("ck11-notable");
    std::fs::remove_file(r.path(&pkg_path("warrant/lemmas.sat"))).unwrap();
    only(&chk(&r), "CK11", "warrant/lemmas.sat: not found");
}

#[test]
fn ck11_none_ran_is_a_failure() {
    let (c, l);
    let r = opted_in("ck11-none");
    (c, l) = shas(&r);
    // No rows at all (every lemma lacks its rows).
    put_table(
        &r,
        &format!("vos-sat 1\ncontract-part {c}\nlemmas-part {l}\nroute-budget conflicts 1 check-work 1\nend\n"),
    );
    let o = chk(&r);
    assert!(
        o.findings.iter().any(|f| f
            .message
            .contains("no row for lemma `put_then_mem` at N = 8")),
        "{}",
        show(&o)
    );
    assert!(
        o.findings
            .iter()
            .any(|f| f.message.contains("zero counted obligations")),
        "{}",
        show(&o)
    );
    // Rows, but every one trivial: still nothing ran.
    put_table(&r, &table(&CLAIMS, &c, &l, SAT_TRIVIAL_LABEL));
    only(&chk(&r), "CK11", "zero counted obligations");
    // An empty file.
    put_table(&r, "");
    assert!(chk(&r).findings.iter().all(|f| f.code == "CK11"));
    assert!(!chk(&r).findings.is_empty());
}

#[test]
fn ck11_a_lemma_with_no_row_at_one_width() {
    let r = opted_in("ck11-missingrow");
    let t = r.read(&pkg_path("warrant/lemmas.sat"));
    let kept: Vec<&str> = t
        .lines()
        .filter(|l| !l.starts_with("lemma ins_range n 32 "))
        .collect();
    put_table(&r, &format!("{}\n", kept.join("\n")));
    let o = chk(&r);
    assert!(
        o.findings
            .iter()
            .any(|f| f.code == "CK11" && f.message.contains("lemma `ins_range` at N = 32")),
        "{}",
        show(&o)
    );
    assert!(o.findings.iter().all(|f| f.code == "CK11"), "{}", show(&o));
}

#[test]
fn ck11_a_stale_digest_an_extra_row_and_a_forged_verdict() {
    let r = opted_in("ck11-malformed");
    let good = r.read(&pkg_path("warrant/lemmas.sat"));
    // Stale: lemmas.part changed since the table was written.
    put_table(&r, &good.replacen("lemmas-part ", "lemmas-part 00", 1));
    only(
        &chk(&r),
        "CK11",
        "is not the digest of the part as it is now",
    );
    // An extra row for a lemma the part does not have.
    put_table(
        &r,
        &good.replacen(
            "end\n",
            "lemma ghost n 8 label x and 1 vars 1 clauses 1 cnf ab verdict unsat collapsed no control sat premise none\nend\n",
            1,
        ),
    );
    only(&chk(&r), "CK11", "a row beyond the claims");
    // A verdict that is not unsat.
    put_table(&r, &good.replacen("verdict unsat", "verdict sat", 1));
    only(
        &chk(&r),
        "CK11",
        "must say `verdict unsat` and `control sat`",
    );
    // No `end`.
    put_table(&r, &good.replacen("end\n", "", 1));
    only(&chk(&r), "CK11", "no closing `end`");
    // Wrong order of widths.
    let swapped = good.replacen("n 8 ", "n 99 ", 1);
    put_table(&r, &swapped);
    only(&chk(&r), "CK11", "at N = \"99\"");
    // Text after `end`.
    put_table(&r, &format!("{good}more\n"));
    only(&chk(&r), "CK11", "text after `end`");
    // Not newline-terminated.
    put_table(&r, good.trim_end());
    only(&chk(&r), "CK11", "does not end with a newline");
    // The good one passes again.
    put_table(&r, &good);
    assert!(chk(&r).findings.is_empty());
}

#[test]
fn ck11_a_trivial_line_names_a_claim_and_is_not_counted() {
    let r = opted_in("ck11-trivial");
    let good = r.read(&pkg_path("warrant/lemmas.sat"));
    put_table(&r, &good.replacen("end\n", "trivial ins_range\nend\n", 1));
    assert!(chk(&r).findings.is_empty());
    put_table(&r, &good.replacen("end\n", "trivial ghost\nend\n", 1));
    only(&chk(&r), "CK11", "trivial ghost");
}

#[test]
fn without_the_key_no_lane_line_and_no_table_is_required_but_a_stray_one_fails() {
    let r = opted_in("ck11-nokey");
    // Remove the key and regenerate BUILD: the opt-in is off, the table is now stray.
    let vt = r
        .read(&pkg_path("vos.toml"))
        .replace("sat = \"model\"\n", "");
    r.write(&pkg_path("vos.toml"), vt);
    // BUILD still has the line, and the table still exists: both are findings.
    let o = chk(&r);
    assert_eq!(o.findings.len(), 3, "{}", show(&o));
    assert!(o.findings.iter().all(|f| f.code == "CK11"));
    assert!(o
        .findings
        .iter()
        .any(|f| f.message.contains("BUILD: runs the model-SAT lane")));
    assert!(o
        .findings
        .iter()
        .any(|f| f.message.contains("BUILD_windows: runs")));
    assert!(o
        .findings
        .iter()
        .any(|f| f.message.contains("warrant/lemmas.sat: exists but")));
    // Regenerate BUILD and delete the table: clean (opt-in, VOS31 10.3).
    gen_build(&r.pkg(C), None).unwrap();
    std::fs::remove_file(r.path(&pkg_path("warrant/lemmas.sat"))).unwrap();
    let o = chk(&r);
    assert!(o.findings.is_empty(), "{}", show(&o));
    let o = check_component(&r.pkg(C), None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
}

#[test]
fn the_sat_key_is_a_closed_enum_at_the_kit_too() {
    let r = opted_in("ck11-badkey");
    let vt = r
        .read(&pkg_path("vos.toml"))
        .replace("\"model\"", "\"smt\"");
    r.write(&pkg_path("vos.toml"), vt);
    let o = chk(&r);
    assert!(
        o.findings.iter().any(|f| f.message.contains("sat must be")),
        "{}",
        show(&o)
    );
}

// --- the table judge, directly ----------------------------------------------------------------

#[test]
fn judge_sat_table_counts_only_non_trivial_rows() {
    let claims: Vec<String> = CLAIMS.iter().map(|s| s.to_string()).collect();
    let t = table(&CLAIMS, "c", "l", OK_LABEL);
    let (bad, facts) = judge_sat_table(&t, &claims, "c", "l");
    assert!(bad.is_empty(), "{bad:?}");
    assert_eq!((facts.rows, facts.counted), (8, 8));
    let t = table(&CLAIMS, "c", "l", SAT_TRIVIAL_LABEL);
    let (bad, facts) = judge_sat_table(&t, &claims, "c", "l");
    assert_eq!((facts.rows, facts.counted), (8, 0));
    assert!(bad.iter().any(|b| b.contains("none ran")));
    // No claims at all: nothing can run.
    let (bad, _) = judge_sat_table("", &[], "c", "l");
    assert!(bad.iter().any(|b| b.contains("none ran")));
}

// --- CC12: an implementation never claims SAT ------------------------------------------------

#[test]
fn the_spec_and_the_ladder_agree() {
    assert_eq!(SAT_LADDER, [8, 16, 32, 64]);
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS31-host-model-sat.md"),
    )
    .unwrap();
    assert!(spec.contains("`CK11`") && spec.contains("`CC12`"));
    assert!(spec.contains(SAT_LINE));
}
