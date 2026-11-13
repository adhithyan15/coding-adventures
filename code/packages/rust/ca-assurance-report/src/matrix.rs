//! # Weakness Matrix Construction & Closure Evaluation (VER15 / VER00 §8.2)
//!
//! Builds and validates the 24-row weakness matrix for a target package.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use crate::evidence::DecompressionBounds;
use crate::json::JsonValue;
use crate::registry::{WeaknessClass, WeaknessRegistry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaknessStatus {
    Excluded,
    Discharged,
    Exemption,
}

impl std::str::FromStr for WeaknessStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "excluded" => Ok(WeaknessStatus::Excluded),
            "discharged" => Ok(WeaknessStatus::Discharged),
            "exemption" => Ok(WeaknessStatus::Exemption),
            _ => Err(()),
        }
    }
}

impl WeaknessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WeaknessStatus::Excluded => "excluded",
            WeaknessStatus::Discharged => "discharged",
            WeaknessStatus::Exemption => "exemption",
        }
    }
}

/// A single row in a package's weakness matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeaknessRow {
    pub cwe: u32,
    pub name: String,
    pub status: WeaknessStatus,
    pub approved: Option<bool>,
    pub reason: String,
    pub evidence: Vec<String>,
}

impl WeaknessRow {
    pub fn to_json_value(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("cwe".into(), JsonValue::Number(self.cwe as f64));
        map.insert("name".into(), JsonValue::String(self.name.clone()));
        map.insert(
            "status".into(),
            JsonValue::String(self.status.as_str().into()),
        );
        if let Some(app) = self.approved {
            map.insert("approved".into(), JsonValue::Bool(app));
        }
        map.insert("reason".into(), JsonValue::String(self.reason.clone()));
        if !self.evidence.is_empty() {
            let ev_arr = self
                .evidence
                .iter()
                .map(|e| JsonValue::String(e.clone()))
                .collect();
            map.insert("evidence".into(), JsonValue::Array(ev_arr));
        }
        JsonValue::Object(map)
    }

    pub fn from_json_value(val: &JsonValue) -> Result<Self, String> {
        let obj = val
            .as_object()
            .ok_or_else(|| "weakness row must be a JSON object".to_string())?;

        let cwe = obj
            .get("cwe")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| "weakness row missing 'cwe'".to_string())?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let status_str = obj
            .get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "weakness row missing 'status'".to_string())?;

        let status: WeaknessStatus = status_str
            .parse()
            .map_err(|_| format!("unknown weakness status '{status_str}'"))?;

        let approved = obj.get("approved").and_then(|v| v.as_bool());

        let reason = obj
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let evidence = obj
            .get("evidence")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(Self {
            cwe,
            name,
            status,
            approved,
            reason,
            evidence,
        })
    }
}

