//! # `gen-build`: the canonical region of BUILD, generated (VOS-039a, VOS04 section 5.6)
//!
//! The first draft of `check-component` *recognised* the required BUILD lines in the author's
//! text.  A security review showed that cannot hold: the build tool runs **every non-comment
//! line as its own `sh -c`**, so `echo cargo test ...`, `true # cargo test ...`,
//! `false && ...`, `... || true`, a `"Linux"` in a comment, a second `--fail-under 0` or an
//! `--exclude-files` all contain the required words and run something else.  A lint over
//! author-written text never converges (the same lesson as VOS-012b0).  So the lines are not
//! recognised, they are **reproduced**: this module writes them from the crate's declared
//! inputs, and `check-component` requires the file to begin with exactly what it writes, the
//! way `check-doc` requires the document to equal what `gen-doc` writes.
//!
//! ```text
//!   vos.toml ─────────────── kind, evidence-features, coverage-floor, excused-controls
//!   evidence.bindings ────── the bound operations and their faults   (the generator's reader)
//!   code/specs/<crate>.warrant ─ claim and example names              (parsed, not scanned)
//!   warrant/sabotage.plan ── which claim attacks which fault          (names only, no commands)
//!        │
//!        ▼  canonical_build()
//!   BUILD, BUILD_windows ─── header, lines 1-9, the Linux coverage gate, check-component,
//!                            then MARKER.  Below the marker: the author's, run, never counted.
//!   warrant/check_evidence.sh ─ only when excused-controls is declared
//! ```
//!
//! ## The sabotage plan
//!
//! The claim-to-operation map lives inside the Warrant expressions, so which claim should
//! attack which fault is a choice the author records, one entry per line:
//! `<tier>/<name> <fault> [attack=<op>]`.  [`validate_plan`] holds the entries to the
//! document (the claim or example exists) and to the bindings (the fault is bound, every
//! bound fault and every `field-flip` operation is attacked).  Whether the claim really
//! *catches* the fault is the run's: the shared checker `code/scripts/warrant_negctl.py`
//! fails a control that does not.  A plan holds names, never command text.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use vos_manifest::warrant::declared_names;

use crate::check::WARRANT_STACK_BYTES;
use crate::component::{read_component_vos_toml, read_text};
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, write_replacing};
use crate::inputs::resolve_layout;
use crate::ops::Outcome;

/// The line that ends the generated region.  Everything after it is the author's.
pub const MARKER: &str = "# --- crate-specific (run, but never counted as evidence) ---";
/// The sabotage plan, relative to the crate directory.
pub const PLAN_FILE: &str = "warrant/sabotage.plan";
/// The generated tolerance script, relative to the crate directory.
pub const TOLERANCE_SCRIPT: &str = "warrant/check_evidence.sh";
/// The record fault that cancels itself when two operations list it (VER04j 4.8).
pub const FIELD_FLIP: &str = "field-flip";

const PLAN_MAX_BYTES: usize = 64 * 1024;
const BINDINGS_MAX_BYTES: usize = 256 * 1024;
const DOC_MAX_BYTES: usize = vos_manifest::caps::DOCUMENT_MAX_BYTES;
const PLAN_MAX_ENTRIES: usize = 512;

/// One line of the sabotage plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEntry {
    /// `exhaustive` or `example`.
    pub tier: String,
    /// The claim or example name.
    pub name: String,
    /// The fault injected.
    pub fault: String,
    /// `attack=<op>`, for `field-flip`.
    pub attack: Option<String>,
    /// 1-based line of the plan, for findings.
    pub line: usize,
}

/// What generation needs, all of it read from files a reviewer sees.
#[derive(Clone, Debug)]
pub struct GenInputs {
    /// The crate name.
    pub krate: String,
    /// `vos.toml` `evidence-features`.
    pub features: Vec<String>,
    /// `vos.toml` `coverage-floor`.
    pub floor: u8,
    /// `vos.toml` `excused-controls`.
    pub excused: Vec<String>,
    /// The sabotage plan, in file order.
    pub plan: Vec<PlanEntry>,
    /// The claims of the document.
    pub claims: Vec<String>,
    /// The examples of the document.
    pub examples: Vec<String>,
    /// `evidence.bindings` `property_cases`.
    pub property_cases: u64,
    /// `evidence.bindings` `fuzz_iterations`.
    pub fuzz_iterations: u64,
}

