//! # `check-component`: is this crate a complete component? (VOS-039a, VOS04 section 5.6)
//!
//! `gen-doc` and `check-doc` judge one file, the generated document.  A component is more
//! than its document: it is a file set, a BUILD that runs the section 5.2 evidence lines in
//! their order, a manifest that proves those lines ran, a row in the build tool, and a
//! coverage floor.  Until this command existed the list lived in VOS04 section 5.5 and a
//! reviewer's memory.  Now it is checked, and **a missing line is a finding**, not a skip.
//!
//! ```text
//!   <crate>/vos.toml ─────────► kind, evidence-features, coverage-floor        CC01, CC07
//!   <crate>/{BUILD, ...} ─────► the required files                             CC02
//!   <crate>/BUILD{,_windows} ─► lines 1-6, 9, ordering, bail-out guards        CC03, CC04
//!   build-tool/repository_inputs.go ─► vosImplementationRow("<crate>")         CC05
//!   <crate>/Cargo.toml ───────► publish = false for kind `any`                 CC06
//!   <crate>/BUILD ────────────► the tarpaulin gate tied to the floor           CC07
//!   <crate>/evidence/test-manifest.json ─► lines 6-9 "none ran = fail"         CC08
//! ```
//!
//! ## Why the BUILD files are read as text
//!
//! A BUILD file is a list of shell command lines.  This module does not parse shell: for each
//! non-comment line it splits at the shell separators (`&`, `;`, `|`, parentheses) and at
//! white space, then asks whether certain **consecutive tokens** occur.  The fragments that
//! carry meaning (`cargo test -p <crate> --doc`, `-- check-doc .`) are matched exactly, on
//! token boundaries, so `-p vos-sched-rr` does not match `-p vos-sched-rr-extra`.  Anything else
//! on the line is ignored, so a harmless extra flag never fails a build.  The Go file with the
//! build-tool rows is read the same narrow way: one line, trimmed, must be exactly
//! `vosImplementationRow("<crate>"),`; a Go parser would be a second tool to trust for one
//! call.  The manifest is read by the repository's `bounded-json`, the `Cargo.toml` and
//! `vos.toml` by the TOML crates.
//!
//! ## Which line is which (VOS04 section 5.2)
//!
//! ```text
//!   1  unit        cargo test -p C --lib --test behaviour            (plain, then --features F, then --doc)
//!   2  shape       ... vos-shape-check -- .
//!   3  capability  ... coding_adventures_ca_capability_analyzer -- --dir .
//!   4  document    ... vos-link/Cargo.toml -- check-doc .
//!   5  drift       ... gen_evidence -- --check C
//!   6-8 generated  WARRANT_EVIDENCE_OUT=evidence/test-manifest.json ... --test warrant_evidence
//!                  (or a line that runs warrant/check_evidence.sh, which does)
//!   9  sabotage    WARRANT_EVIDENCE_FAULT=...          each line bail-out aware
//!   10 SAT         n/a until VOS-029
//!   11 selection   vosImplementationRow("C")
//!   +  this command, AFTER line 6, so it reads the manifest that run just rewrote
//! ```

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_bounded_json::{parse as parse_json, JsonNumber, JsonValue};
use vos_shape_check::manifest::{
    parse_vos_toml, Kind, VosToml, CARGO_TOML_MAX_BYTES, VOS_TOML_MAX_BYTES,
};
use vos_shape_check::toml::{self as toml_reader, Value};

use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, q, read_regular_opt};
use crate::inputs::resolve_layout;
use crate::ops::Outcome;

/// The repository's package standard for library coverage (CLAUDE.md rule 11, VOS04 5.6): a
/// crate may declare a higher floor, never a lower one.
pub const COVERAGE_FLOOR_MIN: u8 = 95;
/// Section 5.2 line 7: random cases per claim.
pub const PROPERTY_CASES_MIN: i64 = 4096;
/// Section 5.2 line 8: fuzz iterations per claim.
pub const FUZZ_ITERATIONS_MIN: i64 = 5000;
/// The manifest schema the generator writes.
pub const MANIFEST_SCHEMA: &str = "warrant-test-evidence-v1";