/// Package facts needed to compute the weakness matrix.
/// Evidence ids and labels the matrix cites, copied from the manifests.
/// Nothing here has a meaningful default: an empty list for an obligation the
/// matrix needs makes the row `OpenRow`.
#[derive(Clone, Debug, Default)]
pub struct AuditEvidence {
    /// Justifications of declared `unsafe_code` exceptions (empty = none).
    pub unsafe_exceptions: Vec<String>,
    pub step_bound_ids: Vec<String>,
    pub step_bound_label: String,
    pub alloc_ids: Vec<String>,
    pub ct_proof_ids: Vec<String>,
    pub ct_proof_label: String,
    pub bench_ids: Vec<String>,
    /// `live` or `not-live`: were retired-instruction counters available?
    pub hw_counters: String,
    /// Proofs whose symbolic execution bounds-checks every access.
    pub oob_ids: Vec<String>,
    pub oob_label: String,
    /// Ids of constant-time obligations the manifest records `not-live`
    /// (attempted, refused by the tooling): an *acknowledged* absence of
    /// proof, as opposed to an absent manifest, which stays an open row.
    pub ct_not_live: Vec<String>,
    /// Ids of every constant-time / steps-bound obligation that is not a
    /// ranked pass (`not-live`, `unchecked`, failed, unranked).  Any entry
    /// here blocks the discharge of the rows that rest on that proof kind.
    pub ct_unproven: Vec<String>,
    pub step_unproven: Vec<String>,
    /// Same for step-bound obligations.
    pub step_not_live: Vec<String>,
    /// The blocker text the manifest recorded for the not-live obligations.
    pub not_live_note: String,
    /// Evidence ids attesting that the Warrant document labels every operand
    /// of every operation `public` (today the single derived attestation
    /// [`crate::evidence::NO_SECRET_OPERANDS_ATTESTATION`]).  Empty -- the
    /// default -- means "not attested", never "no secrets": with no
    /// constant-time evidence either, the CWE-208 row stays open.
    pub no_secret_operand_ids: Vec<String>,
    /// What kind of package the three attestation rows (CWE-326/327/407)
    /// describe; see [`AuditProfile`].
    pub profile: AuditProfile,
    /// The CWE-409 (decompression bomb) facts: the operations with an `output`
    /// bound row and the generated evidence that supports or fails to support
    /// them.  Empty -- the default -- means "the document bounds no output",
    /// which excludes CWE-409 only for a profile that does not decompress.
    pub decompression: DecompressionBounds,
}

/// What the package does, for the rows whose rationale is a static statement
/// about the package rather than a derived fact.  Those rows already carry the
/// `unverified::` evidence prefix; this only selects which true sentence they
/// carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuditProfile {
    /// Constant-time comparison / selection primitives (rust/ct-compare).
    #[default]
    Comparison,
    /// Table-driven finite-field arithmetic (rust/gf256).
    FieldArithmetic,
    /// A variable-length integer codec (rust/wasm-leb128): bytes in, integers
    /// out and back, no key material, no cryptography.
    VarintCodec,
    /// Pure Boolean logic (rust/logic-gates): logic gates and combinational
    /// blocks over single-bit values.  No bytes-as-keys, no cryptography.
    BooleanLogic,
    /// A lossless compressor and decompressor (rust/lzw): bytes in, bytes out
    /// and back, an output that can be far larger than the input, no key
    /// material, no cryptography.  Unlike every other profile this one
    /// *decompresses hostile input*, so CWE-409 is never excluded for it: the
    /// row is evidenced by the document's `output`/`heap` bound rows or the
    /// report cannot be generated (see [`DecompressionBounds`]).
    LosslessCompression,
}

impl AuditProfile {
    /// Does a package with this profile expand attacker-controlled input?
    /// CWE-409 is then applicable by definition, whatever the document says.
    pub fn decompresses(self) -> bool {
        matches!(self, AuditProfile::LosslessCompression)
    }

    /// The name `--profile` accepts and a report records (`FromStr`'s inverse).
    /// It is the on-disk spelling, so it must never change for an existing variant.
    pub fn as_str(self) -> &'static str {
        match self {
            AuditProfile::Comparison => "comparison",
            AuditProfile::FieldArithmetic => "field-arithmetic",
            AuditProfile::VarintCodec => "varint-codec",
            AuditProfile::BooleanLogic => "boolean-logic",
            AuditProfile::LosslessCompression => "lossless-compression",
        }
    }
}

