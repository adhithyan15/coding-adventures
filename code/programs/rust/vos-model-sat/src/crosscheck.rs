//! # The three cross-checks (VOS31 section 6), and their evidence
//!
//! The encoder is hand-written, and that is the whole trust problem (VOS-D072: there is no
//! Warrant-to-QF_BV translator).  These checks are what turn "a hand transcription" into
//! evidence.  Each is a function that returns a [`Report`], so the same code runs in the CLI and
//! in the tests, and a **negative control** is just "run it on a wrong encoder and see the report
//! is not clean" (VOS31 6.4).
//!
//! ```text
//!   (a) lemma check      encoder holds  vs  evaluator, every case of the lemma's X1 domain, N = 8
//!                        on the BASE document and on each MUTANT document; the failing SETS
//!                        must be equal and the count must equal `lemmas.table`
//!   (b) function check   each model function's circuit  vs  the evaluator's value, every case
//!                        of its domain, N = 8 (a wrong `top` cannot hide behind a lemma)
//!   (c) width check      the encoder  vs  a plain Rust reference, exhaustively, N = 1 ..= 10;
//!                        and the reference vs the evaluator at N = 8
//! ```
//!
//! ## Why (a) needs the mutants
//!
//! On the base document the evaluator says "true" 65,536 times and a correct encoder also says
//! "true" 65,536 times: equal, and nearly empty of information (an encoder that returned the
//! constant true would pass).  On a **mutant** document the evaluator fails on a specific,
//! structured set of cases (63,488, 448, 512, ...); an encoder agrees only if its mutant fails on
//! exactly those cases.  That is a real comparison, so it is the half that counts.
//!
//! ## Why (b) and (c) separately
//!
//! Three checks are three chances to catch a fault.  A fault that only one of them catches means
//! the other two have not been shown able to fail; [`Fault`](crate::Fault) has four, and the tests
//! assert each check's verdict on each fault.
//!
//! ## What is recorded in a report
//!
//! Counts, plus the first few disagreements (case number and the two values), never an unbounded
//! list: a badly wrong encoder disagrees on every case.

use std::collections::BTreeMap;

use crate::lemmas::{lemma_circuit, LemmaId};
use crate::model::{Encoder, SchedModel};
use crate::mutants::Mutant;
use crate::oracle::{run_document, with_probe_claims, ContractFiles, DocResults, LemmaFailures};
use crate::probes::{all_probes, Probe};
use crate::reference;
use crate::sweep::{sweep, TooWide};
use crate::types::Val;
use bit_blaster::aig::{Aig, Edge};

/// The width the evaluator can check: the contract's own `config N 8`.
pub const EVALUATOR_N: usize = 8;

/// The widths the plain Rust reference is compared at (VOS31 6.3).
pub const REFERENCE_WIDTHS: std::ops::RangeInclusive<usize> = 1..=10;

/// At most this many disagreements are kept in a report; the total is still counted.
pub const MAX_SAMPLES: usize = 5;

/// One disagreement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// Which check found it: `"lemma"`, `"function"`, `"width"`, `"reference"` or `"table"`.
    pub check: &'static str,
    /// The lemma, probe or relation involved.
    pub subject: String,
    /// The case number, where there is one.
    pub case: Option<u64>,
    /// What the two sides said.
    pub detail: String,
}

/// The outcome of one or more comparisons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Cases (or counts) compared.
    pub compared: u64,
    /// Disagreements found, all of them.
    pub disagreements: u64,
    /// The first [`MAX_SAMPLES`] of them.
    pub samples: Vec<Mismatch>,
}

impl Report {
    /// No disagreement, and at least one comparison made (a check that compared nothing
    /// proved nothing: "none ran = fail", VOS31 10.2).
    pub fn is_clean(&self) -> bool {
        self.disagreements == 0 && self.compared > 0
    }

    fn agree(&mut self) {
        self.compared += 1;
    }

    fn disagree(&mut self, m: Mismatch) {
        self.compared += 1;
        self.disagreements += 1;
        if self.samples.len() < MAX_SAMPLES {
            self.samples.push(m);
        }
    }