/// Caps on what is read (a file over its cap is no verdict, as everywhere in `vos-link`).
const BUILD_MAX_BYTES: usize = 256 * 1024;
const MANIFEST_MAX_BYTES: usize = 4 * 1024 * 1024;
const SCRIPT_MAX_BYTES: usize = 64 * 1024;
const BUILD_ROWS_MAX_BYTES: usize = 4 * 1024 * 1024;

/// The build tool's file of repository-input rows, relative to the repository root.
pub const BUILD_TOOL_ROWS: &str = "code/programs/go/build-tool/repository_inputs.go";

// ---------------------------------------------------------------------------------------
// Reading BUILD lines
// ---------------------------------------------------------------------------------------

/// One command line of a BUILD file.
#[derive(Debug, Clone)]
pub struct BuildLine {
    /// 1-based line number in the file.
    pub number: usize,
    /// The line as written (trimmed).
    pub text: String,
    /// The tokens: the line split at white space and at `& ; | ( )`.
    pub tokens: Vec<String>,
}

/// Split a command line into tokens (see the module docs).  Quote characters are dropped from
/// the ends of a token, so `'vos-sched-rr/src/*'` and `vos-sched-rr/src/*` are one token.
pub fn tokenize(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | '|' | '(' | ')'))
        .map(|t| t.trim_matches(|c| c == '\'' || c == '"'))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// The non-comment, non-blank lines of a BUILD file, with their line numbers.
pub fn build_lines(text: &str) -> Vec<BuildLine> {
    text.lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let t = raw.trim();
            if t.is_empty() || t.starts_with('#') || t.to_ascii_lowercase().starts_with("rem ") {
                return None;
            }
            Some(BuildLine {
                number: i + 1,
                text: t.to_string(),
                tokens: tokenize(t),
            })
        })
        .collect()
}

/// Do the tokens contain `seq` as consecutive tokens?
fn has_seq(tokens: &[String], seq: &[&str]) -> bool {
    !seq.is_empty()
        && tokens
            .windows(seq.len())
            .any(|w| w.iter().zip(seq).all(|(a, b)| a == b))
}

/// The token that follows the first occurrence of `flag`.
fn after<'a>(tokens: &'a [String], flag: &str) -> Option<&'a str> {
    tokens
        .iter()
        .position(|t| t == flag)
        .and_then(|i| tokens.get(i + 1))
        .map(String::as_str)
}

/// A requirement a line must meet: every sequence of tokens must occur, and (for the
/// `vos-link` lines) some token must end with a suffix.
struct Need {
    /// The section 5.2 line this is, for the finding.
    line_no: &'static str,
    /// What the line does, for the finding.
    what: &'static str,
    /// Token sequences that must all occur in one BUILD line.
    seqs: Vec<Vec<String>>,
    /// A token that must end with this, if any.
    suffix: Option<&'static str>,
}

impl Need {
    fn new(line_no: &'static str, what: &'static str, seqs: &[&[&str]]) -> Need {
        Need {
            line_no,
            what,
            seqs: seqs
                .iter()
                .map(|s| s.iter().map(|x| x.to_string()).collect())
                .collect(),
            suffix: None,
        }
    }

    fn matches(&self, l: &BuildLine) -> bool {
        self.seqs.iter().all(|s| {
            let s: Vec<&str> = s.iter().map(String::as_str).collect();
            has_seq(&l.tokens, &s)
        }) && self
            .suffix
            .is_none_or(|suf| l.tokens.iter().any(|t| t.ends_with(suf)))
    }

    fn describe(&self) -> String {
        let mut parts: Vec<String> = self.seqs.iter().map(|s| s.join(" ")).collect();
        if let Some(suf) = self.suffix {
            parts.push(format!("a path ending {suf}"));
        }
        parts.join(" ... ")
    }
}

