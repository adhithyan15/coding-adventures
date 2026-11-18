//! # `check-component`: is this crate a complete component? (VOS-039a, VOS04 section 5.6)
//!
//! `gen-doc` and `check-doc` judge one file, the generated document.  A component is more
//! than its document: it is a file set, a BUILD that runs the section 5.2 evidence lines in
//! their order, a manifest that proves those lines ran, a row in the build tool, and a
//! coverage floor.  Until this command existed the list lived in VOS04 section 5.5 and a
//! reviewer's memory.
//!
//! ## Generated, then compared (not linted)
//!
//! The first draft recognised the required lines in the author's BUILD text, and a security
//! review showed any such lint is satisfiable by text that never runs (the build tool runs each
//! line as its own `sh -c`; see [`crate::build_gen`]).  So the BUILD lines are **generated**
//! by `gen-build` and this command requires **byte equality** of the canonical region, as
//! `check-doc` does for the document.
//!
//! ```text
//!   CC01 kind               CC05 build-tool row        CC08 the evidence manifest
//!   CC02 file set           CC06 publish = false (any)     bound to the document's claims
//!   CC03 BUILD, BUILD_windows == gen-build            CC07 coverage-floor declared, >= 95
//!   CC04 inputs of generation: bindings, document, sabotage plan, tolerance script
//! ```
//!
//! ## The Go file is read narrowly
//!
//! The build-tool row is one call in a Go slice literal.  `vos-link` does not depend on a Go
//! parser, so [`strip_go`] removes the things that can hide or fake a line (`//` and `/* */`
//! comments, raw strings) and [`row_in_consumers`] looks for the exact line only inside the
//! `repositoryInputConsumers = []repositoryInputConsumer{` slice.  A row in a comment, in a raw
//! string, or in another slice does not count.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use coding_adventures_bounded_json::{parse as parse_json, JsonNumber, JsonValue};
use coding_adventures_test_bootstrap::manifest::source_tree_framing;
use vos_shape_check::manifest::{
    parse_vos_toml, Kind, VosToml, CARGO_TOML_MAX_BYTES, VOS_TOML_MAX_BYTES,
};
use vos_shape_check::toml::{self as toml_reader, Value};

use crate::build_gen::{
    canonical_build, load_inputs, tolerance_script, GenInputs, PLAN_FILE, TOLERANCE_SCRIPT,
};
use crate::check::first_difference_line;
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, q, read_regular_opt};
use crate::inputs::{resolve_layout, Layout};
use crate::ops::Outcome;

/// The repository's package standard for library coverage (CLAUDE.md rule 11, VOS04 5.6): a
/// crate may declare a higher floor, never a lower one.
pub const COVERAGE_FLOOR_MIN: u8 = 95;
/// VOS04 section 5.2 line 7: random cases per claim.  A test reads the spec and asserts it.
pub const PROPERTY_CASES_MIN: i64 = 4096;
/// VOS04 section 5.2 line 8: fuzz iterations per claim.  A test reads the spec and asserts it.
pub const FUZZ_ITERATIONS_MIN: i64 = 5000;
/// The manifest schema the generator writes.
pub const MANIFEST_SCHEMA: &str = "warrant-test-evidence-v1";

const BUILD_MAX_BYTES: usize = 256 * 1024;
const MANIFEST_MAX_BYTES: usize = 4 * 1024 * 1024;
const SCRIPT_MAX_BYTES: usize = 64 * 1024;
const BUILD_ROWS_MAX_BYTES: usize = 4 * 1024 * 1024;

/// The build tool's file of repository-input rows, relative to the repository root.
pub const BUILD_TOOL_ROWS: &str = "code/programs/go/build-tool/repository_inputs.go";
/// The line that opens the slice the rows must be in.
pub const CONSUMERS_OPEN: &str = "repositoryInputConsumers = []repositoryInputConsumer{";

// ---------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------

