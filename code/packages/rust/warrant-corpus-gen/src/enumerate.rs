//! # Enumerating a whole document (VER04r 5, 10.3), and the parity view
//!
//! [`enumerate_document`] is the S3b pipeline end to end:
//!
//! ```text
//!   WarrantFile ──analyse──▶ Lowered + costs            (S3a: lowering, level refusals, ir_cost)
//!        │
//!        ├─ DomainEnv + resolve_binders (S2) ──▶ domains per claim, 16.2 refusals
//!        ├─ IR work bound (4.2 rule 6) and the total ordinal bound for ALL suites, BEFORE any planning
//!        ├─ SuitePlan::new  ──▶ admission, stream prefix, limits         (one suite at a time)
//!        └─ SuitePlan::run  ──▶ SuiteResult: chunks, counts, 18 digests (ct-compare), slim
//!                               then the v0 divergence guard, rule 2 (`wraps > 0` in a v0 document)
//! ```
//!
//! It does **not** emit anything: `corpus.json`, the chunk files and `cases.tfv` are slice
//! S3c, and fault IR, controls and witness chunks with them.
//!
//! ## The parity view
//!
//! The independent oracle's `emit` writes its own `warrant-oracle-enumeration-v0` tree, not a
//! v2 `corpus.json` (VER04r 16.3, "Findings recorded").  [`enumeration_view`] renders the
//! same facts from this crate's result in the same canonical JSON, so
//! `tests/fixtures/dump_oracle_enumeration.py` and `tests/enumeration_parity.rs` can compare
//! them byte for byte: per suite the counts, digests, descriptors, slim set and every chunk
//! line.

use coding_adventures_warrant_grammar::domain::DomainEnv;
use coding_adventures_warrant_grammar::parser::WarrantFile;

use crate::arith::check_v0_wraps;
use crate::cost::{admit_check_cost, Depth, IrWork};
use crate::domains::resolve_claim_in;
use crate::error::{code, GenError};
use crate::eval::Program;
use crate::json::Json;
use crate::profile::{Profile, SlimStride};
use crate::suite::{SuiteOptions, SuitePlan, SuiteResult, MAX_TOTAL_WORK_FULL};
use crate::{analyse, Analysis};

/// A whole document enumerated at `full` depth.
#[derive(Debug, Clone)]
pub struct Enumeration {
    /// What S3a computed: the lowered IR and its costs.
    pub analysis: Analysis,
    /// One result per claim, in declaration order.
    pub suites: Vec<SuiteResult>,
}

impl Enumeration {
    /// Admitted cases over all suites.
    pub fn cases(&self) -> u64 {
        self.suites.iter().map(|s| s.cases).sum()
    }

    /// Discarded tuples over all suites.
    pub fn discarded(&self) -> u64 {
        self.suites.iter().map(|s| s.discarded).sum()
    }

    /// Subject calls over all suites.
    pub fn expected_calls(&self) -> u64 {
        self.suites.iter().map(|s| s.expected_calls).sum()
    }

    /// The suite of claim `claim`.
    pub fn suite(&self, claim: &str) -> Option<&SuiteResult> {
        self.suites.iter().find(|s| s.claim == claim)
    }
}

/// Lower, cost, resolve and enumerate every claim of `file` at `full` depth.
///
/// # Errors
///
/// Any [`analyse`] refusal; the domain refusals of [`crate::domains`]; `limit` when the total
/// IR work or case count of a `full` run is past its bound (checked before the first case);
/// `W0706` for a suite that admits nothing; `W0701` for a v0 document whose arithmetic
/// wraps (VER04r 3.3 rule 2) or a value outside its type; `profile` for a `slim_chunk` line
/// naming an unknown claim or a missing chunk, or a stride that is not smaller than the
/// automatic one.
pub fn enumerate_document(
    file: &WarrantFile<'_>,
    profile: &Profile,
) -> Result<Enumeration, GenError> {
    let analysis = analyse(file, profile)?;
    enumerate_analysed(file, profile, analysis, &mut |_| Ok(()))
}