    /// Fold `other` into `self`.
    pub fn merge(&mut self, other: Report) {
        self.compared += other.compared;
        self.disagreements += other.disagreements;
        for s in other.samples {
            if self.samples.len() < MAX_SAMPLES {
                self.samples.push(s);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// (a) the lemma check
// ---------------------------------------------------------------------------------------

/// Check (a), one lemma: on every case of the lemma's domain, "the encoder's `holds` is false"
/// must equal "the evaluator did not say true".  Works on the base model against the base
/// document's result and on a mutant against its mutant document's result.
pub fn check_lemma(model: &dyn SchedModel, lemma: LemmaId, evaluated: &LemmaFailures) -> Report {
    let mut report = Report::default();
    let mut aig = Aig::new();
    let c = lemma_circuit(lemma, model, &mut aig);
    let inputs = c.inputs.len();
    let expected_cases = 1u64 << inputs;
    if evaluated.product != expected_cases || evaluated.failing.len() as u64 != expected_cases {
        report.disagree(Mismatch {
            check: "lemma",
            subject: lemma.name().to_string(),
            case: None,
            detail: format!(
                "the evaluator ranged over {} cases and the encoder has 2^{inputs} = {expected_cases}",
                evaluated.product
            ),
        });
        return report;
    }
    let swept = sweep(&aig, aig.num_inputs(), |case, value| {
        let encoder_fails = !value(c.holds);
        let evaluator_fails = evaluated.failing[case as usize];
        if encoder_fails == evaluator_fails {
            report.agree();
        } else {
            report.disagree(Mismatch {
                check: "lemma",
                subject: lemma.name().to_string(),
                case: Some(case),
                detail: format!(
                    "encoder {}, evaluator {}",
                    verdict(!encoder_fails),
                    verdict(!evaluator_fails)
                ),
            });
        }
    });
    if swept.is_err() {
        report.disagree(Mismatch {
            check: "lemma",
            subject: lemma.name().to_string(),
            case: None,
            detail: "the lemma's domain is too wide to sweep".to_string(),
        });
    }
    report
}

fn verdict(holds: bool) -> &'static str {
    if holds {
        "holds"
    } else {
        "fails"
    }
}

/// The `failed` column of `warrant/lemmas.table` for each lemma (`lemma <name> ... failed <k>`).
/// A line that does not parse is skipped; a lemma with no line is then "missing", which
/// [`check_table_counts`] reports.
pub fn table_failed_counts(table: &str) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for line in table.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        if words.first() != Some(&"lemma") || words.len() < 3 {
            continue;
        }
        let failed = words
            .iter()
            .position(|w| *w == "failed")
            .and_then(|i| words.get(i + 1))
            .and_then(|w| w.parse::<u64>().ok());
        if let Some(k) = failed {
            out.insert(words[1].to_string(), k);
        }
    }
    out
}

/// The table's mutant `failed` count must equal both sides' failing counts.  This ties the
/// committed `lemmas.table` (which `check-contract` already compares byte for byte) to the
/// encoder's mutant, so the counts the spec quotes are the counts the encoder produces.
pub fn check_table_counts(
    table: &str,
    lemma: LemmaId,
    evaluator_failed: u64,
    encoder_failed: u64,
) -> Report {
    let mut report = Report::default();
    let counts = table_failed_counts(table);
    match counts.get(lemma.name()) {
        None => report.disagree(Mismatch {
            check: "table",
            subject: lemma.name().to_string(),
            case: None,
            detail: "no `lemma` row in warrant/lemmas.table".to_string(),
        }),
        Some(&k) => {
            for (who, got) in [("evaluator", evaluator_failed), ("encoder", encoder_failed)] {
                if got == k {
                    report.agree();
                } else {
                    report.disagree(Mismatch {
                        check: "table",
                        subject: lemma.name().to_string(),
                        case: None,
                        detail: format!("lemmas.table says {k} failing cases, the {who} {got}"),
                    });
                }
            }
        }
    }
    report
}

/// Sweep `aig` over its whole domain, calling `f` on every case, and record in `report` any way
/// the sweep fell short: a refused sweep (`TooWide`), or a number of cases visited that is not
/// `2^expected_inputs` (the domain the SUBJECT declares, not the one the circuit happens to
/// have).  A sweep that silently visited nothing would add zero comparisons and hide the
/// shortfall, so a short sweep is a disagreement, never a quiet pass.
fn counted_sweep(
    report: &mut Report,
    check: &'static str,
    subject: &str,
    aig: &Aig,
    expected_inputs: usize,
    mut f: impl FnMut(&mut Report, u64, &dyn Fn(Edge) -> bool),
) {
    let before = report.compared;
    let swept = sweep(aig, aig.num_inputs(), |case, value| f(report, case, value));
    let visited = report.compared - before;
    let expected = 1u64.checked_shl(expected_inputs as u32).unwrap_or(u64::MAX);
    let detail = match swept {
        Err(TooWide(w)) => Some(format!("a sweep over {w} inputs was refused (too wide)")),
        Ok(()) if visited != expected => Some(format!(
            "{visited} cases were compared, the domain has 2^{expected_inputs} = {expected}"
        )),
        Ok(()) => None,
    };
    if let Some(detail) = detail {
        report.disagree(Mismatch {
            check,
            subject: subject.to_string(),
            case: None,
            detail,
        });
    }
}

/// A mutant must BREAK its lemma, on both sides (VOS31 5.2: "a lemma whose mutant is also fine
/// has proved nothing").  Without this check a mutant that fails nowhere, on the evaluator and on
/// the encoder alike, would pass the set comparison (empty equals empty) and a table whose count
/// was also zero.  It applies to every lemma, `closure_at_m` included: the lemma is trivial on
/// the honest model (`inv` is true), but its mutant (`inv` is `m.p0`) fails on every case.
pub fn check_mutant_breaks(lemma: LemmaId, evaluator_failed: u64, encoder_failed: u64) -> Report {
    let mut report = Report::default();
    for (who, failed) in [("evaluator", evaluator_failed), ("encoder", encoder_failed)] {
        if failed > 0 {
            report.agree();
        } else {
            report.disagree(Mismatch {
                check: "mutant",
                subject: lemma.name().to_string(),
                case: None,
                detail: format!("the mutant survives: the {who} fails it on no case"),
            });
        }
    }
    report
}

/// The number of failing cases of `model` on `lemma` (the encoder's own count).
pub fn encoder_failed(model: &dyn SchedModel, lemma: LemmaId) -> Result<u64, TooWide> {
    let mut aig = Aig::new();
    let c = lemma_circuit(lemma, model, &mut aig);
    let mut failed = 0u64;
    sweep(&aig, aig.num_inputs(), |_, value| {
        if !value(c.holds) {
            failed += 1;
        }
    })?;
    Ok(failed)
}

// ---------------------------------------------------------------------------------------
// (b) the function check
// ---------------------------------------------------------------------------------------

/// Check (b), one probe: the circuit's value on every case equals the evaluator's.
pub fn check_function(model: &dyn SchedModel, probe: &Probe, evaluated: &[Val]) -> Report {
    let mut report = Report::default();
    let mut aig = Aig::new();
    let out = probe.circuit(model, &mut aig);
    let inputs = aig.num_inputs();
    if evaluated.len() as u64 != 1u64 << inputs {
        report.disagree(Mismatch {
            check: "function",
            subject: probe.name.to_string(),
            case: None,
            detail: format!(
                "the evaluator ranged over {} cases and the encoder has 2^{inputs}",
                evaluated.len()
            ),
        });
        return report;
    }
    counted_sweep(
        &mut report,
        "function",
        probe.name,
        &aig,
        probe.input_count(model.n()),
        |report, case, value| {
            let got = out.read(value);
            let want = &evaluated[case as usize];
            if &got == want {
                report.agree();
            } else {
                report.disagree(Mismatch {
                    check: "function",
                    subject: probe.name.to_string(),
                    case: Some(case),
                    detail: format!("encoder {got:?}, evaluator {want:?}"),
                });
            }
        },
    );
    report
}

// ---------------------------------------------------------------------------------------
// (c) the width check
// ---------------------------------------------------------------------------------------

/// Check (c), one width: the encoder `model` against the plain Rust reference, exhaustively:
/// every probe on every case of its domain, and every lemma's `holds` on every case.
pub fn check_against_reference(model: &dyn SchedModel) -> Report {
    let n = model.n();
    let mut report = Report::default();
    for probe in all_probes() {
        let mut aig = Aig::new();
        let out = probe.circuit(model, &mut aig);
        let subject = format!("{} at N = {n}", probe.name);
        counted_sweep(
            &mut report,
            "width",
            &subject,
            &aig,
            probe.input_count(n),
            |report, case, value| {
                let got = out.read(value);
                let want = (probe.reference)(n, &probe.decode(n, case));
                if got == want {
                    report.agree();
                } else {
                    report.disagree(Mismatch {
                        check: "width",
                        subject: subject.clone(),
                        case: Some(case),
                        detail: format!("encoder {got:?}, reference {want:?}"),
                    });
                }
            },
        );
    }
    for lemma in LemmaId::ALL {
        let mut aig = Aig::new();
        let c = lemma_circuit(lemma, model, &mut aig);
        let subject = format!("{} at N = {n}", lemma.name());
        counted_sweep(
            &mut report,
            "width",
            &subject,
            &aig,
            lemma.input_count(n),
            |report, case, value| {
                let (m, t) = decode_lemma_case(lemma, n, case);
                let want = reference::lemma_holds(lemma, n, m, t);
                let got = value(c.holds);
                if got == want {
                    report.agree();
                } else {
                    report.disagree(Mismatch {
                        check: "width",
                        subject: subject.clone(),
                        case: Some(case),
                        detail: format!("encoder {}, reference {}", verdict(got), verdict(want)),
                    });
                }
            },
        );
    }
    report
}

/// `(ready mask, id)` of a lemma case at width `n` (the numbering of [`crate::lemmas`]).
pub fn decode_lemma_case(lemma: LemmaId, n: usize, case: u64) -> (u64, u8) {
    let mut c = case;
    let m = if lemma.has_ready_binders() {
        let m = c & reference::mask(n);
        c >>= n;
        m
    } else {
        0
    };
    let t = if lemma.has_id_binder() {
        (c & 0xFF) as u8
    } else {
        0
    };
    (m, t)
}

/// Check (c) over every width of [`REFERENCE_WIDTHS`], building each encoder with `make`.
pub fn check_all_widths(make: &dyn Fn(usize) -> Box<dyn SchedModel>) -> Report {
    let mut report = Report::default();
    for n in REFERENCE_WIDTHS {
        report.merge(check_against_reference(make(n).as_ref()));
    }
    report
}

/// The reference against the evaluator at N = 8, so the chain `evaluator = reference = encoder`
/// has no gap at its left end: every probe's reference value equals the evaluator's, and every
/// lemma's reference verdict equals the evaluator's on the base document.
pub fn check_reference_vs_evaluator(base: &DocResults) -> Report {
    let mut report = Report::default();
    for probe in all_probes() {
        let Some(values) = base.probes.get(probe.name) else {
            report.disagree(Mismatch {
                check: "reference",
                subject: probe.name.to_string(),
                case: None,
                detail: "the evaluator produced no values for this probe".to_string(),
            });
            continue;
        };
        for (case, want) in values.iter().enumerate() {
            let got = (probe.reference)(EVALUATOR_N, &probe.decode(EVALUATOR_N, case as u64));
            if &got == want {
                report.agree();
            } else {
                report.disagree(Mismatch {
                    check: "reference",
                    subject: probe.name.to_string(),
                    case: Some(case as u64),
                    detail: format!("reference {got:?}, evaluator {want:?}"),
                });
            }
        }
    }
    for lemma in LemmaId::ALL {
        let Some(ev) = base.lemmas.get(lemma.name()) else {
            report.disagree(Mismatch {
                check: "reference",
                subject: lemma.name().to_string(),
                case: None,
                detail: "the evaluator produced no result for this lemma".to_string(),
            });
            continue;
        };
        for (case, &fails) in ev.failing.iter().enumerate() {
            let (m, t) = decode_lemma_case(lemma, EVALUATOR_N, case as u64);
            let holds = reference::lemma_holds(lemma, EVALUATOR_N, m, t);
            if holds == !fails {
                report.agree();
            } else {
                report.disagree(Mismatch {
                    check: "reference",
                    subject: lemma.name().to_string(),
                    case: Some(case as u64),
                    detail: format!(
                        "reference {}, evaluator {}",
                        verdict(holds),
                        verdict(!fails)
                    ),
                });
            }
        }
    }
    report
}

// ---------------------------------------------------------------------------------------
// Gathering the evaluator's answers, and running everything
// ---------------------------------------------------------------------------------------

/// The probes that a mutant of model `replaced` can change: the function itself and every probe
/// whose function calls it (a mutant replaces ONE model and every caller sees the change).
pub fn probes_affected_by(replaced: &str) -> Vec<&'static str> {
    match replaced {
        "inv" => vec!["inv"],
        "abs_insert" => vec!["abs_insert_upd", "abs_insert_ready"],
        "abs_remove" => vec!["abs_remove_upd", "abs_remove_ready"],
        "top" => vec!["top", "policy_dispatch"],
        "valid_pick" => vec!["valid_pick_run", "valid_pick_idle", "safe_dispatch"],
        "after" => vec![
            "after_run",
            "after_idle",
            "safe_dispatch",
            "policy_dispatch",
        ],
        "policy_dispatch" => vec!["policy_dispatch"],
        "pre_insert" => vec!["pre_insert"],
        _ => vec![],
    }
}

