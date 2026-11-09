//! # Evidence manifests and report derivation (VER04h section 9)
//!
//! An assurance report may only quote numbers that a run wrote down.  This
//! module reads the manifests those runs produced
//!
//! * `evidence/test-manifest.json`  (generated Warrant test target),
//! * `evidence/proof-manifest.json` (Warrant-driven proof obligations),
//! * `evidence/bench-manifest.json` (microbenchmark target),
//!
//! plus the effect certificate and the Warrant document itself, and does two
//! things with them:
//!
//! 1. **derive** every claim number, label and certificate digest
//!    ([`derive_claims`]) and every `PackageAuditContext` flag
//!    ([`Evidence::audit_context`]) -- the generator contains no literal
//!    evidence;
//! 2. **cross-check** an existing report against them ([`verify_evidence`]),
//!    failing closed on unknown evidence ids, count/label drift, certificate
//!    files that do not match their hashes, Warrant claims without evidence,
//!    stale manifests, and a "no unsafe" claim next to a declared unsafe
//!    exception.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::codegen::{extract_claim_cases, find_operation_in_expr};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, ParserLimits, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use crate::json::JsonValue;
use crate::matrix::{AuditEvidence, AuditProfile, PackageAuditContext, WeaknessStatus};
use crate::report::{
    AssuranceReport, ClaimEvidence, ExhaustiveEvidence, FuzzEvidence, ProofEvidence,
    PropertyEvidence, RungsSummary,
};

/// Why evidence could not be loaded or derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceError(pub String);

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "evidence error: {}", self.0)
    }
}

impl std::error::Error for EvidenceError {}

fn err<T>(msg: impl Into<String>) -> Result<T, EvidenceError> {
    Err(EvidenceError(msg.into()))
}

// ---------------------------------------------------------------------------
// Manifest models
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct TestEntry {
    pub id: String,
    pub kind: String,
    pub op: String,
    pub claim: String,
    pub verdict: String,
    pub cases: u64,
    pub iterations: u64,
    pub admitted: u64,
    pub equal_cases: u64,
    pub caught_by: String,
}

#[derive(Clone, Debug)]
pub struct CertRef {
    pub id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Default)]
pub struct ProofEntry {
    pub id: String,
    pub op: String,
    pub function: String,
    pub kind: String,
    pub label: String,
    /// `pass`, `fail`, `unchecked` (resource not tracked) or `not-live` (the
    /// obligation was attempted and the tooling refused the source).
    pub verdict: String,
    pub max_len: u64,
    pub certificates: Vec<CertRef>,
    /// Free text the driver recorded (for `not-live`, the precise blocker).
    pub note: String,
}

impl ProofEntry {
    /// An obligation whose outcome is a verdict about the code, as opposed to
    /// a recorded inability to run it.
    fn is_live(&self) -> bool {
        self.verdict != "not-live" && self.verdict != "unchecked"
    }
}

/// Facts read from the Warrant document that the manifests must cover.
#[derive(Clone, Debug, Default)]
pub struct SpecFacts {
    pub module: String,
    pub sha256: String,
    /// (claim name, operation) in document order.
    pub claims: Vec<(String, String)>,
    pub examples: Vec<(String, String)>,
    /// Operations in document order with their evidence obligations.
    pub ops: Vec<OpFacts>,
}

