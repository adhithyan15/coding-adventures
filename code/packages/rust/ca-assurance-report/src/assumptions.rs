//! # Assumption rules AC-1..AC-13 (VER13 section 6)
//!
//! A proof label says "*if* these assumptions hold, *then* the claim holds".
//! The label is only honest if the "these" is complete, every id means
//! something, and anything presented as no longer assumed really was
//! discharged by evidence.  This module does two things with the catalogue
//! loaded by [`crate::catalogue`]:
//!
//! 1. **derives** what a report must say: each claim's assumption list, the
//!    report-scope list, and each exemption's list ([`derive_claim_assumptions`],
//!    [`derive_report_assumptions`], [`derive_exemption_assumptions`]); the
//!    generator calls these, so it holds no assumption literal of its own;
//! 2. **checks** a report against the catalogue and the evidence manifests
//!    ([`check_assumptions`]), one rule at a time, each with a stable
//!    diagnostic name taken from the spec:
//!
//! | Rule  | Diagnostic                    | Fails when                                            |
//! |-------|-------------------------------|-------------------------------------------------------|
//! | AC-14 | `CatalogueDrift`              | the report's pin differs from the files on disk       |
//! | AC-1  | `UnknownAssumption`           | an id is not a catalogue id (aliases are not ids)     |
//! | AC-11 | `StaleAttestation`            | an `unverified::` alias whose entry is no longer attested |
//! | AC-2  | `EmptyAssumptions`            | a proved-family label lists no assumption             |
//! | AC-7  | `StackMismatch`               | discharge evidence is for another stack               |
//! | AC-8  | `FloorDischarged`             | a floor / declared entry is under `discharged`        |
//! | AC-6  | `PartialUmbrella`             | an umbrella discharged while a child is not           |
//! | AC-3  | `UnevidencedDischarge`        | a discharge without passing, matching evidence        |
//! | AC-9  | `PinMismatch`                 | per-run evidence without the same pinned build tuple  |
//! | AC-4  | `SilentlyDroppedAssumption`   | an applicable or reachable entry is not listed        |
//! | AC-5  | `LabelOutranksAssumptions`    | a `proved-to-L` label with assumptions above `L`      |
//! | AC-12 | `NonClaimReported`            | a declared non-claim shown as proved                  |
//! | AC-13 | `ExemptionAssumptionMismatch` | an exemption's list differs from its derivation       |
//! | AC-10 | `UnusedAssumption`            | (only with `--strict`) an entry no claim uses         |
//!
//! The rules run in that order, so a report broken in one way is reported
//! under the rule that names the breakage rather than a later consequence.
//!
//! ## The effective assumption set (rule AC-4)
//!
//! For a claim with label `ℓ`, claim kinds `K` and package `p`:
//!
//! ```text
//! seeds(claim) = { e : e names p (or *), ℓ matches e.applies_to.labels,
//!                      K ∩ e.applies_to.claims ≠ ∅ }
//!              ∪ { ids the claim lists }
//! E(claim)     = closure of seeds over depends_on, refines and refined_by,
//!                not expanding past an entry that is discharged
//! ```
//!
//! Every member of `E` must appear under `assumptions` or `discharged`.
//! Walking `refined_by` as well as `refines` means listing the umbrella
//! `compiler.preserves_timing` forces its seven refinements to be listed too:
//! the umbrella is only as strong as its weakest child.

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::catalogue::{
    Assumption, Catalogue, CatalogueSet, CATALOGUE_PATH, CATALOGUE_SCHEMA, RUNGS, STACK_LAYERS,
};
use crate::evidence::{label_rank, Evidence, MEMORY_SAFETY_CWES, UNVERIFIED_PREFIX};
use crate::matrix::WeaknessStatus;
use crate::report::{AssuranceReport, CatalogueRef, ClaimEvidence, DischargedAssumption};

/// The stack every claim of today's reports describes: the package as the
/// host `rustc` builds it.  R0 proofs are about WIR; they say nothing that is
/// specific to one target, so the stack is not named after an ISA.
pub const HOST_STACK: &str = "host-rustc";
/// The stack of the owned RV32I lane (VER13 section 4.3), reserved.
pub const OWNED_RV32I_STACK: &str = "owned-rv32i";
pub const STACKS: [&str; 2] = [HOST_STACK, OWNED_RV32I_STACK];
/// Ladder rung of a claim proved by WIR self-composition (VER13 section 4.1).
pub const BASELINE_RUNG: &str = "R0";
/// Claim kinds owned by the report as a whole rather than by one claim.
pub const REPORT_SCOPE_KINDS: [&str; 3] = ["weakness-row", "capability", "report"];
/// Labels that assert a proof and therefore must list assumptions (AC-2).
pub const PROVED_FAMILY: [&str; 4] = ["proved", "proved-per-run", "proved-to-isa", "proved-to-gate-level"];