impl std::fmt::Display for AuditProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AuditProfile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "comparison" => Ok(AuditProfile::Comparison),
            "field-arithmetic" => Ok(AuditProfile::FieldArithmetic),
            "varint-codec" => Ok(AuditProfile::VarintCodec),
            "boolean-logic" => Ok(AuditProfile::BooleanLogic),
            "lossless-compression" => Ok(AuditProfile::LosslessCompression),
            other => Err(format!(
                "unknown profile `{other}` (comparison | field-arithmetic | varint-codec | boolean-logic | lossless-compression)"
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackageAuditContext {
    pub package_id: String,
    pub is_safe_rust: bool,
    pub declared_capabilities: Vec<String>,
    pub has_wir_timing_proof: bool,
    pub has_instruction_benchmarks: bool,
    pub has_zero_alloc_proof: bool,
    pub has_pbt_coverage: bool,
    pub has_fuzz_coverage: bool,
    pub evidence: AuditEvidence,
}

#[derive(Debug)]
pub enum MatrixError {
    MissingClass(u32),
    /// The evidence manifests needed to build the report are missing or incomplete.
    MissingEvidence(String),
    InvalidExclusion(String),
    OpenRow(String),
}

impl fmt::Display for MatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatrixError::MissingEvidence(msg) => write!(f, "Matrix error: missing evidence: {msg}"),
            MatrixError::MissingClass(cwe) => write!(f, "Matrix error: CWE-{cwe} missing from matrix"),
            MatrixError::InvalidExclusion(msg) => write!(f, "Invalid exclusion: {msg}"),
            MatrixError::OpenRow(msg) => write!(f, "Matrix error: open row left unaddressed: {msg}"),
        }
    }
}

impl std::error::Error for MatrixError {}

/// Construct the weakness matrix for a package against the canonical registry.
pub fn build_weakness_matrix(
    registry: &WeaknessRegistry,
    ctx: &PackageAuditContext,
) -> Result<Vec<WeaknessRow>, MatrixError> {
    let mut rows = Vec::with_capacity(registry.classes.len());

    for class in &registry.classes {
        let mut row = evaluate_row(class, ctx)?;
        // Rows resting on a static claim that no tool in the repository checks
        // say so, rather than borrowing the weight of a verified row.
        if row.evidence.iter().any(|e| e.starts_with("unverified::")) {
            row.reason.push_str(" [attestation not machine-checked by any tool]");
        }
        rows.push(row);
    }

    Ok(rows)
}

/// CWE-409, decompression bomb: an attacker-chosen, well-formed compressed input
/// that expands to a huge output.
///
/// The row is **never excluded for a package that decompresses**, and the
/// matrix cannot be talked out of that: the exclusion needs *both* a profile
/// that does not decompress *and* a Warrant document that declares no `output`
/// bound row.  Either fact alone applies the class.
///
/// ```text
///  profile does not decompress  AND  no `output` bound row   ->  excluded
///  (anything else)  AND  every `output` op has a `heap` row, passing
///                        exhaustive + property + fuzz `<op>/output-bound`
///                        suites, and no observed peak over its bound   ->  discharged (bounded)
///  (anything else)                                                      ->  open row
/// ```
///
/// "Open row" is the generator's fail-closed outcome: no report is produced
/// until the evidence exists.  A discharge is *bounded* (VER15: `bounded(k)`):
/// the bound is checked on the enumerated and sampled inputs and on the
/// declared bomb examples, not proved for every input, and the row says so.
fn cwe_409(class: &WeaknessClass, ctx: &PackageAuditContext) -> Result<WeaknessRow, MatrixError> {
    let d = &ctx.evidence.decompression;
    let applies = ctx.evidence.profile.decompresses() || !d.ops.is_empty();
    if !applies {
        return Ok(WeaknessRow {
            cwe: 409,
            name: class.name.clone(),
            status: WeaknessStatus::Excluded,
            approved: None,
            reason: "pure computation: package implements no decompression or archive decoding routines".into(),
            evidence: vec!["unverified::ast-audit::no-compression-routines".into()],
        });
    }
    if d.ops.is_empty() {
        return Err(MatrixError::OpenRow(
            "CWE-409 applies (the package decompresses) but its Warrant document declares no `output` bound row".into(),
        ));
    }
    if !d.unproven.is_empty() {
        return Err(MatrixError::OpenRow(format!(
            "CWE-409 applies and its output/heap bound evidence is incomplete: {}",
            d.unproven.join("; ")
        )));
    }
    Ok(WeaknessRow {
        cwe: 409,
        name: class.name.clone(),
        status: WeaknessStatus::Discharged,
        approved: None,
        reason: format!(
            "discharged for bounded inputs: the Warrant document bounds the size of what {} returns (`output` row) and the live heap of the call (`heap` row); {} generated cases and iterations of the exhaustive, property and fuzz `output-bound` suites passed, and every observed instance ran under the allocation observer with a peak of at most {} bytes (the bound at that instance: {}). The bound is checked on the enumerated and sampled inputs and the document's bomb examples, not proved for every input",
            d.ops.join(", "),
            d.cases,
            d.max_observed_heap,
            d.bound_at_max
        ),
        evidence: d.ids.clone(),
    })
}