#[derive(Clone, Debug, Default)]
pub struct OpFacts {
    pub name: String,
    pub has_secret: bool,
    pub has_steps_bound: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CertFacts {
    pub label: String,
    pub declared_effects: Vec<String>,
    /// Justifications of `banned_construct_exceptions` for `unsafe_code`.
    pub unsafe_exceptions: Vec<String>,
    pub other_exceptions: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Evidence {
    pub tests: Vec<TestEntry>,
    pub test_spec_sha256: String,
    pub proofs: Vec<ProofEntry>,
    pub proof_spec_sha256: String,
    pub proof_source_sha256: String,
    pub bench: Vec<(String, String)>,
    pub hw_instruction_counters: String,
    pub spec: SpecFacts,
    pub cert: CertFacts,
    pub source_sha256: String,
    pub proofs_dir: PathBuf,
}

fn read(path: &Path) -> Result<String, EvidenceError> {
    std::fs::read_to_string(path)
        .map_err(|e| EvidenceError(format!("cannot read {}: {e}", path.display())))
}

fn parse(path: &Path) -> Result<JsonValue, EvidenceError> {
    JsonValue::parse(&read(path)?)
        .map_err(|e| EvidenceError(format!("{} is not valid JSON: {e}", path.display())))
}

fn text(v: &JsonValue, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn num(v: &JsonValue, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(0).max(0) as u64
}

fn schema(v: &JsonValue, want: &str, path: &Path) -> Result<(), EvidenceError> {
    if text(v, "schema") == want {
        Ok(())
    } else {
        err(format!("{} must have schema {want:?}", path.display()))
    }
}

fn load_spec(path: &Path) -> Result<SpecFacts, EvidenceError> {
    let source = std::fs::read(path)
        .map_err(|e| EvidenceError(format!("cannot read {}: {e}", path.display())))?;
    let doc = SourceDocument::admit(&source, SourceLimits::default())
        .map_err(|e| EvidenceError(format!("spec admission: {e:?}")))?;
    let layout = capture_layout(&doc, LayoutLimits::default())
        .map_err(|e| EvidenceError(format!("spec layout: {e:?}")))?;
    let file: WarrantFile<'_> = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|e| EvidenceError(format!("spec parse: {e:?}")))?;
    let mut facts = SpecFacts {
        module: file.name.to_string(),
        sha256: sha256_hex(&source),
        ..SpecFacts::default()
    };
    let op_names: BTreeSet<&str> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Op(o) => Some(o.name),
            _ => None,
        })
        .collect();
    let names_set: std::collections::HashSet<&str> = op_names.iter().copied().collect();
    for c in extract_claim_cases(&file).map_err(|e| EvidenceError(format!("spec claims: {e}")))? {
        facts.claims.push((c.id.to_string(), c.op_name));
    }
    for d in &file.declarations {
        match d {
            Declaration::Example(ex) => {
                let op = find_operation_in_expr(&ex.body, &names_set).unwrap_or("");
                facts.examples.push((ex.name.to_string(), op.to_string()));
            }
            Declaration::Op(o) => {
                let has_secret = file.declarations.iter().any(|x| {
                    matches!(x, Declaration::Labels(l) if l.name == o.name && l.rows.iter().any(|r| r.is_secret))
                });
                let has_steps_bound = file.declarations.iter().any(|x| {
                    matches!(x, Declaration::Bound(b) if b.name == o.name && b.rows.iter().any(|r| r.name == "steps"))
                });
                facts.ops.push(OpFacts {
                    name: o.name.to_string(),
                    has_secret,
                    has_steps_bound,
                });
            }
            _ => {}
        }
    }
    Ok(facts)
}

fn load_cert(path: &Path) -> Result<CertFacts, EvidenceError> {
    let v = parse(path)?;
    let strings = |key: &str| -> Vec<String> {
        v.get(key)
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let mut facts = CertFacts {
        label: text(&v, "label"),
        declared_effects: strings("declared_effects"),
        ..CertFacts::default()
    };
    for ex in v
        .get("banned_construct_exceptions")
        .and_then(|x| x.as_array())
        .unwrap_or(&[])
    {
        let construct = text(ex, "construct");
        let why = text(ex, "justification");
        if construct == "unsafe_code" {
            facts.unsafe_exceptions.push(why);
        } else {
            facts.other_exceptions.push(construct);
        }
    }
    Ok(facts)
}

/// Locate the Warrant document named by a manifest by walking up from the
/// package directory (the manifest stores a repository-relative path).
fn find_spec(package_dir: &Path, relative: &str) -> Result<PathBuf, EvidenceError> {
    let start = package_dir
        .canonicalize()
        .map_err(|e| EvidenceError(format!("package dir {}: {e}", package_dir.display())))?;
    for dir in start.ancestors() {
        let candidate = dir.join(relative);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    err(format!("spec {relative} not found above {}", package_dir.display()))
}

impl Evidence {
    /// Load every manifest under `<package_dir>/evidence`.
    pub fn load(package_dir: &Path) -> Result<Evidence, EvidenceError> {
        let dir = package_dir.join("evidence");

        let tpath = dir.join("test-manifest.json");
        let tv = parse(&tpath)?;
        schema(&tv, "warrant-test-evidence-v1", &tpath)?;
        let tests = tv
            .get("tests")
            .and_then(|x| x.as_array())
            .ok_or_else(|| EvidenceError("test manifest has no tests".into()))?
            .iter()
            .map(|t| TestEntry {
                id: text(t, "id"),
                kind: text(t, "kind"),
                op: text(t, "op"),
                claim: text(t, "claim"),
                verdict: text(t, "verdict"),
                cases: num(t, "cases"),
                iterations: num(t, "iterations"),
                admitted: num(t, "admitted"),
                equal_cases: num(t, "equal_cases"),
                caught_by: text(t, "caught_by"),
            })
            .collect();
        let spec_rel = text(&tv, "spec");

        // The proof manifest is required: every package's Warrant document
        // declares `labels` and `bound` rows, so there is always at least one
        // obligation to account for, as a verdict or as a `not-live` entry.
        let ppath = dir.join("proof-manifest.json");
        let pv = parse(&ppath)?;
        schema(&pv, "warrant-proof-evidence-v1", &ppath)?;
        let proofs: Vec<ProofEntry> = pv
            .get("proofs")
            .and_then(|x| x.as_array())
            .ok_or_else(|| EvidenceError("proof manifest has no proofs".into()))?
            .iter()
            .map(|p| ProofEntry {
                id: text(p, "id"),
                op: text(p, "op"),
                function: text(p, "function"),
                kind: text(p, "kind"),
                label: text(p, "label"),
                verdict: text(p, "verdict"),
                max_len: num(p, "max_len"),
                note: text(p, "note"),
                certificates: p
                    .get("certificates")
                    .and_then(|x| x.as_array())
                    .unwrap_or(&[])
                    .iter()
                    .map(|c| CertRef {
                        id: text(c, "id"),
                        kind: text(c, "kind"),
                    })
                    .collect(),
            })
            .collect();

        // Benchmarks are optional the same way: no manifest, no benchmark
        // evidence (and the matrix then cannot cite any).
        let bpath = dir.join("bench-manifest.json");
        let bv = if bpath.is_file() {
            let bv = parse(&bpath)?;
            schema(&bv, "warrant-bench-evidence-v1", &bpath)?;
            Some(bv)
        } else {
            None
        };
        let bench: Vec<(String, String)> = bv
            .as_ref()
            .and_then(|b| b.get("tests"))
            .and_then(|x| x.as_array())
            .unwrap_or(&[])
            .iter()
            .map(|t| (text(t, "id"), text(t, "verdict")))
            .collect();

        let spec = load_spec(&find_spec(package_dir, &spec_rel)?)?;
        let cert = load_cert(&package_dir.join("effect_certificate.json"))?;
        let source = std::fs::read(package_dir.join("src/lib.rs"))
            .map_err(|e| EvidenceError(format!("cannot read src/lib.rs: {e}")))?;

        Ok(Evidence {
            tests,
            test_spec_sha256: text(&tv, "spec_sha256"),
            proofs,
            proof_spec_sha256: text(&pv, "spec_sha256"),
            proof_source_sha256: text(&pv, "source_sha256"),
            bench,
            hw_instruction_counters: bv
                .as_ref()
                .map(|b| text(b, "hw_instruction_counters"))
                .unwrap_or_else(|| "not-live".to_string()),
            spec,
            cert,
            source_sha256: sha256_hex(&source),
            proofs_dir: dir.join("proofs"),
        })
    }

    /// Every id a report may cite, with the verdict it carries.
    pub fn known_ids(&self) -> BTreeMap<String, String> {
        let mut ids = BTreeMap::new();
        for t in &self.tests {
            ids.insert(t.id.clone(), t.verdict.clone());
        }
        for p in &self.proofs {
            ids.insert(p.id.clone(), p.verdict.clone());
        }
        for (id, verdict) in &self.bench {
            ids.insert(id.clone(), verdict.clone());
        }
        for id in self.attestation_ids() {
            ids.insert(id, "pass".into());
        }
        ids
    }

    /// Attestations that follow from the effect certificate.  Each exists only
    /// while the certificate supports it, so a claim of "no unsafe" disappears
    /// the moment an `unsafe_code` exception is declared.
    pub fn attestation_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        let declared = &self.cert.declared_effects;
        let has = |prefix: &str| declared.iter().any(|d| d.starts_with(prefix));
        if self.cert.label == "proved" {
            ids.insert("effect-certificate::proved".to_string());
        }
        if self.cert.unsafe_exceptions.is_empty() {
            ids.insert("analyzer::no-unsafe-or-ffi".into());
        } else {
            ids.insert("effect-certificate::unsafe-exception".into());
        }
        for (prefix, id) in [
            ("proc:", "capability-audit::proc-empty"),
            ("fs:", "capability-audit::fs-empty"),
            ("ui:", "capability-audit::ui-empty"),
            ("stdout:", "capability-audit::stdout-empty"),
            ("rand:", "capability-audit::rand-empty"),
            ("thread:", "capability-audit::thread-empty"),
        ] {
            if !has(prefix) {
                ids.insert(id.into());
            }
        }
        if !has("fs:") && !has("proc:") {
            ids.insert("capability-audit::no-external-resources".into());
        }
        if declared.is_empty() {
            ids.insert("capability-audit::no-database-sinks".into());
        }
        if self.cert.other_exceptions.is_empty() {
            ids.insert("capability-audit::banned-constructs-clean".into());
        }
        ids
    }

    fn entries<'a>(&'a self, kind: &'a str, op: &'a str) -> impl Iterator<Item = &'a TestEntry> {
        self.tests.iter().filter(move |t| t.kind == kind && t.op == op)
    }

