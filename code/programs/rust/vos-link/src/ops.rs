//! # `gen-doc` and `check-doc`
//!
//! Both start the same way: load and judge the inputs ([`crate::inputs::load`]), then
//! assemble the document those inputs describe ([`vos_manifest::document::assemble`]).
//!
//! * **`gen-doc`** judges the document it just assembled (every check but regeneration)
//!   and writes it only if there is no finding.  A generator that refuses to write what
//!   `check-doc` would reject means a bad document never reaches `code/specs`.
//! * **`check-doc`** judges the document **on disk**, then compares it byte for byte with
//!   a fresh assembly (`CD08`), so a hand edit that happens to keep every other check
//!   green still fails.  Reading is allowed to fail with a *finding* when the file is
//!   simply absent (a document nobody generated is drift), and with *no verdict* when the
//!   path is a symlink or the file is over its cap.

use std::path::Path;

use vos_manifest::caps::DOCUMENT_MAX_BYTES;
use vos_manifest::document::{assemble, AssembleInput};
use vos_manifest::Tier;

use crate::check::{first_difference_line, verify};
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, read_regular_opt, write_replacing};
use crate::inputs::{load, Load, Loaded};

/// The result of a run that reached a verdict.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The rule failures; empty means pass.
    pub findings: Vec<Finding>,
    /// The tiers the document carries (computed from its parsed claims).
    pub carries: Vec<Tier>,
    /// One line for the report header: crate, kind, contract.
    pub header: String,
    /// What `gen-doc` did: `Some("wrote ...")`, `Some("unchanged ...")`.
    pub action: Option<String>,
    /// Statements about what a pass does NOT establish, printed with the result.
    pub notes: Vec<String>,
}

/// What a PASS means for a crate that carries the `representation` tier (VOS02 2.7, 10.1): the
/// cone rule is not applied, and what is checked is the document's digests and its type check.
pub const REP_NOTE: &str = "representation tier: the document carries the contract's representation claims (init in the image of `rep`, every step for every thread id closed over it, safety over all of it) by claim_sha256 and type-checks against the implementation's `rep` and `hist` (CD04); the R6 cone rule is not applied (the state has history by design) and no claim of the implementation's is linted. Whether the claims HOLD is the evidence's; kind separation is not this tier's (a `-any` crate could truthfully carry it): it stays with the declared kind, the `any` block, SH13, SH16, the central kind registry (VOS-D030) and the bindings check (VOS-D060). kind, contract and obligations are author-declared until the lock (VOS-034).";

/// What a PASS of the document-level R6 rules does and does not mean (VOS02 10.1).
pub const R6_LINT_NOTE: &str = "R6 cone rule: a STRUCTURAL LINT at document level (no Impl field of `build` is a constant or an ignored argument); not proof that the state has no oracle. Not checked here: that `alpha` reads every field, that evidence.bindings binds no oracle write (VOS-D060, a prerequisite of any image profile), the central kind registry (VOS-D030) and the profile manifest (VOS-034). kind, contract and obligations are author-declared until then.";

fn notes_for(l: &Loaded, carries: &[Tier]) -> Vec<String> {
    use crate::inputs::ImplKind;
    match l.kind {
        // The cone note says no field of `build` is a constant: not applied to a
        // representation carrier, which gets its own note instead.
        ImplKind::Production if carries.contains(&Tier::Representation) => {
            vec![REP_NOTE.to_string()]
        }
        ImplKind::Production | ImplKind::Any => vec![R6_LINT_NOTE.to_string()],
        ImplKind::Reference => Vec::new(),
    }
}