/// Is a regular file (not a symlink) present?
fn present(path: &Path) -> Result<bool, NoVerdict> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err(NoVerdict::new(format!("{}: is a symlink", q(path))))
        }
        Ok(m) => Ok(m.is_file()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(NoVerdict::new(format!("{}: {e}", q(path)))),
    }
}

pub(crate) fn read_text(path: &Path, cap: usize) -> Result<Option<String>, NoVerdict> {
    match read_regular_opt(path, cap)? {
        None => Ok(None),
        Some(b) => String::from_utf8(b)
            .map(Some)
            .map_err(|_| NoVerdict::new(format!("{}: not UTF-8", q(path)))),
    }
}

/// `vos.toml` of an implementation crate.  `Ok(Err(finding))` is a `CC01`.
pub(crate) fn read_component_vos_toml(
    layout: &Layout,
) -> Result<Result<VosToml, Finding>, NoVerdict> {
    let Some(text) = read_text(&layout.impl_dir.join("vos.toml"), VOS_TOML_MAX_BYTES)? else {
        return Ok(Err(Finding::new("CC01", "vos.toml: not found")));
    };
    let vt = match parse_vos_toml(&text) {
        Ok(v) => v,
        Err((line, why)) => {
            return Ok(Err(Finding::new(
                "CC01",
                format!("vos.toml:{line}: {}", esc(&why)),
            )))
        }
    };
    match vt.kind {
        Some(Kind::Production | Kind::Reference | Kind::Any) => Ok(Ok(vt)),
        other => {
            let name = other.map_or("(none)", Kind::name);
            Ok(Err(Finding::new(
                "CC01",
                format!(
                    "vos.toml: kind `{name}` is not an implementation kind; check-component judges production, reference and any crates (VOS04 5.6)"
                ),
            )))
        }
    }
}

// ---------------------------------------------------------------------------------------
// The Go file (CC05)
// ---------------------------------------------------------------------------------------