    fn all_pass_nonempty<'a>(entries: impl Iterator<Item = &'a TestEntry>) -> bool {
        let v: Vec<_> = entries.collect();
        !v.is_empty() && v.iter().all(|t| t.verdict == "pass")
    }

    fn proof_obligations<'a>(&'a self, op: &'a str) -> impl Iterator<Item = &'a ProofEntry> {
        self.proofs
            .iter()
            .filter(move |p| p.op == op && (p.kind == "constant-time" || p.kind == "steps-bound"))
    }

    /// Package facts for the weakness matrix, every one read from a manifest.
    pub fn audit_context(&self, package_id: &str) -> PackageAuditContext {
        self.audit_context_with_profile(package_id, AuditProfile::Comparison)
    }

    /// [`Evidence::audit_context`] for a package whose attestation rows are
    /// not about comparison primitives.
    pub fn audit_context_with_profile(
        &self,
        package_id: &str,
        profile: AuditProfile,
    ) -> PackageAuditContext {
        // Only obligations that produced a verdict about the code may be cited
        // as evidence; a `not-live` one is acknowledged separately.
        let ct: Vec<&ProofEntry> = self
            .proofs
            .iter()
            .filter(|p| p.kind == "constant-time" && p.is_live())
            .collect();
        let steps: Vec<&ProofEntry> = self
            .proofs
            .iter()
            .filter(|p| p.kind == "steps-bound" && p.is_live())
            .collect();
        let not_live = |kind: &str| -> Vec<&ProofEntry> {
            self.proofs
                .iter()
                .filter(|p| p.kind == kind && p.verdict == "not-live")
                .collect()
        };
        let (ct_nl, steps_nl) = (not_live("constant-time"), not_live("steps-bound"));
        let mut notes: Vec<&str> = ct_nl.iter().chain(steps_nl.iter()).map(|p| p.note.as_str()).collect();
        notes.sort_unstable();
        notes.dedup();
        let proof_ok = |v: &[&ProofEntry]| {
            !v.is_empty() && v.iter().all(|p| p.verdict == "pass" && label_rank(&p.label).is_some())
        };
        let bench_ct: Vec<&(String, String)> = self
            .bench
            .iter()
            .filter(|(id, _)| id.starts_with("bench/ct_"))
            .collect();
        let bench_ok = !bench_ct.is_empty() && bench_ct.iter().all(|(_, v)| v == "pass");
        let weakest = |v: &[&ProofEntry]| {
            v.iter()
                .map(|p| p.label.clone())
                .min_by_key(|l| label_rank(l).unwrap_or(0))
                .unwrap_or_default()
        };
        let evidence = AuditEvidence {
            unsafe_exceptions: self.cert.unsafe_exceptions.clone(),
            step_bound_ids: steps.iter().map(|p| p.id.clone()).collect(),
            step_bound_label: weakest(&steps),
            alloc_ids: bench_ct.iter().map(|(id, _)| id.clone()).collect(),
            ct_proof_ids: ct.iter().map(|p| p.id.clone()).collect(),
            ct_proof_label: weakest(&ct),
            bench_ids: bench_ct.iter().map(|(id, _)| id.clone()).collect(),
            hw_counters: self.hw_instruction_counters.clone(),
            oob_ids: ct
                .iter()
                .chain(steps.iter())
                .map(|p| p.id.clone())
                .collect(),
            oob_label: weakest(&ct),
            ct_not_live: ct_nl.iter().map(|p| p.id.clone()).collect(),
            step_not_live: steps_nl.iter().map(|p| p.id.clone()).collect(),
            not_live_note: notes.join(" | "),
            profile,
        };
        PackageAuditContext {
            package_id: package_id.to_string(),
            is_safe_rust: self.cert.unsafe_exceptions.is_empty(),
            declared_capabilities: self.cert.declared_effects.clone(),
            has_wir_timing_proof: proof_ok(&ct),
            has_instruction_benchmarks: bench_ok,
            has_zero_alloc_proof: bench_ok,
            has_pbt_coverage: Self::all_pass_nonempty(self.tests.iter().filter(|t| t.kind == "property")),
            has_fuzz_coverage: Self::all_pass_nonempty(self.tests.iter().filter(|t| t.kind == "fuzz")),
            evidence,
        }
    }

    /// Rung statuses computed from which kinds of evidence exist and pass.
    pub fn rungs(&self) -> RungsSummary {
        let status = |ok: bool| if ok { "pass" } else { "fail" }.to_string();
        let kind_ok =
            |k: &[&str]| Self::all_pass_nonempty(self.tests.iter().filter(|t| k.contains(&t.kind.as_str())));
        // E5 is `pass` only when every obligation passed, `fail` when any live
        // obligation did not, and `not-live` when the rest were recorded
        // not-live (or there are none): an acknowledged absence, never a pass.
        let e5 = {
            let v: Vec<&ProofEntry> = self
                .proofs
                .iter()
                .filter(|p| p.kind == "constant-time" || p.kind == "steps-bound")
                .collect();
            if v.iter().any(|p| p.is_live() && p.verdict != "pass") {
                "fail".to_string()
            } else if v.is_empty() || v.iter().any(|p| !p.is_live()) {
                "not-live".to_string()
            } else {
                "pass".to_string()
            }
        };
        RungsSummary {
            e0: status(self.cert.label == "proved" && self.cert.declared_effects.is_empty()),
            e1: status(kind_ok(&["example"])),
            // No manifest of simulated-host or integration runs exists.
            e2: "not-live".into(),
            e3: status(kind_ok(&["property", "exhaustive"])),
            e4: status(kind_ok(&["fuzz"])),
            e5,
        }
    }
}