fn plain_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn plain_fault(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Parse the plan text.  Pure.  Every problem is one message naming the line.
pub fn parse_plan(text: &str) -> (Vec<PlanEntry>, Vec<String>) {
    let mut entries: Vec<PlanEntry> = Vec::new();
    let mut problems = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let at = |m: String| format!("{PLAN_FILE}:{line}: {m}");
        let words: Vec<&str> = t.split_whitespace().collect();
        if words.len() < 2 || words.len() > 3 {
            problems.push(at(format!(
                "an entry is `<tier>/<name> <fault> [attack=<op>]`, found {}",
                esc(t)
            )));
            continue;
        }
        let Some((tier, name)) = words[0].split_once('/') else {
            problems.push(at(format!("{} is not `<tier>/<name>`", esc(words[0]))));
            continue;
        };
        if tier != "exhaustive" && tier != "example" {
            problems.push(at(format!(
                "tier {} is not exhaustive or example",
                esc(tier)
            )));
            continue;
        }
        if !plain_name(name) {
            problems.push(at(format!("{} is not a plain name", esc(name))));
            continue;
        }
        let fault = words[1];
        if !plain_fault(fault) {
            problems.push(at(format!("{} is not a plain fault name", esc(fault))));
            continue;
        }
        let attack = match words.get(2) {
            None => None,
            Some(w) => match w.strip_prefix("attack=") {
                Some(op) if plain_name(op) => Some(op.to_string()),
                _ => {
                    problems.push(at(format!("{} is not `attack=<op>`", esc(w))));
                    continue;
                }
            },
        };
        if (fault == FIELD_FLIP) != attack.is_some() {
            problems.push(at(format!(
                "`{FIELD_FLIP}` takes `attack=<op>` and no other fault does"
            )));
            continue;
        }
        let e = PlanEntry {
            tier: tier.to_string(),
            name: name.to_string(),
            fault: fault.to_string(),
            attack,
            line,
        };
        if let Some(prev) = entries.iter().find(|p| {
            (&p.tier, &p.name, &p.fault, &p.attack) == (&e.tier, &e.name, &e.fault, &e.attack)
        }) {
            problems.push(at(format!("repeats the entry of line {}", prev.line)));
            continue;
        }
        if entries.len() >= PLAN_MAX_ENTRIES {
            problems.push(at(format!("more than {PLAN_MAX_ENTRIES} entries")));
            break;
        }
        entries.push(e);
    }
    (entries, problems)
}

/// Hold a plan to the document and the bindings.  `faults` maps each bound operation to its
/// faults.  Pure.
pub fn validate_plan(
    plan: &[PlanEntry],
    faults: &BTreeMap<String, Vec<String>>,
    claims: &[String],
    examples: &[String],
) -> Vec<String> {
    let mut p = Vec::new();
    let bound: BTreeSet<&str> = faults.values().flatten().map(String::as_str).collect();
    for e in plan {
        let at = |m: String| format!("{PLAN_FILE}:{}: {m}", e.line);
        let (pool, what) = if e.tier == "exhaustive" {
            (claims, "claim")
        } else {
            (examples, "example")
        };
        if !pool.contains(&e.name) {
            p.push(at(format!(
                "{} is not a {what} of the document",
                esc(&e.name)
            )));
        }
        if !bound.contains(e.fault.as_str()) {
            p.push(at(format!(
                "fault {} is bound to no operation in evidence.bindings",
                esc(&e.fault)
            )));
        }
        if let Some(op) = &e.attack {
            if !faults
                .get(op)
                .is_some_and(|f| f.iter().any(|x| x == FIELD_FLIP))
            {
                p.push(at(format!(
                    "attack={} names no operation that lists `{FIELD_FLIP}`",
                    esc(op)
                )));
            }
        }
    }
    for f in &bound {
        if *f != FIELD_FLIP && !plan.iter().any(|e| e.fault == *f) {
            p.push(format!(
                "{PLAN_FILE}: no entry attacks the bound fault `{f}`: nothing proves the evidence can fail on it"
            ));
        }
    }
    for (op, fs) in faults {
        if fs.iter().any(|f| f == FIELD_FLIP)
            && !plan
                .iter()
                .any(|e| e.fault == FIELD_FLIP && e.attack.as_deref() == Some(op))
        {
            p.push(format!(
                "{PLAN_FILE}: no entry attacks `{FIELD_FLIP}` on operation `{op}` (`attack={op}`)"
            ));
        }
    }
    p
}