/// The lines 1 to 5 of section 5.2 for crate `c` with evidence features `features`.
fn needs_1_to_5(c: &str, features: &[String]) -> Vec<Need> {
    let mut v = vec![Need::new(
        "1",
        "unit tests without the ghost face",
        &[&["cargo", "test", "-p", c, "--lib", "--test", "behaviour"]],
    )];
    for f in features {
        v.push(Need::new(
            "1",
            "unit tests with the evidence feature",
            &[&[
                "cargo",
                "test",
                "-p",
                c,
                "--features",
                f,
                "--lib",
                "--test",
                "behaviour",
            ]],
        ));
    }
    v.push(Need::new(
        "1",
        "doc tests",
        &[&["cargo", "test", "-p", c, "--doc"]],
    ));
    v.push(Need::new(
        "2",
        "the shape check",
        &[&["vos-shape-check", "--", "."]],
    ));
    v.push(Need::new(
        "3",
        "the capability analyzer",
        &[&[
            "coding_adventures_ca_capability_analyzer",
            "--",
            "--dir",
            ".",
        ]],
    ));
    let mut doc = Need::new("4", "the document check", &[&["--", "check-doc", "."]]);
    doc.suffix = Some("vos-link/Cargo.toml");
    v.push(doc);
    v.push(Need::new(
        "5",
        "harness drift",
        &[&["gen_evidence", "--", "--check", c]],
    ));
    v
}

/// The line that runs `check-component`.
fn check_component_need() -> Need {
    let mut n = Need::new(
        "check-component",
        "this command",
        &[&["--", "check-component", "."]],
    );
    n.suffix = Some("vos-link/Cargo.toml");
    n
}

/// Does this line write the manifest by running the generated suite (section 5.2 line 6)?
fn writes_manifest(l: &BuildLine, c: &str) -> bool {
    l.text
        .contains("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json")
        && has_seq(&l.tokens, &["-p", c])
        && has_seq(&l.tokens, &["--test", "warrant_evidence"])
}

// ---------------------------------------------------------------------------------------
// The checks
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

fn read_text(path: &Path, cap: usize) -> Result<Option<String>, NoVerdict> {
    match read_regular_opt(path, cap)? {
        None => Ok(None),
        Some(b) => String::from_utf8(b)
            .map(Some)
            .map_err(|_| NoVerdict::new(format!("{}: not UTF-8", q(path)))),
    }
}

/// What the checks share.
struct Ctx {
    krate: String,
    dir: PathBuf,
    root: PathBuf,
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

    // --- CC01: vos.toml and the kind -------------------------------------------------
    let vt_path = layout.impl_dir.join("vos.toml");
    let Some(text) = read_text(&vt_path, VOS_TOML_MAX_BYTES)? else {
        out.findings
            .push(Finding::new("CC01", "vos.toml: not found"));
        return Ok(out);
    };
    let vt = match parse_vos_toml(&text) {
        Ok(v) => v,
        Err((line, why)) => {
            out.findings.push(Finding::new(
                "CC01",
                format!("vos.toml:{line}: {}", esc(&why)),
            ));
            return Ok(out);
        }
    };
    let kind = match vt.kind {
        Some(k @ (Kind::Production | Kind::Reference | Kind::Any)) => k,
        other => {
            let name = other.map_or("(none)", Kind::name);
            out.findings.push(Finding::new(
                "CC01",
                format!(
                    "vos.toml: kind `{name}` is not an implementation kind; check-component judges production, reference and any crates (VOS04 5.6)"
                ),
            ));
            return Ok(out);
        }
    };
    out.header = format!("crate {}, kind {}", layout.impl_pkg, kind.name());
    let mut cx = Ctx {
        krate: layout.impl_pkg.clone(),
        dir: layout.impl_dir.clone(),
        root: layout.root.clone(),
        vt,
        kind,
        f: Vec::new(),
    };