/// Order of proof labels: `proved` is strongest, `bounded(k)` ranks by `k`.
/// Anything else (`unknown`, `falsified`, `unchecked`) has no rank.
pub fn label_rank(label: &str) -> Option<u64> {
    if label == "proved" {
        return Some(u64::MAX);
    }
    label
        .strip_prefix("bounded(")
        .and_then(|r| r.strip_suffix(')'))
        .and_then(|k| k.parse::<u64>().ok())
}

// ---------------------------------------------------------------------------
// Claim derivation
// ---------------------------------------------------------------------------

fn status(ok: bool) -> String {
    if ok { "pass" } else { "fail" }.to_string()
}

/// Digest of the certificates backing `op`: SHA-256 over the sorted, unique
/// certificate ids, newline terminated.  Each id is itself the SHA-256 of the
/// certificate's `cnf ++ 0x00 ++ drat` bytes, which `verify_evidence`
/// recomputes from the files on disk.
pub fn certificate_digest(ev: &Evidence, op: &str) -> Option<String> {
    let ids: BTreeSet<&str> = ev
        .proof_obligations(op)
        .flat_map(|p| p.certificates.iter().map(|c| c.id.as_str()))
        .collect();
    if ids.is_empty() {
        return None;
    }
    let mut joined = String::new();
    for id in ids {
        joined.push_str(id);
        joined.push('\n');
    }
    Some(format!("drat:sha256:{}", sha256_hex(joined.as_bytes())))
}