// ---------------------------------------------------------------------------------------
// The text
// ---------------------------------------------------------------------------------------

fn features_arg(g: &GenInputs) -> String {
    if g.features.is_empty() {
        String::new()
    } else {
        format!(" --features {}", g.features.join(","))
    }
}

/// The canonical region of `BUILD` (`windows == false`) or `BUILD_windows`, ending with
/// [`MARKER`] and a newline.
pub fn canonical_build(g: &GenInputs, windows: bool) -> String {
    let c = &g.krate;
    let feat = features_arg(g);
    let vl = "../../../programs/rust/vos-link/Cargo.toml";
    let mut s = String::new();
    let mut l = |line: String| {
        s.push_str(&line);
        s.push('\n');
    };
    l(format!("# GENERATED by `vos-link gen-build` (VOS04 5.6) from vos.toml, evidence.bindings, {PLAN_FILE} and code/specs/{c}.warrant."));
    l("# Edit those and run gen-build: `vos-link check-component` requires this region byte for byte.".to_string());
    l(
        "# Your own lines go after the marker at the end; they run but satisfy no requirement."
            .to_string(),
    );
    l("# needs-toolchain: python".to_string());
    l("# Lines 1 to 5 of VOS04 5.2: unit (with and without the evidence feature, then doc), shape, capability, document, harness drift.".to_string());
    l(format!(
        "cargo test -p {c} --lib --test behaviour -- --nocapture"
    ));
    if !g.features.is_empty() {
        l(format!("cargo test -p {c}{feat} --lib --test behaviour"));
    }
    l(format!("cargo test -p {c} --doc"));
    l("cargo run -q -p vos-shape-check -- .".to_string());
    l("cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .".to_string());
    l(format!("cargo run -q --manifest-path {vl} -- check-doc ."));
    l(format!(
        "cargo run -q -p coding_adventures_warrant_grammar --example gen_evidence -- --check {c}"
    ));
    l("# Lines 6 to 8: the generated suite (exhaustive, property, fuzz) rewrites the evidence manifest.".to_string());
    if !g.excused.is_empty() {
        let sh = if windows { "bash" } else { "sh" };
        l(format!("{sh} {TOLERANCE_SCRIPT}"));
    } else if windows {
        l(format!("set WARRANT_EVIDENCE_OUT=evidence/test-manifest.json&& cargo test -q -p {c}{feat} --test warrant_evidence"));
    } else {
        l(format!("WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {c}{feat} --test warrant_evidence"));
    }
    l("# Line 9: sabotage.  Each fault must make the suite of the claim it names report `not ok` (the shared checker, VOS-D099).".to_string());
    for e in &g.plan {
        let filter = format!("/{}/{}", e.tier, e.name);
        let tail = format!("../../../scripts/warrant_negctl.py -- cargo test -q -p {c}{feat} --test warrant_evidence");
        if windows {
            let mut x = format!("set WARRANT_EVIDENCE_FAULT={}&& ", e.fault);
            if let Some(a) = &e.attack {
                x.push_str(&format!("set WARRANT_EVIDENCE_ATTACK={a}&& "));
            }
            x.push_str(&format!(
                "set WARRANT_EVIDENCE_FILTER={filter}&& python {tail}"
            ));
            l(x);
        } else {
            let mut x = format!("WARRANT_EVIDENCE_FAULT={} ", e.fault);
            if let Some(a) = &e.attack {
                x.push_str(&format!("WARRANT_EVIDENCE_ATTACK={a} "));
            }
            x.push_str(&format!("WARRANT_EVIDENCE_FILTER={filter} python3 {tail}"));
            l(x);
        }
    }
    if !windows {
        l(format!("# VOS-D070: line coverage of src/ must stay at or above `coverage-floor` ({}) of vos.toml; Linux only (tarpaulin).", g.floor));
        l(format!(
            "if [ \"$(uname)\" = \"Linux\" ]; then cargo tarpaulin -p {c}{feat} --lib --test behaviour --ignore-tests --engine llvm --include-files '{c}/src/*' --fail-under {} --skip-clean; fi",
            g.floor
        ));
    }
    l(
        "# The component kit's check, last: it reads the manifest the run above rewrote."
            .to_string(),
    );
    l(format!(
        "cargo run -q --manifest-path {vl} -- check-component ."
    ));
    l(MARKER.to_string());
    s
}