fn evaluate_row(
    class: &WeaknessClass,
    ctx: &PackageAuditContext,
) -> Result<WeaknessRow, MatrixError> {
    match class.cwe {
        // Memory safety: excluded by construction in safe Rust, but only when
        // the effect certificate declares no `unsafe_code` exception.  With an
        // exception the exclusion is false, so the row is derived from evidence:
        // out-of-bounds read/write are discharged by bounds-checked symbolic
        // execution of the lowered code (bounded), the rest stay unapproved
        // exemptions until a tool (for example Miri) produces evidence.
        416 | 787 | 125 | 415 | 476 | 190 => {
            if ctx.is_safe_rust {
                Ok(WeaknessRow {
                    cwe: class.cwe,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "safe Rust: static analyzer proved no unsafe blocks and no FFI reachability; ownership and bounds checks prevent memory corruption".into(),
                    evidence: vec!["analyzer::no-unsafe-or-ffi".into(), "effect-certificate::proved".into()],
                })
            } else if matches!(class.cwe, 125 | 787) && !ctx.evidence.oob_ids.is_empty() {
                Ok(WeaknessRow {
                    cwe: class.cwe,
                    name: class.name.clone(),
                    status: WeaknessStatus::Discharged,
                    approved: None,
                    reason: format!(
                        "unsafe exception declared ({}): every memory access of the lowered code is bounds-checked by symbolic execution for all length shapes covered by {}; not a proof for larger buffers and not a check of the real unsafe block under Miri",
                        ctx.evidence.unsafe_exceptions.join("; "),
                        ctx.evidence.oob_label
                    ),
                    evidence: {
                        let mut ids = ctx.evidence.oob_ids.clone();
                        ids.push("effect-certificate::unsafe-exception".into());
                        ids
                    },
                })
            } else {
                Ok(WeaknessRow {
                    cwe: class.cwe,
                    name: class.name.clone(),
                    status: WeaknessStatus::Exemption,
                    approved: Some(false),
                    reason: format!(
                        "computed exemption (unapproved): unsafe exception declared ({}); no evidence for this class beyond inspection (Miri not run)",
                        ctx.evidence.unsafe_exceptions.join("; ")
                    ),
                    evidence: vec!["effect-certificate::unsafe-exception".into()],
                })
            }
        }

        // Injection: excluded when ambient capabilities are absent
        78 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("proc:")) {
                Ok(WeaknessRow {
                    cwe: 78,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package declares no proc capability; analyzer verified zero process or shell execution".into(),
                    evidence: vec!["capability-audit::proc-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-78 proc capability present without injection proof".into()))
            }
        }
        89 => {
            Ok(WeaknessRow {
                cwe: 89,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "pure computation: package contains no database interaction or SQL query construction".into(),
                evidence: vec!["capability-audit::no-database-sinks".into()],
            })
        }
        22 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("fs:")) {
                Ok(WeaknessRow {
                    cwe: 22,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package declares no fs capability; analyzer verified zero filesystem operations".into(),
                    evidence: vec!["capability-audit::fs-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-22 fs capability present without traversal validation".into()))
            }
        }
        79 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("ui:")) {
                Ok(WeaknessRow {
                    cwe: 79,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package declares no ui capability and renders no HTML or DOM structures".into(),
                    evidence: vec!["capability-audit::ui-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-79 ui capability present without escaping proof".into()))
            }
        }
        94 => {
            Ok(WeaknessRow {
                cwe: 94,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "compiled Rust: language has no dynamic code evaluation or runtime eval primitives; analyzer verified no dynamic loading".into(),
                evidence: vec!["capability-audit::banned-constructs-clean".into()],
            })
        }
        116 => {
            Ok(WeaknessRow {
                cwe: 116,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "pure computation: package performs no serialized protocol formatting with unescaped control delimiters".into(),
                evidence: vec!["unverified::ast-audit::no-protocol-sinks".into()],
            })
        }
        117 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("stdout:")) {
                Ok(WeaknessRow {
                    cwe: 117,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package declares no stdout or logging capabilities; analyzer verified zero log sinks".into(),
                    evidence: vec!["capability-audit::stdout-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-117 stdout capability present without CRLF neutralization".into()))
            }
        }

        // Parsing / DoS
        400 => {
            if ctx.has_zero_alloc_proof
                && !ctx.evidence.step_bound_ids.is_empty()
                && ctx.evidence.step_unproven.is_empty()
            {
                let mut ids = ctx.evidence.alloc_ids.clone();
                ids.extend(ctx.evidence.step_bound_ids.iter().cloned());
                Ok(WeaknessRow {
                    cwe: 400,
                    name: class.name.clone(),
                    status: WeaknessStatus::Discharged,
                    approved: None,
                    reason: format!(
                        "discharged for bounded inputs: zero-allocation microbenchmark budgets passed; the Warrant steps bound (steps <= c*n + d) holds for every length shape covered by {} (bounded, not for all lengths)",
                        ctx.evidence.step_bound_label
                    ),
                    evidence: ids,
                })
            } else if !ctx.evidence.step_not_live.is_empty() {
                // The manifest says, in its own words, that the step-bound
                // obligation was attempted and could not run.  That is a
                // recorded, unapproved exemption, not an open row.
                Ok(WeaknessRow {
                    cwe: 400,
                    name: class.name.clone(),
                    status: WeaknessStatus::Exemption,
                    approved: Some(false),
                    reason: format!(
                        "computed exemption (unapproved): no step-bound proof is live, so the Warrant steps bounds are declarations that no tool checks ({})",
                        ctx.evidence.not_live_note
                    ),
                    evidence: vec![],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-400 resource exhaustion evidence missing".into()))
            }
        }
        409 => cwe_409(class, ctx),
        674 => {
            Ok(WeaknessRow {
                cwe: 674,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "iterative algorithms: package contains zero recursive function calls; call graph is strictly acyclic".into(),
                evidence: vec!["unverified::ast-call-graph::acyclic".into()],
            })
        }
        1333 => {
            Ok(WeaknessRow {
                cwe: 1333,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "zero regular expressions: package contains no regular expression engines or pattern matching dependencies".into(),
                evidence: vec!["unverified::analyzer::name-allowlist-regex-free".into()],
            })
        }
        407 => {
            Ok(WeaknessRow {
                cwe: 407,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: match ctx.evidence.profile {
                    AuditProfile::Comparison => "linear complexity: algorithms operate in strict linear time O(n) independent of input values",
                    AuditProfile::FieldArithmetic => "constant work per call: every operation is a bounded number of table lookups and arithmetic steps after a one-time 255-iteration table build; no input-dependent iteration",
                    AuditProfile::VarintCodec => "bounded work per call: a LEB128 encoder emits at most ceil(64/7) = 10 bytes and a decoder reads at most ceil(max_bits/7) bytes from its offset (10 for the documented widths, max_bits <= 64), one constant-time step per byte; no other input-dependent iteration",
                    AuditProfile::LosslessCompression => "bounded work per call: the encoder performs one dictionary lookup per input byte and the decoder writes each delivered byte once, through a dictionary of at most 65,536 fixed-size entries, delivering at most the documented output cap; no other input-dependent iteration",
                    AuditProfile::BooleanLogic => "bounded work per call: a gate is a fixed number of CMOS evaluations, an n-input gate or multiplexer is linear in its inputs, and a decoder or demultiplexer produces 2^n lines for at most 16 inputs (a width cap enforced by a panic); no other input-dependent iteration",
                }
                .into(),
                evidence: vec!["unverified::warrant-ir::cost-semantics-linear".into()],
            })
        }

        // Cryptography: CWE-208 observable timing discrepancy
        208 => {
            if ctx.has_wir_timing_proof
                && ctx.has_instruction_benchmarks
                && ctx.evidence.ct_unproven.is_empty()
            {
                // Computed exemption: proof floor is proved-to-gate-level (CPU netlist).
                // Backed by the WIR self-composition proofs and the benchmark
                // manifest, but gate-level proof is deferred to the Busicom
                // track, and release approval is blocked by VER00 §12.1.
                let mut ids = ctx.evidence.ct_proof_ids.clone();
                ids.extend(ctx.evidence.bench_ids.iter().cloned());
                Ok(WeaknessRow {
                    cwe: 208,
                    name: class.name.clone(),
                    status: WeaknessStatus::Exemption,
                    approved: Some(false),
                    reason: format!(
                        "computed exemption (unapproved): floor requires proof to gate-level CPU netlist; backed by WIR-level 2-safety proofs at label {} and microbenchmark runs (no retired-instruction evidence is claimed: hardware counters are host-dependent and recorded in evidence/bench-manifest.json)",
                        ctx.evidence.ct_proof_label
                    ),
                    evidence: ids,
                })
            } else if !ctx.evidence.ct_not_live.is_empty() {
                // Applicable and unaddressed: the package is labelled secret,
                // no constant-time proof exists, and the manifest says why.
                Ok(WeaknessRow {
                    cwe: 208,
                    name: class.name.clone(),
                    status: WeaknessStatus::Exemption,
                    approved: Some(false),
                    reason: format!(
                        "computed exemption (unapproved): applicable and unaddressed; operands are labelled secret but no constant-time proof is live and no benchmark evidence exists ({})",
                        ctx.evidence.not_live_note
                    ),
                    evidence: vec![],
                })
            } else if !ctx.evidence.no_secret_operand_ids.is_empty()
                && ctx.evidence.ct_proof_ids.is_empty()
                && ctx.evidence.ct_unproven.is_empty()
                && ctx.evidence.ct_not_live.is_empty()
            {
                // Not applicable: the document labels every operand public, so
                // it states no constant-time obligation, and the proof manifest
                // holds none.  This is a separate branch with its own
                // precondition (an attestation the checker re-derives from the
                // spec on disk), not a relaxation of the two above: any
                // constant-time entry at all, proven or not, keeps the old
                // outcome.  The registry gives CWE-208 no safe subset, so the
                // row cannot be `excluded` (VER15 section 4, rule 2); it is an
                // unapproved exemption whose reason says why it does not apply.
                Ok(WeaknessRow {
                    cwe: 208,
                    name: class.name.clone(),
                    status: WeaknessStatus::Exemption,
                    approved: Some(false),
                    reason: "not applicable: no secret operands; the Warrant document labels every operand of every operation public, so it states no constant-time obligation and none was run. Recorded as an unapproved exemption rather than an exclusion because the registry defines no safe subset for CWE-208, and because the labels are the specification's own classification, which no analyzer here confirms".into(),
                    evidence: ctx.evidence.no_secret_operand_ids.clone(),
                })
            } else {
                Err(MatrixError::OpenRow("CWE-208 timing discrepancy lacks WIR or benchmark evidence".into()))
            }
        }
        330 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("rand:")) {
                Ok(WeaknessRow {
                    cwe: 330,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package performs purely deterministic computation and declares no rand capability".into(),
                    evidence: vec!["capability-audit::rand-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-330 rand capability present without CSPRNG entropy audit".into()))
            }
        }
        327 => {
            Ok(WeaknessRow {
                cwe: 327,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: match ctx.evidence.profile {
                    AuditProfile::Comparison => "primitive comparison: package implements constant-time comparison primitives rather than legacy ciphers",
                    AuditProfile::FieldArithmetic => "field arithmetic: package implements GF(2^8) arithmetic only, no cipher, hash or MAC",
                    AuditProfile::VarintCodec => "integer encoding: package implements LEB128 variable-length integer encoding only, no cipher, hash or MAC",
                    AuditProfile::BooleanLogic => "boolean logic: package implements logic gates and combinational blocks over single-bit values only, no cipher, hash or MAC",
                    AuditProfile::LosslessCompression => "lossless compression: package implements LZW compression and decompression only, no cipher, hash or MAC",
                }
                .into(),
                evidence: vec!["unverified::ast-audit::no-legacy-ciphers".into()],
            })
        }
        326 => {
            Ok(WeaknessRow {
                cwe: 326,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: match ctx.evidence.profile {
                    AuditProfile::Comparison => "arbitrary buffer lengths: comparison functions accept arbitrary slice lengths without fixed sub-strength keys",
                    AuditProfile::FieldArithmetic => "no keys: the package takes and generates no key material; operands are field elements, not key bytes of a fixed strength",
                    AuditProfile::VarintCodec => "no keys: the package takes and generates no key material; operands are integers and LEB128 byte strings, not key bytes of a fixed strength",
                    AuditProfile::BooleanLogic => "no keys: the package takes and generates no key material; operands are single bits and short bit vectors, not key bytes of a fixed strength",
                    AuditProfile::LosslessCompression => "no keys: the package takes and generates no key material; operands are byte strings to compress and compressed streams, not key bytes of a fixed strength",
                }
                .into(),
                evidence: vec!["unverified::type-signatures::unconstrained-slices".into()],
            })
        }

        // Concurrency
        362 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("thread:")) {
                Ok(WeaknessRow {
                    cwe: 362,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package declares no thread capability and uses zero multithreading or shared mutable state".into(),
                    evidence: vec!["capability-audit::thread-empty".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-362 thread capability present without race freedom proof".into()))
            }
        }
        367 => {
            if !ctx.declared_capabilities.iter().any(|c| c.starts_with("fs:") || c.starts_with("proc:")) {
                Ok(WeaknessRow {
                    cwe: 367,
                    name: class.name.clone(),
                    status: WeaknessStatus::Excluded,
                    approved: None,
                    reason: "pure computation: package interacts with no external filesystem or process state".into(),
                    evidence: vec!["capability-audit::no-external-resources".into()],
                })
            } else {
                Err(MatrixError::OpenRow("CWE-367 external resource capability present without atomic operations".into()))
            }
        }

        // Deserialization
        502 => {
            Ok(WeaknessRow {
                cwe: 502,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "pure computation: package contains no dynamic object deserialization, polymorphic factories, or reflection".into(),
                evidence: vec!["unverified::ast-audit::no-deserialization".into()],
            })
        }
        1321 => {
            Ok(WeaknessRow {
                cwe: 1321,
                name: class.name.clone(),
                status: WeaknessStatus::Excluded,
                approved: None,
                reason: "strongly typed language: Rust has no object prototypes or dynamic prototype inheritance chains".into(),
                evidence: vec!["unverified::language-type-system::no-prototype-pollution".into()],
            })
        }

        other => Err(MatrixError::MissingClass(other)),
    }
}