    file_set(&mut cx)?;
    build_files(&mut cx)?;
    build_tool_row(&mut cx)?;
    publish_rule(&mut cx)?;
    manifest(&mut cx)?;

    out.findings = cx.f;
    out.notes.push(
        "check-component: the file set, the BUILD lines of VOS04 5.2 (1 to 6, 9), their guards, the build-tool row, publish = false for `-any` and the coverage gate are present and consistent. Not checked: that the lines PASS (the build does that), the exact operations-times-tiers control count (VOS-D104), a parser's fuzz corpus (VOS-D105), the command-type check (VOS-D077), and line 10 (SAT, n/a until VOS-029).".to_string(),
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
        "evidence.bindings",
        "evidence/test-manifest.json",
        "tests/generated/warrant_evidence.rs",
    ];
    for rel in required {
        if !present(&cx.dir.join(rel))? {
            cx.find("CC02", format!("file set: {rel} is missing"));
        }
    }
    if !present(&cx.dir.join("CHANGELOG.md"))? && !present(&cx.dir.join("CHANGELOG.d/_meta.md"))? {
        cx.find(
            "CC02",
            "file set: neither CHANGELOG.md nor CHANGELOG.d/_meta.md exists",
        );
    }
    let doc = cx
        .root
        .join("code")
        .join("specs")
        .join(format!("{}.warrant", cx.krate));
    if !present(&doc)? {
        cx.find(
            "CC02",
            format!(
                "file set: the generated document code/specs/{}.warrant is missing (run `vos-link gen-doc`)",
                cx.krate
            ),
        );
    }
    Ok(())
}

/// CC03, CC04, CC07 (the BUILD half): both BUILD files.
fn build_files(cx: &mut Ctx) -> Result<(), NoVerdict> {
    for (name, linux) in [("BUILD", true), ("BUILD_windows", false)] {
        let Some(text) = read_text(&cx.dir.join(name), BUILD_MAX_BYTES)? else {
            continue; // CC02 reported it
        };
        let lines = build_lines(&text);
        check_one_build(cx, name, &lines)?;
        if linux {
            coverage_line(cx, &lines);
        }
    }
    Ok(())
}

fn check_one_build(cx: &mut Ctx, name: &str, lines: &[BuildLine]) -> Result<(), NoVerdict> {
    let krate = cx.krate.clone();
    // Lines 1 to 5.
    for need in needs_1_to_5(&krate, &cx.vt.evidence_features) {
        if !lines.iter().any(|l| need.matches(l)) {
            cx.find(
                "CC03",
                format!(
                    "{name}: line {} of VOS04 5.2 ({}) is missing; no line contains {}",
                    need.line_no,
                    need.what,
                    need.describe()
                ),
            );
        }
    }

    // Line 6: the generated suite rewrites the manifest.
    let mut run_at: Option<usize> = None;
    for (i, l) in lines.iter().enumerate() {
        if writes_manifest(l, &krate) {
            run_at = Some(i);
            break;
        }
        // A line that runs the crate's tolerance script, which must itself do it.
        if l.tokens
            .iter()
            .any(|t| t.ends_with("warrant/check_evidence.sh"))
        {
            let script = cx.dir.join("warrant").join("check_evidence.sh");
            if let Some(s) = read_text(&script, SCRIPT_MAX_BYTES)? {
                if s.contains("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json")
                    && s.contains("--test warrant_evidence")
                    && s.contains(&format!("-p {krate}"))
                {
                    run_at = Some(i);
                    break;
                }
            }
        }
    }
    if run_at.is_none() {
        cx.find(
            "CC03",
            format!(
                "{name}: line 6 of VOS04 5.2 (the generated suite, which rewrites the manifest) is missing; no line sets WARRANT_EVIDENCE_OUT=evidence/test-manifest.json with `-p {krate} --test warrant_evidence`, and none runs warrant/check_evidence.sh to do it"
            ),
        );
    }

    // Line 9: sabotage, each line bail-out aware.
    let sabotage: Vec<&BuildLine> = lines
        .iter()
        .filter(|l| l.text.contains("WARRANT_EVIDENCE_FAULT="))
        .collect();
    if sabotage.is_empty() {
        cx.find(
            "CC03",
            format!("{name}: line 9 of VOS04 5.2 (sabotage, WARRANT_EVIDENCE_FAULT=...) is missing: no negative control runs"),
        );
    }
    for l in sabotage {
        if !(l.text.contains("Bail out!") || l.text.contains("warrant_negctl.py")) {
            cx.find(
                "CC04",
                format!(
                    "{name}:{}: a sabotage line is not bail-out-aware (it must test for `Bail out!` or run code/scripts/warrant_negctl.py): a hung run would pass it (VOS-D088)",
                    l.number
                ),
            );
        }
    }

    // This command, after line 6.
    let need = check_component_need();
    let at = lines.iter().position(|l| need.matches(l));
    match (at, run_at) {
        (None, _) => cx.find(
            "CC03",
            format!(
                "{name}: the check-component line is missing; no line contains {}",
                need.describe()
            ),
        ),
        (Some(a), Some(r)) if a < r => cx.find(
            "CC04",
            format!(
                "{name}:{}: check-component runs before the generated suite (line {}) and would read a stale manifest; it must come after it",
                lines[a].number, lines[r].number
            ),
        ),
        _ => {}
    }
    Ok(())
}