/// Remove Go `//` and `/* */` comments and raw strings (back-quoted, possibly many lines) from
/// `src`, keeping every newline so line structure survives.  Interpreted strings and rune
/// literals are copied (they cannot span lines) but *scanned*, so a quote inside one does not
/// start a comment or a raw string.
pub fn strip_go(src: &str) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    if b[i] == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
                i = (i + 2).min(b.len());
                out.push(b' ');
            }
            b'`' => {
                i += 1;
                while i < b.len() && b[i] != b'`' {
                    if b[i] == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
                i = (i + 1).min(b.len());
                out.push(b' ');
            }
            quote @ (b'"' | b'\'') => {
                out.push(quote);
                i += 1;
                while i < b.len() && b[i] != quote && b[i] != b'\n' {
                    if b[i] == b'\\' && i + 1 < b.len() && b[i + 1] != b'\n' {
                        out.push(b[i]);
                        i += 1;
                    }
                    out.push(b[i]);
                    i += 1;
                }
                if i < b.len() && b[i] == quote {
                    out.push(quote);
                    i += 1;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Is `vosImplementationRow("<crate>"),` a line of the `repositoryInputConsumers` slice of
/// the Go file `src`?  The slice runs from the line holding [`CONSUMERS_OPEN`] to the next line
/// that is exactly `}`.
pub fn row_in_consumers(src: &str, krate: &str) -> bool {
    let stripped = strip_go(src);
    let row = format!("vosImplementationRow(\"{krate}\"),");
    let mut inside = false;
    for line in stripped.lines() {
        if !inside {
            inside = line.contains(CONSUMERS_OPEN);
        } else if line == "}" {
            inside = false;
        } else if line.trim() == row {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------------------
// The checks
// ---------------------------------------------------------------------------------------

struct Ctx {
    krate: String,
    layout: Layout,
    vt: VosToml,
    kind: Kind,
    f: Vec<Finding>,
}

impl Ctx {
    fn find(&mut self, code: &'static str, msg: impl Into<String>) {
        self.f.push(Finding::new(code, msg));
    }
}

/// `vos-link check-component <crate-dir>`.
pub fn check_component(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let layout = resolve_layout(crate_arg, root_arg)?;
    ensure_plain_dirs(&layout.root, &["code", "packages", "rust"])?;
    let mut out = Outcome::default();
    let vt = match read_component_vos_toml(&layout)? {
        Ok(v) => v,
        Err(finding) => {
            out.findings.push(finding);
            return Ok(out);
        }
    };
    let Some(kind) = vt.kind else {
        return Ok(out); // unreachable: read_component_vos_toml demands a kind
    };
    out.header = format!("crate {}, kind {}", layout.impl_pkg, kind.name());
    let mut cx = Ctx {
        krate: layout.impl_pkg.clone(),
        layout,
        vt,
        kind,
        f: Vec::new(),
    };

    file_set(&mut cx)?;
    match load_inputs(&cx.layout, &cx.vt)? {
        Ok(g) => {
            builds(&mut cx, &g)?;
            manifest(&mut cx, &g)?;
        }
        Err(findings) => cx.f.extend(findings),
    }
    build_tool_row(&mut cx)?;
    publish_rule(&mut cx)?;

    out.findings = cx.f;
    out.notes.push(
        "check-component: the file set, BUILD and BUILD_windows byte-equal to `gen-build` (VOS04 5.2 lines 1 to 9, the coverage gate and this command), the sabotage plan, the tolerance script, the build-tool row, publish = false for `-any` and the evidence manifest bound to the document are present and consistent. Not checked: that the lines PASS (the build does that), the exact operations-times-tiers control count (VOS-D104), a parser's fuzz corpus (VOS-D105), the command-type check (VOS-D077), deriving the plan (VOS-D107), and line 10 (SAT, n/a until VOS-029).".to_string(),
    );
    Ok(out)
}

/// CC02: the file set of VOS04 section 5.1.
fn file_set(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let required = [
        "BUILD",
        "BUILD_windows",
        "README.md",
        "Cargo.toml",
        "vos.toml",
        "required_capabilities.json",
        "warrant/impl.part",
        PLAN_FILE,
        "evidence.bindings",
        "evidence/test-manifest.json",
        "tests/generated/warrant_evidence.rs",
    ];
    let dir = cx.layout.impl_dir.clone();
    for rel in required {
        if !present(&dir.join(rel))? {
            cx.find("CC02", format!("file set: {rel} is missing"));
        }
    }
    if !present(&dir.join("CHANGELOG.md"))? && !present(&dir.join("CHANGELOG.d/_meta.md"))? {
        cx.find(
            "CC02",
            "file set: neither CHANGELOG.md nor CHANGELOG.d/_meta.md exists",
        );
    }
    if !present(&cx.layout.specs_dir().join(cx.layout.doc_name()))? {
        cx.find(
            "CC02",
            format!(
                "file set: the generated document {} is missing (run `vos-link gen-doc`)",
                cx.layout.doc_display()
            ),
        );
    }
    Ok(())
}

/// A line of `text`, for a finding: escaped and cut.
fn show_line(text: &str, n: usize) -> String {
    let l = text
        .lines()
        .nth(n.saturating_sub(1))
        .unwrap_or("(end of file)");
    let cut: String = l.chars().take(160).collect();
    esc(&cut)
}

/// CC03 and CC04: BUILD, BUILD_windows and the tolerance script equal what `gen-build` writes.
fn builds(cx: &mut Ctx, g: &GenInputs) -> Result<(), NoVerdict> {
    for (name, windows) in [("BUILD", false), ("BUILD_windows", true)] {
        let Some(found) = read_text(&cx.layout.impl_dir.join(name), BUILD_MAX_BYTES)? else {
            continue; // CC02 reported it
        };
        let want = canonical_build(g, windows);
        if !found.starts_with(&want) {
            let n = first_difference_line(&found, &want);
            cx.find(
                "CC03",
                format!(
                    "{name}:{n}: the file does not begin with the region `vos-link gen-build` writes; expected {}, found {}",
                    show_line(&want, n),
                    show_line(&found, n)
                ),
            );
        }
    }
    let script_path = cx.layout.impl_dir.join(TOLERANCE_SCRIPT);
    let found = read_text(&script_path, SCRIPT_MAX_BYTES)?;
    match (tolerance_script(g), found) {
        (Some(want), Some(found)) if want != found => {
            let n = first_difference_line(&found, &want);
            cx.find(
                "CC04",
                format!(
                    "{TOLERANCE_SCRIPT}:{n}: differs from the script `vos-link gen-build` writes from `excused-controls`; expected {}, found {}",
                    show_line(&want, n),
                    show_line(&found, n)
                ),
            );
        }
        (Some(_), None) => cx.find(
            "CC04",
            format!("{TOLERANCE_SCRIPT}: missing (vos.toml declares `excused-controls`; run `vos-link gen-build`)"),
        ),
        (None, Some(_)) => cx.find(
            "CC04",
            format!("{TOLERANCE_SCRIPT}: exists but vos.toml declares no `excused-controls`: a tolerance script nobody declared excuses nothing and runs nothing"),
        ),
        _ => {}
    }
    Ok(())
}

/// CC05: the build-tool row (section 5.2 line 11).
fn build_tool_row(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let path = cx.layout.root.join(BUILD_TOOL_ROWS);
    match read_text(&path, BUILD_ROWS_MAX_BYTES)? {
        None => cx.find(
            "CC05",
            format!("build-tool row: {BUILD_TOOL_ROWS} not found"),
        ),
        Some(src) => {
            if !row_in_consumers(&src, &cx.krate) {
                cx.find(
                    "CC05",
                    format!(
                        "build-tool row: {BUILD_TOOL_ROWS} has no line `vosImplementationRow(\"{}\"),` inside the repositoryInputConsumers slice (comments and raw strings do not count), so an edit to a VOS tool or to code/specs/{}.warrant selects nothing (VOS04 5.2 line 11, VOS-D066)",
                        cx.krate, cx.krate
                    ),
                );
            }
        }
    }
    Ok(())
}

/// CC06: VOS-D082.
fn publish_rule(cx: &mut Ctx) -> Result<(), NoVerdict> {
    if cx.kind != Kind::Any {
        return Ok(());
    }
    let Some(text) = read_text(&cx.layout.impl_dir.join("Cargo.toml"), CARGO_TOML_MAX_BYTES)?
    else {
        return Ok(()); // CC02 reported it
    };
    let publish_false = match toml_reader::parse(&text) {
        Ok(entries) => entries.iter().any(|e| {
            e.path == ["package".to_string(), "publish".to_string()]
                && e.value == Value::Bool(false)
        }),
        Err(e) => {
            cx.find(
                "CC06",
                format!("Cargo.toml:{}: {}", e.line, esc(&e.message)),
            );
            return Ok(());
        }
    };
    if !publish_false {
        cx.find(
            "CC06",
            "Cargo.toml: an `-any` crate is evidence only and must say `publish = false` in [package] (VOS-D082, next to SH16)",
        );
    }
    Ok(())
}

type Obj = [(String, JsonValue)];

fn get<'a>(o: &'a Obj, key: &str) -> Option<&'a JsonValue> {
    o.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn int(o: &Obj, key: &str) -> Option<i64> {
    match get(o, key) {
        Some(JsonValue::Number(JsonNumber::Integer(n))) => Some(*n),
        _ => None,
    }
}

fn string<'a>(o: &'a Obj, key: &str) -> Option<&'a str> {
    match get(o, key) {
        Some(JsonValue::String(s)) => Some(s),
        _ => None,
    }
}

fn set_of(v: &[String]) -> BTreeSet<String> {
    v.iter().cloned().collect()
}

/// CC08 (review M2): the manifest says which document and which source it ran against.
/// `spec_sha256` is the SHA-256 of `code/specs/<crate>.warrant` (the generated harness embeds
/// it) and `source_sha256` the digest of the crate's `src/` tree under the framing the
/// harness uses (`test-bootstrap`'s `source_tree_framing`).  A manifest of an older document
/// or older source, or copied from another crate, is stale evidence and fails here.
fn bind_digests(cx: &mut Ctx, top: &Obj) -> Result<(), NoVerdict> {
    let doc = cx.layout.specs_dir().join(cx.layout.doc_name());
    if let Some(bytes) = read_regular_opt(&doc, vos_manifest::caps::DOCUMENT_MAX_BYTES)? {
        let want = coding_adventures_sha256::sha256_hex(&bytes);
        if string(top, "spec_sha256") != Some(want.as_str()) {
            cx.find(
                "CC08",
                format!(
                    "manifest: spec_sha256 is {}, but {} has digest {want}: the evidence was not run against this document",
                    esc(string(top, "spec_sha256").unwrap_or("(absent)")),
                    cx.layout.doc_display()
                ),
            );
        }
    }
    match source_tree_framing(&cx.layout.impl_dir.join("src")) {
        Ok(framed) => {
            let want = coding_adventures_sha256::sha256_hex(&framed);
            if string(top, "source_sha256") != Some(want.as_str()) {
                cx.find(
                    "CC08",
                    format!(
                        "manifest: source_sha256 is {}, but the digest of src/ is {want}: the evidence was not run against this source",
                        esc(string(top, "source_sha256").unwrap_or("(absent)"))
                    ),
                );
            }
        }
        Err(e) => cx.find(
            "CC08",
            format!("manifest: cannot digest src/: {}", esc(&e.to_string())),
        ),
    }
    Ok(())
}

/// CC08: the manifest the generated suite wrote, bound to the document.
fn manifest(cx: &mut Ctx, g: &GenInputs) -> Result<(), NoVerdict> {
    let path = cx
        .layout
        .impl_dir
        .join("evidence")
        .join("test-manifest.json");
    let Some(text) = read_text(&path, MANIFEST_MAX_BYTES)? else {
        return Ok(()); // CC02 reported it
    };
    let json = match parse_json(&text) {
        Ok(j) => j,
        Err(e) => {
            cx.find(
                "CC08",
                format!("manifest: not JSON: {}", esc(&e.to_string())),
            );
            return Ok(());
        }
    };
    let JsonValue::Object(top) = &json else {
        cx.find("CC08", "manifest: the top level is not an object");
        return Ok(());
    };
    if string(top, "schema") != Some(MANIFEST_SCHEMA) {
        cx.find(
            "CC08",
            format!("manifest: `schema` is not `{MANIFEST_SCHEMA}`"),
        );
    }
    bind_digests(cx, top)?;
    let Some(JsonValue::Array(tests)) = get(top, "tests") else {
        cx.find("CC08", "manifest: no `tests` array");
        return Ok(());
    };
    if tests.is_empty() {
        cx.find(
            "CC08",
            "manifest: `tests` is empty: no test ran (line 6, none ran = fail)",
        );
        return Ok(());
    }

    // The thresholds: the bindings' own parameters, never below the spec's (VOS04 5.2).
    let prop_min = PROPERTY_CASES_MIN.max(i64::try_from(g.property_cases).unwrap_or(i64::MAX));
    let fuzz_min = FUZZ_ITERATIONS_MIN.max(i64::try_from(g.fuzz_iterations).unwrap_or(i64::MAX));

    let (mut exhaustive, mut property, mut fuzz) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    let mut examples: BTreeSet<String> = BTreeSet::new();
    let mut negctl_pass_tiers: BTreeSet<String> = BTreeSet::new();
    let mut failing_controls: BTreeSet<String> = BTreeSet::new();
    let mut malformed = 0usize;

    for t in tests {
        let JsonValue::Object(o) = t else {
            malformed += 1;
            continue;
        };
        let (Some(kind), Some(id), Some(verdict)) =
            (string(o, "kind"), string(o, "id"), string(o, "verdict"))
        else {
            malformed += 1;
            continue;
        };
        let claim = string(o, "claim").unwrap_or("").to_string();
        let passed = verdict == "pass";
        if kind == "negctl" {
            if passed {
                if let Some(target) = string(o, "target") {
                    negctl_pass_tiers.insert(target.to_string());
                }
            } else {
                failing_controls.insert(id.to_string());
            }
            continue;
        }
        if !passed {
            cx.find(
                "CC08",
                format!(
                    "manifest: test {} has verdict {}, not pass",
                    esc(id),
                    esc(verdict)
                ),
            );
        }
        match kind {
            "exhaustive" => {
                if int(o, "cases").unwrap_or(0) <= 0 {
                    cx.find(
                        "CC08",
                        format!(
                            "manifest: claim {} ran zero exhaustive cases (line 6, none ran = fail)",
                            esc(&claim)
                        ),
                    );
                }
                exhaustive.insert(claim);
            }
            "property" => {
                let n = int(o, "cases").unwrap_or(0);
                if n < prop_min {
                    cx.find(
                        "CC08",
                        format!(
                            "manifest: claim {} ran {n} property cases, fewer than {prop_min} (line 7)",
                            esc(&claim)
                        ),
                    );
                }
                property.insert(claim);
            }
            "fuzz" => {
                let n = int(o, "iterations").unwrap_or(0);
                if n < fuzz_min {
                    cx.find(
                        "CC08",
                        format!(
                            "manifest: claim {} ran {n} fuzz iterations, fewer than {fuzz_min} (line 8)",
                            esc(&claim)
                        ),
                    );
                }
                fuzz.insert(claim);
            }
            "example" => {
                if let Some((_, name)) = id.rsplit_once("/example/") {
                    examples.insert(name.to_string());
                }
            }
            _ => {}
        }
    }
    if malformed > 0 {
        cx.find(
            "CC08",
            format!("manifest: {malformed} entries lack a string kind, id or verdict"),
        );
    }

    // The manifest is claims x tiers of THE DOCUMENT: nothing dropped, nothing invented.
    let doc_claims = set_of(&g.claims);
    for (set, tier, line) in [
        (&exhaustive, "exhaustive", "6"),
        (&property, "property", "7"),
        (&fuzz, "fuzz", "8"),
    ] {
        for c in doc_claims.difference(set) {
            cx.find(
                "CC08",
                format!(
                    "manifest: claim {} of the document has no {tier} entry (line {line}, none ran = fail)",
                    esc(c)
                ),
            );
        }
        for c in set.difference(&doc_claims) {
            cx.find(
                "CC08",
                format!(
                    "manifest: the {tier} entry for claim {} names no claim of the document",
                    esc(c)
                ),
            );
        }
    }
    let doc_examples = set_of(&g.examples);
    for e in doc_examples.difference(&examples) {
        cx.find(
            "CC08",
            format!("manifest: example {} of the document has no entry", esc(e)),
        );
    }
    for e in examples.difference(&doc_examples) {
        cx.find(
            "CC08",
            format!(
                "manifest: the entry for example {} names no example of the document",
                esc(e)
            ),
        );
    }
    for tier in ["exhaustive", "property", "fuzz"] {
        if !negctl_pass_tiers.contains(tier) {
            cx.find(
                "CC08",
                format!(
                    "manifest: no negative control PASSED in the {tier} tier (line 9, none ran = fail; a failing control counts for nothing)"
                ),
            );
        }
    }
    // The controls that may fail are exactly the declared ones, as whole ids.
    let excused: BTreeSet<String> = cx
        .vt
        .excused_controls
        .iter()
        .map(|e| e.id.clone())
        .collect();
    for id in failing_controls.difference(&excused) {
        cx.find(
            "CC08",
            format!(
                "manifest: negative control {} did not pass and vos.toml does not excuse it",
                esc(id)
            ),
        );
    }
    for id in excused.difference(&failing_controls) {
        cx.find(
            "CC08",
            format!("manifest: vos.toml excuses {} but that control passes or does not exist (a stale excuse)", esc(id)),
        );
    }
    Ok(())
}
