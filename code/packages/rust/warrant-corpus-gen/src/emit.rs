//! # Assembling a corpus: `corpus.json`, `chunks/*.txt`, `cases.tfv` (VER04r 1.1, 1.4, 5.5, 8)
//!
//! [`generate`] is the whole pipeline:
//!
//! ```text
//!   profile ─┐
//!   spec ────┼─ analyse ─▶ faults (registry, by signature) ─▶ enumerate every suite
//!            │                                                   │ observer, per suite:
//!            │                                                   └─ witness search (faults.rs)
//!            ▼
//!   slim sets += witness chunks ─▶ control rows (witnessed / blind / unproven / deferred)
//!            ▼
//!   corpus.json (canonical)   chunks/<claim>.txt   cases.tfv
//! ```
//!
//! ## Ordering decisions this module makes (VER04r 16.6)
//!
//! * `faults` and `controls` are sorted by `(op, fault)` by bytes, so the order does not
//!   depend on the order of `fault` lines in the profile.
//! * `required_claims`, `blind_claims` and `slim_witness` name claims by **suite id**
//!   (`warrant/<module>/exhaustive/<claim>`, as v1 does), in suite declaration order.
//! * `levels` is the sorted, duplicate-free union of every `requires`.
//! * Nothing is written as `null`: `width` of an example is omitted when there is none.
//!
//! ## Generation failures (VER04r 5.6)
//!
//! A control whose op no claim calls, and a control whose every candidate is **blind**, are
//! errors (`fault`): the corpus could never notice the fault.  The profile can excuse the
//! second, and only the second, with an `accept_blind <op> <fault>` line ([D15], VER04r
//! 16.8): the control row is then emitted (no `required_claims`, every candidate in
//! `blind_claims`, no `slim_witness`) and its **control id** is recorded in
//! `accepted_blind_faults`, sorted by bytes.  An `accept_blind` line for a fault that is not
//! all-blind (it has a witnessed, unproven or deferred claim) is a `profile` error, not a
//! no-op: an acceptance that excuses nothing is a stale profile (fail closed).

use std::collections::BTreeMap;

use coding_adventures_sha256::sha256_hex;

use crate::enumerate::{enumerate_analysed, SuiteView};
use crate::error::{code, GenError};
use crate::faults::{
    first_unimplemented, instantiate_with, search, type_fault, stand_in, ClaimState, Fault, Level, Probe,
    SHIPPED_GATE,
};
use crate::json::Json;
use crate::lower::{Lowered, SuiteKind};
use crate::profile::{parse_profile, Profile};
use crate::tfv::examples_omitting;
use crate::{analyse, CorpusV2, GenInput};

/// The generator's name in `corpus.json`.
pub const GENERATOR_NAME: &str = "warrant-corpus-gen";
/// The generator's version in `corpus.json` (the crate version).
pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Device names Windows reserves with or without an extension: a chunk file of one of them
/// cannot be created there, so the corpus could not be checked out (VER04r 16.6).
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// The Rust bindings' `faults=` lists, as `(op, names)`.
pub type BindingFaults = [(String, Vec<String>)];

/// Instantiate every `fault <op> <name>` line of the profile, sorted by `(op, fault)`, and
/// cross-check them against the bindings' `faults=` lists when given (VER04r 8.4).
///
/// # Errors
///
/// `profile` for an op the document does not declare; `drift` when the lists differ;
/// the errors of [`instantiate`].
pub fn build_faults(
    lowered: &Lowered,
    profile: &Profile,
    bindings: Option<&BindingFaults>,
) -> Result<Vec<Fault>, GenError> {
    for op in &lowered.ops {
        crate::faults::check_reserved(op)?;
    }
    let decls = crate::types::Decls::new(&lowered.types);
    let mut faults = Vec::new();
    for (op_name, name) in &profile.faults {
        let op = lowered.op(op_name).ok_or_else(|| {
            GenError::new(
                code::PROFILE,
                "corpus.profile",
                "a fault line names an operation the document does not declare",
            )
        })?;
        faults.push(instantiate_with(op, name, &lowered.errors, &decls)?);
    }
    faults.sort_by(|a, b| (a.op.as_bytes(), a.name.as_bytes()).cmp(&(b.op.as_bytes(), b.name.as_bytes())));
    if let Some(b) = bindings {
        let mut want: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for op in &lowered.ops {
            want.entry(op.name.as_str()).or_default();
        }
        let mut have = want.clone();
        for f in &faults {
            have.entry(f.op.as_str()).or_default().push(f.name.as_str());
        }
        let mut bound: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (op, names) in b {
            bound
                .entry(op.as_str())
                .or_default()
                .extend(names.iter().map(String::as_str));
        }
        for v in bound.values_mut().chain(have.values_mut()) {
            v.sort_unstable();
            v.dedup();
        }
        if bound != have {
            let ops: Vec<&str> = bound
                .keys()
                .chain(have.keys())
                .copied()
                .filter(|k| bound.get(k) != have.get(k))
                .collect();
            return Err(GenError::new(
                code::DRIFT,
                ops.first().copied().unwrap_or("").to_string(),
                "the fault names of corpus.profile differ from the bindings' faults= lists (VER04r 8.4)",
            ));
        }
    }
    Ok(faults)
}