/// CC07: the declared floor and the Linux BUILD line that keeps it true.
fn coverage_line(cx: &mut Ctx, lines: &[BuildLine]) {
    let krate = cx.krate.clone();
    let Some(floor) = cx.vt.coverage_floor else {
        cx.find(
            "CC07",
            "coverage: vos.toml declares no `coverage-floor` (VOS04 5.6 item G, VOS-D070)",
        );
        return;
    };
    if floor < COVERAGE_FLOOR_MIN {
        cx.find(
            "CC07",
            format!("coverage: coverage-floor = {floor} is below the repository minimum {COVERAGE_FLOOR_MIN}"),
        );
    }
    let want_include = format!("{krate}/src/*");
    let ok = lines
        .iter()
        .filter(|l| has_seq(&l.tokens, &["cargo", "tarpaulin"]))
        .any(|l| {
            let t = &l.tokens;
            let features_ok = cx
                .vt
                .evidence_features
                .iter()
                .all(|f| after(t, "--features").is_some_and(|v| v.split(',').any(|x| x == f)));
            l.text.contains("\"Linux\"")
                && l.text.contains("uname")
                && has_seq(t, &["-p", &krate])
                && features_ok
                && t.iter().any(|x| x == "--ignore-tests")
                && has_seq(t, &["--engine", "llvm"])
                && after(t, "--include-files") == Some(want_include.as_str())
                && after(t, "--fail-under") == Some(floor.to_string().as_str())
        });
    if !ok {
        cx.find(
            "CC07",
            format!(
                "coverage: BUILD has no Linux line `cargo tarpaulin -p {krate} --features <evidence features> --lib --test behaviour --ignore-tests --engine llvm --include-files '{want_include}' --fail-under {floor}` guarded by `uname` = \"Linux\" (the floor of vos.toml, VOS04 5.6 item G)"
            ),
        );
    }
}

