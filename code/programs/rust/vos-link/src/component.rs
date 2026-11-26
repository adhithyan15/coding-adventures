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
//!   CC09 `bound` rows against the ceilings of contract.vos (VOS-039b-2b, [`crate::bounds`])
//!   CC10 a contract crate has no dependency, build script or proc-macro (VOS-D112)
//!   CC11 every `-any` attacks the oracle of each state-producing operation (VOS-039c, [`crate::oracle`])
//! ```
//!
//! ## Contract crates (VOS-039b-1)
//!
//! A crate of kind `contract` is judged by a shorter branch ([`contract_component`]): CC02 (its
//! own file set), CC03 (BUILD and BUILD_windows against `canonical_contract_build`), CC05
//! (`vosContractRow`) and CC10 (no dependency at all, VOS-D112).  CC04, CC06, CC07 and CC08 are about evidence, a document, `-any` crates
//! and coverable code, none of which a contract has.  The lemmas themselves are `check-contract`'s,
//! which the generated BUILD runs.
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

use crate::bounds;
use crate::build_gen::{
    canonical_build, canonical_contract_build, load_inputs, tolerance_script, GenInputs, PLAN_FILE,
    TOLERANCE_SCRIPT,
};
use crate::check::first_difference_line;
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, list_names, q, read_regular_opt};
use crate::inputs::{resolve_layout, Layout};
use crate::ops::Outcome;
use crate::oracle;

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
const BINDINGS_MAX_BYTES: usize = 256 * 1024;

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
        Some(Kind::Production | Kind::Reference | Kind::Any | Kind::Contract) => Ok(Ok(vt)),
        other => {
            let name = other.map_or("(none)", Kind::name);
            Ok(Err(Finding::new(
                "CC01",
                format!(
                    "vos.toml: kind `{name}` is not a component kind; check-component judges production, reference, any and contract crates (VOS04 5.6)"
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
    row_in_consumers_with(src, "vosImplementationRow", krate)
}

/// [`row_in_consumers`] for the row function `func`: `vosImplementationRow` for an
/// implementation, `vosContractRow` for a contract crate (VOS-039b-1).
pub fn row_in_consumers_with(src: &str, func: &str, krate: &str) -> bool {
    let stripped = strip_go(src);
    let row = format!("{func}(\"{krate}\"),");
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
    /// What a pass does not establish, and what was left out (printed with the result).
    notes: Vec<String>,
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
        notes: Vec::new(),
    };

    if kind == Kind::Contract {
        contract_component(&mut cx)?;
        out.findings = cx.f;
        out.notes = cx.notes;
        out.notes.push(
            "check-component (contract crate): the file set, BUILD and BUILD_windows byte-equal to `gen-build`'s contract region (VOS04 5.6, which runs `check-contract`: lemmas, mutants, config mutants, the Rust-enum/Warrant-variant correspondence and the generated table, and `vos-shape-check` with its command-type rule SH19) and the build-tool row `vosContractRow` are present and consistent. Not checked: that the lines PASS (the build does that). A contract has no evidence manifest, coverage floor or `-any` rule (VOS10 section 15).".to_string(),
        );
        return Ok(out);
    }
    file_set(&mut cx)?;
    match load_inputs(&cx.layout, &cx.vt)? {
        Ok(g) => {
            builds(&mut cx, &g)?;
            manifest(&mut cx, &g)?;
            oracle_rule(&mut cx, &g)?;
        }
        Err(findings) => cx.f.extend(findings),
    }
    build_tool_row(&mut cx)?;
    publish_rule(&mut cx)?;
    bounds_rule(&mut cx)?;
    no_sat_claim(&mut cx)?;

    out.findings = cx.f;
    out.notes = cx.notes;
    out.notes.push(
        "check-component: the file set, BUILD and BUILD_windows byte-equal to `gen-build` (VOS04 5.2 lines 1 to 9, the coverage gate and this command), the sabotage plan, the tolerance script, the build-tool row, publish = false for `-any`, the `bound` rows at or under the contract's ceilings (CC09), the oracle attack of each state-producing operation of an `-any` (CC11) and the evidence manifest bound to the document are present and consistent. Not checked: that the lines PASS (the build does that), that a bound is the TRUE worst case (the step counter, VOS-D089), the exact operations-times-tiers control count (VOS-D105), a parser's fuzz corpus (VOS-D106), the command-type check (VOS-D077), deriving the plan (VOS-D108), and line 10 (SAT, n/a until VOS-029; CC12 fails an implementation whose vos.toml, BUILD or warrant/ makes any SAT claim).".to_string(),
    );
    Ok(out)
}

/// CC12 (VOS-040c, VOS31 10.2): an **implementation** crate makes no SAT claim.
///
/// VOS04 5.2 line 10 reads "no SAT evidence" until VOS-029 can lower a scheduler to SAT, and
/// "the line fails if it prints any SAT claim for one".  The model-SAT lane certifies the
/// LEMMAS of a contract's model (VOS31 section 1); it says nothing about code.  So nothing of
/// the lane may appear on an implementation, and this rule is the negative control that keeps
/// it so.  It fails when the implementation
///
/// * says `sat = "model"` in `vos.toml` (the key belongs to a contract),
/// * has any file under `warrant/` whose name contains `.sat` (`lemmas.sat`, but also
///   `lemmas.sat.bak` or `foo.sat`: a table of SAT certificates by any name), or
/// * names `vos-model-sat` or `lemmas.sat` on any non-comment LOGICAL line of `BUILD` or
///   `BUILD_windows`, or runs cargo with a `--manifest-path` into `programs/rust/vos-model-sat`.
///
/// A logical line joins backslash-continued physical lines first (`vos-model-\` newline `sat`
/// is one name).  Quotes are dropped before the scan, so `vos-model-"sat"` is caught too.
///
/// **Defence in depth, not a proof.**  A substring scan cannot be complete: shell concatenation
/// (`vos-model-$X`), globs (`s?t`), command substitution or a script that builds the path at run
/// time all evade it.  Generated BUILD files never contain any of these (`canonical_build` has
/// no SAT line), so a hit is a hand edit or a copy from a contract crate, and the generated-region
/// byte comparison (CC03) is what really keeps a hand-added line out.
fn no_sat_claim(cx: &mut Ctx) -> Result<(), NoVerdict> {
    if cx.vt.sat.is_some() {
        cx.find(
            "CC12",
            "SAT: vos.toml says `sat = \"model\"` on an implementation crate; the model-SAT lane certifies a contract's lemmas only, and an implementation prints \"no SAT evidence\" (VOS04 5.2 line 10, VOS31 10.2)",
        );
    }
    for name in list_names(&cx.layout.impl_dir.join("warrant"))? {
        if name.contains(".sat") {
            cx.find(
                "CC12",
                format!(
                    "SAT: the file {} under warrant/ exists in an implementation crate; a table of SAT certificates, by any name, belongs to a contract crate only (VOS31 10.2)",
                    esc(&name)
                ),
            );
        }
    }
    for name in ["BUILD", "BUILD_windows"] {
        let Some(text) = read_text(&cx.layout.impl_dir.join(name), BUILD_MAX_BYTES)? else {
            continue; // CC02 reported it
        };
        let hit = logical_lines(&text)
            .into_iter()
            .find(|(_, l)| !l.trim_start().starts_with('#') && mentions_sat_lane(l));
        if let Some((n, line)) = hit {
            cx.find(
                "CC12",
                format!(
                    "SAT: {name}:{n}: an implementation's BUILD runs the model-SAT lane (`{}`); an implementation prints \"no SAT evidence\" (VOS31 10.2)",
                    esc(line.trim())
                ),
            );
        }
    }
    Ok(())
}

/// The logical lines of a BUILD file, each with the 1-based number of its first physical line:
/// a physical line ending in a backslash continues onto the next (the backslash is dropped and
/// the pieces are joined with nothing between them, as `sh` does for an unquoted continuation).
pub fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (i, raw) in text.lines().enumerate() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let (start, mut acc) = open.take().unwrap_or((i + 1, String::new()));
        match raw.strip_suffix('\\') {
            Some(piece) => {
                acc.push_str(piece);
                open = Some((start, acc));
            }
            None => {
                acc.push_str(raw);
                out.push((start, acc));
            }
        }
    }
    if let Some(rest) = open {
        out.push(rest);
    }
    out
}

/// Does a LOGICAL BUILD line name the model-SAT lane or its table, or point cargo at the lane's
/// manifest?  Quotes and backslashes are dropped first, and path separators normalised, so
/// `vos-model-"sat"` and a Windows-style `programs\rust\vos-model-sat\Cargo.toml` are seen as what they name.
/// Not complete (see [`no_sat_claim`]).
pub fn mentions_sat_lane(line: &str) -> bool {
    let norm: String = line
        .chars()
        .filter(|c| *c != '"' && *c != '\\')
        .collect::<String>();
    let slashed = line.replace('\\', "/").replace(['"', '\''], "");
    let manifest_into_lane =
        (slashed.contains("--manifest-path")) && slashed.contains("programs/rust/vos-model-sat");
    manifest_into_lane || norm.contains("vos-model-sat") || norm.contains("lemmas.sat")
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

/// The checks of a **contract crate** (VOS-039b-1): CC02 (file set), CC03 (BUILD files byte for
/// byte) and CC05 (the `vosContractRow` row).  CC04 and CC06 to CC08 do not apply: a contract has
/// no bindings, no document of its own, no evidence manifest and no coverable code.
fn contract_component(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let dir = cx.layout.impl_dir.clone();
    let mut required: Vec<String> = [
        "BUILD",
        "BUILD_windows",
        "README.md",
        "Cargo.toml",
        "vos.toml",
        "required_capabilities.json",
        "src/lib.rs",
        "contract.vos",
        "warrant/contract.part",
        "warrant/lemmas.part",
        "warrant/lemmas.table",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // One obligations part per `tier` row: the tier list is read by the tool's own parser.
    match read_regular_opt(
        &dir.join("contract.vos"),
        vos_manifest::caps::CONTRACT_MAX_BYTES,
    )? {
        None => {}
        Some(bytes) => match vos_manifest::contract::parse_contract(&bytes) {
            Ok(c) => {
                for t in &c.tiers {
                    required.push(format!("warrant/{}", crate::inputs::obligations_file(*t)));
                }
            }
            Err(e) if e.is_cap() => return Err(NoVerdict::new(e.to_string())),
            Err(e) => cx.find("CC02", format!("contract.vos: {}", esc(&e.to_string()))),
        },
    }
    for rel in &required {
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

    // CC03: BUILD and BUILD_windows begin with the generated region, byte for byte.
    let want = canonical_contract_build(&cx.krate, cx.vt.sat);
    for name in ["BUILD", "BUILD_windows"] {
        let Some(found) = read_text(&dir.join(name), BUILD_MAX_BYTES)? else {
            continue; // CC02 reported it
        };
        if !found.starts_with(&want) {
            let n = first_difference_line(&found, &want);
            cx.find(
                "CC03",
                format!(
                    "{name}:{n}: the file does not begin with the region `vos-link gen-build` writes for a contract crate; expected {}, found {}",
                    show_line(&want, n),
                    show_line(&found, n)
                ),
            );
        }
    }

    // CC10: no dependency, no build script, no proc-macro (VOS-D112).
    contract_dependencies(cx)?;

    // CC05: the build-tool row.
    let path = cx.layout.root.join(BUILD_TOOL_ROWS);
    match read_text(&path, BUILD_ROWS_MAX_BYTES)? {
        None => cx.find(
            "CC05",
            format!("build-tool row: {BUILD_TOOL_ROWS} not found"),
        ),
        Some(src) => {
            if !row_in_consumers_with(&src, "vosContractRow", &cx.krate) {
                cx.find(
                    "CC05",
                    format!(
                        "build-tool row: {BUILD_TOOL_ROWS} has no line `vosContractRow(\"{}\"),` inside the repositoryInputConsumers slice (comments and raw strings do not count), so an edit to a VOS tool selects nothing (VOS04 5.2 line 11, VOS-039b-1)",
                        cx.krate
                    ),
                );
            }
        }
    }
    Ok(())
}

/// The dependency tables Cargo reads for code that is part of a build (the underscore spellings
/// are accepted by Cargo as aliases, so they count).  `dev-dependencies` are NOT here: they run
/// in tests only.
const BUILD_DEP_TABLES: [&str; 3] = ["dependencies", "build-dependencies", "build_dependencies"];

/// CC10 (VOS-D112): a contract crate is declarations only, so nothing in its manifest may bring
/// foreign code into its build.
///
/// SH19 judges the crate's own text; a path dependency could supply an attribute macro or a
/// foreign derive that rewrites a command enum after SH19 judged it (`#[evil::rewrite]`).  So the
/// manifest of a contract crate has:
///
/// ```text
///   [dependencies]  [build-dependencies]  [target.*.dependencies]    no entry (an empty table is fine)
///   package.build  /  a build.rs file                                 none
///   [lib] proc-macro                                                  absent, or the literal false
///   [lib] crate-type                                                  never names "proc-macro"
///   [patch]  [replace]  [workspace]                                   none (each can redirect code)
/// ```
///
/// `[dev-dependencies]` are allowed: they build only under `cargo test`, and SH12 judges their
/// allowlist.  The tables are found by FLATTENED KEY PATH (`toml_reader`), so `[dependencies.x]`,
/// an inline table, a dotted key and a quoted key all land in the same place.  A `Cargo.toml` that
/// cannot be read is no verdict (exit 2): the rule could not be applied.
fn contract_dependencies(cx: &mut Ctx) -> Result<(), NoVerdict> {
    let dir = cx.layout.impl_dir.clone();
    let Some(text) = read_text(&dir.join("Cargo.toml"), CARGO_TOML_MAX_BYTES)? else {
        return Ok(()); // CC02 reported it
    };
    let entries = toml_reader::parse(&text).map_err(|e| {
        NoVerdict::new(format!(
            "Cargo.toml:{}: {} (CC10 needs the manifest)",
            e.line,
            esc(&e.message)
        ))
    })?;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for e in &entries {
        let p: Vec<&str> = e.path.iter().map(String::as_str).collect();
        // The position of a dependency-table name: the root, or under `target.<cfg>`.
        let at = match p.as_slice() {
            [t, ..] if BUILD_DEP_TABLES.contains(t) => Some(0),
            ["target", _, t, ..] if BUILD_DEP_TABLES.contains(t) => Some(2),
            _ => None,
        };
        if let Some(at) = at {
            // A member (`p.len() > at + 1`), or a table written as a non-table value, is a
            // dependency.  The bare header `[dependencies]` is `Value::Table` with no member.
            // `a = { path = ".." }` flattens to TWO entries (`a` and `a.path`); one finding
            // per dependency name, at its first line.
            if (p.len() > at + 1 || e.value != Value::Table)
                && seen.insert(p[..(at + 2).min(p.len())].join("."))
            {
                cx.find(
                    "CC10",
                    format!(
                        "Cargo.toml:{}: `{}` is a dependency of a contract crate; a contract has none (an attribute macro or derive it supplied could rewrite a command enum after SH19 judged it, VOS-D112)",
                        e.line,
                        esc(&p[..(at + 2).min(p.len())].join("."))
                    ),
                );
            }
        }
        // `[patch]` and `[replace]` swap the code behind a name; `[workspace]` can pull in a
        // member or a `[workspace.dependencies]` the crate then inherits.  None belongs to a
        // declarations-only crate, so each is refused at its first line.
        if let [top @ ("patch" | "replace" | "workspace"), ..] = p.as_slice() {
            if seen.insert((*top).to_string()) {
                cx.find(
                    "CC10",
                    format!(
                        "Cargo.toml:{}: `{}` can redirect or inherit dependencies; a contract crate has none of these tables (VOS-D112)",
                        e.line,
                        esc(top)
                    ),
                );
            }
        }
        // `crate-type = [.., "proc-macro"]` makes the library a procedural macro just as
        // `proc-macro = true` does.
        let is_crate_type_proc = matches!(p.as_slice(), ["lib", "crate-type" | "crate_type"])
            && match &e.value {
                Value::Array(items) => items.iter().any(|v| *v == Value::Str("proc-macro".into())),
                Value::Str(s) => s == "proc-macro",
                _ => false,
            };
        let is_build_key = matches!(p.as_slice(), ["package" | "project", "build"]);
        let is_proc_macro = matches!(p.as_slice(), ["lib", "proc-macro" | "proc_macro"])
            && e.value != Value::Bool(false);
        if is_build_key || is_proc_macro || is_crate_type_proc {
            cx.find(
                "CC10",
                format!(
                    "Cargo.toml:{}: `{}` runs code at build time; a contract crate has no build script or procedural macro (VOS-D112)",
                    e.line,
                    esc(&p.join("."))
                ),
            );
        }
    }
    for name in list_names(&dir)? {
        if name.eq_ignore_ascii_case("build.rs") {
            cx.find(
                "CC10",
                format!(
                    "{}: a build script exists; Cargo runs it before the crate (VOS-D112)",
                    esc(&name)
                ),
            );
        }
    }
    Ok(())
}

/// CC11 (VOS-039c), kind `any` only: the evidence can fail when an operation loses the oracle.
///
/// The inputs are read here and judged by [`oracle::judge`], which is pure.  A crate of another
/// kind is not subject to the rule: a production crate has no oracle (CD07 requires every field
/// of its `Impl` to be in the cone of `build`).
fn oracle_rule(cx: &mut Ctx, g: &GenInputs) -> Result<(), NoVerdict> {
    if cx.kind != Kind::Any {
        return Ok(());
    }
    let Some(bindings) = read_text(
        &cx.layout.impl_dir.join("evidence.bindings"),
        BINDINGS_MAX_BYTES,
    )?
    else {
        return Ok(()); // CC02 reported it
    };
    let Some(document) = read_text(
        &cx.layout.specs_dir().join(cx.layout.doc_name()),
        vos_manifest::caps::DOCUMENT_MAX_BYTES,
    )?
    else {
        return Ok(()); // CC02 reported it
    };
    // The contract's `any` claims, by name and digest: the claims the contract owns.  A contract
    // that cannot be read, or that has no `any` claim, leaves nothing to attack (fails closed;
    // the other rules report why the contract cannot be read).
    let mut unread = Vec::new();
    let contract_any: Vec<vos_manifest::warrant::ClaimFact> = match cx.vt.contract.as_deref() {
        Some(pkg) => crate::inputs::load_contract_package(&cx.layout, pkg, &mut unread)?
            .map(|p| {
                p.tier_claims
                    .into_iter()
                    .filter(|(t, _)| *t == vos_manifest::Tier::Any)
                    .flat_map(|(_, c)| c)
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    match oracle::load(&bindings, document)? {
        Err(message) => cx.find("CC11", format!("oracle: {message}")),
        Ok(d) => {
            let owned: BTreeSet<String> = d
                .claims
                .iter()
                .filter(|c| contract_any.contains(c))
                .map(|c| c.name.clone())
                .collect();
            let facts = oracle::Facts {
                build: &d.build,
                successors: &d.successors,
                last_field: d.last_field.as_deref(),
                unclassified: &d.unclassified,
                owned: &owned,
                faults: &g.faults,
                plan: &g.plan,
                calls: &g.calls,
            };
            for m in oracle::judge(&facts) {
                cx.find("CC11", m);
            }
        }
    }
    Ok(())
}

/// CC09 (VOS-039b-2b): the document's `bound` rows against the ceilings of `contract.vos`.
fn bounds_rule(cx: &mut Ctx) -> Result<(), NoVerdict> {
    match bounds::load(&cx.layout, cx.vt.contract.as_deref())? {
        bounds::Inputs::Stop(messages) => {
            for m in messages {
                cx.find("CC09", m);
            }
        }
        bounds::Inputs::Ready(contract, facts) => {
            let verdict = bounds::judge(cx.kind != Kind::Reference, &contract, &facts);
            for m in verdict.findings {
                cx.find("CC09", m);
            }
            cx.notes.extend(verdict.notes);
        }
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
