//! `adj-verify` — re-execute an ADJ program's reasoning and report, step by
//! step, whether it still holds (`ADJ-REASON-MATH.md` §E.5, §E.6).
//!
//! # What this is for
//!
//! `adj-lang-cli` answers a question and prints a trail. That trail is
//! **testimony**: the engine describing its own work, in a format a confidently
//! wrong system produces just as fluently. `adj-verify` reads the same program
//! and *does the work again* — re-unifying every fact and rule, re-running every
//! negated subgoal to confirm the absence is still an absence, re-multiplying
//! every log-odds contribution, and checking every quoted span against the
//! snapshot it was pinned to. What survives that is **evidence**.
//!
//! # It is not `adj-replay`
//!
//! `adj-replay` (ADJ08) lints a whole trail artifact — extraction, checkers,
//! dialogue, engine — and may re-invoke a model. `adj-verify` never invokes a
//! model and never leaves this process: it is the deep re-execution *inside* one
//! engine artifact. ADJ08's linter should call this, not reimplement it.
//!
//! # Offline, always
//!
//! No network access, by construction — there is no HTTP client in this binary.
//! Quotes are checked against the **pinned snapshot** whose bytes the caller
//! supplies via `--snapshots <DIR>` (files named by their lowercase SHA-256
//! hex). `locator`s are spider-authored strings from untrusted pages; fetching
//! one would turn every audit run into a request aimed by whoever landed a
//! single KB entry. Live re-fetch, when it exists, belongs behind ADJ39's
//! adapter registry — not here.
//!
//! # Output
//!
//! ```json
//! { "verified": false, "fully_verified": false,
//!   "totals": {"steps": 12, "rechecked": 11, "quotes_verified": 4},
//!   "first_failure": {"query":"...", "index":3, "kind":"FromNegation",
//!                     "logic":"negated_goal_provable"},
//!   "queries": [ {"query":"...", "proofs":[ {"steps":[ ... ]} ]} ] }
//! ```
//!
//! Exit status is **1 when anything failed**, so this composes into CI as a gate
//! rather than as something a human has to read.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use adj_lang::{compile_with_imports, decide, ImportLimits};
use adj_lang_cli::{esc, payload, query_echo, FsProvider};
use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_str, Parser};
use logic_engine::{
    enumerate_all, recheck_narrowings, verify_derived, verify_proof, ComputationFailure,
    ComputationStatus, ContentHash, DerivedVerification, LogicFailure, LogicStatus, NarrowingCheck,
    Proof, QuoteMiss, QuoteStatus, SnapshotStore, StepVerification, TraceVerification,
    UnverifiedReason,
};

const SPEC: &str = r#"{
  "cli_builder_spec_version": "1.0",
  "name": "adj-verify",
  "description": "Re-execute an .adj program's reasoning and report, step by step, whether it still holds. Offline: quotes are checked against pinned snapshots, never re-fetched.",
  "version": "0.1.0",
  "arguments": [
    {"id": "program", "name": "PROGRAM", "description": "Path to a .adj program (rulebook + case)", "type": "string", "required": true}
  ],
  "flags": [
    {"id": "snapshots", "long": "snapshots", "description": "Directory of pinned source snapshots, each file named by its lowercase SHA-256 hex", "type": "directory", "value_name": "DIR"}
  ]
}"#;

// ---------------------------------------------------------------------------
// A snapshot store backed by a directory of content-addressed files
// ---------------------------------------------------------------------------