/// The generated tolerance script, if the crate declares `excused-controls`.
pub fn tolerance_script(g: &GenInputs) -> Option<String> {
    if g.excused.is_empty() {
        return None;
    }
    let c = &g.krate;
    let feat = features_arg(g);
    let ids = g.excused.join("\n");
    Some(format!(
        "#!/bin/sh\n\
# GENERATED by `vos-link gen-build` (VOS04 5.6) from `excused-controls` in vos.toml.  Do not edit:\n\
# `vos-link check-component` requires this file byte for byte.\n\
#\n\
# Runs the generated suite and rewrites the manifest.  Exactly the controls listed below may fail\n\
# (VOS-D065, VOS-D098: a claim that cannot see the fault); anything else failing, or one of them\n\
# passing, or no test passing at all, is an error.\n\
set -eu\n\
\n\
known_ids='{ids}'\n\
\n\
status=0\n\
out=$(WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test -q -p {c}{feat} --test warrant_evidence 2>/dev/null) || status=$?\n\
\n\
ok=$(printf '%s\\n' \"$out\" | grep -c '^ok ' || true)\n\
failed=$(printf '%s\\n' \"$out\" | sed -n 's/^not ok [0-9][0-9]* - //p' | sort)\n\
expected=$(printf '%s\\n' \"$known_ids\" | sort)\n\
\n\
echo \"evidence: cargo status $status, $ok ok\"\n\
if [ \"$status\" -ne 1 ] || [ \"$ok\" -eq 0 ] || [ \"$failed\" != \"$expected\" ]; then\n\
    echo \"evidence: expected status 1, some ok and exactly these failures:\" >&2\n\
    printf '%s\\n' \"$expected\" >&2\n\
    echo \"evidence: got these failures:\" >&2\n\
    printf '%s\\n' \"$failed\" >&2\n\
    exit 1\n\
fi\n"
    ))
}

// ---------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------