/// Derive one claim per Warrant operation from the manifests.
pub fn derive_claims(ev: &Evidence) -> Result<Vec<ClaimEvidence>, EvidenceError> {
    let mut claims = Vec::new();
    for op in &ev.spec.ops {
        let obligations: Vec<&ProofEntry> = ev.proof_obligations(&op.name).collect();
        let Some(first) = obligations.first() else {
            return err(format!("operation `{}` has no proof obligations in the manifest", op.name));
        };
        let symbol = first.function.clone();
        let sum = |kind: &str, f: fn(&TestEntry) -> u64| -> u64 {
            ev.entries(kind, &op.name).map(f).sum()
        };
        let weakest = obligations
            .iter()
            .map(|p| p.label.clone())
            .min_by_key(|l| label_rank(l).unwrap_or(0))
            .unwrap_or_default();
        let proofs_pass = obligations.iter().all(|p| p.verdict == "pass");
        let proof_label = if proofs_pass && label_rank(&weakest).is_some() {
            weakest
        } else if obligations.iter().any(|p| p.label == "falsified") {
            "falsified".to_string()
        } else {
            "unknown".to_string()
        };
        let buffers = obligations.iter().any(|p| p.max_len > 0 && p.label != "proved");
        let mut assumptions = vec!["compiler.preserves_timing".to_string()];
        if buffers {
            assumptions.push("memory.unaliased".to_string());
        }
        // Rows the prover does not discharge stay visible as assumptions.
        for p in ev.proofs.iter().filter(|p| p.op == op.name && p.verdict == "unchecked") {
            assumptions.push(format!("{}:unchecked", p.kind));
        }
        assumptions.sort();
        // Every obligation recorded `not-live` means no proof exists for this
        // operation: the claim carries no proof block at all (never a label),
        // and the manifest's note says why.
        let proof = if obligations.iter().all(|p| p.verdict == "not-live") {
            None
        } else {
            let certificate = certificate_digest(ev, &op.name)
                .ok_or_else(|| EvidenceError(format!("operation `{}` has no certificates", op.name)))?;
            Some(ProofEvidence {
                label: proof_label,
                assumptions,
                certificate,
            })
        };

        let negctl_ok = Evidence::all_pass_nonempty(ev.entries("negctl", &op.name));
        let mut evidence_ids: Vec<String> = ev
            .tests
            .iter()
            .filter(|t| t.op == op.name)
            .map(|t| t.id.clone())
            .chain(
                ev.proofs
                    .iter()
                    .filter(|p| p.op == op.name && p.is_live())
                    .map(|p| p.id.clone()),
            )
            .collect();
        evidence_ids.sort();
        claims.push(ClaimEvidence {
            id: format!("{symbol}_soundness"),
            symbol,
            example: status(Evidence::all_pass_nonempty(ev.entries("example", &op.name))),
            property: PropertyEvidence {
                cases: sum("property", |t| t.cases),
                status: status(
                    Evidence::all_pass_nonempty(ev.entries("property", &op.name)) && negctl_ok,
                ),
            },
            exhaustive: Some(ExhaustiveEvidence {
                cases: sum("exhaustive", |t| t.cases),
                status: status(Evidence::all_pass_nonempty(ev.entries("exhaustive", &op.name))),
            }),
            fuzz: Some(FuzzEvidence {
                iterations: sum("fuzz", |t| t.iterations),
                status: status(Evidence::all_pass_nonempty(ev.entries("fuzz", &op.name))),
            }),
            proof,
            evidence: evidence_ids,
        });
    }
    Ok(claims)
}