/// What the observer of [`enumerate_analysed`] sees for each suite, while its plan is alive.
pub struct SuiteView<'a> {
    /// The document's models.
    pub program: &'a Program<'a>,
    /// The whole lowered document.
    pub lowered: &'a crate::lower::Lowered,
    /// The suite's position in declaration order.
    pub index: usize,
    /// The lowered suite.
    pub suite: &'a crate::lower::LoweredSuite,
    /// The plan (admission, enumeration).
    pub plan: &'a SuitePlan<'a>,
    /// The suite's result (without witness chunks).
    pub result: &'a SuiteResult,
}

/// [`enumerate_document`] over an [`Analysis`] the caller already holds, calling `observer`
/// once per suite after it ran and before its plan is dropped (S3c's witness search).
///
/// # Errors
///
/// As [`enumerate_document`], and whatever `observer` returns.
pub fn enumerate_analysed(
    file: &WarrantFile<'_>,
    profile: &Profile,
    analysis: Analysis,
    observer: &mut dyn FnMut(&SuiteView<'_>) -> Result<(), GenError>,
) -> Result<Enumeration, GenError> {
    let suites = {
        let lowered = &analysis.lowered;
        for (claim, _) in &profile.slim_chunks {
            if !lowered.suites.iter().any(|s| s.claim == *claim) {
                return Err(GenError::new(
                    code::PROFILE,
                    "corpus.profile",
                    "a slim_chunk line names a claim the document does not declare",
                ));
            }
        }
        let program = Program::new(&lowered.models)
            .with_ops(&lowered.ops, lowered.version == coding_adventures_warrant_grammar::parser::Version::V0)
            .with_decls(crate::types::Decls::new(&lowered.types));
        let env = DomainEnv::new(file);
        let mut work = IrWork::new(Depth::Full);
        let mut ordinals = 0u64;
        // Pass 1: resolve every claim's domains and charge every bound (IR work, product
        // ordinals) BEFORE any suite is planned: planning builds the admission table, up to
        // 2^28 `admit` evaluations, and must not run for suite i while suite i+1 would be
        // refused (VER04r 14: bounds are checked before the first case).
        let mut resolved = Vec::with_capacity(lowered.suites.len());
        for (suite, cost) in lowered.suites.iter().zip(&analysis.suite_costs) {
            let domains = resolve_claim_in(file, &env, suite, profile.budget, program.decls())?;
            work.charge(cost.total, domains.product)
                .map_err(|e| e.at(&suite.claim))?;
            // [D19]: the per-case check of every call's arguments against the callee's
            // `admit` is IR work too, charged on every product ordinal (conservatively: the
            // check runs only on admitted cases).  The recorded `ir_cost` is unchanged.
            let check = admit_check_cost(suite, &lowered.ops, &analysis.model_costs, profile.max_ir_cost)
                .map_err(|e| e.at(&suite.claim))?;
            work.charge(check, domains.product)
                .map_err(|e| e.at(&suite.claim))?;
            ordinals = ordinals.saturating_add(domains.product);
            if ordinals > MAX_TOTAL_WORK_FULL {
                return Err(GenError::new(
                    code::LIMIT,
                    &suite.claim,
                    "the total product ordinals of a full run exceed 2^32 (VER04r 14)",
                ));
            }
            resolved.push(domains);
        }
        let stride = match profile.slim_stride {
            SlimStride::Auto => None,
            SlimStride::Every(s) => Some(s),
        };
        // Pass 2: plan, run and drop one suite at a time.
        let mut results = Vec::with_capacity(resolved.len());
        for (index, ((suite, cost), domains)) in lowered
            .suites
            .iter()
            .zip(&analysis.suite_costs)
            .zip(resolved)
            .enumerate()
        {
            let plan = SuitePlan::new(&program, suite, domains, cost.total, profile.chunk_size)?;
            let opts = SuiteOptions {
                stride,
                slim_chunks: profile
                    .slim_chunks
                    .iter()
                    .filter(|(c, _)| *c == suite.claim)
                    .map(|(_, i)| *i)
                    .collect(),
            };
            let result = plan.run(&opts)?;
            check_v0_wraps(lowered.version, result.wraps).map_err(|e| e.at(&suite.claim))?;
            observer(&SuiteView {
                program: &program,
                lowered,
                index,
                suite,
                plan: &plan,
                result: &result,
            })?;
            results.push(result);
        }
        results
    };
    Ok(Enumeration { analysis, suites })
}

fn opt_str(row: &mut Vec<(String, Json)>, key: &str, v: &Option<String>) {
    if let Some(s) = v {
        row.push((key.to_string(), Json::str(s.as_str())));
    }
}

/// The parity view of an enumeration (see the module documentation).  `chunk_size`,
/// `budget` and `max_ir_cost` are the profile's.
///
/// # Errors
///
/// Never in practice; the writer's own refusals pass through.
pub fn enumeration_view(e: &Enumeration, profile: &Profile) -> Result<Json, GenError> {
    let mut rows = Vec::new();
    for s in &e.suites {
        let mut row = vec![
            ("id".to_string(), Json::str(s.id.as_str())),
            ("claim".to_string(), Json::str(s.claim.as_str())),
            ("kind".to_string(), Json::str(s.kind)),
            ("binders".to_string(), Json::strs(s.binder_types.iter())),
            (
                "descriptors".to_string(),
                Json::arr(s.descriptors.iter().cloned()),
            ),
            ("demoted".to_string(), Json::strs(s.demoted.iter())),
            ("domain_label".to_string(), Json::str(s.label.name())),
            ("stream_header".to_string(), Json::str(s.header.name())),
            ("result_type".to_string(), Json::str(s.result_type.as_str())),
            ("product".to_string(), Json::Int(s.product)),
            ("chunk_size".to_string(), Json::Int(s.chunk_size)),
            ("chunks_total".to_string(), Json::Int(s.chunks_total)),
            ("cases".to_string(), Json::Int(s.cases)),
            ("discarded".to_string(), Json::Int(s.discarded)),
            ("expected_calls".to_string(), Json::Int(s.expected_calls)),
            ("wraps".to_string(), Json::Int(s.wraps)),
            ("ir_cost".to_string(), Json::Int(s.ir_cost)),
            (
                "slim_chunks".to_string(),
                Json::arr(s.slim.chunks.iter().map(|c| Json::Int(*c))),
            ),
            (
                "slim_inputs_sha256".to_string(),
                Json::str(s.slim.inputs_sha256.as_str()),
            ),
            (
                "inputs_sha256".to_string(),
                Json::str(s.inputs_sha256.as_str()),
            ),
            (
                "chunk_lines".to_string(),
                Json::strs(s.chunk_lines().iter()),
            ),
        ];
        opt_str(&mut row, "slim_expected_sha256", &s.slim.expected_sha256);
        opt_str(&mut row, "expected_sha256", &s.expected_sha256);
        opt_str(&mut row, "v1_inputs_sha256", &s.v1_inputs_sha256);
        opt_str(&mut row, "v1_expected_sha256", &s.v1_expected_sha256);
        rows.push((s.claim.clone(), Json::Obj(row)));
    }
    Ok(Json::obj([
        ("schema", Json::str("warrant-enumeration-parity-v0")),
        ("module", Json::str(e.analysis.lowered.module.as_str())),
        ("document", Json::str(e.analysis.lowered.document())),
        ("chunk_size", Json::Int(profile.chunk_size)),
        ("budget", Json::Int(profile.budget)),
        ("max_ir_cost", Json::Int(profile.max_ir_cost)),
        ("suites", Json::Obj(rows)),
    ]))
}