/// The report header.  A crate whose document carries the `representation` tier prints
/// `mode representation` after its kind; the mode is DERIVED from `carries`, never declared, and a
/// canonical crate prints no mode (as before VOS-D061).
fn header(l: &Loaded, carries: &[Tier]) -> String {
    let mode = if carries.contains(&Tier::Representation) {
        ", mode representation"
    } else {
        ""
    };
    format!(
        "crate {}, kind {}{mode}, contract {}, obligations [{}]",
        l.layout.impl_pkg,
        l.kind.name(),
        l.contract_pkg,
        l.obligations
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Assemble the document the inputs describe.
fn expected(l: &Loaded) -> Result<String, NoVerdict> {
    let obligations: Vec<(Tier, &vos_manifest::Part)> = l
        .obligations
        .iter()
        .filter_map(|t| {
            l.tier_parts
                .iter()
                .find(|(x, _)| x == t)
                .map(|(x, p)| (*x, p))
        })
        .collect();
    assemble(&AssembleInput {
        impl_package: &l.layout.impl_pkg,
        contract_package: &l.contract_pkg,
        contract: &l.contract_part,
        implementation: &l.impl_part,
        obligations: &obligations,
    })
    .map_err(|e| NoVerdict::new(format!("cannot assemble the document: {e}")))
}

/// `vos-link gen-doc <crate-dir>`.
pub fn gen_doc(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let loaded = match load(crate_arg, root_arg)? {
        Load::Ready(l) => *l,
        Load::Findings(findings) => {
            return Ok(Outcome {
                findings,
                ..Outcome::default()
            })
        }
    };
    let mut out = Outcome {
        header: header(&loaded, &[]),
        notes: notes_for(&loaded, &[]),
        ..Outcome::default()
    };
    let doc = expected(&loaded)?;
    let verified = verify(&loaded, doc.as_bytes())?;
    out.carries = verified.carries;
    out.findings = verified.findings;
    out.header = header(&loaded, &out.carries);
    out.notes = notes_for(&loaded, &out.carries);
    if !out.findings.is_empty() {
        return Ok(out);
    }
    let specs = ensure_plain_dirs(&loaded.layout.root, &["code", "specs"])?;
    let name = loaded.layout.doc_name();
    let current = read_regular_opt(&specs.join(&name), DOCUMENT_MAX_BYTES)?;
    out.action = Some(if current.as_deref() == Some(doc.as_bytes()) {
        format!(
            "unchanged {} ({} bytes)",
            loaded.layout.doc_display(),
            doc.len()
        )
    } else {
        write_replacing(&specs, &name, doc.as_bytes())?;
        format!(
            "wrote {} ({} bytes)",
            loaded.layout.doc_display(),
            doc.len()
        )
    });
    Ok(out)
}

/// `vos-link check-doc <crate-dir>`.
pub fn check_doc(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let loaded = match load(crate_arg, root_arg)? {
        Load::Ready(l) => *l,
        Load::Findings(findings) => {
            return Ok(Outcome {
                findings,
                ..Outcome::default()
            })
        }
    };
    let mut out = Outcome {
        header: header(&loaded, &[]),
        notes: notes_for(&loaded, &[]),
        ..Outcome::default()
    };
    let fresh = expected(&loaded)?;
    let specs = ensure_plain_dirs(&loaded.layout.root, &["code", "specs"])?;
    let path = specs.join(loaded.layout.doc_name());
    let Some(disk) = read_regular_opt(&path, DOCUMENT_MAX_BYTES)? else {
        out.findings.push(Finding::new(
            "CD01",
            format!(
                "{} does not exist: run `vos-link gen-doc` (a document nobody generated is drift)",
                loaded.layout.doc_display()
            ),
        ));
        return Ok(out);
    };
    let verified = verify(&loaded, &disk)?;
    out.carries = verified.carries;
    out.findings = verified.findings;
    out.header = header(&loaded, &out.carries);
    out.notes = notes_for(&loaded, &out.carries);
    if disk != fresh.as_bytes() {
        let at = first_difference_line(&String::from_utf8_lossy(&disk), &fresh);
        out.findings.push(Finding::new(
            "CD08",
            format!(
                "{} differs from what gen-doc would write now (first difference at line {at}); run `vos-link gen-doc` and review the diff",
                loaded.layout.doc_display()
            ),
        ));
    }
    Ok(out)
}