// ---------------------------------------------------------------------------
// Fail-closed verification
// ---------------------------------------------------------------------------

/// Failure modes of cross-checking a report against its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceCheckError {
    /// An evidence id is absent from every manifest.
    UnknownEvidence(String),
    /// A cited manifest entry did not pass.
    FailingEvidence(String),
    /// A count, label or digest disagrees with the manifests.
    EvidenceMismatch(String),
    /// A certificate file is missing, mis-hashed, or rejected by the DRAT checker.
    BadCertificate(String),
    /// The Warrant document declares something with no evidence.
    UncoveredClaim(String),
    /// A manifest was produced from a different spec or source than is on disk.
    StaleEvidence(String),
    /// The report claims no-unsafe while the certificate declares an exception.
    UnsafeContradiction(String),
    /// The manifests could not be read.
    Load(String),
}

impl fmt::Display for EvidenceCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (name, m) = match self {
            Self::UnknownEvidence(m) => ("unknown evidence id", m),
            Self::FailingEvidence(m) => ("cited evidence did not pass", m),
            Self::EvidenceMismatch(m) => ("report disagrees with evidence manifest", m),
            Self::BadCertificate(m) => ("bad certificate", m),
            Self::UncoveredClaim(m) => ("Warrant declaration without evidence", m),
            Self::StaleEvidence(m) => ("stale evidence", m),
            Self::UnsafeContradiction(m) => ("unsafe claim contradicts declared exception", m),
            Self::Load(m) => ("cannot load evidence", m),
        };
        write!(f, "{name}: {m}")
    }
}