/// Reads snapshot bytes from a directory whose filenames are the lowercase
/// SHA-256 hex of their contents.
///
/// Two properties matter more than convenience here:
///
/// - **The filename is never used as a path fragment from untrusted input.**
///   The hex comes from [`ContentHash::as_hex`], which by construction is 64
///   lowercase hex characters — no separators, no `..`, nothing that could walk
///   out of `root`.
/// - **The bytes are re-hashed after reading.** A store that trusted the
///   filename would let anyone who can write into that directory make an
///   arbitrary document answer to a pinned hash, which is exactly the
///   substitution the pinning exists to prevent.
struct DirSnapshots {
    root: PathBuf,
    /// Hash → verified bytes, so a document is read and hashed **once** per run
    /// no matter how many steps quote it.
    ///
    /// Without this, cost is `O(steps × snapshot_size)`: a program with ten
    /// thousand quoting steps against a 50 MB pinned document would drive
    /// hundreds of gigabytes of reads and SHA-256 through what is advertised as
    /// a cheap offline check. The step count comes from the `.adj` program, so
    /// that multiplier is chosen by whoever wrote the input.
    cache: RefCell<HashMap<String, Option<Vec<u8>>>>,
}

/// Largest snapshot this tool will read into memory.
///
/// 64 MiB is far above any real cited document. The cap is enforced on the
/// **read itself**, not on `metadata().len()`, because a size read from `stat`
/// is not the amount that will actually be read: a character device such as
/// `/dev/zero` reports `st_size == 0` yet streams forever, so a metadata-based
/// cap would wave it straight through into an unbounded allocation. It also
/// closes the `metadata`→`read` TOCTOU, where a small file is swapped for a
/// large one between the two syscalls.
const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;

impl DirSnapshots {
    fn read_verified(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        let path = self.root.join(hash.as_hex());
        // Only a regular file is a snapshot. Rejecting symlinks/devices/FIFOs
        // outright means the read below cannot be aimed at `/dev/zero` at all;
        // `symlink_metadata` does NOT follow the final link, so a link posing as
        // a hash-named file is refused here rather than followed.
        if !fs::symlink_metadata(&path).ok()?.file_type().is_file() {
            return None;
        }
        // Bound the READ, not a reported size. `take(cap + 1)` stops one byte
        // past the ceiling, so an over-large (or infinite) file is detected by
        // overshoot instead of trusting `st_size`.
        let mut file = fs::File::open(&path).ok()?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_SNAPSHOT_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return None;
        }
        // Re-derive rather than believe the filename: anyone who can write into
        // this directory could otherwise make an arbitrary document answer to a
        // pinned hash, which is the exact substitution pinning prevents.
        hash.matches(&bytes).then_some(bytes)
    }
}

impl SnapshotStore for DirSnapshots {
    fn get(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        if let Some(hit) = self.cache.borrow().get(hash.as_hex()) {
            return hit.clone();
        }
        let result = self.read_verified(hash);
        self.cache
            .borrow_mut()
            .insert(hash.as_hex().to_string(), result.clone());
        result
    }
}

/// A store that has nothing, used when `--snapshots` was not given. Quote
/// checks then report `Unverified(SnapshotUnavailable)` — honest, never a pass.
struct EmptyStore;

