//! `gen-build` and `check-component` (VOS-039a, VOS04 section 5.6).
//!
//! The fixture crate is *generated* by `gen-build`, so it passes by construction; then it is
//! broken one way at a time.  The first draft recognised BUILD lines in author text, and a
//! security review listed the single-line tricks that satisfied it while running something
//! else (the build tool runs each line as its own `sh -c`).  Each trick is a test below, named
//! `bypass_*`, and each must now FAIL with its own code and text.  Every control asserts the
//! finding's code AND its text AND that nothing else fired, because a control that passes for
//! the wrong reason is a defect of the gate (VER04h section 6).

mod common;
use common::*;

use std::process::Command;

use vos_link::build_gen::{parse_plan, validate_plan, MARKER};
use vos_link::component::{row_in_consumers, strip_go, FUZZ_ITERATIONS_MIN, PROPERTY_CASES_MIN};
use vos_link::{check_component, gen_build, Outcome};

const PROD: &str = "vos-fix-comp";
const ANY: &str = "vos-fix-comp-any";

// --- the fixture --------------------------------------------------------------------------

const DOC: &str = "warrant vos_fix_comp v1
  record Impl(x: U8)
  op init() -> Impl
  op insert_out(s: Impl, t: U8) -> Impl
  op insert_img(s: Impl, t: U8) -> Impl
  op remove(s: Impl, t: U8) -> Impl
  op dispatch_out(s: Impl) -> Impl
  op alpha_w(s: Impl) -> U8
  bound init:
    steps <= 80
    heap <= 0
  bound insert_out:
    steps <= 160
    heap <= 0
  bound insert_img:
    steps <= 160
    heap <= 0
  bound remove:
    steps <= 160
    heap <= 0
  bound dispatch_out:
    steps <= 160
    heap <= 0
  bound alpha_w:
    steps <= 999
  claim c1:
    forall v: Bool. f(v) == f(v)
  claim c2:
    forall v: Bool. h(v) == h(v)
  example e1:
    1u8 == 1u8
";

fn bindings(c: &str, property: u64, fuzz: u64) -> String {
    let m = c.replace('-', "_");
    format!(
        "package {m}\nrevision 0.1.0\nspec code/specs/{c}.warrant\noutput code/packages/rust/{c}/tests/generated/warrant_evidence.rs\nseed 0x1\nproperty_cases {property}\nfuzz_iterations {fuzz}\nnegctl_fuzz_iterations 1000\nexhaustive_budget 400000\n\nop f export=vos_ghost_f args=u8 ret=u8 faults=field-zero,wrong-ctor\nop g export=vos_ghost_g ret=u8 faults=field-flip\nop h export=vos_ghost_h ret=u8 faults=field-flip\n"
    )
}

/// The fixture document of an `-any` crate.  CC11 type-checks the document (it needs the shape of
/// `model build`), so unlike [`DOC`] this one is a valid Warrant document: `Impl(x, c)` with the
/// oracle `c` last and outside the cone of `build`, and the state-producing operation `f`, which
/// claim `c1` calls alone.
fn any_doc() -> String {
    // CC11 reads the successors off the document's signatures (F1), so every operation of this
    // fixture that is not the one state-to-state operation `f` is made a projection.
    DOC.replacen(
        "  op insert_out(s: Impl, t: U8) -> Impl\n  op insert_img(s: Impl, t: U8) -> Impl\n  op remove(s: Impl, t: U8) -> Impl\n  op dispatch_out(s: Impl) -> Impl\n",
        "  op insert_out(s: Impl, t: U8) -> U8\n  op insert_img(s: Impl, t: U8) -> U8\n  op remove(s: Impl, t: U8) -> U8\n  op dispatch_out(s: Impl) -> U8\n",
        1,
    )
    .replacen(
        "  record Impl(x: U8)\n",
        "  record M(a: Bool)\n  record Impl(x: U8, c: U8)\n  model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8, 0u8))\n",
        1,
    )
    .replacen(
        "  op alpha_w(s: Impl) -> U8\n",
        "  op alpha_w(s: Impl) -> U8\n  op f(s: Impl, t: U8) -> Impl\n  op g() -> U8\n  op h() -> U8\n",
        1,
    )
    .replacen(
        "forall v: Bool. f(v) == f(v)",
        "forall v: Bool. f(Impl(1u8, 2u8), 3u8).c == 2u8",
        1,
    )
    .replacen("forall v: Bool. h(v) == h(v)", "forall v: Bool. h() == h()", 1)
    // The type checker wants a secrecy label for every parameter of every operation.
    .replacen(
        "  claim c1:",
        "  labels insert_out:\n    s public\n    t public\n  labels insert_img:\n    s public\n    t public\n  labels remove:\n    s public\n    t public\n  labels dispatch_out:\n    s public\n  labels alpha_w:\n    s public\n  labels f:\n    s public\n    t public\n  claim c1:",
        1,
    )
}

/// The bindings of an `-any` fixture: `f` produces a state from a state and lists `field-flip-last`.
fn any_bindings(c: &str) -> String {
    let m = c.replace('-', "_");
    bindings(c, 4096, 5000)
        .replacen(
            "\nop f export=vos_ghost_f args=u8 ret=u8 faults=field-zero,wrong-ctor\n",
            &format!("\nstruct Impl path={m}::Impl fields=x:u8,c:u8\nop f export=vos_ghost_f args=Impl,u8 ret=Impl faults=field-zero,wrong-ctor,field-flip-last\n"),
            1,
        )
}

/// The plan of an `-any` fixture: the ordinary plan and the CC11 entry.
fn any_plan() -> String {
    format!("{PLAN}exhaustive/c1 field-flip-last\n")
}

const PLAN: &str = "# plan\nexhaustive/c1 field-zero\nexhaustive/c2 wrong-ctor\nexample/e1 field-zero\nexhaustive/c1 field-flip attack=g\nexhaustive/c2 field-flip attack=h\n";