/// One AC rule violation.  The variant is the spec's diagnostic name; the
/// message says what and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssumptionCheckError {
    CatalogueDrift(String),
    UnknownAssumption(String),
    EmptyAssumptions(String),
    UnevidencedDischarge(String),
    SilentlyDroppedAssumption(String),
    LabelOutranksAssumptions(String),
    PartialUmbrella(String),
    StackMismatch(String),
    FloorDischarged(String),
    PinMismatch(String),
    UnusedAssumption(String),
    StaleAttestation(String),
    NonClaimReported(String),
    ExemptionAssumptionMismatch(String),
}

impl AssumptionCheckError {
    /// `(rule, diagnostic)`, e.g. `("AC-4", "SilentlyDroppedAssumption")`.
    pub fn code(&self) -> (&'static str, &'static str) {
        use AssumptionCheckError::*;
        match self {
            CatalogueDrift(_) => ("AC-14", "CatalogueDrift"),
            UnknownAssumption(_) => ("AC-1", "UnknownAssumption"),
            EmptyAssumptions(_) => ("AC-2", "EmptyAssumptions"),
            UnevidencedDischarge(_) => ("AC-3", "UnevidencedDischarge"),
            SilentlyDroppedAssumption(_) => ("AC-4", "SilentlyDroppedAssumption"),
            LabelOutranksAssumptions(_) => ("AC-5", "LabelOutranksAssumptions"),
            PartialUmbrella(_) => ("AC-6", "PartialUmbrella"),
            StackMismatch(_) => ("AC-7", "StackMismatch"),
            FloorDischarged(_) => ("AC-8", "FloorDischarged"),
            PinMismatch(_) => ("AC-9", "PinMismatch"),
            UnusedAssumption(_) => ("AC-10", "UnusedAssumption"),
            StaleAttestation(_) => ("AC-11", "StaleAttestation"),
            NonClaimReported(_) => ("AC-12", "NonClaimReported"),
            ExemptionAssumptionMismatch(_) => ("AC-13", "ExemptionAssumptionMismatch"),
        }
    }

    fn message(&self) -> &str {
        use AssumptionCheckError::*;
        match self {
            CatalogueDrift(m) | UnknownAssumption(m) | EmptyAssumptions(m) | UnevidencedDischarge(m)
            | SilentlyDroppedAssumption(m) | LabelOutranksAssumptions(m) | PartialUmbrella(m)
            | StackMismatch(m) | FloorDischarged(m) | PinMismatch(m) | UnusedAssumption(m)
            | StaleAttestation(m) | NonClaimReported(m) | ExemptionAssumptionMismatch(m) => m,
        }
    }
}

impl fmt::Display for AssumptionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (rule, name) = self.code();
        write!(f, "{rule} {name}: {}", self.message())
    }
}

impl std::error::Error for AssumptionCheckError {}

/// What a passing check still has to say: the AC-10 warnings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssumptionSummary {
    /// Entries naming this package whose `applies_to` matches nothing in the
    /// report (VER00 section 7.5.2: flagged for removal).
    pub unused: Vec<String>,
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// Does a label kind of `applies_to.labels` cover the concrete `label`?
/// `bounded(k)` covers every ranked `bounded(N)`; the others are exact.
pub fn label_matches(kind: &str, label: &str) -> bool {
    if kind == "bounded(k)" {
        return label.starts_with("bounded(") && label_rank(label).is_some();
    }
    kind == label
}

/// Does `e` apply to a claim of `package` with `label` and claim kinds `kinds`?
/// `label` is `None` at report scope, where no label exists.
fn applies(e: &Assumption, package: &str, label: Option<&str>, kinds: &BTreeSet<&str>) -> bool {
    e.names_package(package)
        && label.is_none_or(|l| e.applies_to.labels.iter().any(|k| label_matches(k, l)))
        && e.applies_to.claims.iter().any(|c| kinds.contains(c.as_str()))
}

/// Is `e` discharged in the catalogue itself for `stack`?
fn catalogue_discharged(e: &Assumption, stack: &str) -> bool {
    e.status == "discharged" && e.discharged_by.iter().any(|d| d.stack == stack)
}