/// What the evaluator said, on the base document and on each lemma's mutant document.
#[derive(Debug)]
pub struct Evidence {
    /// The base document: every lemma and every probe.
    pub base: DocResults,
    /// Each mutant document: its own lemma and the probes its mutant can change.
    pub mutants: Vec<(LemmaId, DocResults)>,
    /// `warrant/lemmas.table` as read, if there is one.
    pub table: Option<String>,
}

/// Evaluate the base document and the ten mutant documents.  Documents run on parallel threads
/// (each evaluation has its own big-stack thread); results are keyed by lemma, so the order is
/// the same on every run.
pub fn gather(files: &ContractFiles) -> Result<Evidence, String> {
    let probes = all_probes();
    let names: Vec<&str> = LemmaId::ALL.iter().map(|l| l.name()).collect();
    let base_doc = with_probe_claims(&files.base_document(), &probes);
    let mut mutant_docs: Vec<(LemmaId, String, Vec<Probe>)> = Vec::new();
    for lemma in LemmaId::ALL {
        let affected = probes_affected_by(lemma.mutant_model());
        let chosen: Vec<Probe> = probes
            .iter()
            .filter(|p| affected.contains(&p.name))
            .cloned()
            .collect();
        let doc = with_probe_claims(&files.mutant_document(lemma.name())?, &chosen);
        mutant_docs.push((lemma, doc, chosen));
    }
    std::thread::scope(|scope| {
        let base_handle = scope.spawn(|| run_document(&base_doc, &names, &probes));
        let handles: Vec<_> = mutant_docs
            .iter()
            .map(|(lemma, doc, chosen)| {
                scope.spawn(move || run_document(doc, &[lemma.name()], chosen))
            })
            .collect();
        let base = base_handle
            .join()
            .map_err(|_| "an evaluation thread panicked".to_string())??;
        let mut mutants = Vec::new();
        for ((lemma, _, _), h) in mutant_docs.iter().zip(handles) {
            let r = h
                .join()
                .map_err(|_| "an evaluation thread panicked".to_string())??;
            mutants.push((*lemma, r));
        }
        Ok(Evidence {
            base,
            mutants,
            table: files.table.clone(),
        })
    })
}