/// A manifest of the fixture document.  `failing` ids get verdict `fail`.
fn manifest(claims: &[&str], property: i64, fuzz: i64, cases: i64, failing: &[&str]) -> String {
    let v = |id: &str| {
        if failing.contains(&id) {
            "fail"
        } else {
            "pass"
        }
    };
    let mut t = vec![
        r#"{"cases":1,"claim":"","id":"m/example/e1","kind":"example","verdict":"pass"}"#
            .to_string(),
    ];
    for c in claims {
        t.push(format!(r#"{{"cases":{cases},"claim":"{c}","id":"m/exhaustive/{c}","kind":"exhaustive","verdict":"pass"}}"#));
        t.push(format!(r#"{{"cases":{property},"claim":"{c}","id":"m/property/{c}","kind":"property","verdict":"pass"}}"#));
        t.push(format!(r#"{{"claim":"{c}","id":"m/fuzz/{c}","iterations":{fuzz},"kind":"fuzz","verdict":"pass"}}"#));
    }
    for tier in ["example", "exhaustive", "property", "fuzz"] {
        for (op, fault) in [("f", "field-zero"), ("g", "field-flip")] {
            let id = format!("m/negctl/{tier}/{op}/{fault}");
            let verdict = v(&id);
            t.push(format!(r#"{{"claim":"","fault":"{fault}","id":"{id}","kind":"negctl","op":"{op}","target":"{tier}","verdict":"{verdict}"}}"#));
        }
    }
    format!(
        r#"{{"schema":"warrant-test-evidence-v1","source_sha256":"@SRC@","spec_sha256":"@SPEC@","tests":[{}]}}"#,
        t.join(",")
    )
}

fn vos_toml(kind: &str, extra: &str) -> String {
    format!("class = \"K\"\nkind = \"{kind}\"\ncontract = \"vos-sched-contract\"\nevidence-features = [\"ghost\"]\ncoverage-floor = 95\n{extra}")
}

const EXCUSED: &str = "excused-controls = [{ id = \"m/negctl/exhaustive/g/field-flip\", reason = \"VOS-D098\" }, { id = \"m/negctl/property/g/field-flip\", reason = \"VOS-D098\" }, { id = \"m/negctl/fuzz/g/field-flip\", reason = \"VOS-D098\" }]\n";
const EXCUSED_IDS: [&str; 3] = [
    "m/negctl/exhaustive/g/field-flip",
    "m/negctl/property/g/field-flip",
    "m/negctl/fuzz/g/field-flip",
];

fn cargo_toml(c: &str, publish: &str) -> String {
    format!("[package]\nname = \"{c}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{publish}\n[lib]\npath = \"src/lib.rs\"\n")
}

fn p(name: &str, rel: &str) -> String {
    format!("code/packages/rust/{name}/{rel}")
}

/// The fixture's backlog: D098 is open, D065 is done.
const BACKLOG: &str =
    "| VOS-D065 | x | y | P2 | **done** (somewhere) |\n| VOS-D098 | x | y | P2: later | queued |\n";

const SRC: &str = "pub fn x() {}\n";

/// Write a manifest, filling in the digests of the fixture's document and source.
fn put_manifest(r: &Repo, name: &str, text: String) {
    let spec = coding_adventures_sha256::sha256_hex(
        r.read(&format!("code/specs/{name}.warrant")).as_bytes(),
    );
    let framed =
        coding_adventures_test_bootstrap::manifest::source_tree_framing(&r.path(&p(name, "src")))
            .unwrap();
    let src = coding_adventures_sha256::sha256_hex(&framed);
    r.write(
        &p(name, "evidence/test-manifest.json"),
        text.replace("@SPEC@", &spec).replace("@SRC@", &src),
    );
}

const ROWS_GO: &str = "package main\n\nvar repositoryInputConsumers = []repositoryInputConsumer{\n\tvosImplementationRow(\"vos-other\"),\n\tvosImplementationRow(\"@\"),\n}\n";

/// A repository holding one crate whose BUILD files `gen-build` wrote.
fn good(label: &str, name: &str, kind: &str, excused: bool) -> Repo {
    let r = Repo::new(label);
    let (extra, failing): (&str, &[&str]) = if excused {
        (EXCUSED, &EXCUSED_IDS)
    } else {
        ("", &[])
    };
    r.write(&p(name, "vos.toml"), vos_toml(kind, extra));
    r.write(&p(name, "README.md"), "# x\n");
    r.write(&p(name, "CHANGELOG.md"), "# x\n");
    let publish = if kind == "any" { "publish = false" } else { "" };
    r.write(&p(name, "Cargo.toml"), cargo_toml(name, publish));
    r.write(&p(name, "required_capabilities.json"), "{}\n");
    r.write(&p(name, "warrant/impl.part"), "-- x\n");
    let any = kind == "any";
    r.write(
        &p(name, "warrant/sabotage.plan"),
        if any { any_plan() } else { PLAN.to_string() },
    );
    r.write(
        &p(name, "evidence.bindings"),
        if any {
            any_bindings(name)
        } else {
            bindings(name, 4096, 5000)
        },
    );
    r.write(&p(name, "tests/generated/warrant_evidence.rs"), "// x\n");
    r.write(
        &format!("code/specs/{name}.warrant"),
        if any { any_doc() } else { DOC.to_string() },
    );
    if any {
        // CC11 asks for a claim the CONTRACT owns (F3): the fixture's contract defines `c1` in its
        // `any` tier, with the text the document's `c1` has, so the two have one claim_sha256.
        r.write(
            "code/packages/rust/vos-sched-contract/warrant/obligations-any.part",
            "  claim c1:\n    forall v: Bool. f(Impl(1u8, 2u8), 3u8).c == 2u8\n",
        );
    }
    r.write(&p(name, "src/lib.rs"), SRC);
    r.write("code/specs/VOS-BACKLOG.md", BACKLOG);
    put_manifest(&r, name, manifest(&["c1", "c2"], 4096, 5000, 10, failing));
    r.write(
        "code/programs/go/build-tool/repository_inputs.go",
        ROWS_GO.replace('@', name),
    );
    let o = gen_build(&r.pkg(name), None).expect("a verdict");
    assert!(o.findings.is_empty(), "{}", show(&o));
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

const CC03: &str = "does not begin with the region `vos-link gen-build` writes";

// --- the passing fixtures, and gen-build itself --------------------------------------------

#[test]
fn a_generated_production_crate_passes() {
    let r = good("cc-pass-prod", PROD, "production", false);
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(o.header.contains("kind production"));
}

#[test]
fn a_generated_any_and_reference_crate_pass_and_an_excusing_crate_too() {
    let r = good("cc-pass-any", ANY, "any", false);
    assert!(run(&r, ANY).findings.is_empty());
    let r = good("cc-pass-ref", PROD, "reference", false);
    assert!(run(&r, PROD).findings.is_empty());
    let r = good("cc-pass-exc", PROD, "production", true);
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(r
        .read(&p(PROD, "BUILD"))
        .contains("\nsh warrant/check_evidence.sh\n"));
    assert!(r
        .read(&p(PROD, "BUILD_windows"))
        .contains("\nbash warrant/check_evidence.sh\n"));
}

#[test]
fn gen_build_writes_the_lines_of_section_5_2_in_order() {
    let r = good("gb-lines", PROD, "production", false);
    let b = r.read(&p(PROD, "BUILD"));
    let order = [
        "cargo test -p vos-fix-comp --lib --test behaviour -- --nocapture\n",
        "cargo test -p vos-fix-comp --features ghost --lib --test behaviour\n",
        "cargo test -p vos-fix-comp --doc\n",
        "cargo run -q -p vos-shape-check -- .\n",
        "cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .\n",
        "Cargo.toml -- check-doc .\n",
        "--example gen_evidence -- --check vos-fix-comp\n",
        "\nrm -f evidence/test-manifest.json\n",
        "WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p vos-fix-comp --features ghost --test warrant_evidence\n",
        "WARRANT_EVIDENCE_FAULT=field-zero WARRANT_EVIDENCE_FILTER=/exhaustive/c1 python3 ../../../scripts/warrant_negctl.py -- cargo test -q -p vos-fix-comp --features ghost --test warrant_evidence\n",
        "WARRANT_EVIDENCE_FILTER=/example/e1 python3 ",
        "WARRANT_EVIDENCE_FAULT=field-flip WARRANT_EVIDENCE_ATTACK=g WARRANT_EVIDENCE_FILTER=/exhaustive/c1 python3 ../../../scripts/warrant_negctl.py -- ",
        "if [ \"$(uname)\" = \"Linux\" ]; then cargo tarpaulin -p vos-fix-comp --features ghost --lib --test behaviour --ignore-tests --engine llvm --include-files 'vos-fix-comp/src/*' --fail-under 95 --skip-clean; fi\n",
        "Cargo.toml -- check-component .\n",
        MARKER,
    ];
    let mut at = 0;
    for needle in order {
        let i = b[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not after offset {at} in\n{b}"));
        at += i + needle.len();
    }
    assert!(b.contains("# needs-toolchain: python\n"));
    assert!(b.ends_with(&format!("{MARKER}\n")));
}

#[test]
fn the_windows_build_has_set_lines_python_and_no_coverage_gate() {
    let r = good("gb-win", PROD, "production", false);
    let w = r.read(&p(PROD, "BUILD_windows"));
    assert!(!w.contains("tarpaulin"));
    // The stale manifest is removed first; `if exist` makes it safe and idempotent under cmd.
    assert!(w.contains("\nif exist evidence\\test-manifest.json del /f /q evidence\\test-manifest.json\nset WARRANT_EVIDENCE_OUT"));
    assert!(w.contains("\nset WARRANT_EVIDENCE_OUT=evidence/test-manifest.json&& cargo test -q -p vos-fix-comp --features ghost --test warrant_evidence\n"));
    assert!(w.contains("\nset WARRANT_EVIDENCE_FAULT=field-flip&& set WARRANT_EVIDENCE_ATTACK=g&& set WARRANT_EVIDENCE_FILTER=/exhaustive/c1&& python ../../../scripts/warrant_negctl.py -- cargo test"));
    assert!(!w.contains("python3"));
}

#[test]
fn a_crate_without_an_evidence_feature_gets_no_features_flag() {
    let r = good("gb-nofeat", PROD, "production", false);
    r.write(
        &p(PROD, "vos.toml"),
        "kind = \"production\"\ncontract = \"vos-sched-contract\"\ncoverage-floor = 96\n",
    );
    assert!(gen_build(&r.pkg(PROD), None).unwrap().findings.is_empty());
    let b = r.read(&p(PROD, "BUILD"));
    assert!(!b.contains("--features"), "{b}");
    assert!(b.contains("--fail-under 96"));
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn gen_build_is_idempotent_and_keeps_the_authors_own_lines_below_the_marker() {
    let r = good("gb-idem", PROD, "production", false);
    let before = r.read(&p(PROD, "BUILD"));
    let again = gen_build(&r.pkg(PROD), None).unwrap();
    assert_eq!(
        again.action.as_deref(),
        Some("unchanged BUILD, BUILD_windows")
    );
    assert_eq!(r.read(&p(PROD, "BUILD")), before);
    // The author's lines survive a regeneration, and do not fail the check.
    let b = p(PROD, "BUILD");
    r.write(&b, format!("{before}echo mine\n"));
    r.edit(
        &p(PROD, "vos.toml"),
        "coverage-floor = 95",
        "coverage-floor = 97",
    );
    gen_build(&r.pkg(PROD), None).unwrap();
    let after = r.read(&b);
    assert!(
        after.ends_with(&format!("{MARKER}\necho mine\n")),
        "{after}"
    );
    assert!(after.contains("--fail-under 97"));
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn gen_build_refuses_bad_inputs_and_writes_nothing() {
    let r = good("gb-bad", PROD, "production", false);
    let before = r.read(&p(PROD, "BUILD"));
    r.write(
        &p(PROD, "warrant/sabotage.plan"),
        "exhaustive/c1 nonsense\n",
    );
    let o = gen_build(&r.pkg(PROD), None).unwrap();
    assert!(!o.findings.is_empty());
    assert_eq!(r.read(&p(PROD, "BUILD")), before);
    r.write(&p(PROD, "vos.toml"), "kind = \"spine\"\n");
    only(
        &gen_build(&r.pkg(PROD), None).unwrap(),
        "CC01",
        "kind `spine`",
    );
}

#[test]
fn gen_build_writes_the_tolerance_script_from_excused_controls() {
    let r = good("gb-script", PROD, "production", true);
    let s = r.read(&p(PROD, "warrant/check_evidence.sh"));
    assert!(s.contains("known_ids='m/negctl/exhaustive/g/field-flip\nm/negctl/property/g/field-flip\nm/negctl/fuzz/g/field-flip'"), "{s}");
    assert!(s.contains("cargo test -q -p vos-fix-comp --features ghost --test warrant_evidence"));
    assert!(s.contains("[ \"$ok\" -eq 0 ]"));
}

// --- CC01: scope --------------------------------------------------------------------------

#[test]
fn a_crate_that_is_not_a_component_kind_is_out_of_scope_and_says_so() {
    // Contract crates are components since VOS-039b-1 (tests/contract.rs); a spine is not.
    let r = good("cc01-spine", PROD, "production", false);
    r.write(&p(PROD, "vos.toml"), "kind = \"spine\"\n");
    only(
        &run(&r, PROD),
        "CC01",
        "kind `spine` is not a component kind",
    );
}

#[test]
fn a_crate_with_no_kind_a_bad_vos_toml_or_none_is_cc01() {
    let r = good("cc01-nokind", PROD, "production", false);
    r.write(&p(PROD, "vos.toml"), "class = \"H\"\n");
    only(&run(&r, PROD), "CC01", "kind `(none)`");
    r.write(&p(PROD, "vos.toml"), "kind = \"nonsense\"\n");
    only(&run(&r, PROD), "CC01", "vos.toml:1");
    std::fs::remove_file(r.path(&p(PROD, "vos.toml"))).unwrap();
    only(&run(&r, PROD), "CC01", "vos.toml: not found");
}

#[test]
fn bypass_an_empty_excused_list_or_an_empty_or_short_id_is_refused_at_vos_toml() {
    // The first draft excused a control by `script.contains(id)`: an empty id excused every one.
    for extra in [
        "excused-controls = []\n",
        "excused-controls = [{ id = \"\", reason = \"VOS-D098\" }]\n",
    ] {
        let r = good("cc01-excuse", PROD, "production", false);
        r.write(&p(PROD, "vos.toml"), vos_toml("production", extra));
        only(&run(&r, PROD), "CC01", "vos.toml:");
    }
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
        "evidence/test-manifest.json",
        "tests/generated/warrant_evidence.rs",
    ] {
        let r = good("cc02", PROD, "production", false);
        std::fs::remove_file(r.path(&p(PROD, rel))).unwrap();
        only(
            &run(&r, PROD),
            "CC02",
            &format!("file set: {rel} is missing"),
        );
    }
}

#[test]
fn a_missing_plan_bindings_or_document_is_the_file_and_its_consequence() {
    let r = good("cc02-plan", PROD, "production", false);
    std::fs::remove_file(r.path(&p(PROD, "warrant/sabotage.plan"))).unwrap();
    let o = run(&r, PROD);
    has(&o, "CC02", "file set: warrant/sabotage.plan is missing");
    has(&o, "CC04", "warrant/sabotage.plan: not found");
    let r = good("cc02-bind", PROD, "production", false);
    std::fs::remove_file(r.path(&p(PROD, "evidence.bindings"))).unwrap();
    has(
        &run(&r, PROD),
        "CC02",
        "file set: evidence.bindings is missing",
    );
    let r = good("cc02-doc", PROD, "production", false);
    std::fs::remove_file(r.path(&format!("code/specs/{PROD}.warrant"))).unwrap();
    let o = run(&r, PROD);
    has(
        &o,
        "CC02",
        "the generated document code/specs/vos-fix-comp.warrant is missing",
    );
    has(&o, "CC04", "not found (run `vos-link gen-doc`)");
}

#[test]
fn changelog_d_meta_satisfies_the_changelog_rule() {
    let r = good("cc02-chlog", PROD, "production", false);
    std::fs::remove_file(r.path(&p(PROD, "CHANGELOG.md"))).unwrap();
    only(
        &run(&r, PROD),
        "CC02",
        "neither CHANGELOG.md nor CHANGELOG.d/_meta.md",
    );
    r.write(&p(PROD, "CHANGELOG.d/_meta.md"), "# x\n");
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC03: BUILD equals what gen-build writes; every bypass of the first draft -------------

const DOCLINE: &str = "cargo test -p vos-fix-comp --doc\n";
const COVLINE: &str = "if [ \"$(uname)\" = \"Linux\" ]; then cargo tarpaulin -p vos-fix-comp --features ghost --lib --test behaviour --ignore-tests --engine llvm --include-files 'vos-fix-comp/src/*' --fail-under 95 --skip-clean; fi\n";

fn break_build(label: &str, file: &str, from: &str, to: &str) -> Outcome {
    let r = good(label, PROD, "production", false);
    r.edit(&p(PROD, file), from, to);
    run(&r, PROD)
}

#[test]
fn bypass_echo_prefixed_line() {
    only(
        &break_build("by-echo", "BUILD", DOCLINE, &format!("echo {DOCLINE}")),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_true_then_comment() {
    only(
        &break_build(
            "by-true",
            "BUILD",
            DOCLINE,
            "true # cargo test -p vos-fix-comp --doc\n",
        ),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_false_and_line() {
    only(
        &break_build("by-false", "BUILD", DOCLINE, &format!("false && {DOCLINE}")),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_line_or_true() {
    only(
        &break_build(
            "by-ortrue",
            "BUILD",
            DOCLINE,
            "cargo test -p vos-fix-comp --doc || true\n",
        ),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_darwin_test_with_linux_in_a_comment() {
    let fake = "[ \"$(uname)\" = \"Darwin\" ] && cargo tarpaulin -p vos-fix-comp --features ghost --lib --test behaviour --ignore-tests --engine llvm --include-files 'vos-fix-comp/src/*' --fail-under 95 --skip-clean # \"Linux\"\n";
    only(
        &break_build("by-uname", "BUILD", COVLINE, fake),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_a_second_fail_under() {
    let to = COVLINE.replace("--skip-clean", "--skip-clean --fail-under 0");
    only(&break_build("by-fu0", "BUILD", COVLINE, &to), "CC03", CC03);
}

#[test]
fn bypass_exclude_files() {
    let to = COVLINE.replace(
        "--skip-clean",
        "--skip-clean --exclude-files 'vos-fix-comp/src/*'",
    );
    only(&break_build("by-excl", "BUILD", COVLINE, &to), "CC03", CC03);
}

#[test]
fn bypass_a_lower_floor_in_the_line_than_in_vos_toml() {
    only(
        &break_build("by-floor", "BUILD", "--fail-under 95", "--fail-under 50"),
        "CC03",
        CC03,
    );
}

#[test]
fn bypass_echo_of_the_tolerance_script() {
    let r = good("by-echo-script", PROD, "production", true);
    r.edit(
        &p(PROD, "BUILD"),
        "\nsh warrant/check_evidence.sh\n",
        "\necho warrant/check_evidence.sh\n",
    );
    only(&run(&r, PROD), "CC03", CC03);
}

#[test]
fn bypass_script_strings_in_comments_do_not_make_a_script() {
    // The first draft accepted any script that CONTAINED the run's words, comments included.
    let r = good("by-script-comment", PROD, "production", true);
    r.write(
        &p(PROD, "warrant/check_evidence.sh"),
        "# WARRANT_EVIDENCE_OUT=evidence/test-manifest.json --test warrant_evidence -p vos-fix-comp\nexit 0\n",
    );
    only(
        &run(&r, PROD),
        "CC04",
        "check_evidence.sh:1: differs from the script `vos-link gen-build` writes",
    );
}

#[test]
fn a_tolerance_script_nobody_declared_or_a_missing_one_fails() {
    let r = good("cc04-script", PROD, "production", false);
    r.write(&p(PROD, "warrant/check_evidence.sh"), "exit 0\n");
    only(
        &run(&r, PROD),
        "CC04",
        "exists but vos.toml declares no `excused-controls`",
    );
    let r = good("cc04-script2", PROD, "production", true);
    std::fs::remove_file(r.path(&p(PROD, "warrant/check_evidence.sh"))).unwrap();
    only(&run(&r, PROD), "CC04", "check_evidence.sh: missing");
}

#[test]
fn bypass_windows_comment_forms_hide_nothing() {
    // `::`, `@rem` and `rem` are comments to cmd.exe: a required line behind one never runs.
    for hide in [":: ", "@rem ", "rem ", "REM "] {
        let r = good("by-win", PROD, "production", false);
        r.edit(
            &p(PROD, "BUILD_windows"),
            DOCLINE,
            &format!("{hide}{DOCLINE}"),
        );
        only(&run(&r, PROD), "CC03", CC03);
    }
}

#[test]
fn bypass_a_required_line_moved_below_the_marker_satisfies_nothing() {
    let r = good("by-marker", PROD, "production", false);
    let b = p(PROD, "BUILD");
    r.edit(&b, DOCLINE, "");
    r.write(&b, format!("{}{DOCLINE}", r.read(&b)));
    only(&run(&r, PROD), "CC03", CC03);
}

#[test]
fn bypass_the_marker_removed_or_text_before_the_header() {
    let r = good("by-nomarker", PROD, "production", false);
    r.edit(&p(PROD, "BUILD"), MARKER, "# marker");
    only(&run(&r, PROD), "CC03", CC03);
    let r = good("by-before", PROD, "production", false);
    let b = p(PROD, "BUILD");
    r.write(&b, format!("exit 0\n{}", r.read(&b)));
    only(&run(&r, PROD), "CC03", CC03);
}

#[test]
fn each_missing_build_line_fails_in_both_build_files_and_names_the_line() {
    let cases = [
        "cargo test -p vos-fix-comp --lib --test behaviour -- --nocapture\n",
        "cargo test -p vos-fix-comp --features ghost --lib --test behaviour\n",
        DOCLINE,
        "cargo run -q -p vos-shape-check -- .\n",
        "cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .\n",
        "Cargo.toml -- check-doc .\n",
        "--example gen_evidence -- --check vos-fix-comp\n",
        "WARRANT_EVIDENCE_OUT=",
        "Cargo.toml -- check-component .\n",
    ];
    for file in ["BUILD", "BUILD_windows"] {
        for needle in cases {
            let r = good("cc03-missing", PROD, "production", false);
            let path = p(PROD, file);
            let kept: Vec<String> = r
                .read(&path)
                .lines()
                .filter(|l| !l.contains(needle.trim_end()))
                .map(String::from)
                .collect();
            assert!(
                kept.len() < r.read(&path).lines().count(),
                "{file}: no line has {needle:?}"
            );
            r.write(&path, kept.join("\n") + "\n");
            let o = run(&r, PROD);
            only(&o, "CC03", CC03);
            assert!(
                o.findings[0].message.starts_with(&format!("{file}:")),
                "{}",
                show(&o)
            );
        }
    }
}

#[test]
fn check_component_before_the_generated_suite_is_a_difference_too() {
    let r = good("cc03-order", PROD, "production", false);
    let path = p(PROD, "BUILD");
    let text = r.read(&path);
    let cc = "cargo run -q --manifest-path ../../../programs/rust/vos-link/Cargo.toml -- check-component .\n";
    let moved = text
        .replace(cc, "")
        .replace("# Lines 6 to 8", &format!("{cc}# Lines 6 to 8"));
    r.write(&path, moved);
    only(&run(&r, PROD), "CC03", CC03);
}

#[test]
fn the_finding_names_the_first_differing_line_expected_and_found() {
    let o = break_build(
        "cc03-text",
        "BUILD",
        DOCLINE,
        "cargo test -p vos-fix-comp --doc --nocapture\n",
    );
    only(&o, "CC03", "BUILD:8: ");
    has(&o, "CC03", "expected \"cargo test -p vos-fix-comp --doc\"");
    has(
        &o,
        "CC03",
        "found \"cargo test -p vos-fix-comp --doc --nocapture\"",
    );
}

// --- CC04: the sabotage plan ---------------------------------------------------------------

fn with_plan(label: &str, plan: &str) -> Outcome {
    let r = good(label, PROD, "production", false);
    r.write(&p(PROD, "warrant/sabotage.plan"), plan);
    run(&r, PROD)
}

#[test]
fn a_plan_that_drops_a_bound_fault_fails() {
    // wrong-ctor is bound to `f` and attacked by nothing.
    let plan = PLAN.replace("exhaustive/c2 wrong-ctor\n", "");
    only(
        &with_plan("pl-drop", &plan),
        "CC04",
        "no entry attacks the bound fault `wrong-ctor`",
    );
}

#[test]
fn a_plan_that_leaves_a_field_flip_operation_unattacked_fails() {
    let plan = PLAN.replace("exhaustive/c2 field-flip attack=h\n", "");
    only(
        &with_plan("pl-flip", &plan),
        "CC04",
        "no entry attacks `field-flip` on operation `h`",
    );
}

#[test]
fn plan_entries_are_held_to_the_document_and_the_bindings() {
    let o = with_plan(
        "pl-claim",
        &format!("{PLAN}exhaustive/ghost_claim field-zero\n"),
    );
    only(
        &o,
        "CC04",
        "sabotage.plan:7: \"ghost_claim\" is not a claim of the document",
    );
    let o = with_plan("pl-ex", &format!("{PLAN}example/nope field-zero\n"));
    only(&o, "CC04", "\"nope\" is not a example of the document");
    let o = with_plan("pl-fault", &format!("{PLAN}exhaustive/c1 invert\n"));
    only(&o, "CC04", "fault \"invert\" is bound to no operation");
    let o = with_plan(
        "pl-attack",
        &format!("{PLAN}exhaustive/c2 field-flip attack=f\n"),
    );
    only(
        &o,
        "CC04",
        "attack=\"f\" names no operation that lists `field-flip`",
    );
}

#[test]
fn malformed_and_repeated_plan_entries_fail_with_their_line() {
    for (entry, text) in [
        (
            "exhaustive/c1\n",
            "an entry is `<tier>/<name> <fault> [attack=<op>]`",
        ),
        ("exhaustive/c1 field-zero x y\n", "an entry is"),
        ("c1 field-zero\n", "is not `<tier>/<name>`"),
        ("property/c1 field-zero\n", "is not exhaustive or example"),
        ("exhaustive/c-1 field-zero\n", "is not a plain name"),
        ("exhaustive/c1 Field_Zero\n", "is not a plain fault name"),
        (
            "exhaustive/c1 field-zero attack=g\n",
            "takes `attack=<op>` and no other fault does",
        ),
        (
            "exhaustive/c1 field-flip\n",
            "takes `attack=<op>` and no other fault does",
        ),
        (
            "exhaustive/c1 field-flip attacks=g\n",
            "is not `attack=<op>`",
        ),
        ("exhaustive/c1 field-zero; rm -rf x\n", "an entry is"),
        ("exhaustive/c1 field-zero\n", "repeats the entry of line 2"),
    ] {
        let o = with_plan("pl-bad", &format!("{PLAN}{entry}"));
        has(&o, "CC04", "sabotage.plan:7:");
        has(&o, "CC04", text);
    }
}

#[test]
fn parse_plan_and_validate_plan_are_pure() {
    let (e, problems) = parse_plan(PLAN);
    assert!(problems.is_empty());
    assert_eq!(e.len(), 5);
    assert_eq!(
        (e[3].fault.as_str(), e[3].attack.as_deref(), e[3].line),
        ("field-flip", Some("g"), 5)
    );
    let faults = [("f".to_string(), vec!["field-zero".to_string()])]
        .into_iter()
        .collect();
    let v = validate_plan(&e[..1], &faults, &["c1".to_string()], &[], &[], &[]);
    assert!(v.is_empty(), "{v:?}");
    assert!(!validate_plan(&[], &faults, &[], &[], &[], &[]).is_empty());
}

#[test]
fn bindings_that_do_not_parse_or_a_document_that_does_not_parse_is_cc04() {
    let r = good("cc04-bind", PROD, "production", false);
    r.write(&p(PROD, "evidence.bindings"), "package\n");
    has(&run(&r, PROD), "CC04", "evidence.bindings:");
    let r = good("cc04-doc", PROD, "production", false);
    r.write(&format!("code/specs/{PROD}.warrant"), "not warrant\n");
    has(&run(&r, PROD), "CC04", "code/specs/vos-fix-comp.warrant:");
}

// --- CC05: the build-tool row --------------------------------------------------------------

fn with_rows(label: &str, go: &str) -> Outcome {
    let r = good(label, PROD, "production", false);
    r.write("code/programs/go/build-tool/repository_inputs.go", go);
    run(&r, PROD)
}

#[test]
fn a_missing_build_tool_row_fails() {
    let o = with_rows("cc05", "var repositoryInputConsumers = []repositoryInputConsumer{\n\tvosImplementationRow(\"vos-other\"),\n}\n");
    only(&o, "CC05", "has no line `vosImplementationRow(\"vos-fix-comp\"),` inside the repositoryInputConsumers slice");
}

#[test]
fn a_row_in_a_comment_a_raw_string_another_slice_or_a_lookalike_does_not_count() {
    let row = "\tvosImplementationRow(\"vos-fix-comp\"),\n";
    let open = "var repositoryInputConsumers = []repositoryInputConsumer{\n";
    for (what, go) in [
        (
            "line comment",
            format!("{open}\t// vosImplementationRow(\"vos-fix-comp\"),\n}}\n"),
        ),
        (
            "block comment",
            format!("{open}\t/*\n\tvosImplementationRow(\"vos-fix-comp\"),\n\t*/\n}}\n"),
        ),
        ("raw string", format!("{open}\tx := `\n{row}`\n}}\n")),
        (
            "another slice",
            format!("var other = []row{{\n{row}}}\n{open}}}\n"),
        ),
        ("after the slice closed", format!("{open}}}\n{row}")),
        (
            "lookalike",
            format!("{open}\tvosImplementationRow(\"vos-fix-comp-2\"),\n}}\n"),
        ),
        (
            "no comma",
            format!("{open}\tvosImplementationRow(\"vos-fix-comp\")\n}}\n"),
        ),
        (
            "a trailing comment hides nothing but a prefix does",
            format!("{open}\tx, vosImplementationRow(\"vos-fix-comp\"),\n}}\n"),
        ),
    ] {
        let o = with_rows("cc05-look", &go);
        assert!(
            o.findings.len() == 1 && o.findings[0].code == "CC05",
            "{what}: {}",
            show(&o)
        );
    }
}

#[test]
fn a_real_row_is_found_even_after_comments_and_strings_that_mention_quotes() {
    let go = "var repositoryInputConsumers = []repositoryInputConsumer{\n\t// a comment with a ` backtick and a \" quote\n\tvosImplementationRow(\"vos-fix-comp\"), // trailing\n\t{name: \"a`b\"},\n}\n";
    assert!(with_rows("cc05-ok", go).findings.is_empty());
}

#[test]
fn a_missing_build_tool_file_is_cc05() {
    let r = good("cc05-file", PROD, "production", false);
    std::fs::remove_file(r.path("code/programs/go/build-tool/repository_inputs.go")).unwrap();
    only(&run(&r, PROD), "CC05", "repository_inputs.go not found");
}

#[test]
fn strip_go_and_row_in_consumers_are_pure() {
    assert_eq!(strip_go("a // b\nc"), "a \nc");
    assert_eq!(
        strip_go(
            "a /* b
c */ d"
        ),
        "a 
  d"
    );
    assert_eq!(
        strip_go(
            "x `r
w` y"
        ),
        "x 
  y"
    );
    assert_eq!(strip_go("\"a // not\" 'x' '\\''"), "\"a // not\" 'x' '\\''");
    assert_eq!(strip_go("\"a\\\"//b\" c"), "\"a\\\"//b\" c");
    assert_eq!(strip_go("/* unterminated"), " ");
    assert_eq!(strip_go("`unterminated"), " ");
    assert!(row_in_consumers("var repositoryInputConsumers = []repositoryInputConsumer{\n\tvosImplementationRow(\"x\"),\n}\n", "x"));
    assert!(!row_in_consumers("", "x"));
}

// --- CC06: VOS-D082 -----------------------------------------------------------------------

#[test]
fn an_any_crate_without_publish_false_fails() {
    let r = good("cc06", ANY, "any", false);
    for cargo in [
        cargo_toml(ANY, ""),
        cargo_toml(ANY, "publish = true"),
        cargo_toml(ANY, "# publish = false"),
    ] {
        r.write(&p(ANY, "Cargo.toml"), cargo);
        only(&run(&r, ANY), "CC06", "must say `publish = false`");
    }
}

#[test]
fn publish_false_must_be_in_the_package_table() {
    let r = good("cc06-tbl", ANY, "any", false);
    r.write(
        &p(ANY, "Cargo.toml"),
        format!("[package]\nname = \"{ANY}\"\nversion = \"0.1.0\"\n[lib]\npublish = false\n"),
    );
    only(&run(&r, ANY), "CC06", "publish = false");
}

#[test]
fn an_unreadable_cargo_toml_is_cc06_and_a_production_crate_may_publish() {
    let r = good("cc06-bad", ANY, "any", false);
    r.write(&p(ANY, "Cargo.toml"), "[package\nname = \n");
    has(&run(&r, ANY), "CC06", "Cargo.toml:");
    let r = good("cc06-prod", PROD, "production", false);
    r.write(&p(PROD, "Cargo.toml"), cargo_toml(PROD, "publish = true"));
    assert!(run(&r, PROD).findings.is_empty());
}

// --- CC07: the coverage declaration -------------------------------------------------------

#[test]
fn a_missing_floor_fails_and_gen_build_refuses_it() {
    let r = good("cc07-none", PROD, "production", false);
    r.write(
        &p(PROD, "vos.toml"),
        "kind = \"production\"\ncontract = \"vos-sched-contract\"\nevidence-features = [\"ghost\"]\n",
    );
    only(&run(&r, PROD), "CC07", "declares no `coverage-floor`");
    only(
        &gen_build(&r.pkg(PROD), None).unwrap(),
        "CC07",
        "declares no `coverage-floor`",
    );
}

#[test]
fn a_floor_below_the_repository_minimum_fails() {
    let r = good("cc07-low", PROD, "production", false);
    r.edit(
        &p(PROD, "vos.toml"),
        "coverage-floor = 95",
        "coverage-floor = 94",
    );
    only(
        &run(&r, PROD),
        "CC07",
        "coverage-floor = 94 is below the repository minimum 95",
    );
}

// --- CC08: the manifest, bound to the document ----------------------------------------------

fn with_manifest(label: &str, text: String) -> Outcome {
    let r = good(label, PROD, "production", false);
    put_manifest(&r, PROD, text);
    run(&r, PROD)
}

#[test]
fn an_empty_manifest_fails_line_six() {
    let o = with_manifest(
        "cc08-empty",
        r#"{"schema":"warrant-test-evidence-v1","source_sha256":"@SRC@","spec_sha256":"@SPEC@","tests":[]}"#.into(),
    );
    only(
        &o,
        "CC08",
        "`tests` is empty: no test ran (line 6, none ran = fail)",
    );
}

#[test]
fn zero_exhaustive_cases_fail_line_six() {
    let o = with_manifest("cc08-zero", manifest(&["c1", "c2"], 4096, 5000, 0, &[]));
    has(&o, "CC08", "claim \"c1\" ran zero exhaustive cases");
    assert_eq!(o.findings.len(), 2, "{}", show(&o));
}

#[test]
fn fewer_than_4096_property_cases_fail_line_seven() {
    let o = with_manifest("cc08-prop", manifest(&["c1", "c2"], 4095, 5000, 10, &[]));
    has(
        &o,
        "CC08",
        "claim \"c1\" ran 4095 property cases, fewer than 4096 (line 7)",
    );
}

#[test]
fn fewer_than_5000_fuzz_iterations_fail_line_eight() {
    let o = with_manifest("cc08-fuzz", manifest(&["c1", "c2"], 4096, 4999, 10, &[]));
    has(
        &o,
        "CC08",
        "claim \"c1\" ran 4999 fuzz iterations, fewer than 5000 (line 8)",
    );
}

#[test]
fn the_bindings_may_raise_the_threshold_never_lower_it() {
    let r = good("cc08-bind", PROD, "production", false);
    r.write(&p(PROD, "evidence.bindings"), bindings(PROD, 8192, 9000));
    let o = run(&r, PROD);
    has(&o, "CC08", "ran 4096 property cases, fewer than 8192");
    has(&o, "CC08", "ran 5000 fuzz iterations, fewer than 9000");
    // A bindings file that asks for LESS than the spec does not lower the spec's number.
    r.write(&p(PROD, "evidence.bindings"), bindings(PROD, 10, 10));
    put_manifest(&r, PROD, manifest(&["c1", "c2"], 100, 100, 10, &[]));
    has(
        &run(&r, PROD),
        "CC08",
        "ran 100 property cases, fewer than 4096",
    );
}

#[test]
fn the_spec_states_the_thresholds_the_code_uses() {
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS04-component-catalogue.md"),
    )
    .unwrap();
    assert!(
        spec.contains(&format!("**{PROPERTY_CASES_MIN}** (this section, line 7)")),
        "VOS04 5.6 no longer states the property threshold"
    );
    assert!(
        spec.contains(&format!("**{FUZZ_ITERATIONS_MIN}** (line 8)")),
        "VOS04 5.6 no longer states the fuzz threshold"
    );
    assert!(spec.contains("| 7 | Property | 4096 random cases per claim"));
    assert!(spec.contains("5000 iterations"));
}

#[test]
fn a_manifest_that_drops_a_claim_of_the_document_fails() {
    // The first draft took the claim set from the manifest itself: dropping a claim passed.
    let o = with_manifest("cc08-drop", manifest(&["c1"], 4096, 5000, 10, &[]));
    has(
        &o,
        "CC08",
        "claim \"c2\" of the document has no exhaustive entry",
    );
    has(
        &o,
        "CC08",
        "claim \"c2\" of the document has no property entry",
    );
    has(&o, "CC08", "claim \"c2\" of the document has no fuzz entry");
    assert_eq!(o.findings.len(), 3, "{}", show(&o));
}

#[test]
fn a_manifest_that_invents_a_claim_fails() {
    let o = with_manifest(
        "cc08-invent",
        manifest(&["c1", "c2", "c3"], 4096, 5000, 10, &[]),
    );
    has(
        &o,
        "CC08",
        "the exhaustive entry for claim \"c3\" names no claim of the document",
    );
    assert_eq!(o.findings.len(), 3, "{}", show(&o));
}

#[test]
fn a_claim_missing_one_tier_fails_with_that_tier() {
    let m = manifest(&["c1", "c2"], 4096, 5000, 10, &[]).replace(
        r#"{"cases":4096,"claim":"c2","id":"m/property/c2","kind":"property","verdict":"pass"},"#,
        "",
    );
    only(
        &with_manifest("cc08-tier", m),
        "CC08",
        "claim \"c2\" of the document has no property entry (line 7",
    );
}

#[test]
fn examples_are_bound_to_the_document_too() {
    let m = manifest(&["c1", "c2"], 4096, 5000, 10, &[]);
    let gone = m.replace(
        r#"{"cases":1,"claim":"","id":"m/example/e1","kind":"example","verdict":"pass"},"#,
        "",
    );
    only(
        &with_manifest("cc08-ex1", gone),
        "CC08",
        "example \"e1\" of the document has no entry",
    );
    let extra = m.replace(
        r#""tests":["#,
        r#""tests":[{"cases":1,"claim":"","id":"m/example/zz","kind":"example","verdict":"pass"},"#,
    );
    only(
        &with_manifest("cc08-ex2", extra),
        "CC08",
        "the entry for example \"zz\" names no example of the document",
    );
}

#[test]
fn a_tier_whose_only_controls_failed_has_no_passing_control() {
    // `failing` marks both controls of the fuzz tier: a failing control counts for nothing.
    let o = with_manifest(
        "cc08-neg",
        manifest(
            &["c1", "c2"],
            4096,
            5000,
            10,
            &["m/negctl/fuzz/f/field-zero", "m/negctl/fuzz/g/field-flip"],
        ),
    );
    has(&o, "CC08", "no negative control PASSED in the fuzz tier");
    has(&o, "CC08", "negative control \"m/negctl/fuzz/f/field-zero\" did not pass and vos.toml does not excuse it");
    assert_eq!(o.findings.len(), 3, "{}", show(&o));
}

#[test]
fn a_failing_control_needs_its_whole_id_excused() {
    let m = manifest(
        &["c1", "c2"],
        4096,
        5000,
        10,
        &["m/negctl/exhaustive/g/field-flip"],
    );
    let o = with_manifest("cc08-fail", m.clone());
    only(
        &o,
        "CC08",
        "\"m/negctl/exhaustive/g/field-flip\" did not pass and vos.toml does not excuse it",
    );
    // A prefix, a suffix or a bare word excuses nothing (the first draft used `contains`).
    for excuse in ["m/negctl/exhaustive", "field-flip", "negctl"] {
        let r = good("cc08-short", PROD, "production", false);
        r.write(
            &p(PROD, "vos.toml"),
            vos_toml(
                "production",
                &format!("excused-controls = [{{ id = \"{excuse}\", reason = \"VOS-D098\" }}]\n"),
            ),
        );
        put_manifest(&r, PROD, m.clone());
        gen_build(&r.pkg(PROD), None).unwrap();
        let o = run(&r, PROD);
        has(&o, "CC08", "did not pass and vos.toml does not excuse it");
        has(
            &o,
            "CC08",
            &format!("vos.toml excuses \"{excuse}\" but that control passes or does not exist"),
        );
    }
}

#[test]
fn a_stale_excuse_fails() {
    let r = good("cc08-stale", PROD, "production", true);
    put_manifest(&r, PROD, manifest(&["c1", "c2"], 4096, 5000, 10, &[]));
    let o = run(&r, PROD);
    has(&o, "CC08", "vos.toml excuses \"m/negctl/exhaustive/g/field-flip\" but that control passes or does not exist");
    assert_eq!(o.findings.len(), 3, "{}", show(&o));
}

#[test]
fn a_failing_test_that_is_not_a_control_is_never_excused() {
    let r = good("cc08-nonctl", PROD, "production", true);
    let m = manifest(&["c1", "c2"], 4096, 5000, 10, &EXCUSED_IDS).replace(
        r#""id":"m/exhaustive/c1","kind":"exhaustive","verdict":"pass""#,
        r#""id":"m/exhaustive/c1","kind":"exhaustive","verdict":"fail""#,
    );
    put_manifest(&r, PROD, m);
    only(
        &run(&r, PROD),
        "CC08",
        "test \"m/exhaustive/c1\" has verdict \"fail\", not pass",
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
            r#"{"schema":"warrant-test-evidence-v1","source_sha256":"@SRC@","spec_sha256":"@SPEC@"}"#.into(),
        ),
        "CC08",
        "no `tests` array",
    );
    let o = with_manifest(
        "cc08-schema",
        manifest(&["c1", "c2"], 4096, 5000, 10, &[]).replace("evidence-v1", "evidence-v2"),
    );
    only(&o, "CC08", "`schema` is not `warrant-test-evidence-v1`");
    let o = with_manifest(
        "cc08-malformed",
        manifest(&["c1", "c2"], 4096, 5000, 10, &[])
            .replace(r#""tests":["#, r#""tests":[5,{"kind":"x"},"#),
    );
    only(&o, "CC08", "2 entries lack a string kind, id or verdict");
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
    let r = good("cc-cli", PROD, "production", false);
    let dir = r.pkg(PROD).to_string_lossy().into_owned();
    let ok = cli(&["check-component", &dir]);
    let text = String::from_utf8_lossy(&ok.stdout).into_owned();
    assert_eq!(ok.status.code(), Some(0), "{text}");
    assert!(
        text.contains("vos-link check-component: crate vos-fix-comp, kind production"),
        "{text}"
    );
    assert!(text.contains("result: PASS, 0 findings"), "{text}");

    let g = cli(&["gen-build", &dir]);
    let text = String::from_utf8_lossy(&g.stdout).into_owned();
    assert_eq!(g.status.code(), Some(0), "{text}");
    assert!(text.contains("unchanged BUILD, BUILD_windows"), "{text}");

    std::fs::remove_file(r.path(&p(PROD, "README.md"))).unwrap();
    let bad = cli(&["check-component", &dir, "--root", &r.root.to_string_lossy()]);
    let text = String::from_utf8_lossy(&bad.stdout).into_owned();
    assert_eq!(bad.status.code(), Some(1), "{text}");
    assert!(
        text.contains("CC02 file set: README.md is missing"),
        "{text}"
    );
    assert!(text.contains("result: FAIL, 1 findings"), "{text}");

    assert_eq!(
        cli(&["check-component", "/no/such/crate-dir"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        cli(&["gen-build", "/no/such/crate-dir"]).status.code(),
        Some(2)
    );
    let usage = cli(&["check-component"]);
    assert_eq!(usage.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&usage.stderr).contains("no crate directory"));
}

#[test]
fn a_symlinked_file_is_no_verdict_not_a_pass() {
    #[cfg(unix)]
    {
        let r = good("cc-sym", PROD, "production", false);
        let path = r.path(&p(PROD, "README.md"));
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(r.path(&p(PROD, "BUILD")), &path).unwrap();
        let e = check_component(&r.pkg(PROD), None).unwrap_err();
        assert!(e.to_string().contains("is a symlink"), "{e}");
    }
}

#[test]
fn an_oversized_build_file_is_no_verdict() {
    let r = good("cc-big", PROD, "production", false);
    r.write(&p(PROD, "BUILD"), "x".repeat(300 * 1024));
    let e = check_component(&r.pkg(PROD), None).unwrap_err();
    assert!(e.to_string().contains("over the"), "{e}");
}

// --- review round 2 ------------------------------------------------------------------------

use vos_link::build_gen::{
    backlog_row_status, interpolation_problems, GenInputs, PlanEntry, RowState,
};
use vos_shape_check::manifest::Excuse;

/// M1: a feature pasted into BUILD is shell text.  Each hostile value is refused at generation.
#[test]
fn m1_a_hostile_or_unlisted_evidence_feature_is_refused_by_gen_build_and_check() {
    for feature in [
        "ghost #",
        "ghost&&calc",
        "$(id)",
        "ghost;x",
        "Ghost",
        "gh ost",
        "ghost`x`",
        "other",
        "9ghost",
    ] {
        let r = good("m1", PROD, "production", false);
        r.write(
            &p(PROD, "vos.toml"),
            format!(
                "kind = \"production\"\nevidence-features = [\"{feature}\"]\ncoverage-floor = 95\n"
            ),
        );
        let before = r.read(&p(PROD, "BUILD"));
        let g = gen_build(&r.pkg(PROD), None).unwrap();
        assert_eq!(g.findings.len(), 1, "{feature}: {}", show(&g));
        assert_eq!(g.findings[0].code, "CC04");
        assert!(
            g.findings[0].message.contains("evidence feature"),
            "{feature}: {}",
            show(&g)
        );
        assert_eq!(
            r.read(&p(PROD, "BUILD")),
            before,
            "{feature}: gen-build must write nothing"
        );
        let c = run(&r, PROD);
        has(&c, "CC04", "evidence feature");
    }
    // The listed ones are fine.
    for feature in ["ghost", "cov"] {
        let r = good("m1-ok", PROD, "production", false);
        r.write(
            &p(PROD, "vos.toml"),
            format!(
                "kind = \"production\"\nevidence-features = [\"{feature}\"]\ncoverage-floor = 95\n"
            ),
        );
        assert!(gen_build(&r.pkg(PROD), None).unwrap().findings.is_empty());
    }
}

/// M1: the one place every interpolated value is checked.  Each field of `GenInputs` that
/// reaches a BUILD line or the script, with a hostile value; and the clean base passes.
#[test]
fn m1_every_interpolated_value_is_checked_in_one_place() {
    let entry = |tier: &str, name: &str, fault: &str, attack: Option<&str>| PlanEntry {
        tier: tier.into(),
        name: name.into(),
        fault: fault.into(),
        attack: attack.map(String::from),
        line: 1,
    };
    let base = GenInputs {
        krate: "vos-x".into(),
        features: vec!["ghost".into()],
        floor: 95,
        excused: vec![Excuse {
            id: "m/negctl/exhaustive/g/field-flip".into(),
            reason: "VOS-D098".into(),
        }],
        plan: vec![
            entry("exhaustive", "c1", "field-zero", None),
            entry("exhaustive", "c1", "field-flip", Some("g")),
        ],
        claims: vec![],
        examples: vec![],
        faults: Default::default(),
        calls: vec![],
        property_cases: 4096,
        fuzz_iterations: 5000,
    };
    assert!(interpolation_problems(&base).is_empty());
    let mut cases: Vec<(&str, GenInputs)> = Vec::new();
    let mut with = |what: &'static str, f: &dyn Fn(&mut GenInputs)| {
        let mut g = base.clone();
        f(&mut g);
        cases.push((what, g));
    };
    with("crate name", &|g| g.krate = "x; rm".into());
    with("crate name case", &|g| g.krate = "Vos".into());
    with("feature text", &|g| g.features = vec!["ghost #".into()]);
    with("feature list", &|g| g.features = vec!["nolisted".into()]);
    with("plan tier", &|g| g.plan[0].tier = "x y".into());
    with("plan name", &|g| g.plan[0].name = "c1 && x".into());
    with("plan fault", &|g| g.plan[0].fault = "f; x".into());
    with("plan attack", &|g| g.plan[1].attack = Some("g`x`".into()));
    with("excused id", &|g| {
        g.excused[0].id = "a'; rm -rf x; '".into()
    });
    with("excused reason", &|g| {
        g.excused[0].reason = "VOS-D98".into()
    });
    for (what, g) in cases {
        assert!(
            !interpolation_problems(&g).is_empty(),
            "{what} was not refused"
        );
    }
}

/// M2: a manifest of another document, or of other source, is stale evidence.
#[test]
fn m2_a_manifest_with_the_wrong_spec_digest_is_refused() {
    let r = good("m2-spec", PROD, "production", false);
    let path = p(PROD, "evidence/test-manifest.json");
    let m = r.read(&path);
    let digest = m
        .split("\"spec_sha256\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    r.write(&path, m.replace(&digest, &"0".repeat(64)));
    only(&run(&r, PROD), "CC08", "manifest: spec_sha256 is \"0000");
    // Editing the document after the run makes the committed manifest stale in the same way.
    let r = good("m2-doc", PROD, "production", false);
    r.write(
        &format!("code/specs/{PROD}.warrant"),
        format!("{DOC}  -- edited\n"),
    );
    only(
        &run(&r, PROD),
        "CC08",
        "the evidence was not run against this document",
    );
}

#[test]
fn m2_a_manifest_of_other_source_is_refused() {
    let r = good("m2-src", PROD, "production", false);
    r.write(&p(PROD, "src/lib.rs"), "pub fn changed() {}\n");
    only(
        &run(&r, PROD),
        "CC08",
        "the evidence was not run against this source",
    );
    let r = good("m2-nosrc", PROD, "production", false);
    std::fs::remove_dir_all(r.path(&p(PROD, "src"))).unwrap();
    only(&run(&r, PROD), "CC08", "manifest: cannot digest src/");
    let r = good("m2-absent", PROD, "production", false);
    let path = p(PROD, "evidence/test-manifest.json");
    let m = r
        .read(&path)
        .replace("\"source_sha256\"", "\"x\"")
        .replace("\"spec_sha256\"", "\"y\"");
    r.write(&path, m);
    let o = run(&r, PROD);
    has(&o, "CC08", "spec_sha256 is \"(absent)\"");
    has(&o, "CC08", "source_sha256 is \"(absent)\"");
}

/// M2: the generated region removes the manifest before the run, so a suite that exits 0
/// without rewriting it leaves NO file and fails the file set, instead of passing on a stale one.
#[test]
fn m2_a_manifest_that_was_not_rewritten_is_a_missing_file() {
    let r = good("m2-gone", PROD, "production", false);
    let b = r.read(&p(PROD, "BUILD"));
    let rm = b
        .find("\nrm -f evidence/test-manifest.json\n")
        .expect("rm line");
    let run_at = b.find("\nWARRANT_EVIDENCE_OUT=").expect("run line");
    assert!(rm < run_at, "the delete must come first");
    // What the BUILD leaves behind when the run writes nothing:
    std::fs::remove_file(r.path(&p(PROD, "evidence/test-manifest.json"))).unwrap();
    only(
        &run(&r, PROD),
        "CC02",
        "file set: evidence/test-manifest.json is missing",
    );
    // And the delete cannot be dropped from BUILD: that is a different region.
    let r = good("m2-norm", PROD, "production", false);
    r.edit(&p(PROD, "BUILD"), "rm -f evidence/test-manifest.json\n", "");
    only(&run(&r, PROD), "CC03", CC03);
    let r = good("m2-norm-w", PROD, "production", false);
    r.edit(
        &p(PROD, "BUILD_windows"),
        "if exist evidence\\test-manifest.json del /f /q evidence\\test-manifest.json\n",
        "",
    );
    only(&run(&r, PROD), "CC03", CC03);
}

/// M3: every claim that calls a bound operation needs an entry.
#[test]
fn m3_a_claim_that_calls_a_bound_operation_and_is_never_attacked_fails() {
    let r = good("m3", PROD, "production", false);
    r.write(
        &format!("code/specs/{PROD}.warrant"),
        format!("{DOC}  claim c3:\n    forall v: Bool. g(v) == g(v)\n"),
    );
    only(
        &run(&r, PROD),
        "CC04",
        "claim `c3` calls the bound operation(s) g and no entry attacks it",
    );
    // An entry makes it pass the plan check (the manifest is the next check's).
    r.write(
        &p(PROD, "warrant/sabotage.plan"),
        format!("{PLAN}exhaustive/c3 field-zero\n"),
    );
    assert!(!run(&r, PROD)
        .findings
        .iter()
        .any(|f| f.message.contains("no entry attacks it")));
    // A claim that calls no bound operation needs none; a comment is not a call.
    let r = good("m3-none", PROD, "production", false);
    r.write(
        &format!("code/specs/{PROD}.warrant"),
        format!("{DOC}  claim c3:\n    forall v: Bool. v == v -- f(v)\n"),
    );
    assert!(!run(&r, PROD).findings.iter().any(|f| f.code == "CC04"));
}

#[test]
fn m3_a_claim_whose_faults_are_all_excused_is_exempt() {
    use vos_link::build_gen::validate_plan;
    let faults = [("g".to_string(), vec!["field-flip".to_string()])]
        .into_iter()
        .collect();
    let claims = vec!["k".to_string()];
    let calls = vec![("k".to_string(), vec!["g".to_string()])];
    let plan: Vec<PlanEntry> = vec![];
    let excused = [Excuse {
        id: "m/negctl/exhaustive/g/field-flip".into(),
        reason: "VOS-D098".into(),
    }];
    let none = validate_plan(&plan, &faults, &claims, &[], &calls, &[]);
    assert!(
        none.iter().any(|m| m.contains("claim `k` calls")),
        "{none:?}"
    );
    let some = validate_plan(&plan, &faults, &claims, &[], &calls, &excused);
    assert!(
        !some.iter().any(|m| m.contains("claim `k` calls")),
        "{some:?}"
    );
}

/// M4: an excuse names a backlog row that exists and is not done.
#[test]
fn m4_an_excuse_needs_an_open_backlog_row() {
    let ex = |reason: &str| EXCUSED.replace("VOS-D098", reason);
    let set = |label: &str, reason: &str, backlog: Option<&str>| {
        let r = good(label, PROD, "production", false);
        r.write(&p(PROD, "vos.toml"), vos_toml("production", &ex(reason)));
        if let Some(b) = backlog {
            r.write("code/specs/VOS-BACKLOG.md", b);
        } else {
            std::fs::remove_file(r.path("code/specs/VOS-BACKLOG.md")).unwrap();
        }
        let o = gen_build(&r.pkg(PROD), None).unwrap();
        (r, o)
    };
    // A row that is done: the excuse has expired.
    let (_r, o) = set("m4-done", "VOS-D065", Some(BACKLOG));
    assert!(o.findings.iter().all(|f| f.code == "CC04"), "{}", show(&o));
    has(
        &o,
        "CC04",
        "VOS-D065 has status \"done** (somewhere)\", which is not open or queued",
    );
    // A row that does not exist.
    let (_r, o) = set("m4-absent", "VOS-D999", Some(BACKLOG));
    has(
        &o,
        "CC04",
        "VOS-D999 is not a row of code/specs/VOS-BACKLOG.md",
    );
    // No backlog at all.
    let (_r, o) = set("m4-nofile", "VOS-D098", None);
    has(&o, "CC04", "is not a row");
    // An open row is accepted, and check-component agrees.
    let (r, o) = set("m4-open", "VOS-D098", Some(BACKLOG));
    assert!(o.findings.is_empty(), "{}", show(&o));
    assert!(r
        .read(&p(PROD, "warrant/check_evidence.sh"))
        .contains("#   m/negctl/exhaustive/g/field-flip (VOS-D098)"));
    // The reason must be a backlog reference at all (vos.toml refuses it).
    let r = good("m4-bad", PROD, "production", false);
    r.write(&p(PROD, "vos.toml"), vos_toml("production", &ex("because")));
    only(&run(&r, PROD), "CC01", "backlog row");
}

#[test]
fn backlog_row_status_is_an_allowlist_of_open_statuses() {
    let row = |id: &str, st: &str| format!("| {id} | a | b | P2 | {st} |\n");
    let mut b = String::new();
    for (id, st) in [
        ("VOS-D001", "queued"),
        ("VOS-D002", "**done** (#1)"),
        ("VOS-D003", "Done"),
        ("VOS-D004", "open"),
        ("VOS-D005", "resolved in VOS-039a"),
        ("VOS-D006", "withdrawn"),
        ("VOS-D007", "done in this PR (VOS-039a)"),
        ("VOS-D008", "queued (folded into VOS-D009)"),
        ("VOS-D009", "partly done"),
        ("VOS-D010", "in progress"),
        ("VOS-D012", "**Queued**"),
    ] {
        b += &row(id, st);
    }
    b += &row("VOS-D011", "queued");
    b += &row("VOS-D011", "queued");
    b += "VOS-D013 | no leading bar |\n";
    let state = |id: &str| backlog_row_status(&b, id);
    for open in ["VOS-D001", "VOS-D004", "VOS-D008", "VOS-D012"] {
        assert_eq!(state(open), RowState::Open, "{open}");
    }
    for (closed, st) in [
        ("VOS-D002", "done** (#1)"),
        ("VOS-D003", "Done"),
        ("VOS-D005", "resolved in VOS-039a"),
        ("VOS-D006", "withdrawn"),
        ("VOS-D007", "done in this PR (VOS-039a)"),
        ("VOS-D009", "partly done"),
        ("VOS-D010", "in progress"),
    ] {
        assert!(
            matches!(state(closed), RowState::Closed(ref s) if s.starts_with(&st[..4])),
            "{closed}: {:?}",
            state(closed)
        );
    }
    assert_eq!(state("VOS-D011"), RowState::Duplicate);
    assert_eq!(state("VOS-D013"), RowState::Missing);
    assert_eq!(state("VOS-D099"), RowState::Missing);
}

/// M4 through the whole check: a resolved, withdrawn or "done in this PR" row, a duplicate id
/// and an absent row are all refused as an excuse's reason.
#[test]
fn m4_resolved_withdrawn_done_in_this_pr_and_duplicate_rows_do_not_excuse() {
    for (what, backlog, want) in [
        (
            "resolved",
            "| VOS-D098 | x | y | P2 | resolved in VOS-040 |\n",
            "has status \"resolved in VOS-040\"",
        ),
        (
            "withdrawn",
            "| VOS-D098 | x | y | P2 | withdrawn |\n",
            "has status \"withdrawn\"",
        ),
        (
            "done in",
            "| VOS-D098 | x | y | P2 | done in this PR (#1) |\n",
            "has status \"done in this PR (#1)\"",
        ),
        (
            "duplicate",
            "| VOS-D098 | x | y | P2 | queued |\n| VOS-D098 | x | y | P2 | queued |\n",
            "more than one backlog row",
        ),
    ] {
        let r = good("m4-allow", PROD, "production", false);
        r.write(&p(PROD, "vos.toml"), vos_toml("production", EXCUSED));
        r.write("code/specs/VOS-BACKLOG.md", backlog);
        let o = gen_build(&r.pkg(PROD), None).unwrap();
        assert!(!o.findings.is_empty(), "{what}");
        assert!(
            o.findings.iter().all(|f| f.code == "CC04"),
            "{what}: {}",
            show(&o)
        );
        has(&o, "CC04", want);
    }
    let r = good("m4-allow-ok", PROD, "production", false);
    r.write(&p(PROD, "vos.toml"), vos_toml("production", EXCUSED));
    r.write(
        "code/specs/VOS-BACKLOG.md",
        "| VOS-D098 | x | y | P2 | open |\n",
    );
    assert!(gen_build(&r.pkg(PROD), None).unwrap().findings.is_empty());
}

// --- CC09: the `bound` rows against the ceilings (VOS-039b-2b) -------------------------------------

/// Rewrite the fixture document with `f`, and re-bind the evidence manifest to it (CC08 would
/// otherwise report the stale digest, and each control must fail for exactly one reason).
fn with_doc(r: &Repo, name: &str, f: impl Fn(&str) -> String) {
    with_doc_from(r, name, DOC, f)
}

/// [`with_doc`] over another base document (the `-any` fixture's).
fn with_doc_from(r: &Repo, name: &str, base: &str, f: impl Fn(&str) -> String) {
    let doc = f(base);
    assert_ne!(doc, base, "the edit changed nothing");
    r.write(&format!("code/specs/{name}.warrant"), doc);
    put_manifest(r, name, manifest(&["c1", "c2"], 4096, 5000, 10, &[]));
}

#[test]
fn cc09_the_passing_fixture_carries_its_notes() {
    let r = good("cc09-pass", PROD, "production", false);
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    // `alpha_w` (ghost-only, a row above every ceiling) is listed, not compared; `depth` absent.
    assert!(o
        .notes
        .iter()
        .any(|n| n.contains("\"alpha_w\" map to no contract ceiling")));
    assert!(o
        .notes
        .iter()
        .any(|n| n.contains("`depth` is not evidenced for")));
}

#[test]
fn cc09_control_one_step_over_the_ceiling_on_one_image_operation() {
    let r = good("cc09-steps", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound insert_img:\n    steps <= 160",
            "bound insert_img:\n    steps <= 161",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "bound \"insert_img\" row \"steps\" <= 161 is above the ceiling 160 of the contract operation \"insert\"",
    );
}

#[test]
fn cc09_control_a_heap_bound_above_zero() {
    let r = good("cc09-heap", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound init:\n    steps <= 80\n    heap <= 0",
            "bound init:\n    steps <= 80\n    heap <= 1",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "row \"heap\" <= 1 is above the ceiling 0 of the contract operation \"init\"",
    );
}

#[test]
fn cc09_control_a_deleted_heap_or_steps_row() {
    let r = good("cc09-noheap", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound remove:\n    steps <= 160\n    heap <= 0\n",
            "bound remove:\n    steps <= 160\n",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "operation \"remove\" (ceiling operation \"remove\") has no heap row",
    );
    let r = good("cc09-nosteps", PROD, "any", false);
    with_doc_from(&r, PROD, &any_doc(), |d| {
        d.replacen(
            "bound dispatch_out:\n    steps <= 160\n    heap <= 0\n",
            "bound dispatch_out:\n    heap <= 0\n",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "operation \"dispatch_out\" (ceiling operation \"dispatch\") has no steps row",
    );
}

#[test]
fn cc09_control_a_deleted_bound_block_and_a_deleted_operation() {
    // The `bound` block is gone but the operation is still declared: both rows are missing.
    let r = good("cc09-noblock", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen("  bound remove:\n    steps <= 160\n    heap <= 0\n", "", 1)
    });
    let o = run(&r, PROD);
    assert_eq!(o.findings.len(), 2, "{}", show(&o));
    has(
        &o,
        "CC09",
        "operation \"remove\" (ceiling operation \"remove\") has no steps row",
    );
    has(&o, "CC09", "has no heap row");
    // The operation itself is gone: the ceiling operation has no mapped operation at all.
    let r = good("cc09-noop", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen("  op remove(s: Impl, t: U8) -> Impl\n", "", 1)
            .replacen("  bound remove:\n    steps <= 160\n    heap <= 0\n", "", 1)
    });
    only(
        &run(&r, PROD),
        "CC09",
        "ceilings for operation \"remove\" but no operation of the document maps to it",
    );
}

#[test]
fn cc09_control_a_bound_that_is_not_a_literal_is_not_a_bound() {
    let r = good("cc09-expr", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound init:\n    steps <= 80",
            "bound init:\n    steps <= 40 + 40",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "bound \"init\" row \"steps\" <= is not an integer literal",
    );
}

#[test]
fn cc09_depth_is_compared_when_present_and_a_note_when_absent() {
    let r = good("cc09-depth", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound init:\n    steps <= 80",
            "bound init:\n    depth <= 3\n    steps <= 80",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "row \"depth\" <= 3 is above the ceiling 2",
    );
    let r = good("cc09-depth-ok", PROD, "production", false);
    with_doc(&r, PROD, |d| {
        d.replacen(
            "bound init:\n    steps <= 80",
            "bound init:\n    depth <= 2\n    steps <= 80",
            1,
        )
    });
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    // `init` now has its depth row, so it is not in the absent list.
    let n = o
        .notes
        .iter()
        .find(|n| n.contains("`depth` is not evidenced"))
        .unwrap();
    assert!(!n.contains("\"init\""), "{n}");
}

#[test]
fn cc09_a_reference_crate_needs_no_rows_but_still_realises_every_ceiling_operation() {
    // No `bound` block at all: a reference implementation has none (VOS02 section 4.7).
    let r = good("cc09-ref", PROD, "reference", false);
    with_doc(&r, PROD, |d| {
        let cut = d.find("  bound init:").unwrap();
        d[..cut].to_string() + &d[d.find("  claim c1:").unwrap()..]
    });
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
    // Rows that ARE present are compared.
    let r = good("cc09-ref-over", PROD, "reference", false);
    with_doc(&r, PROD, |d| d.replacen("steps <= 80", "steps <= 81", 1));
    only(
        &run(&r, PROD),
        "CC09",
        "row \"steps\" <= 81 is above the ceiling 80",
    );
    // And an operation of the contract that nobody realises fails for every kind.
    let r = good("cc09-ref-noop", PROD, "reference", false);
    with_doc(&r, PROD, |d| {
        d.replacen("  op init() -> Impl\n", "", 1).replacen(
            "  bound init:\n    steps <= 80\n    heap <= 0\n",
            "",
            1,
        )
    });
    only(
        &run(&r, PROD),
        "CC09",
        "ceilings for operation \"init\" but no operation",
    );
}

#[test]
fn cc09_cannot_be_applied_without_a_contract_or_a_document() {
    let r = good("cc09-nocontract", PROD, "production", false);
    r.edit(
        &p(PROD, "vos.toml"),
        "contract = \"vos-sched-contract\"\n",
        "",
    );
    only(
        &run(&r, PROD),
        "CC09",
        "vos.toml names no valid `contract` package",
    );
    let r = good("cc09-nocontract2", PROD, "production", false);
    r.edit(
        &p(PROD, "vos.toml"),
        "\"vos-sched-contract\"",
        "\"vos-nothere\"",
    );
    only(
        &run(&r, PROD),
        "CC09",
        "contract package \"vos-nothere\" has no contract.vos",
    );
    let r = good("cc09-badcontract", PROD, "production", false);
    r.write(
        "code/packages/rust/vos-sched-contract/contract.vos",
        "garbage\n",
    );
    only(
        &run(&r, PROD),
        "CC09",
        "contract.vos of \"vos-sched-contract\"",
    );
    let r = good("cc09-nodoc", PROD, "production", false);
    std::fs::remove_file(r.path(&format!("code/specs/{PROD}.warrant"))).unwrap();
    has(
        &run(&r, PROD),
        "CC09",
        "is missing, so there are no bound rows to compare",
    );
    let r = good("cc09-baddoc", PROD, "production", false);
    r.write(&format!("code/specs/{PROD}.warrant"), "not a document\n");
    has(&run(&r, PROD), "CC09", "code/specs/vos-fix-comp.warrant:");
}

// --- CC11: oracle preservation of every `-any` (VOS-039c, VOS-D080) ---------------------------------

/// The `-any` fixture with its document, bindings and plan rewritten, regenerated (BUILD follows the
/// plan) and re-bound to the manifest, so each control fails for exactly one reason.
fn any_with(
    label: &str,
    doc: impl Fn(&str) -> String,
    bind: impl Fn(&str) -> String,
    plan: impl Fn(&str) -> String,
) -> Outcome {
    let r = good(label, ANY, "any", false);
    let d = doc(&any_doc());
    r.write(&format!("code/specs/{ANY}.warrant"), d.clone());
    // The contract owns `c1` with the text this document has (a test that changes `c1` changes the
    // contract's too, so the claim stays the contract's; `cc11_a_decoy_claim...` breaks that on purpose).
    r.write(CONTRACT_ANY, contract_c1(&d));
    r.write(&p(ANY, "evidence.bindings"), bind(&any_bindings(ANY)));
    r.write(&p(ANY, "warrant/sabotage.plan"), plan(&any_plan()));
    put_manifest(&r, ANY, manifest(&["c1", "c2"], 4096, 5000, 10, &[]));
    let g = gen_build(&r.pkg(ANY), None).expect("a verdict");
    assert!(g.findings.is_empty(), "{}", show(&g));
    run(&r, ANY)
}

const CONTRACT_ANY: &str = "code/packages/rust/vos-sched-contract/warrant/obligations-any.part";

/// The two lines of claim `c1` of a document, as the contract's `any` part.
fn contract_c1(doc: &str) -> String {
    let at = doc.find("  claim c1:\n").expect("claim c1");
    let rest = &doc[at..];
    let end = rest.find("  claim c2:").expect("claim c2");
    rest[..end].to_string()
}

/// `g` takes the `field-flip-last` fault `f` lost, so the plan's entry for it stays bound to an
/// operation and only CC11 has something to say.
fn give_g_the_fault(b: String) -> String {
    b.replace(
        "op g export=vos_ghost_g ret=u8 faults=field-flip",
        "op g export=vos_ghost_g ret=u8 faults=field-flip,field-flip-last",
    )
}

fn same(s: &str) -> String {
    s.to_string()
}

const FLIP_LAST: &str = "no `exhaustive/<claim> field-flip-last` entry";

#[test]
fn cc11_the_passing_any_fixture_is_clean() {
    let o = any_with("cc11-pass", same, same, same);
    assert!(o.findings.is_empty(), "{}", show(&o));
}

#[test]
fn cc11_a_state_producing_operation_without_the_fault_fails() {
    // `f` loses the fault; `g` takes it, so the bound fault stays attacked and the plan is valid.
    let o = any_with(
        "cc11-nofault",
        same,
        |b| {
            b.replace("wrong-ctor,field-flip-last", "wrong-ctor")
                .replace(
                    "op g export=vos_ghost_g ret=u8 faults=field-flip",
                    "op g export=vos_ghost_g ret=u8 faults=field-flip,field-flip-last",
                )
        },
        same,
    );
    only(
        &o,
        "CC11",
        "operation \"f\" produces a state from a state but does not list `field-flip-last`",
    );
}

/// A type alias is the same state to the generator (it resolves `model State = Impl`), so it is one
/// to CC11: dropping `field-flip-last` from `f` fails whichever way `f`'s signature spells `Impl`.
/// (The round-2 finding: a gate that matched the literal name `Impl` saw `State -> State` as no
/// successor and passed.)
#[test]
fn cc11_an_aliased_state_is_still_a_successor() {
    let drop = |b: &str| {
        b.replace("wrong-ctor,field-flip-last", "wrong-ctor")
            .replace(
                "op g export=vos_ghost_g ret=u8 faults=field-flip",
                "op g export=vos_ghost_g ret=u8 faults=field-flip,field-flip-last",
            )
    };
    for (label, aliases, sig) in [
        (
            "cc11-alias-both",
            "  model State = Impl\n",
            "op f(s: State, t: U8) -> State",
        ),
        (
            "cc11-alias-param",
            "  model State = Impl\n",
            "op f(s: State, t: U8) -> Impl",
        ),
        (
            "cc11-alias-return",
            "  model State = Impl\n",
            "op f(s: Impl, t: U8) -> State",
        ),
        (
            "cc11-alias-chain",
            "  model State = Impl\n  model St2 = State\n",
            "op f(s: St2, t: U8) -> St2",
        ),
    ] {
        let o = any_with(
            label,
            |d| {
                d.replacen(
                    "  record Impl(x: U8, c: U8)\n",
                    &format!("  record Impl(x: U8, c: U8)\n{aliases}"),
                    1,
                )
                .replacen("op f(s: Impl, t: U8) -> Impl", sig, 1)
            },
            drop,
            same,
        );
        only(
            &o,
            "CC11",
            "operation \"f\" produces a state from a state but does not list `field-flip-last`",
        );
    }
}

#[test]
fn cc11_an_operation_no_claim_attacks_with_the_oracle_fault_fails() {
    // The entry names `c2`, which calls `h` and no state-producing operation; and an `example` entry
    // never counts: only an exhaustive claim that calls the operation can be shown to fail.
    for (label, plan) in [
        ("cc11-other-claim", "exhaustive/c2 field-flip-last\n"),
        ("cc11-example-tier", "example/e1 field-flip-last\n"),
    ] {
        let o = any_with(label, same, same, |p| {
            p.replace("exhaustive/c1 field-flip-last\n", plan)
        });
        only(&o, "CC11", FLIP_LAST);
        has(
            &o,
            "CC11",
            "calls \"f\" and no other state-producing operation",
        );
    }
}

#[test]
fn cc11_a_claim_that_calls_two_state_producing_operations_attacks_neither() {
    // `c1` calls both `f` and `f2`: a flipped oracle in either would fail it, so it proves neither.
    let o = any_with(
        "cc11-two",
        |d| {
            d.replacen(
                "  op g() -> U8\n",
                "  op f2(s: Impl, t: U8) -> Impl\n  op g() -> U8\n",
                1,
            )
            .replacen(
                "  labels f:",
                "  labels f2:\n    s public\n    t public\n  labels f:",
                1,
            )
            .replacen(
                "f(Impl(1u8, 2u8), 3u8).c == 2u8",
                "f(Impl(1u8, 2u8), 3u8).c == 2u8 and f2(Impl(1u8, 2u8), 3u8).c == 2u8",
                1,
            )
        },
        |b| {
            b.replacen(
                "op g export",
                "op f2 export=vos_ghost_f2 args=Impl,u8 ret=Impl faults=field-zero,field-flip-last\nop g export",
                1,
            )
        },
        same,
    );
    assert_eq!(o.findings.len(), 2, "{}", show(&o));
    has(
        &o,
        "CC11",
        "calls \"f\" and no other state-producing operation",
    );
    has(
        &o,
        "CC11",
        "calls \"f2\" and no other state-producing operation",
    );
}

#[test]
fn cc11_the_oracle_must_be_the_last_field_and_the_only_one_outside_the_cone() {
    let build = "model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8, 0u8))";
    // The oracle first: `field-flip-last` would corrupt the bitmap.
    let o = any_with(
        "cc11-first",
        |d| {
            d.replacen("record Impl(x: U8, c: U8)", "record Impl(c: U8, x: U8)", 1)
                .replacen(
                    build,
                    "model build(m: M) -> Impl = (Impl(0u8, if m.a then 1u8 else 0u8))",
                    1,
                )
                .replacen("3u8).c == 2u8", "3u8).c == 1u8", 1)
        },
        same,
        same,
    );
    only(&o, "CC11", "outside the cone of `build` are 0;");
    // No oracle at all, and two fields outside the cone.
    for (label, new, shown) in [
        ("cc11-none", "model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8, if m.a then 1u8 else 0u8))", "are none;"),
        ("cc11-two-oracles", "model build(m: M) -> Impl = (Impl(0u8, 0u8))", "are 0, 1;"),
    ] {
        let o = any_with(label, |d| d.replacen(build, new, 1), same, same);
        only(&o, "CC11", shown);
    }
}

#[test]
fn cc11_a_build_that_is_not_a_constructor_application_fails_closed() {
    let build = "  model build(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8, 0u8))\n";
    let o = any_with(
        "cc11-ctor",
        |d| {
            d.replacen(
                build,
                "  model mk(m: M) -> Impl = (Impl(if m.a then 1u8 else 0u8, 0u8))\n  model build(m: M) -> Impl = mk(m)\n",
                1,
            )
        },
        same,
        same,
    );
    only(&o, "CC11", "is not an `Impl(..)` constructor application");
    let o = any_with("cc11-nobuild", |d| d.replacen(build, "", 1), same, same);
    only(&o, "CC11", "the `build` model is missing");
}

#[test]
fn cc11_the_successors_are_the_documents_not_the_bindings_spelling() {
    // F1: `ret=` and `args=` are optional, and the generator then falls back to the document's
    // signature, so an `f` bound without them is still a state-to-state operation, and without
    // `field-flip-last` it must be a finding in every spelling.
    let full = "op f export=vos_ghost_f args=Impl,u8 ret=Impl faults=field-zero,wrong-ctor,field-flip-last";
    for (label, spelled) in [
        (
            "cc11-bare",
            "op f export=vos_ghost_f faults=field-zero,wrong-ctor",
        ),
        (
            "cc11-noargs",
            "op f export=vos_ghost_f ret=Impl faults=field-zero,wrong-ctor",
        ),
        (
            "cc11-noret",
            "op f export=vos_ghost_f args=Impl,u8 faults=field-zero,wrong-ctor",
        ),
        (
            "cc11-full",
            "op f export=vos_ghost_f args=Impl,u8 ret=Impl faults=field-zero,wrong-ctor",
        ),
    ] {
        let o = any_with(
            label,
            same,
            |b| give_g_the_fault(b.replace(full, spelled)),
            same,
        );
        has(
            &o,
            "CC11",
            "operation \"f\" produces a state from a state but does not list `field-flip-last`",
        );
    }
    // And the bare spelling WITH the fault is accepted: the rule reads the document, so it sees `f`
    // either way.
    let o = any_with(
        "cc11-bare-ok",
        same,
        |b| {
            b.replace(
                full,
                "op f export=vos_ghost_f faults=field-zero,wrong-ctor,field-flip-last",
            )
        },
        same,
    );
    assert!(o.findings.is_empty(), "{}", show(&o));
}

#[test]
fn cc11_a_binding_that_is_no_op_of_the_document_fails_closed() {
    let o = any_with(
        "cc11-ghost-op",
        same,
        |b| {
            b.replacen(
                "op g export",
                "op zz export=vos_ghost_zz ret=u8 faults=field-zero\nop g export",
                1,
            )
        },
        same,
    );
    has(
        &o,
        "CC11",
        "the bindings bind \"zz\", which is no `op` of the generated document",
    );
}

#[test]
fn cc11_an_inout_state_operation_is_a_successor() {
    // An `inout` Impl is mutated in place: its result carries the post-state (VER04k), so the
    // operation must be attacked like any state-producing one, here it is not.
    let o = any_with(
        "cc11-inout",
        |d| {
            d.replacen(
                "  op g() -> U8\n",
                "  op bump(s: inout Impl, t: U8) -> (U8, Impl)\n  op g() -> U8\n",
                1,
            )
            .replacen(
                "  labels f:",
                "  labels bump:\n    s public\n    t public\n  labels f:",
                1,
            )
        },
        |b| {
            b.replacen(
                "op g export",
                "op bump export=vos_ghost_bump faults=field-zero\nop g export",
                1,
            )
        },
        same,
    );
    has(
        &o,
        "CC11",
        "operation \"bump\" produces a state from a state but does not list `field-flip-last`",
    );
}

#[test]
fn cc11_an_oracle_that_is_not_numeric_fails() {
    // F2: with a `Bool` last field `field-flip-last` has nothing to flip.
    let o = any_with(
        "cc11-bool-oracle",
        |d| {
            d.replacen(
                "record Impl(x: U8, c: U8)",
                "record Impl(x: U8, c: Bool)",
                1,
            )
            .replacen("else 0u8, 0u8))", "else 0u8, false))", 1)
            .replacen(
                "Impl(1u8, 2u8), 3u8).c == 2u8",
                "Impl(1u8, true), 3u8).c == true",
                1,
            )
        },
        same,
        same,
    );
    only(&o, "CC11", "the last field of `Impl` has type \"Bool\"");
}

#[test]
fn cc11_a_decoy_claim_of_the_implementation_does_not_attack_the_contracts_claim() {
    // F3: an implementation-own claim `own` calls only `f` and carries the flip-last entry; the
    // contract's `c1` (the one that says `f` keeps the oracle) has none.  Nothing the contract
    // owns is attacked, so CC11 fails although a name-blind rule would pass.
    let r = good("cc11-decoy", ANY, "any", false);
    let doc = any_doc().replacen(
        "  claim c2:",
        "  claim own:\n    forall v: Bool. f(Impl(1u8, 2u8), 3u8).c == 2u8\n  claim c2:",
        1,
    );
    r.write(&format!("code/specs/{ANY}.warrant"), doc);
    // The contract's `any` tier still holds `c1` only (written by `good`): `c1` is the contract's,
    // `own` is not, and the flip-last entry moved from `c1` to `own`.
    r.write(
        &p(ANY, "warrant/sabotage.plan"),
        any_plan().replace(
            "exhaustive/c1 field-flip-last\n",
            "exhaustive/own field-flip-last\n",
        ),
    );
    put_manifest(&r, ANY, manifest(&["c1", "c2", "own"], 4096, 5000, 10, &[]));
    let o = run(&r, ANY);
    has(
        &o,
        "CC11",
        "claim of the contract's `any` tier that calls \"f\"",
    );
}

#[test]
fn cc11_a_document_or_bindings_the_tools_refuse_is_a_finding() {
    // The document no longer type-checks (an undeclared operation): the analysis refuses, and
    // so does the plan's claim scan, but only the oracle rule's finding is the CC11 one.
    let r = good("cc11-badstack", ANY, "any", false);
    r.write(
        &format!("code/specs/{ANY}.warrant"),
        any_doc().replacen(
            "f(Impl(1u8, 2u8), 3u8).c == 2u8",
            "zz(Impl(1u8, 2u8), 3u8).c == 2u8",
            1,
        ),
    );
    put_manifest(&r, ANY, manifest(&["c1", "c2"], 4096, 5000, 10, &[]));
    has(&run(&r, ANY), "CC11", "the document:");
}

#[test]
fn cc11_does_not_apply_to_a_production_crate() {
    // A production crate may bind a state-producing operation without `field-flip-last`: it has no oracle.
    let r = good("cc11-prod", PROD, "production", false);
    r.write(
        &p(PROD, "evidence.bindings"),
        any_bindings(PROD).replace(",field-flip-last", ""),
    );
    let o = run(&r, PROD);
    assert!(o.findings.is_empty(), "{}", show(&o));
}

#[test]
fn the_oracle_fault_is_the_one_the_spec_names() {
    let spec = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS04-component-catalogue.md"),
    )
    .unwrap();
    assert_eq!(vos_link::oracle::ORACLE_FAULT, "field-flip-last");
    assert!(
        spec.contains("`ORACLE_FAULT`")
            && spec.contains("`field-flip-last`")
            && spec.contains("CC11"),
        "VOS04 5.6 must state CC11 and the constant"
    );
}

// --- CC12 (VOS-040c, VOS31 10.2): an implementation never claims SAT ---------------------------

#[test]
fn cc12_an_implementation_with_the_sat_key_is_refused() {
    let r = good("cc12-key", PROD, "production", false);
    let t = r.read(&p(PROD, "vos.toml"));
    r.write(&p(PROD, "vos.toml"), format!("{t}sat = \"model\"\n"));
    only(
        &run(&r, PROD),
        "CC12",
        "`sat = \"model\"` on an implementation crate",
    );
}

#[test]
fn cc12_an_implementation_with_a_sat_table_is_refused() {
    let r = good("cc12-table", PROD, "production", false);
    r.write(&p(PROD, "warrant/lemmas.sat"), "vos-sat 1\nend\n");
    only(
        &run(&r, PROD),
        "CC12",
        "under warrant/ exists in an implementation",
    );
}

#[test]
fn cc12_an_implementation_build_that_runs_the_lane_is_refused_in_both_files() {
    for (file, name) in [("BUILD", "BUILD:"), ("BUILD_windows", "BUILD_windows:")] {
        let r = good("cc12-build", PROD, "production", false);
        let text = r.read(&p(PROD, file));
        r.write(
            &p(PROD, file),
            format!("{text}cargo run -q --release --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- check .\n"),
        );
        let o = run(&r, PROD);
        only(&o, "CC12", name);
    }
    // A comment that merely mentions the lane is not a run.
    let r = good("cc12-comment", PROD, "production", false);
    let text = r.read(&p(PROD, "BUILD"));
    r.write(
        &p(PROD, "BUILD"),
        format!("{text}# no vos-model-sat here\n"),
    );
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn cc12_any_file_under_warrant_with_sat_in_its_name_is_refused() {
    for name in ["lemmas.sat.bak", "foo.sat", "x.sat.old"] {
        let r = good("cc12-satname", PROD, "production", false);
        r.write(
            &p(PROD, &format!("warrant/{name}")),
            "x
",
        );
        only(
            &run(&r, PROD),
            "CC12",
            "under warrant/ exists in an implementation",
        );
    }
    // A neighbour that merely contains the letters is fine.
    let r = good("cc12-sat-ok", PROD, "production", false);
    r.write(
        &p(PROD, "warrant/saturate.txt"),
        "x
",
    );
    assert!(run(&r, PROD).findings.is_empty());
}

#[test]
fn cc12_a_continued_or_quoted_name_and_a_manifest_path_are_caught() {
    use vos_link::component::{logical_lines, mentions_sat_lane};
    // Unit level: continuation, quotes, Windows separators, a manifest path.
    let lines = logical_lines("echo a\ncargo run vos-model-\\\nsat check .\nlast\n");
    assert_eq!(lines[1], (2, "cargo run vos-model-sat check .".to_string()));
    assert_eq!(lines.len(), 3);
    assert!(mentions_sat_lane("cargo run vos-model-\"sat\" check"));
    assert!(mentions_sat_lane(
        r"cargo run --manifest-path ..\..\programs\rust\vos-model-sat\Cargo.toml"
    ));
    assert!(mentions_sat_lane("cat warrant/lemmas.sat"));
    assert!(!mentions_sat_lane("cargo test --manifest-path Cargo.toml"));
    // Through the rule, in both BUILD files.
    for file in ["BUILD", "BUILD_windows"] {
        let r = good("cc12-split", PROD, "production", false);
        let t = r.read(&p(PROD, file));
        r.write(
            &p(PROD, file),
            format!("{t}cargo run -q vos-model-\\\nsat check .\n"),
        );
        only(&run(&r, PROD), "CC12", &format!("{file}:"));
        let r = good("cc12-mp", PROD, "production", false);
        let t = r.read(&p(PROD, file));
        r.write(
            &p(PROD, file),
            format!(
                "{t}cargo run -q --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml\n"
            ),
        );
        only(&run(&r, PROD), "CC12", &format!("{file}:"));
    }
    // A comment that mentions the lane (even a continued one) is still allowed.
    let r = good("cc12-comment2", PROD, "production", false);
    let t = r.read(&p(PROD, "BUILD"));
    r.write(
        &p(PROD, "BUILD"),
        format!("{t}# see vos-model-sat \\\n# and lemmas.sat\n"),
    );
    assert!(run(&r, PROD).findings.is_empty());
}