impl std::error::Error for EvidenceCheckError {}

/// Memory-safety CWE classes that "safe Rust" would exclude.
pub const MEMORY_SAFETY_CWES: [u32; 6] = [416, 787, 125, 415, 476, 190];

/// Evidence ids that are not machine-checked attestations (VER04h section 10).
pub const UNVERIFIED_PREFIX: &str = "unverified::";

/// Cross-check `report` against `ev`.  On success returns the ids that were
/// accepted only as `unverified::` attestations, so callers can surface them.
pub fn verify_evidence(
    report: &AssuranceReport,
    ev: &Evidence,
) -> Result<Vec<String>, EvidenceCheckError> {
    use EvidenceCheckError as E;

    // 1. Staleness: manifests must describe the spec and source on disk.
    if ev.test_spec_sha256 != ev.spec.sha256 {
        return Err(E::StaleEvidence(format!(
            "test manifest was generated from spec {} but the spec on disk is {}",
            ev.test_spec_sha256, ev.spec.sha256
        )));
    }
    if ev.proof_spec_sha256 != ev.spec.sha256 {
        return Err(E::StaleEvidence("proof manifest was generated from a different spec".into()));
    }
    if ev.proof_source_sha256 != ev.source_sha256 {
        return Err(E::StaleEvidence(
            "proof manifest was generated from a different src/lib.rs".into(),
        ));
    }

    // 2. Certificates: file present, name == sha256(cnf ++ 0 ++ drat), and the
    //    DRAT checker still accepts it.
    let mut seen = BTreeSet::new();
    for p in &ev.proofs {
        for c in &p.certificates {
            if !seen.insert(c.id.clone()) {
                continue;
            }
            let cnf = std::fs::read(ev.proofs_dir.join(format!("{}.cnf", c.id)))
                .map_err(|e| E::BadCertificate(format!("{}: cnf unreadable: {e}", c.id)))?;
            let drat = std::fs::read(ev.proofs_dir.join(format!("{}.drat", c.id)))
                .map_err(|e| E::BadCertificate(format!("{}: drat unreadable: {e}", c.id)))?;
            let mut bytes = cnf.clone();
            bytes.push(0);
            bytes.extend_from_slice(&drat);
            if sha256_hex(&bytes) != c.id {
                return Err(E::BadCertificate(format!(
                    "{}: file contents hash to {}",
                    c.id,
                    sha256_hex(&bytes)
                )));
            }
            let (cnf_s, drat_s) = (String::from_utf8_lossy(&cnf), String::from_utf8_lossy(&drat));
            drat_check::check(&cnf_s, &drat_s)
                .map_err(|e| E::BadCertificate(format!("{}: DRAT checker rejected: {e}", c.id)))?;
        }
    }

    // 3. Coverage: everything the Warrant document declares has evidence.
    for (name, op) in &ev.spec.claims {
        for kind in ["exhaustive", "property", "fuzz"] {
            let id = format!("warrant/{}/{kind}/{name}", ev.spec.module);
            if !ev.tests.iter().any(|t| t.id == id) {
                return Err(E::UncoveredClaim(format!("claim `{name}` (op {op}) has no {kind} entry")));
            }
        }
    }
    for (name, _) in &ev.spec.examples {
        let id = format!("warrant/{}/example/{name}", ev.spec.module);
        if !ev.tests.iter().any(|t| t.id == id) {
            return Err(E::UncoveredClaim(format!("example `{name}` has no entry")));
        }
    }
    for op in &ev.spec.ops {
        let has = |kind: &str| ev.proofs.iter().any(|p| p.op == op.name && p.kind == kind);
        if op.has_secret && !has("constant-time") {
            return Err(E::UncoveredClaim(format!("op `{}` has secret labels but no constant-time proof", op.name)));
        }
        if op.has_steps_bound && !has("steps-bound") {
            return Err(E::UncoveredClaim(format!("op `{}` has a steps bound but no bound proof", op.name)));
        }
        if !report.claims.iter().any(|c| {
            ev.proofs.iter().any(|p| p.op == op.name && p.function == c.symbol)
        }) {
            return Err(E::UncoveredClaim(format!("op `{}` has no claim in the report", op.name)));
        }
    }

    // 3b. A declared unsafe exception forbids a "no unsafe" exclusion.
    if !ev.cert.unsafe_exceptions.is_empty() {
        for w in &report.weaknesses {
            let claims_no_unsafe = w.evidence.iter().any(|e| e == "analyzer::no-unsafe-or-ffi")
                || w.reason.contains("no unsafe blocks")
                || (w.status == WeaknessStatus::Excluded && MEMORY_SAFETY_CWES.contains(&w.cwe));
            if claims_no_unsafe {
                return Err(E::UnsafeContradiction(format!(
                    "CWE-{} is {} on a no-unsafe basis but the effect certificate declares an unsafe_code exception",
                    w.cwe,
                    w.status.as_str()
                )));
            }
        }
    }
    // 4. Every evidence id the report cites exists and passed.
    let known = ev.known_ids();
    let mut unverified = Vec::new();
    let mut check_ids = |ids: &[String]| -> Result<(), E> {
        for id in ids {
            if id.starts_with(UNVERIFIED_PREFIX) {
                unverified.push(id.clone());
                continue;
            }
            match known.get(id) {
                None => return Err(E::UnknownEvidence(id.clone())),
                Some(v) if v != "pass" => {
                    return Err(E::FailingEvidence(format!("{id} has verdict {v}")))
                }
                Some(_) => {}
            }
        }
        Ok(())
    };
    for c in &report.claims {
        check_ids(&c.evidence)?;
    }
    for w in &report.weaknesses {
        check_ids(&w.evidence)?;
    }
    for x in &report.exemptions {
        check_ids(&x.evidence)?;
    }

    // 5. Counts, labels and digests equal what the manifests say.
    let derived = derive_claims(ev).map_err(|e| E::Load(e.0))?;
    for want in &derived {
        let Some(got) = report.claims.iter().find(|c| c.id == want.id) else {
            return Err(E::UncoveredClaim(format!("report has no claim `{}`", want.id)));
        };
        let fields: Vec<(&str, String, String)> = vec![
            ("symbol", got.symbol.clone(), want.symbol.clone()),
            ("example status", got.example.clone(), want.example.clone()),
            ("property cases", got.property.cases.to_string(), want.property.cases.to_string()),
            ("property status", got.property.status.clone(), want.property.status.clone()),
            (
                "exhaustive",
                format!("{:?}", got.exhaustive),
                format!("{:?}", want.exhaustive),
            ),
            ("fuzz", format!("{:?}", got.fuzz), format!("{:?}", want.fuzz)),
            ("proof", format!("{:?}", got.proof), format!("{:?}", want.proof)),
            ("evidence ids", format!("{:?}", got.evidence), format!("{:?}", want.evidence)),
        ];
        for (what, g, w) in fields {
            if g != w {
                return Err(E::EvidenceMismatch(format!(
                    "claim {}: {what} is {g} but the manifest says {w}",
                    want.id
                )));
            }
        }
    }
    for c in &report.claims {
        if !derived.iter().any(|d| d.id == c.id) {
            return Err(E::UnknownEvidence(format!("claim `{}` is backed by no manifest entry", c.id)));
        }
    }
    let want_rungs = ev.rungs();
    if report.rungs != want_rungs {
        return Err(E::EvidenceMismatch(format!(
            "rungs are {:?} but the manifests say {:?}",
            report.rungs, want_rungs
        )));
    }

    Ok(unverified)
}