/// Check (a) and (b) for the base model and for every mutant, given the evidence.  `base` builds
/// the encoder under test at N = 8 and `mutant` builds a lemma's mutant at N = 8 (the real ones,
/// or a deliberately wrong pair for a negative control).
pub fn check_evaluator_side(
    ev: &Evidence,
    base: &dyn SchedModel,
    mutant: &dyn Fn(LemmaId) -> Box<dyn SchedModel>,
) -> Report {
    let mut report = Report::default();
    let probes = all_probes();
    for lemma in LemmaId::ALL {
        match ev.base.lemmas.get(lemma.name()) {
            Some(e) => report.merge(check_lemma(base, lemma, e)),
            None => report.disagree(missing(lemma.name())),
        }
    }
    for p in &probes {
        match ev.base.probes.get(p.name) {
            Some(v) => report.merge(check_function(base, p, v)),
            None => report.disagree(missing(p.name)),
        }
    }
    for (lemma, results) in &ev.mutants {
        let m = mutant(*lemma);
        let Some(e) = results.lemmas.get(lemma.name()) else {
            report.disagree(missing(lemma.name()));
            continue;
        };
        report.merge(check_lemma(m.as_ref(), *lemma, e));
        let enc_failed = match encoder_failed(m.as_ref(), *lemma) {
            Ok(k) => k,
            Err(TooWide(w)) => {
                report.disagree(Mismatch {
                    check: "mutant",
                    subject: lemma.name().to_string(),
                    case: None,
                    detail: format!("a sweep over {w} inputs was refused (too wide)"),
                });
                continue;
            }
        };
        report.merge(check_mutant_breaks(*lemma, e.failed(), enc_failed));
        if let Some(table) = &ev.table {
            report.merge(check_table_counts(table, *lemma, e.failed(), enc_failed));
        } else {
            report.disagree(Mismatch {
                check: "table",
                subject: lemma.name().to_string(),
                case: None,
                detail: "there is no warrant/lemmas.table to compare counts with".to_string(),
            });
        }
        for p in &probes {
            if let Some(v) = results.probes.get(p.name) {
                report.merge(check_function(m.as_ref(), p, v));
            }
        }
    }
    report
}

fn missing(what: &str) -> Mismatch {
    Mismatch {
        check: "lemma",
        subject: what.to_string(),
        case: None,
        detail: "the evaluator produced no result".to_string(),
    }
}

/// The honest base encoder and the ten real mutants at N = 8, for [`check_evaluator_side`].
pub fn real_encoders() -> (Encoder, impl Fn(LemmaId) -> Box<dyn SchedModel>) {
    (Encoder { n: EVALUATOR_N }, |l| {
        Box::new(Mutant {
            lemma: l,
            n: EVALUATOR_N,
        })
    })
}

/// The honest encoder at width `n`, boxed (for [`check_all_widths`]).
pub fn honest_at(n: usize) -> Box<dyn SchedModel> {
    Box::new(Encoder { n })
}