/// CC05: the build-tool row (section 5.2 line 11).
fn build_tool_row(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let path = cx.root.join(BUILD_TOOL_ROWS);
    let row = format!("vosImplementationRow(\"{}\"),", cx.krate);
    match read_text(&path, BUILD_ROWS_MAX_BYTES)? {
        None => cx.find(
            "CC05",
            format!("build-tool row: {BUILD_TOOL_ROWS} not found"),
        ),
        Some(src) => {
            if !src.lines().any(|l| l.trim() == row) {
                cx.find(
                    "CC05",
                    format!(
                        "build-tool row: {BUILD_TOOL_ROWS} has no line `{row}`, so an edit to a VOS tool or to code/specs/{}.warrant selects nothing (VOS04 5.2 line 11, VOS-D066)",
                        cx.krate
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
    let path = cx.dir.join("Cargo.toml");
    let Some(text) = read_text(&path, CARGO_TOML_MAX_BYTES)? else {
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

fn get<'a>(o: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    o.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn int(o: &[(String, JsonValue)], key: &str) -> Option<i64> {
    match get(o, key) {
        Some(JsonValue::Number(JsonNumber::Integer(n))) => Some(*n),
        _ => None,
    }
}

fn string<'a>(o: &'a [(String, JsonValue)], key: &str) -> Option<&'a str> {
    match get(o, key) {
        Some(JsonValue::String(s)) => Some(s),
        _ => None,
    }
}

/// CC08: the guards of lines 6 to 9, read from the manifest the generated suite wrote.
fn manifest(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let path = cx.dir.join("evidence").join("test-manifest.json");
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

    // The exceptions: the ids the crate's tolerance script names (VOS-D065, VOS-D098).
    let script = read_text(
        &cx.dir.join("warrant").join("check_evidence.sh"),
        SCRIPT_MAX_BYTES,
    )?
    .unwrap_or_default();

    let mut exhaustive: BTreeSet<String> = BTreeSet::new();
    let mut property: BTreeSet<String> = BTreeSet::new();
    let mut fuzz: BTreeSet<String> = BTreeSet::new();
    let mut negctl_tiers: BTreeSet<String> = BTreeSet::new();
    let mut claims: BTreeSet<String> = BTreeSet::new();
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
        if verdict != "pass" {
            let excused = kind == "negctl" && script.contains(id);
            if !excused {
                cx.find(
                    "CC08",
                    format!(
                        "manifest: test {} has verdict {}, not pass",
                        esc(id),
                        esc(verdict)
                    ),
                );
            }
        }
        match kind {
            "exhaustive" => {
                claims.insert(claim.clone());
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
                claims.insert(claim.clone());
                let n = int(o, "cases").unwrap_or(0);
                if n < PROPERTY_CASES_MIN {
                    cx.find(
                        "CC08",
                        format!(
                            "manifest: claim {} ran {n} property cases, fewer than {PROPERTY_CASES_MIN} (line 7)",
                            esc(&claim)
                        ),
                    );
                }
                property.insert(claim);
            }
            "fuzz" => {
                claims.insert(claim.clone());
                let n = int(o, "iterations").unwrap_or(0);
                if n < FUZZ_ITERATIONS_MIN {
                    cx.find(
                        "CC08",
                        format!(
                            "manifest: claim {} ran {n} fuzz iterations, fewer than {FUZZ_ITERATIONS_MIN} (line 8)",
                            esc(&claim)
                        ),
                    );
                }
                fuzz.insert(claim);
            }
            "negctl" => {
                if let Some(target) = string(o, "target") {
                    negctl_tiers.insert(target.to_string());
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
    if claims.is_empty() {
        cx.find(
            "CC08",
            "manifest: no claim has an exhaustive, property or fuzz entry (lines 6 to 8, none ran = fail)",
        );
    }
    for c in &claims {
        for (set, tier, line) in [
            (&exhaustive, "exhaustive", "6"),
            (&property, "property", "7"),
            (&fuzz, "fuzz", "8"),
        ] {
            if !set.contains(c) {
                cx.find(
                    "CC08",
                    format!(
                        "manifest: claim {} has no {tier} entry (line {line}, none ran = fail)",
                        esc(c)
                    ),
                );
            }
        }
    }
    for tier in ["exhaustive", "property", "fuzz"] {
        if !negctl_tiers.contains(tier) {
            cx.find(
                "CC08",
                format!(
                    "manifest: no negative control ran in the {tier} tier (line 9, none ran = fail)"
                ),
            );
        }
    }
    Ok(())
}