/// The claim kinds a Warrant operation's evidence speaks to: every claim is
/// functional (examples, properties, exhaustive suites); an operation with a
/// constant-time obligation adds `constant-time`, one with any `*-bound`
/// obligation adds `bound`.
pub fn op_claim_kinds(ev: &Evidence, op: &str) -> BTreeSet<&'static str> {
    let mut kinds = BTreeSet::from(["functional"]);
    for p in ev.proofs.iter().filter(|p| p.op == op) {
        if p.kind == "constant-time" {
            kinds.insert("constant-time");
        }
        if p.kind.ends_with("-bound") {
            kinds.insert("bound");
        }
    }
    kinds
}

/// The Warrant operation behind a report claim (`symbol` is the export).
fn op_of_symbol<'a>(ev: &'a Evidence, symbol: &str) -> Option<&'a str> {
    ev.proofs.iter().find(|p| p.function == symbol).map(|p| p.op.as_str())
}

/// Rank of a rung: `R0` (no discharge) < `R0+` < `R1` < ... < `R4`.
fn rung_rank(r: &str) -> Option<usize> {
    if r == BASELINE_RUNG {
        return Some(0);
    }
    RUNGS.iter().position(|x| *x == r).map(|i| i + 1)
}

/// For a `proved-to-L` label: the index of `L` in [`STACK_LAYERS`].  Every
/// undischarged non-floor assumption must sit at `L` or below.  `proved` and
/// `bounded(k)` are the WIR baseline and assert no discharge (`None`).
fn label_floor_layer(label: &str) -> Option<usize> {
    let idx = |l: &str| STACK_LAYERS.iter().position(|x| *x == l);
    match label {
        "proved-per-run" | "proved-to-isa" => idx("mc"),
        "proved-to-gate-level" => idx("gate"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Closure
// ---------------------------------------------------------------------------

/// Breadth-first closure over `depends_on`, `refines` and `refined_by`.
/// Returns `(required, discharged_reached)`: entries that must be listed as
/// assumptions, and discharged entries the walk reached (not expanded: once
/// an assumption is discharged its own dependencies are no longer relied on
/// through it).  Every seed must be a canonical id.
fn closure(
    cat: &Catalogue,
    seeds: impl IntoIterator<Item = String>,
    is_discharged: &dyn Fn(&Assumption) -> bool,
) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let mut required = BTreeSet::new();
    let mut discharged = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<String> = seeds.into_iter().collect();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let e = cat.get(&id).ok_or_else(|| format!("`{id}` is not a catalogue id"))?;
        if is_discharged(e) {
            discharged.insert(id);
            continue;
        }
        required.insert(id);
        for next in e.depends_on.iter().chain(e.refines.iter()).chain(e.refined_by.iter()) {
            queue.push_back(next.clone());
        }
    }
    Ok((required, discharged))
}

/// Resolve a legacy spelling (an id or an alias) to its canonical id.
fn canonical(cat: &Catalogue, s: &str) -> Result<String, String> {
    cat.resolve(s)
        .map(|e| e.id.clone())
        .ok_or_else(|| format!("`{s}` is neither a catalogue id nor an alias"))
}

// ---------------------------------------------------------------------------
// Derivation (used by the generator; the checker re-derives independently)
// ---------------------------------------------------------------------------

/// A claim's assumption list and discharged list, derived from the catalogue.
///
/// `legacy` are the strings the evidence layer has always emitted
/// (`compiler.preserves_timing`, `memory.unaliased`, `heap-bound:unchecked`,
/// ...); they are resolved through aliases and seeded, so nothing the old
/// report said can disappear in the migration.
pub fn derive_claim_assumptions(
    set: &CatalogueSet,
    package: &str,
    label: &str,
    kinds: &BTreeSet<&str>,
    legacy: &[String],
    stack: &str,
) -> Result<(Vec<String>, Vec<DischargedAssumption>), String> {
    let cat = &set.catalogue;
    let mut seeds: Vec<String> = cat
        .entries
        .iter()
        .filter(|e| applies(e, package, Some(label), kinds))
        .map(|e| e.id.clone())
        .collect();
    for l in legacy {
        seeds.push(canonical(cat, l)?);
    }
    let (required, reached) = closure(cat, seeds, &|e| catalogue_discharged(e, stack))?;
    let discharged = reached
        .iter()
        .map(|id| {
            let e = cat.get(id).expect("closure only yields catalogue ids");
            let rows: Vec<_> = e.discharged_by.iter().filter(|d| d.stack == stack).collect();
            let pinned = rows.iter().find(|d| !d.pinned.is_empty()).map(|d| d.pinned.clone()).unwrap_or_default();
            DischargedAssumption {
                id: id.clone(),
                rung: e.discharge.as_ref().map(|d| d.rung.clone()).unwrap_or_default(),
                stack: stack.to_string(),
                evidence: rows.iter().map(|d| d.evidence.clone()).collect(),
                scope: pinned.get("artefact_sha256").map(|a| format!("for build {a}")).unwrap_or_default(),
                pinned,
            }
        })
        .collect();
    Ok((required.into_iter().collect(), discharged))
}

/// `remaining_by_layer`: how many listed assumptions sit at each layer.
pub fn remaining_by_layer(cat: &Catalogue, ids: &[String]) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for id in ids {
        if let Some(e) = cat.get(id) {
            *out.entry(e.layer.clone()).or_insert(0) += 1;
        }
    }
    out
}