impl SnapshotStore for EmptyStore {
    fn get(&self, _hash: &ContentHash) -> Option<Vec<u8>> {
        None
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The stable machine-readable name of a logic failure.
///
/// Deliberately a closed set of snake_case tags rather than `format!("{:?}")`:
/// a CI gate greps these, and a Debug rendering is not an interface.
fn logic_json(status: &LogicStatus) -> String {
    let tag = match status {
        LogicStatus::ReChecked => return "\"rechecked\"".to_string(),
        LogicStatus::Failed(f) => match f {
            LogicFailure::UnknownFact(_) => "unknown_fact",
            LogicFailure::UnknownRule(_) => "unknown_rule",
            LogicFailure::GoalDoesNotUnify => "goal_does_not_unify",
            LogicFailure::GoalIsBareVariable => "goal_is_bare_variable",
            LogicFailure::RuleBodyNotDischarged { .. } => "rule_body_not_discharged",
            LogicFailure::NegatedGoalProvable => "negated_goal_provable",
            LogicFailure::NegationSearchTruncated => "negation_search_truncated",
            LogicFailure::UnknownPrior(_) => "unknown_prior",
            LogicFailure::UnknownContribution(_) => "unknown_contribution",
            LogicFailure::UnknownJointContribution(_) => "unknown_joint_contribution",
            LogicFailure::UnknownPredicateContribution(_) => "unknown_predicate_contribution",
            LogicFailure::EvidenceNotObservable => "evidence_not_observable",
            LogicFailure::LogitDiffers { .. } => "logit_differs",
            LogicFailure::SlotNotObserved(_) => "slot_not_observed",
            LogicFailure::ThresholdNotEvaluable => "threshold_not_evaluable",
            LogicFailure::PredicateDoesNotHold { .. } => "predicate_does_not_hold",
        },
    };
    format!("\"{tag}\"")
}

fn computation_json(status: &ComputationStatus) -> String {
    match status {
        ComputationStatus::ReChecked => "{\"status\":\"rechecked\"}".to_string(),
        ComputationStatus::Unverifiable(why) => {
            format!("{{\"status\":\"unverifiable\",\"why\":\"{}\"}}", esc(why))
        }
        ComputationStatus::Failed(failure) => {
            let detail = match failure {
                ComputationFailure::PlanUnavailable => "\"why\":\"plan_unavailable\"".to_string(),
                ComputationFailure::ArtifactDoesNotMatchPlan => {
                    "\"why\":\"artifact_does_not_match_plan\"".to_string()
                }
                ComputationFailure::ScopeUnavailable => "\"why\":\"scope_unavailable\"".to_string(),
                ComputationFailure::EvaluationFailed(error) => format!(
                    "\"why\":\"evaluation_failed\",\"detail\":\"{}\"",
                    esc(&format!("{error:?}"))
                ),
                ComputationFailure::ValueDiffers {
                    recorded,
                    recomputed,
                } => format!(
                    "\"why\":\"value_differs\",\"recorded\":{},\"recomputed\":{}",
                    recorded, recomputed
                ),
                ComputationFailure::ExactValueDiffers => {
                    "\"why\":\"exact_value_differs\"".to_string()
                }
                ComputationFailure::DimensionDiffers => "\"why\":\"dimension_differs\"".to_string(),
                ComputationFailure::TreeDiffers => "\"why\":\"tree_differs\"".to_string(),
                ComputationFailure::ReferencedDerivedDiffers(name) => format!(
                    "\"why\":\"referenced_derived_differs\",\"derived\":\"{}\"",
                    esc(name)
                ),
            };
            format!("{{\"status\":\"failed\",{detail}}}")
        }
    }
}

/// Render a quote verdict.
///
/// `byte_len` is always reported alongside a verified span. §E.3 declines to
/// impose a minimum span length — any threshold would be false precision — so
/// the honest alternative is to *show* the length and let a reviewer judge
/// whether a nine-byte "verified" quote is really carrying the claim.
fn quote_json(status: &QuoteStatus) -> String {
    match status {
        QuoteStatus::Verified {
            byte_offset,
            byte_len,
        } => format!(
            "{{\"status\":\"verified\",\"byte_offset\":{byte_offset},\"byte_len\":{byte_len}}}"
        ),
        QuoteStatus::QuoteMissing(miss) => {
            let (why, extra) = match miss {
                QuoteMiss::BlankSpan => ("blank_span", String::new()),
                QuoteMiss::RangeOutOfBounds {
                    byte_offset,
                    byte_len,
                    snapshot_len,
                } => (
                    "range_out_of_bounds",
                    format!(",\"byte_offset\":{byte_offset},\"byte_len\":{byte_len},\"snapshot_len\":{snapshot_len}"),
                ),
                QuoteMiss::NotACharBoundary {
                    byte_offset,
                    byte_len,
                } => (
                    "not_a_char_boundary",
                    format!(",\"byte_offset\":{byte_offset},\"byte_len\":{byte_len}"),
                ),
                QuoteMiss::TextDiffers {
                    byte_offset,
                    byte_len,
                } => (
                    "text_differs",
                    format!(",\"byte_offset\":{byte_offset},\"byte_len\":{byte_len}"),
                ),
            };
            format!("{{\"status\":\"quote_missing\",\"why\":\"{why}\"{extra}}}")
        }
        QuoteStatus::Unverified(reason) => {
            let why = match reason {
                UnverifiedReason::Unmigrated => "unmigrated",
                UnverifiedReason::NoSnapshotPinned => "no_snapshot_pinned",
                UnverifiedReason::SnapshotUnavailable => "snapshot_unavailable",
                UnverifiedReason::NoByteOffset => "no_byte_offset",
                UnverifiedReason::NoProvenance => "no_provenance",
            };
            format!("{{\"status\":\"unverified\",\"why\":\"{why}\"}}")
        }
        QuoteStatus::SourceDrifted => "{\"status\":\"source_drifted\"}".to_string(),
        QuoteStatus::SourceUnreachable => "{\"status\":\"source_unreachable\"}".to_string(),
        QuoteStatus::NotApplicable => "{\"status\":\"not_applicable\"}".to_string(),
    }
}

/// Render one step's verdict.
///
/// The goal goes out through [`payload`], not `esc`. A goal term can carry the
/// caller's own free text — a chart phrase, a pasted question — and this
/// artifact is designed to be replayed and shared. Redaction and the length cap
/// therefore apply here for the same reason they apply to an abstention's
/// echoed fields.
fn step_json(s: &StepVerification) -> String {
    format!(
        "{{\"index\":{},\"depth\":{},\"kind\":\"{}\",\"goal\":\"{}\",\"logic\":{},\"quote\":{}}}",
        s.index,
        s.depth,
        esc(s.kind),
        payload(&format!("{}", s.goal)),
        logic_json(&s.logic),
        quote_json(&s.quote)
    )
}

fn derived_json(report: &DerivedVerification) -> String {
    let formulas = report
        .formula_quotes
        .iter()
        .map(quote_json)
        .collect::<Vec<_>>()
        .join(",");
    let inputs = report
        .input_quotes
        .iter()
        .map(|input| {
            format!(
                "{{\"fact_id\":{},\"quote\":{}}}",
                input.fact_id.0,
                quote_json(&input.quote)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"name\":\"{}\",\"query_answer\":{},\"passed\":{},\"fully_verified\":{},\"computation\":{},\"formula_quotes\":[{}],\"input_quotes\":[{}]}}",
        esc(&report.name),
        report.is_query_answer,
        report.passed(),
        report.fully_verified(),
        computation_json(&report.computation),
        formulas,
        inputs
    )
}

#[derive(Default)]
struct Totals {
    steps: usize,
    rechecked: usize,
    quotes_verified: usize,
    proofs: usize,
    proofs_fully_verified: usize,
    computations: usize,
    computations_rechecked: usize,
    computations_fully_verified: usize,
    query_computations: usize,
    query_computations_fully_verified: usize,
}

impl Totals {
    fn absorb(&mut self, report: &TraceVerification) {
        self.proofs += 1;
        if report.fully_verified() {
            self.proofs_fully_verified += 1;
        }
        for s in &report.steps {
            self.steps += 1;
            if matches!(s.logic, LogicStatus::ReChecked) {
                self.rechecked += 1;
            }
            if matches!(s.quote, QuoteStatus::Verified { .. }) {
                self.quotes_verified += 1;
            }
        }
    }

    fn absorb_derived(&mut self, report: &DerivedVerification) {
        self.computations += 1;
        if matches!(report.computation, ComputationStatus::ReChecked) {
            self.computations_rechecked += 1;
        }
        if report.fully_verified() {
            self.computations_fully_verified += 1;
        }
        if report.is_query_answer {
            self.query_computations += 1;
            if report.fully_verified() {
                self.query_computations_fully_verified += 1;
            }
        }
        self.quotes_verified += report
            .formula_quotes
            .iter()
            .filter(|quote| matches!(quote, QuoteStatus::Verified { .. }))
            .count();
        self.quotes_verified += report
            .input_quotes
            .iter()
            .filter(|input| matches!(input.quote, QuoteStatus::Verified { .. }))
            .count();
    }
}

fn main() -> ExitCode {
    let spec = load_spec_from_str(SPEC).expect("internal: invalid CLI spec");
    let parser = Parser::new(spec);
    let argv: Vec<String> = std::env::args().collect();
    let result = match parser.parse(&argv) {
        Ok(ParserOutput::Help(h)) => {
            print!("{}", h.text);
            return ExitCode::SUCCESS;
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
            return ExitCode::SUCCESS;
        }
        Ok(ParserOutput::Parse(r)) => r,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::from(2);
        }
    };

    let path = result
        .arguments
        .get("program")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let store: Box<dyn SnapshotStore> = match result.flags.get("snapshots").and_then(|v| v.as_str())
    {
        Some(dir) => Box::new(DirSnapshots {
            root: PathBuf::from(dir),
            cache: RefCell::new(HashMap::new()),
        }),
        None => Box::new(EmptyStore),
    };

    let root_id = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("adj-verify: cannot read {}: {}", path, e);
            return ExitCode::from(2);
        }
    };
    let Some(root_dir) = root_id.parent().map(|d| d.to_path_buf()) else {
        eprintln!("adj-verify: {} has no parent directory", path);
        return ExitCode::from(2);
    };
    let provider = FsProvider { root: root_dir };
    let lowered = match compile_with_imports(
        &root_id.to_string_lossy(),
        &provider,
        ImportLimits::default(),
    ) {
        Ok(l) => l,
        Err(e) => {
            // Through `payload`, not `esc`: a lowering error embeds
            // program-derived strings (an undefined term, a failed computation's
            // detail), which on a sensitive run can be the same chart text every
            // other echo in this binary redacts.
            println!("{{\"error\":\"{}\"}}", payload(&format!("{:?}", e)));
            return ExitCode::from(1);
        }
    };

    // Both reasoning paths are re-checked. `enumerate_all` covers the SLD side
    // (recall, rules, tables, negation); `decide` covers likelihood-ratio
    // aggregation. Verifying only one would leave half the trail unexamined
    // while the report said "verified".
    let mut per_query: Vec<String> = Vec::new();
    let mut totals = Totals::default();
    let mut first_failure: Option<String> = None;

    let mut record = |pass: &str, query: &str, proofs: &[Proof], totals: &mut Totals| {
        let mut proof_blobs: Vec<String> = Vec::new();
        for proof in proofs {
            let report = verify_proof(proof, &lowered.kb, store.as_ref());
            totals.absorb(&report);
            if first_failure.is_none() {
                if let Some(f) = report.first_failure() {
                    first_failure = Some(format!(
                        "{{\"pass\":\"{}\",\"query\":\"{}\",\"index\":{},\"kind\":\"{}\",\"logic\":{},\"quote\":{}}}",
                        esc(pass),
                        query_echo(query),
                        f.index,
                        esc(f.kind),
                        logic_json(&f.logic),
                        quote_json(&f.quote)
                    ));
                }
            }
            let steps: Vec<String> = report.steps.iter().map(step_json).collect();
            proof_blobs.push(format!(
                "{{\"passed\":{},\"fully_verified\":{},\"steps\":[{}]}}",
                report.passed(),
                report.fully_verified(),
                steps.join(",")
            ));
        }
        per_query.push(format!(
            "{{\"pass\":\"{}\",\"query\":\"{}\",\"proofs\":[{}]}}",
            esc(pass),
            query_echo(query),
            proof_blobs.join(",")
        ));
    };

    // A truncated search is NOT an examined program. `enumerate_all` reports
    // when it hit the resolver's depth cap, and a run that gave up found zero
    // proofs for a reason that has nothing to do with the program being sound.
    // Letting that exit 0 would hand a green CI gate to any input crafted to
    // make the search bail — the same "I stopped looking" / "there is none"
    // conflation the negation re-check treats as a hard failure.
    let mut truncated_queries: Vec<String> = Vec::new();
    for q in &lowered.queries {
        let text = format!("{}", q);
        let dag = enumerate_all(q, &lowered.kb);
        if dag.truncated {
            truncated_queries.push(format!("\"{}\"", query_echo(&text)));
        }
        record("sld", &text, &dag.proofs, &mut totals);
    }

    let diff = decide(&lowered);
    for r in &diff.ranked {
        let text = format!("{}", r.hypothesis);
        record("lr", &text, &r.result.dag.proofs, &mut totals);
    }

    // Formula queries do not produce SLD proofs: their evidence is a CPU
    // derivation tree plus the formula and input provenance carried by the
    // resulting binding. Re-check that independent channel explicitly so a
    // formula-only program can become fully verified without inventing a fake
    // logic proof, and so tampered arithmetic localizes to the computed value.
    let mut computation_blobs: Vec<String> = Vec::new();
    for derived in lowered.kb.derived_bindings() {
        let report = verify_derived(derived, &lowered.kb, store.as_ref());
        totals.absorb_derived(&report);
        if first_failure.is_none() && !report.passed() {
            first_failure = Some(match &report.computation {
                status if !matches!(status, ComputationStatus::ReChecked) => format!(
                    "{{\"pass\":\"computation\",\"name\":\"{}\",\"computation\":{}}}",
                    esc(&report.name),
                    computation_json(&report.computation)
                ),
                _ if report
                    .formula_quotes
                    .iter()
                    .any(|quote| matches!(quote, QuoteStatus::QuoteMissing(_))) =>
                {
                    let quote = report
                        .formula_quotes
                        .iter()
                        .find(|quote| matches!(quote, QuoteStatus::QuoteMissing(_)))
                        .expect("a failed formula quote is present");
                    format!(
                        "{{\"pass\":\"formula_quote\",\"name\":\"{}\",\"quote\":{}}}",
                        esc(&report.name),
                        quote_json(quote)
                    )
                }
                _ => {
                    let input = report
                        .input_quotes
                        .iter()
                        .find(|input| matches!(input.quote, QuoteStatus::QuoteMissing(_)))
                        .expect("a failed derived report has a localized failure");
                    format!(
                        "{{\"pass\":\"input_quote\",\"name\":\"{}\",\"fact_id\":{},\"quote\":{}}}",
                        esc(&report.name),
                        input.fact_id.0,
                        quote_json(&input.quote)
                    )
                }
            });
        }
        computation_blobs.push(derived_json(&report));
    }

    // NUM-6 audit-exactness re-check (ADJ-NUMERIC-SUBSTRATE §4.3, §6, §7). The SLD
    // and LR passes above re-derive the *logic*; the compute derivation trees carry
    // the *arithmetic*, and every `round_to`/`round_sig`/`to_scientific`/`to_percent`/
    // `to_currency` narrowing recorded a rounded number (and boundary string) that a
    // confidently-wrong or since-edited artifact prints just as fluently. Here that
    // testimony becomes evidence: for every `let`-bound derived value we walk its tree
    // and re-round / re-format each narrowing's recorded EXACT source under the recorded
    // spec/mode (`recheck_narrowings`), confirming the recorded result and rendering
    // reproduce. A `Mismatch` is a hard failure — the same "valid-looking number from an
    // invented rounding" class the negation and logit re-checks guard against on their
    // own paths.
    let mut narrowing_blobs: Vec<String> = Vec::new();
    let mut narrowings_rechecked = 0usize;
    let mut narrowings_unverifiable = 0usize;
    let mut narrowings_mismatched = 0usize;
    for d in lowered.kb.derived_bindings() {
        let checks = recheck_narrowings(&d.tree);
        if checks.is_empty() {
            continue;
        }
        let mut check_blobs: Vec<String> = Vec::new();
        for (depth, check) in &checks {
            let blob = match check {
                NarrowingCheck::ReChecked => {
                    narrowings_rechecked += 1;
                    format!("{{\"depth\":{depth},\"status\":\"rechecked\"}}")
                }
                NarrowingCheck::Unverifiable => {
                    narrowings_unverifiable += 1;
                    format!("{{\"depth\":{depth},\"status\":\"unverifiable\"}}")
                }
                NarrowingCheck::Mismatch {
                    why,
                    recorded,
                    recomputed,
                } => {
                    narrowings_mismatched += 1;
                    if first_failure.is_none() {
                        first_failure = Some(format!(
                            "{{\"pass\":\"narrowing\",\"name\":\"{}\",\"depth\":{},\"why\":\"{}\",\"recorded\":\"{}\",\"recomputed\":\"{}\"}}",
                            esc(&d.name),
                            depth,
                            esc(why),
                            esc(recorded),
                            esc(recomputed),
                        ));
                    }
                    format!(
                        "{{\"depth\":{},\"status\":\"mismatch\",\"why\":\"{}\",\"recorded\":\"{}\",\"recomputed\":\"{}\"}}",
                        depth,
                        esc(why),
                        esc(recorded),
                        esc(recomputed),
                    )
                }
                // A non-narrowing verdict never appears here — `recheck_narrowings`
                // only returns entries for actual narrowing nodes.
                NarrowingCheck::NotANarrowing => continue,
            };
            check_blobs.push(blob);
        }
        narrowing_blobs.push(format!(
            "{{\"name\":\"{}\",\"checks\":[{}]}}",
            esc(&d.name),
            check_blobs.join(",")
        ));
    }

    let verified =
        first_failure.is_none() && truncated_queries.is_empty() && narrowings_mismatched == 0;
    // `fully_verified` is NOT `verified` with extra steps counted. It requires
    // that every proof had its quotes affirmatively confirmed against a pinned
    // snapshot — and that there was something to check at all. An earlier draft
    // computed it from re-execution alone, so a library whose every quote was
    // `unmigrated` reported the system's strongest verdict while no span had
    // been checked. That is the fail-open shape this tool exists to catch, and
    // it is no less wrong for appearing in the tool itself.
    let fully = verified
        && totals.proofs + totals.query_computations > 0
        && (totals.proofs == 0 || totals.proofs_fully_verified == totals.proofs)
        && (totals.computations == 0 || totals.computations_fully_verified == totals.computations)
        && (totals.query_computations == 0
            || totals.query_computations_fully_verified == totals.query_computations)
        && totals.quotes_verified > 0;
    println!(
        "{{\"verified\":{},\"fully_verified\":{},\"totals\":{{\"proofs\":{},\"proofs_fully_verified\":{},\"steps\":{},\"rechecked\":{},\"quotes_verified\":{},\"computations\":{},\"computations_rechecked\":{},\"computations_fully_verified\":{},\"query_computations\":{},\"query_computations_fully_verified\":{},\"narrowings_rechecked\":{},\"narrowings_unverifiable\":{},\"narrowings_mismatched\":{}}},\"truncated_queries\":[{}],\"computations\":[{}],\"narrowings\":[{}],\"first_failure\":{},\"queries\":[{}]}}",
        verified,
        fully,
        totals.proofs,
        totals.proofs_fully_verified,
        totals.steps,
        totals.rechecked,
        totals.quotes_verified,
        totals.computations,
        totals.computations_rechecked,
        totals.computations_fully_verified,
        totals.query_computations,
        totals.query_computations_fully_verified,
        narrowings_rechecked,
        narrowings_unverifiable,
        narrowings_mismatched,
        truncated_queries.join(","),
        computation_blobs.join(","),
        narrowing_blobs.join(","),
        first_failure.as_deref().unwrap_or("null"),
        per_query.join(",")
    );

    if verified {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