/// Emit a v2 corpus with the shipped fault-evaluation gate ([`SHIPPED_GATE`]: `ir.core`, `ir.fixed`, `ir.tuple`).
///
/// # Errors
///
/// As [`generate_gated`].
pub fn generate(input: &GenInput<'_>) -> Result<CorpusV2, GenError> {
    generate_gated(input, &SHIPPED_GATE)
}

/// Emit a v2 corpus with `gate` as the **fault-evaluation gate**: the levels whose faults the
/// witness search evaluates.  A fault whose `requires` names a level outside the gate is emitted as
/// data and every candidate claim that has a stand-in is `deferred`, naming the first level of the
/// fault's `requires` (sorted) the gate does not hold (VER04r 16.10 R12); no witness search runs.
///
/// The shipped gate has been `ir.core`, `ir.fixed`, `ir.tuple` since the flip (S10c, [D29]); callers
/// pass `&[Level::Core]` for the closed gate.  This function is the
/// **library parameter** the committed fault dump of S10t-1 is reproduced through
/// (`tests/tuple_parity.rs` passes `ir.core` and `ir.tuple`, `tests/fixed_eval_parity.rs` all three
/// levels: the oracle's `GATE_OPEN`); no command-line or environment switch reaches it.  Since S10c-2
/// the evaluator runs the 35 `ir.fixed` built-ins, so a gate that holds `ir.fixed` searches the
/// `xor-lsb`, `ignore-high-bit` and `flip-result-bit` faults and finds the witnesses the oracle's
/// committed dump records.
///
/// # Errors
///
/// Any [`analyse`] or enumeration refusal; `profile`, `fault` and `drift` for the fault
/// rules above; `W0701` for two suites whose chunk file names are equal ignoring case.
pub fn generate_gated(input: &GenInput<'_>, gate: &[Level]) -> Result<CorpusV2, GenError> {
    let profile = parse_profile(input.profile_bytes)?;
    let analysis = analyse(input.file, &profile)?;
    let faults = build_faults(&analysis.lowered, &profile, input.binding_faults)?;
    // Every fault row is typed once (VER04r 16.17 R67, 16.19 R94, 16.20 R96 to R98), even when no claim can
    // stand in for the subject: an op with a width parameter is typed at the widths of each stand-in instead.
    let decls = crate::types::Decls::new(&analysis.lowered.types);
    for f in &faults {
        if let Some(op) = analysis.lowered.op(&f.op) {
            if op.statics.is_empty() {
                type_fault(f, op, &[], &decls)?;
            }
        }
    }

    // chunk file names: derived from the claim, never from the document's free text
    let mut names: Vec<String> = Vec::new();
    for s in &analysis.lowered.suites {
        let file = format!("{}.txt", s.claim.replace('/', "."));
        let lower = file.to_ascii_lowercase();
        if names.iter().any(|n| n.to_ascii_lowercase() == lower) {
            return Err(GenError::new(
                code::W0701,
                &s.claim,
                "two chunk files have names equal ignoring case (VER04r 5.5)",
            ));
        }
        if file
            .split('.')
            .any(|seg| WINDOWS_RESERVED.contains(&seg.to_ascii_lowercase().as_str()))
        {
            return Err(GenError::new(
                code::W0701,
                &s.claim,
                "a chunk file name is a Windows reserved device name (con, nul, aux, prn, com1-9, lpt1-9)",
            ));
        }
        names.push(file);
    }

    // witness search, suite by suite, while each plan is alive
    let mut states: BTreeMap<(usize, usize), ClaimState> = BTreeMap::new();
    let mut enumeration = {
        let faults = &faults;
        let states = &mut states;
        enumerate_analysed(input.file, &profile, analysis, &mut |view: &SuiteView<'_>| {
            let mut probes: Vec<(usize, Probe<'_>)> = Vec::new();
            for (fi, f) in faults.iter().enumerate() {
                if !view.suite.ops.contains(&f.op) {
                    continue;
                }
                let Some(op) = view.lowered.op(&f.op) else {
                    continue;
                };
                match stand_in(view.suite, f, op) {
                    None => {
                        states.insert((view.index, fi), ClaimState::Unproven);
                    }
                    Some(_) if first_unimplemented(&f.requires, gate).is_some() => {
                        let level = first_unimplemented(&f.requires, gate).unwrap_or(Level::Core);
                        states.insert((view.index, fi), ClaimState::Deferred(level));
                    }
                    Some(_) if !op.statics.is_empty() => {
                        // typed at the static widths of this stand-in's subject call
                        if let SuiteKind::Reference {
                            subject: crate::ir::Ir::Subject { widths, .. },
                            ..
                        } = &view.suite.kind
                        {
                            let statics: Vec<(String, u64)> =
                                op.statics.iter().cloned().zip(widths.iter().copied()).collect();
                            type_fault(f, op, &statics, &decls)?;
                        }
                        let si = stand_in(view.suite, f, op).ok_or_else(|| GenError::internal("a stand-in vanished"))?;
                        probes.push((fi, Probe { fault: f, op, stand_in: si }));
                    }
                    Some(si) => probes.push((
                        fi,
                        Probe {
                            fault: f,
                            op,
                            stand_in: si,
                        },
                    )),
                }
            }
            let (order, only): (Vec<usize>, Vec<Probe<'_>>) = probes.into_iter().unzip();
            let found = search(view.program, view.suite, view.plan, &only)?;
            for (fi, w) in order.into_iter().zip(found) {
                let state = match w {
                    Some(ordinal) => ClaimState::Witnessed(ordinal / view.result.chunk_size),
                    None => ClaimState::Blind,
                };
                states.insert((view.index, fi), state);
            }
            Ok(())
        })?
    };

    // witness chunks join each suite's slim set
    for (si, suite) in enumeration.suites.iter_mut().enumerate() {
        let witnesses: Vec<u64> = states
            .iter()
            .filter(|((s, _), _)| *s == si)
            .filter_map(|(_, st)| match st {
                ClaimState::Witnessed(c) => Some(*c),
                _ => None,
            })
            .collect();
        suite.slim = suite.slim_set(&witnesses)?;
    }

    let lowered = &enumeration.analysis.lowered;
    let ex = examples_omitting(
        lowered,
        &crate::eval::Program::new(&lowered.models).with_decls(crate::types::Decls::new(&lowered.types)),
        &profile.omit_examples,
    )?;

    // chunk files
    let mut chunk_files: Vec<(String, String)> = Vec::new();
    let mut suite_rows: Vec<Json> = Vec::new();
    for (si, (s, ls)) in enumeration.suites.iter().zip(&lowered.suites).enumerate() {
        let mut text = s.chunk_lines().join("\n");
        text.push('\n');
        let chunks_sha = sha256_hex(text.as_bytes());
        chunk_files.push((names[si].clone(), text));

        let mut row = vec![
            ("id", Json::str(s.id.as_str())),
            ("kind", Json::str("exhaustive")),
            ("claim", Json::str(s.claim.as_str())),
            ("ops", Json::strs(ls.ops.iter())),
            ("requires", Json::strs(ls.requires.iter())),
            (
                "binders",
                Json::arr(
                    s.binder_names
                        .iter()
                        .zip(&s.binder_types)
                        .zip(&s.descriptors)
                        .map(|((n, t), d)| {
                            Json::arr([
                                Json::str(n.as_str()),
                                Json::obj([("type", Json::str(t.as_str())), ("domain", d.clone())]),
                            ])
                        }),
                ),
            ),
            ("demoted", Json::strs(s.demoted.iter())),
            ("domain_label", Json::str(s.label.name())),
            ("admit", ls.admit.to_json()),
            ("body", ls.body.to_json()),
            ("result_type", Json::str(s.result_type.as_str())),
            ("stream", Json::str(s.header.name())),
            ("product", Json::Int(s.product)),
            ("cases", Json::Int(s.cases)),
            ("discarded", Json::Int(s.discarded)),
            ("expected_calls", Json::Int(s.expected_calls)),
            ("wraps", Json::Int(s.wraps)),
            ("ir_cost", Json::Int(s.ir_cost)),
            ("chunks", Json::Int(s.chunks_total)),
            ("chunks_sha256", Json::str(chunks_sha)),
            ("inputs_sha256", Json::str(s.inputs_sha256.as_str())),
            (
                "slim_chunks",
                Json::arr(s.slim.chunks.iter().map(|c| Json::Int(*c))),
            ),
            ("slim_inputs_sha256", Json::str(s.slim.inputs_sha256.as_str())),
        ];
        let oracle = match &ls.kind {
            SuiteKind::Reference {
                subject, expected, ..
            } => Json::obj([
                ("kind", Json::str("reference")),
                ("subject", subject.to_json()),
                ("expected", expected.to_json()),
            ]),
            SuiteKind::Predicate => Json::obj([("kind", Json::str("predicate"))]),
        };
        row.push(("oracle", oracle));
        let opt = |row: &mut Vec<(&str, Json)>, k: &'static str, v: &Option<String>| {
            if let Some(v) = v {
                row.push((k, Json::str(v.as_str())));
            }
        };
        opt(&mut row, "expected_sha256", &s.expected_sha256);
        opt(&mut row, "v1_inputs_sha256", &s.v1_inputs_sha256);
        opt(&mut row, "v1_expected_sha256", &s.v1_expected_sha256);
        opt(&mut row, "slim_expected_sha256", &s.slim.expected_sha256);
        suite_rows.push(Json::obj(row));
    }

    // controls
    let mut control_rows = Vec::new();
    let mut accepted_blind: Vec<String> = Vec::new();
    // `levels`: the union of every `requires` of a suite, op, fault and control row (and core).
    let mut levels: Vec<&'static str> = vec![Level::Core.name()];
    let mut add_levels = |names: &mut dyn Iterator<Item = &'static str>| {
        for n in names {
            if !levels.contains(&n) {
                levels.push(n);
            }
        }
    };
    for ls in &lowered.suites {
        add_levels(&mut ls.requires.iter().copied());
    }
    for op in &lowered.ops {
        add_levels(&mut op.requires.iter().copied());
    }
    for (fi, f) in faults.iter().enumerate() {
        // the fault row and its control row carry the same list
        add_levels(&mut f.requires.iter().map(|l| l.name()));
        let mut required = Vec::new();
        let mut blind = Vec::new();
        let mut witness = Vec::new();
        let mut candidates = 0usize;
        for (si, ls) in lowered.suites.iter().enumerate() {
            let Some(state) = states.get(&(si, fi)) else {
                continue;
            };
            candidates += 1;
            let id = Json::str(ls.id.as_str());
            let entry = |extra: (&str, Json)| {
                Json::obj([("claim", Json::str(ls.id.as_str())), extra])
            };
            match state {
                ClaimState::Blind => blind.push(id),
                ClaimState::Witnessed(c) => {
                    required.push(id);
                    witness.push(entry(("chunk", Json::Int(*c))));
                }
                ClaimState::Unproven => {
                    required.push(id);
                    witness.push(entry(("unproven", Json::Bool(true))));
                }
                ClaimState::Deferred(l) => {
                    required.push(id);
                    witness.push(entry(("deferred", Json::str(l.name()))));
                }
            }
        }
        let at = format!("{}/{}", f.op, f.name);
        if candidates == 0 {
            return Err(GenError::new(
                code::FAULT,
                at,
                "no claim calls the faulted operation, so no claim could catch the fault",
            ));
        }
        let control_id = format!(
            "warrant/{}/negctl/exhaustive/{}/{}",
            lowered.module, f.op, f.name
        );
        let accepted = profile
            .accept_blind
            .iter()
            .any(|(o, n)| *o == f.op && *n == f.name);
        if required.is_empty() {
            if !accepted {
                return Err(GenError::new(
                    code::FAULT,
                    at,
                    "every candidate claim is blind to this fault: the corpus could never notice it (VER04r 5.6); an accept_blind line would excuse it (VER04r 16.8)",
                ));
            }
            accepted_blind.push(control_id.clone());
        } else if accepted {
            return Err(GenError::new(
                code::PROFILE,
                at,
                "an accept_blind line names a fault that is not all-blind: a claim can see it (VER04r 16.8)",
            ));
        }
        control_rows.push(Json::obj([
            ("id", Json::str(control_id)),
            ("op", Json::str(f.op.as_str())),
            ("fault", Json::str(f.name.as_str())),
            ("target_kind", Json::str("exhaustive")),
            ("requires", Json::strs(f.requires.iter().map(|l| l.name()))),
            ("required_claims", Json::Arr(required)),
            ("blind_claims", Json::Arr(blind)),
            ("slim_witness", Json::Arr(witness)),
        ]));
    }
    levels.sort_unstable();
    accepted_blind.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    // error types, then the records and variants of `ir.data` (VER04r 2.2; the oracle's `_types`)
    let types = Json::Obj(
        lowered
            .errors
            .iter()
            .map(|e| {
                (
                    e.name.clone(),
                    Json::obj([
                        ("kind", Json::str("error")),
                        ("cases", Json::strs(e.cases.iter())),
                    ]),
                )
            })
            .chain(
                lowered
                    .types
                    .iter()
                    .map(|t| (t.name().to_string(), crate::view::type_row(t))),
            )
            .collect(),
    );
    let mut corpus = Json::obj([
        ("profile", Json::str("warrant-corpus-v2")),
        ("schema_version", Json::Int(2)),
        ("module", Json::str(lowered.module.as_str())),
        ("document", Json::str(lowered.document())),
        ("spec", Json::str(input.spec_path)),
        ("spec_sha256", Json::str(sha256_hex(input.spec_bytes))),
        ("profile_sha256", Json::str(profile.sha256.as_str())),
        (
            "generator",
            Json::obj([
                ("name", Json::str(GENERATOR_NAME)),
                ("version", Json::str(GENERATOR_VERSION)),
            ]),
        ),
        ("budget", Json::Int(profile.budget)),
        ("chunk_size", Json::Int(profile.chunk_size)),
        ("max_ir_cost", Json::Int(profile.max_ir_cost)),
        ("levels", Json::strs(levels)),
        ("types", types),
        (
            "ops",
            Json::Obj(
                lowered
                    .ops
                    .iter()
                    .map(|o| (o.name.clone(), o.to_json()))
                    .collect(),
            ),
        ),
        (
            "models",
            Json::Obj(
                lowered
                    .models
                    .iter()
                    .map(|m| {
                        (
                            m.name.clone(),
                            Json::obj([
                                (
                                    "params",
                                    Json::arr(m.params.iter().map(|(n, t)| {
                                        Json::arr([Json::str(n.as_str()), Json::str(t.name())])
                                    })),
                                ),
                                ("body", m.body.to_json()),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "model_order",
            Json::strs(lowered.models.iter().map(|m| m.name.as_str())),
        ),
        ("fault_models", Json::Obj(Vec::new())),
        ("suites", Json::Arr(suite_rows)),
        ("examples", Json::Arr(ex.rows.clone())),
        ("faults", Json::arr(faults.iter().map(Fault::to_json))),
        ("controls", Json::Arr(control_rows)),
        ("accepted_blind_faults", Json::strs(accepted_blind.iter())),
        ("assumes", Json::Arr(Vec::new())),
    ]);
    // A conditional key (VER04r 1.4, 16.10 R4): written only when an example is omitted, so
    // no committed fixture moves a byte.  `[]` is never written.
    if !ex.omitted.is_empty() {
        let Json::Obj(members) = &mut corpus else {
            return Err(GenError::internal("the corpus is an object"));
        };
        members.push((
            "omitted_examples".to_string(),
            Json::arr(ex.omitted.iter().map(|(id, why)| {
                Json::obj([("id", Json::str(id.as_str())), ("reason", Json::str(*why))])
            })),
        ));
    }
    Ok(CorpusV2 {
        corpus_json: corpus.render()?,
        chunk_files,
        cases_tfv: ex.cases_tfv(),
    })
}