/// Every `unverified::` id a report cites, anywhere.
fn cited_unverified(report: &AssuranceReport) -> BTreeSet<String> {
    report
        .claims
        .iter()
        .flat_map(|c| c.evidence.iter())
        .chain(report.weaknesses.iter().flat_map(|w| w.evidence.iter()))
        .chain(report.exemptions.iter().flat_map(|x| x.evidence.iter()))
        .filter(|id| id.starts_with(UNVERIFIED_PREFIX))
        .cloned()
        .collect()
}

/// The report-scope list: entries about the weakness matrix, the capability
/// evidence and the verdict, plus every attestation a row cites through its
/// `unverified::` alias (a cited attestation is relied upon, whatever package
/// its entry names), closed over the catalogue edges.
pub fn derive_report_assumptions(
    set: &CatalogueSet,
    package: &str,
    unverified: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let cat = &set.catalogue;
    let kinds: BTreeSet<&str> = REPORT_SCOPE_KINDS.into_iter().collect();
    let mut seeds: Vec<String> =
        cat.entries.iter().filter(|e| applies(e, package, None, &kinds)).map(|e| e.id.clone()).collect();
    for u in unverified {
        seeds.push(cat.by_alias(u).map(|e| e.id.clone()).ok_or_else(|| format!("`{u}` is no catalogue alias"))?);
    }
    let (required, _) = closure(cat, seeds, &|e| catalogue_discharged(e, HOST_STACK))?;
    Ok(required.into_iter().collect())
}

/// Which catalogue ids an exemption id derives from (the VER13 section 3.9
/// table), as a predicate over entries.  `None` for an id with no rule.
/// A predicate over catalogue entries.
type Selector = Box<dyn Fn(&Assumption) -> bool>;

fn exemption_selector(id: &str) -> Option<Selector> {
    if id == "EXEMPT-CWE-208-GATE-LEVEL" {
        // "Everything below the label's layer": the WIR proof says nothing
        // about cc, mc, uarch, gate or phys; plus the WIR cost model, which is
        // only discharged by replacement lower down the ladder.
        let below_wir: [&str; 5] = ["cc", "mc", "uarch", "gate", "phys"];
        return Some(Box::new(move |e| below_wir.contains(&e.layer.as_str()) || e.id == "wir.cost.step-model"));
    }
    if id == "EXEMPT-CWE-400-NO-STEP-PROOF" {
        // No section 3.9 row: the exemption records an *absence* of proof
        // (not-live obligations), not a reliance on assumptions.
        return Some(Box::new(|_| false));
    }
    if id == crate::evidence::NOT_APPLICABLE_208_EXEMPTION {
        // The class does not apply (no secret operand, no constant-time
        // obligation), so nothing is assumed about timing: the empty list.
        return Some(Box::new(|_| false));
    }
    // The section 3.9 row names 416/415/476/190 (ct-compare today); the
    // matrix emits the same unsafe-exception record for 125/787 when the
    // bounds-checked proofs are unproven, so the row covers every
    // memory-safety class (implementation clarification, AS-001a part 2).
    let cwe: u32 = id.strip_prefix("EXEMPT-CWE-")?.strip_suffix("-UNSAFE-EXCEPTION")?.parse().ok()?;
    if MEMORY_SAFETY_CWES.contains(&cwe) {
        return Some(Box::new(|e| e.id == "src.rust.unsafe-region.sound" || e.id == "wir.lowering.volatile-by-name"));
    }
    None
}