/// Read everything generation needs.  `Ok(Err(findings))` is "the inputs are wrong".
pub fn load_inputs(
    layout: &crate::inputs::Layout,
    vt: &vos_shape_check::manifest::VosToml,
) -> Result<Result<GenInputs, Vec<Finding>>, NoVerdict> {
    let mut f: Vec<Finding> = Vec::new();
    let krate = layout.impl_pkg.clone();
    let Some(floor) = vt.coverage_floor else {
        f.push(Finding::new(
            "CC07",
            "coverage: vos.toml declares no `coverage-floor` (VOS04 5.6 item G, VOS-D070)",
        ));
        return Ok(Err(f));
    };
    if floor < crate::component::COVERAGE_FLOOR_MIN {
        f.push(Finding::new(
            "CC07",
            format!(
                "coverage: coverage-floor = {floor} is below the repository minimum {}",
                crate::component::COVERAGE_FLOOR_MIN
            ),
        ));
        return Ok(Err(f));
    }

    // evidence.bindings, by the evidence generator's own reader.
    let mut faults: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (mut property_cases, mut fuzz_iterations) = (0, 0);
    match read_text(
        &layout.impl_dir.join("evidence.bindings"),
        BINDINGS_MAX_BYTES,
    )? {
        None => f.push(Finding::new("CC04", "evidence.bindings: not found")),
        Some(t) => match parse_evidence_bindings(&t) {
            Ok(b) => {
                for (op, ob) in &b.manifest.bindings {
                    faults.insert(op.clone(), ob.negative_controls.clone());
                }
                property_cases = b.property_cases;
                fuzz_iterations = b.fuzz_iterations;
            }
            Err(e) => f.push(Finding::new(
                "CC04",
                format!("evidence.bindings: {}", esc(&e)),
            )),
        },
    }

    // The document's claims and examples.
    let doc_path = layout.specs_dir().join(layout.doc_name());
    let (mut claims, mut examples) = (Vec::new(), Vec::new());
    match read_text(&doc_path, DOC_MAX_BYTES)? {
        None => f.push(Finding::new(
            "CC04",
            format!(
                "{}: not found (run `vos-link gen-doc`)",
                layout.doc_display()
            ),
        )),
        Some(t) => {
            let h = std::thread::Builder::new()
                .stack_size(WARRANT_STACK_BYTES)
                .spawn(move || declared_names(&t))
                .map_err(|e| NoVerdict::new(format!("cannot start the Warrant thread: {e}")))?;
            match h
                .join()
                .map_err(|_| NoVerdict::new("the Warrant thread panicked"))?
            {
                Ok((c, e)) => {
                    claims = c;
                    examples = e;
                }
                Err(e) => f.push(Finding::new(
                    "CC04",
                    format!("{}: {}", layout.doc_display(), esc(&e.to_string())),
                )),
            }
        }
    }

    // The plan.
    let mut plan = Vec::new();
    match read_text(&layout.impl_dir.join(PLAN_FILE), PLAN_MAX_BYTES)? {
        None => f.push(Finding::new("CC04", format!("{PLAN_FILE}: not found"))),
        Some(t) => {
            let (entries, problems) = parse_plan(&t);
            f.extend(problems.into_iter().map(|m| Finding::new("CC04", m)));
            if f.is_empty() {
                for m in validate_plan(&entries, &faults, &claims, &examples) {
                    f.push(Finding::new("CC04", m));
                }
            }
            plan = entries;
        }
    }
    if !f.is_empty() {
        return Ok(Err(f));
    }
    Ok(Ok(GenInputs {
        krate,
        features: vt.evidence_features.clone(),
        floor,
        excused: vt.excused_controls.clone(),
        plan,
        claims,
        examples,
        property_cases,
        fuzz_iterations,
    }))
}

/// What follows the marker line of an existing BUILD file: the author's own lines.
fn trailer_of(existing: &str) -> &str {
    let needle = format!("{MARKER}\n");
    match existing.find(&needle) {
        Some(i) if i == 0 || existing.as_bytes()[i - 1] == b'\n' => &existing[i + needle.len()..],
        _ => "",
    }
}

/// `vos-link gen-build <crate-dir>`.
pub fn gen_build(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
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
    out.header = format!("crate {}", layout.impl_pkg);
    let g = match load_inputs(&layout, &vt)? {
        Ok(g) => g,
        Err(findings) => {
            out.findings = findings;
            return Ok(out);
        }
    };
    let mut wrote = Vec::new();
    for (name, windows) in [("BUILD", false), ("BUILD_windows", true)] {
        let existing = read_text(&layout.impl_dir.join(name), 256 * 1024)?.unwrap_or_default();
        let text = format!("{}{}", canonical_build(&g, windows), trailer_of(&existing));
        if text != existing {
            write_replacing(&layout.impl_dir, name, text.as_bytes())?;
            wrote.push(name.to_string());
        }
    }
    if let Some(script) = tolerance_script(&g) {
        let existing = read_text(&layout.impl_dir.join(TOLERANCE_SCRIPT), 64 * 1024)?;
        if existing.as_deref() != Some(script.as_str()) {
            write_replacing(
                &layout.impl_dir.join("warrant"),
                "check_evidence.sh",
                script.as_bytes(),
            )?;
            wrote.push(TOLERANCE_SCRIPT.to_string());
        }
    }
    out.action = Some(if wrote.is_empty() {
        "unchanged BUILD, BUILD_windows".to_string()
    } else {
        format!("wrote {}", wrote.join(", "))
    });
    Ok(out)
}