/// An exemption's assumption list: the section 3.9 selector applied to what
/// the claims still assume (their listed, undischarged assumptions).
pub fn derive_exemption_assumptions(
    set: &CatalogueSet,
    exemption_id: &str,
    claims: &[ClaimEvidence],
) -> Result<Vec<String>, String> {
    let select = exemption_selector(exemption_id)
        .ok_or_else(|| format!("exemption `{exemption_id}` has no derivation rule in VER13 section 3.9"))?;
    let remaining: BTreeSet<&str> = claims
        .iter()
        .filter_map(|c| c.proof.as_ref())
        .flat_map(|p| p.assumptions.iter().map(String::as_str))
        .collect();
    let mut out = Vec::new();
    for id in remaining {
        let e = set.catalogue.get(id).ok_or_else(|| format!("`{id}` is not a catalogue id"))?;
        if select(e) {
            out.push(id.to_string());
        }
    }
    Ok(out)
}

/// The report's pin of `set` (VER13 section 5.3).
pub fn catalogue_ref(set: &CatalogueSet) -> CatalogueRef {
    CatalogueRef {
        schema: CATALOGUE_SCHEMA.into(),
        path: CATALOGUE_PATH.into(),
        sha256: set.catalogue_sha256.clone(),
        evidence_kinds_sha256: set.evidence_kinds_sha256.clone(),
        non_claims_sha256: set.non_claims_sha256.clone(),
    }
}

// ---------------------------------------------------------------------------
// The check
// ---------------------------------------------------------------------------

/// Check `report` against the catalogue in `ev` and the evidence manifests.
/// With `strict`, an unused assumption (AC-10) is an error, not a warning.
pub fn check_assumptions(
    report: &AssuranceReport,
    ev: &Evidence,
    strict: bool,
) -> Result<AssumptionSummary, AssumptionCheckError> {
    use AssumptionCheckError as A;
    let set = &ev.catalogue;
    let cat = &set.catalogue;
    let package = report.package.as_str();

    // ---- AC-14: the pin equals the files on disk -------------------------
    let want = catalogue_ref(set);
    if report.catalogue != want {
        return Err(A::CatalogueDrift(format!(
            "the report pins {:?} but the files on disk are {:?}; regenerate the report",
            report.catalogue, want
        )));
    }

    // ---- AC-1: every listed id is a canonical catalogue id, once ----------
    let proofs: Vec<(&ClaimEvidence, &crate::report::ProofEvidence)> =
        report.claims.iter().filter_map(|c| c.proof.as_ref().map(|p| (c, p))).collect();
    let mut lists: Vec<(String, Vec<&str>)> = vec![(
        "report assumptions".into(),
        report.assumptions.iter().map(String::as_str).collect(),
    )];
    for (c, p) in &proofs {
        lists.push((format!("claim {} assumptions", c.id), p.assumptions.iter().map(String::as_str).collect()));
        lists.push((format!("claim {} discharged", c.id), p.discharged.iter().map(|d| d.id.as_str()).collect()));
    }
    for x in &report.exemptions {
        lists.push((format!("exemption {} assumptions", x.id), x.assumptions.iter().map(String::as_str).collect()));
    }
    for (where_, ids) in &lists {
        let mut seen = BTreeSet::new();
        for id in ids {
            if cat.get(id).is_none() {
                let hint = match cat.by_alias(id) {
                    Some(e) => format!(" (it is an alias of `{}`: cite the id)", e.id),
                    None => String::new(),
                };
                return Err(A::UnknownAssumption(format!("{where_}: `{id}` is not a catalogue id{hint}")));
            }
            if !seen.insert(*id) {
                return Err(A::UnknownAssumption(format!("{where_}: `{id}` is listed twice")));
            }
        }
    }
    for (c, p) in &proofs {
        if let Some(d) = p.discharged.iter().find(|d| p.assumptions.contains(&d.id)) {
            return Err(A::UnknownAssumption(format!(
                "claim {}: `{}` is listed both as an assumption and as discharged",
                c.id, d.id
            )));
        }
    }

    // ---- AC-11: `unverified::` ids resolve to attestations still attested -
    let unverified = cited_unverified(report);
    for u in &unverified {
        match cat.by_alias(u) {
            None => return Err(A::UnknownAssumption(format!("`{u}` is no catalogue alias (AC-11 cannot judge it)"))),
            Some(e) if e.status != "attested" => {
                return Err(A::StaleAttestation(format!(
                    "`{u}` is an alias of `{}`, whose status is `{}`: cite the checked evidence instead",
                    e.id, e.status
                )))
            }
            Some(_) => {}
        }
    }

    // ---- AC-2: a proved-family label never has an empty list --------------
    for (c, p) in &proofs {
        let proved_family = PROVED_FAMILY.contains(&p.label.as_str()) || label_matches("bounded(k)", &p.label);
        if proved_family && p.assumptions.is_empty() {
            return Err(A::EmptyAssumptions(format!(
                "claim {} has label {} and no assumptions (even proved-to-gate-level lists the phys.* floor)",
                c.id, p.label
            )));
        }
    }

    // ---- AC-7: stacks are known and discharges match the claim's stack ----
    for (c, p) in &proofs {
        if !STACKS.contains(&p.stack.as_str()) {
            return Err(A::StackMismatch(format!("claim {}: unknown stack {:?} (one of {STACKS:?})", c.id, p.stack)));
        }
        if let Some(d) = p.discharged.iter().find(|d| d.stack != p.stack) {
            return Err(A::StackMismatch(format!(
                "claim {} is about stack {} but `{}` is discharged for stack {}",
                c.id, p.stack, d.id, d.stack
            )));
        }
    }

    // ---- AC-8: the floor is never discharged -----------------------------
    for (c, p) in &proofs {
        for d in &p.discharged {
            let e = cat.get(&d.id).expect("AC-1 resolved every id");
            if e.is_floor() || e.discharge.is_none() {
                return Err(A::FloorDischarged(format!(
                    "claim {}: `{}` (status {}, kind {}) cannot be discharged by software",
                    c.id, d.id, e.status, e.kind
                )));
            }
        }
    }

    // ---- AC-6: an umbrella is discharged only with every child ------------
    for (c, p) in &proofs {
        let here: BTreeSet<&str> = p.discharged.iter().map(|d| d.id.as_str()).collect();
        for d in &p.discharged {
            let e = cat.get(&d.id).expect("AC-1 resolved every id");
            for child in &e.refined_by {
                let child_e = cat.get(child).expect("catalogue edges resolve");
                if !here.contains(child.as_str()) && !catalogue_discharged(child_e, &p.stack) {
                    return Err(A::PartialUmbrella(format!(
                        "claim {}: umbrella `{}` is discharged but its refinement `{child}` is not (stack {})",
                        c.id, d.id, p.stack
                    )));
                }
            }
        }
    }

    // ---- AC-3 and AC-9: every discharge rests on matching, passing evidence
    for (c, p) in &proofs {
        for d in &p.discharged {
            let e = cat.get(&d.id).expect("AC-1 resolved every id");
            let where_ = format!("claim {}: discharged `{}`", c.id, d.id);
            let required = e.discharge.as_ref().map(|x| x.rung.as_str()).unwrap_or("R4");
            match (rung_rank(&d.rung), rung_rank(required)) {
                (Some(got), Some(need)) if got >= need && got > 0 => {}
                _ => {
                    return Err(A::UnevidencedDischarge(format!(
                        "{where_}: rung {:?} is below the catalogue's required rung {required}",
                        d.rung
                    )))
                }
            }
            check_discharge_evidence(ev, e, &d.evidence, None, &where_)?;
            check_pins(ev, &d.evidence, &d.pinned, Some(&d.scope), &where_)?;
        }
    }
    // Entries the catalogue itself marks discharged must be backed by this
    // package's manifests, digest for digest.
    for e in cat.entries.iter().filter(|e| e.status == "discharged" && e.names_package(package)) {
        for row in &e.discharged_by {
            let where_ = format!("catalogue entry `{}`", e.id);
            check_discharge_evidence(ev, e, std::slice::from_ref(&row.evidence), Some(&row.digest), &where_)?;
            check_pins(ev, std::slice::from_ref(&row.evidence), &row.pinned, None, &where_)?;
        }
    }

    // ---- AC-4: nothing applicable or reachable is silently dropped --------
    let mut used: BTreeSet<String> = BTreeSet::new();
    for (c, p) in &proofs {
        let op = op_of_symbol(ev, &c.symbol)
            .ok_or_else(|| A::SilentlyDroppedAssumption(format!("claim {}: no proof obligation names `{}`", c.id, c.symbol)))?;
        let kinds = op_claim_kinds(ev, op);
        let here: BTreeSet<&str> = p.discharged.iter().map(|d| d.id.as_str()).collect();
        let seeds = cat
            .entries
            .iter()
            .filter(|e| applies(e, package, Some(&p.label), &kinds))
            .map(|e| e.id.clone())
            .chain(p.assumptions.iter().cloned())
            .chain(p.discharged.iter().map(|d| d.id.clone()));
        let stack = p.stack.clone();
        let (required, reached) = closure(cat, seeds, &|e| here.contains(e.id.as_str()) || catalogue_discharged(e, &stack))
            .map_err(A::UnknownAssumption)?;
        let listed: BTreeSet<&str> = p.assumptions.iter().map(String::as_str).collect();
        let missing: Vec<&String> = required
            .iter()
            .filter(|id| !listed.contains(id.as_str()))
            .chain(reached.iter().filter(|id| !here.contains(id.as_str())))
            .collect();
        if !missing.is_empty() {
            return Err(A::SilentlyDroppedAssumption(format!(
                "claim {} (label {}, kinds {:?}) omits {} assumption(s): {}",
                c.id,
                p.label,
                kinds,
                missing.len(),
                missing.iter().take(8).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
        used.extend(required);
        used.extend(reached);
    }
    let report_required = derive_report_assumptions(set, package, &unverified).map_err(A::UnknownAssumption)?;
    let listed: BTreeSet<&str> = report.assumptions.iter().map(String::as_str).collect();
    let missing: Vec<&str> = report_required.iter().map(String::as_str).filter(|id| !listed.contains(id)).collect();
    if !missing.is_empty() {
        return Err(A::SilentlyDroppedAssumption(format!(
            "the report-scope list omits {} assumption(s): {}",
            missing.len(),
            missing.iter().take(8).copied().collect::<Vec<_>>().join(", ")
        )));
    }
    used.extend(report_required);

    // ---- AC-5: a label never outranks what it still assumes ---------------
    for (c, p) in &proofs {
        if rung_rank(&p.rung).is_none() {
            return Err(A::LabelOutranksAssumptions(format!("claim {}: unknown rung {:?}", c.id, p.rung)));
        }
        if p.remaining_by_layer != remaining_by_layer(cat, &p.assumptions) {
            return Err(A::LabelOutranksAssumptions(format!(
                "claim {}: remaining_by_layer {:?} does not count the listed assumptions",
                c.id, p.remaining_by_layer
            )));
        }
        let Some(floor) = label_floor_layer(&p.label) else { continue };
        for id in &p.assumptions {
            let e = cat.get(id).expect("AC-1 resolved every id");
            if e.is_floor() {
                continue;
            }
            if let Some(i) = e.stack_index() {
                if i < floor {
                    return Err(A::LabelOutranksAssumptions(format!(
                        "claim {}: label {} requires every assumption above `{}` discharged, but `{id}` ({}, {}) is not",
                        c.id, p.label, STACK_LAYERS[floor], e.layer, e.status
                    )));
                }
            }
        }
    }

    // ---- AC-12: a declared non-claim is never reported as proved ----------
    for nc in set.non_claims.iter().filter(|n| n.package == package) {
        let proved_ids: BTreeSet<&str> = ev
            .proofs
            .iter()
            .filter(|p| p.kind == nc.claim && p.verdict == "pass" && label_rank(&p.label).is_some())
            .map(|p| p.id.as_str())
            .collect();
        if let Some(id) = proved_ids.iter().next() {
            return Err(A::NonClaimReported(format!(
                "{package} declares CWE-{} `{}` a non-claim ({}), but proof `{id}` passes: retire the non-claim first",
                nc.cwe, nc.claim, nc.source
            )));
        }
        for w in report.weaknesses.iter().filter(|w| w.cwe == nc.cwe) {
            let cites_proof = w.evidence.iter().any(|e| ev.proofs.iter().any(|p| p.id == *e && p.kind == nc.claim));
            if matches!(w.status, WeaknessStatus::Discharged | WeaknessStatus::Excluded) || cites_proof {
                return Err(A::NonClaimReported(format!(
                    "{package} declares CWE-{} `{}` a non-claim ({}), but the row is {}{}",
                    nc.cwe,
                    nc.claim,
                    nc.source,
                    w.status.as_str(),
                    if cites_proof { " and cites a proof" } else { "" }
                )));
            }
        }
    }

    // ---- AC-13: exemptions list exactly what they derive from --------------
    for x in &report.exemptions {
        let want = derive_exemption_assumptions(set, &x.id, &report.claims).map_err(A::ExemptionAssumptionMismatch)?;
        let got: BTreeSet<&str> = x.assumptions.iter().map(String::as_str).collect();
        let want_set: BTreeSet<&str> = want.iter().map(String::as_str).collect();
        if got != want_set {
            let extra: Vec<&&str> = got.difference(&want_set).collect();
            let lacking: Vec<&&str> = want_set.difference(&got).collect();
            return Err(A::ExemptionAssumptionMismatch(format!(
                "{} lists {} id(s) it does not derive from {extra:?} and lacks {lacking:?}",
                x.id,
                extra.len()
            )));
        }
        used.extend(want);
    }

    // ---- AC-10: entries this package names that nothing uses -------------
    let unused: Vec<String> = cat
        .entries
        .iter()
        .filter(|e| e.names_package(package) && !used.contains(&e.id))
        .map(|e| e.id.clone())
        .collect();
    if strict && !unused.is_empty() {
        return Err(A::UnusedAssumption(format!(
            "{} catalogue entr(ies) name {package} but match no claim of the report: {}",
            unused.len(),
            unused.join(", ")
        )));
    }
    Ok(AssumptionSummary { unused })
}

/// AC-3 for one discharge: evidence ids exist, each matches exactly one
/// evidence kind the entry accepts, that kind's manifest holds the id with the
/// kind's verdict, and (for catalogue rows) the manifest digest matches.
fn check_discharge_evidence(
    ev: &Evidence,
    e: &Assumption,
    evidence: &[String],
    digest: Option<&str>,
    where_: &str,
) -> Result<(), AssumptionCheckError> {
    use AssumptionCheckError as A;
    if evidence.is_empty() {
        return Err(A::UnevidencedDischarge(format!("{where_}: no evidence ids")));
    }
    let accepted = e.discharge.as_ref().map(|d| d.evidence_kinds.as_slice()).unwrap_or(&[]);
    for id in evidence {
        let kind = match ev.catalogue.kinds.classify(id) {
            Ok(Some(k)) => k,
            Ok(None) => return Err(A::UnevidencedDischarge(format!("{where_}: `{id}` matches no evidence kind"))),
            Err(m) => return Err(A::UnevidencedDischarge(format!("{where_}: {m}"))),
        };
        if !accepted.contains(&kind.kind) {
            return Err(A::UnevidencedDischarge(format!(
                "{where_}: `{id}` is evidence of kind `{}`, which cannot discharge it (accepted: {accepted:?})",
                kind.kind
            )));
        }
        if kind.manifest == "planned" {
            return Err(A::UnevidencedDischarge(format!(
                "{where_}: `{id}` is evidence of kind `{}`, which no manifest produces yet (planned)",
                kind.kind
            )));
        }
        let Some((manifest, verdict)) = ev.manifest_entry(id) else {
            return Err(A::UnevidencedDischarge(format!("{where_}: `{id}` is absent from every manifest")));
        };
        if manifest != kind.manifest {
            return Err(A::UnevidencedDischarge(format!(
                "{where_}: `{id}` is in the {manifest} manifest, but kind `{}` lives in the {} manifest",
                kind.kind, kind.manifest
            )));
        }
        if verdict != kind.verdict {
            return Err(A::UnevidencedDischarge(format!("{where_}: `{id}` has verdict {verdict}, not {}", kind.verdict)));
        }
        if let Some(digest) = digest {
            let want = ev.manifest_sha256.get(manifest).map(|h| format!("sha256:{h}"));
            if want.as_deref() != Some(digest) {
                return Err(A::UnevidencedDischarge(format!(
                    "{where_}: digest {digest} is not the {manifest} manifest's digest {want:?}"
                )));
            }
        }
    }
    Ok(())
}

/// AC-9 for one discharge: every per-run evidence id needs the pinned tuple,
/// identical to the tuple the manifest entry recorded for the current build,
/// and (in a report) the scope text `for build <artefact_sha256>`.
fn check_pins(
    ev: &Evidence,
    evidence: &[String],
    pinned: &BTreeMap<String, String>,
    scope: Option<&str>,
    where_: &str,
) -> Result<(), AssumptionCheckError> {
    use AssumptionCheckError as A;
    for id in evidence {
        let per_run = matches!(ev.catalogue.kinds.classify(id), Ok(Some(k)) if k.per_run);
        if !per_run {
            continue;
        }
        if pinned.is_empty() {
            return Err(A::PinMismatch(format!("{where_}: per-run evidence `{id}` without a pinned build tuple")));
        }
        match ev.pinned.get(id) {
            None => {
                return Err(A::PinMismatch(format!(
                    "{where_}: the manifest entry `{id}` records no build tuple, so the discharge is void"
                )))
            }
            Some(current) if current != pinned => {
                return Err(A::PinMismatch(format!(
                    "{where_}: pinned {pinned:?} but the current build of `{id}` is {current:?}; regenerate the evidence"
                )))
            }
            Some(_) => {}
        }
        if let Some(scope) = scope {
            let want = format!("for build {}", pinned.get("artefact_sha256").map(String::as_str).unwrap_or(""));
            if scope != want {
                return Err(A::PinMismatch(format!("{where_}: scope must say {want:?}, not {scope:?}")));
            }
        }
    }
    Ok(())
}
