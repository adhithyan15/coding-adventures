//! # `gen-contract` and `check-contract`: the generic contract check (VOS-039b-1, VOS04 5.6)
//!
//! A contract package ships a model of what an interface MEANS (`warrant/contract.part`) and
//! lemmas about that model (`warrant/lemmas.part`).  Until VOS-039b-1 a hand-written sibling
//! crate, `vos-sched-contract-check`, evaluated the scheduler's nine lemmas and a mutant for
//! each; every new contract would have needed its own copy.  This module is the same check,
//! generic, and it follows the principle of `check-doc` and `check-component`: **generate the
//! canonical artifact and compare it byte for byte; never lint author text.**
//!
//! ```text
//!   vos.toml, contract.vos ───────────┐
//!   warrant/contract.part ────────────┤   cfg_<KEY>() models, generated from `config` rows
//!   warrant/lemmas.part ──────────────┼─► one evaluation document, in memory ─► every lemma,
//!   warrant/mutants/<lemma>.part ─────┘   over EVERY value of its binders (Bool, U8)
//!                                                │
//!        base run ───► every lemma must hold (CK03, CK04)
//!        mutant run ─► every lemma must FAIL on its mutant (CK05, CK06, CK07)
//!        config run ─► every config key is used, and V-1 / V+1 each break a lemma (CK08)
//!                                                │
//!                                                ▼
//!                            warrant/lemmas.table  (generated, never parsed)
//!                            gen-contract writes it, check-contract compares (CK09)
//!
//!   src/**/*.rs enums ──┐   one `variant ...` line per Rust enum, one per Warrant variant,
//!   contract.part ──────┴─► rendered by one function each and compared byte for byte (CK10);
//!                           the lines are rows of the table (see `enums`)
//! ```
//!
//! ## A mutant is an ordinary part, not a new file format
//!
//! `warrant/mutants/<lemma>.part` is admitted like every other part and holds **exactly one**
//! `model` declaration.  The Warrant parser (via `vos_manifest::warrant::declarations_of_block`)
//! finds the model of the same name in `contract.part` and its **span**; the mutant replaces
//! that span.  No text search is involved, so a mutant cannot silently replace the wrong place.
//!
//! The mutant is not gameable:
//!
//! * the file stems equal the lemma names, in both directions (CK05);
//! * the mutated document must type-check and the lemma must **evaluate without error** on it
//!   (CK06): an evaluation error is a finding, never a kill;
//! * the lemma must fail at least one case (CK07);
//! * the number of failing cases is written to the table, so a reviewer sees it move.
//!
//! ## Configuration is generated, and mutated by the tool
//!
//! For each `config KEY V` row of `contract.vos` the tool writes `model cfg_KEY() -> T = V<t>`
//! (`T` the smallest of U8, U16, U32, U64 that holds `V`) in front of the contract block of the
//! evaluation document.  A lemma that uses it (`t < cfg_N()`) ties the Warrant text to the
//! number `contract.vos` states.  Two rules keep that honest (CK08): every key must be **called
//! by at least one lemma** (the claim bodies are walked as parsed expressions), and the
//! tool-generated mutants `V-1` and `V+1` must each falsify a lemma that calls the key.  An
//! author cannot write those mutants, so a lemma that ignores its configuration cannot pass.
//!
//! ## Cost and determinism
//!
//! Each lemma is `product(binder domains)` evaluations (at most [`LEMMA_BUDGET`]).  Jobs run on
//! at most [`MAX_WORKERS`] threads, each on the large stack the Warrant checker needs, and the
//! results are collected **by job index**, so the report and the table are the same on every
//! run whatever the scheduler does.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, TypeExpr, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use vos_manifest::digest::sha256_hex;
use vos_manifest::document::document_name;
use vos_manifest::warrant::{claim_calls, claims_of_block, declarations_of_block, DeclKind};
use vos_manifest::{Contract, Part};
use vos_shape_check::manifest::{parse_vos_toml, SatLane, VOS_TOML_MAX_BYTES};
use vos_shape_check::SourceFile;

use crate::check::{first_difference_line, WARRANT_STACK_BYTES};
use crate::component::read_text;
use crate::enums;
use crate::error::{Finding, NoVerdict};
use crate::fsio::{ensure_plain_dirs, esc, list_names, read_regular_opt, write_replacing};
use crate::inputs::{admit, load_contract_package, resolve_layout, Layout};
use crate::ops::Outcome;

/// The X1 case budget per claim (VOS04 5.6, VER04h 3): a lemma whose binder domains multiply to
/// more is refused (CK03), never run.  A test reads VOS04 and asserts the spec states it.
pub const LEMMA_BUDGET: u64 = 400_000;
/// At most this many lemmas in `lemmas.part` (CK03), the same number as the entries of the
/// mutants directory (`fsio::MAX_DIR_ENTRIES`).  Every lemma costs up to [`LEMMA_BUDGET`]
/// evaluations plus one mutant run, so an unbounded lemma list would be an unbounded amount of
/// work for an author-controlled file.  A test reads VOS04 5.6 and asserts it states the number.
pub const MAX_LEMMAS: usize = 64;
/// At most this many evaluation threads.
pub const MAX_WORKERS: usize = 8;
/// The generated table, relative to the contract crate directory.
pub const TABLE_FILE: &str = "warrant/lemmas.table";
/// The directory of mutant parts, relative to the contract crate directory.
pub const MUTANTS_DIR: &str = "warrant/mutants";

const TABLE_MAX_BYTES: usize = 1024 * 1024;
/// The committed model-SAT table (VOS31 section 8), relative to the contract crate directory.
pub const SAT_TABLE_FILE: &str = "warrant/lemmas.sat";
const BUILD_MAX_BYTES: usize = 256 * 1024;

// ---------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------

/// Everything `check-contract` reads.
#[derive(Debug)]
pub struct ContractInputs {
    /// Where the crate is.
    pub layout: Layout,
    /// `contract.vos`.
    pub contract: Contract,
    /// `warrant/contract.part`.
    pub contract_part: Part,
    /// `warrant/lemmas.part`.
    pub lemmas_part: Part,
    /// `warrant/mutants/<stem>.part`, sorted by stem.
    pub mutants: Vec<(String, Part)>,
    /// `warrant/lemmas.table` as on disk, if present.
    pub table: Option<String>,
    /// Every `*.rs` file under `src/` (the Rust side of CK10), empty when there is no `src/`.
    pub sources: Vec<SourceFile>,
    /// The `vos.toml` key `sat` (CK11); `None` when absent.  A `vos.toml` that does not parse is
    /// already a `GD05`/`CK01` finding and never reaches here.
    pub sat: Option<SatLane>,
    /// `BUILD` and `BUILD_windows` as on disk (`None` when absent), for CK11.
    pub builds: Vec<(&'static str, Option<String>)>,
    /// `warrant/lemmas.sat` as on disk, if present (CK11).
    pub sat_table: Option<String>,
}

/// GD01, GD02 and GD05 are the inputs of a document; for `check-contract` they are `CK01`
/// (VOS04 5.6).  GD03 and GD04 (the tier files, the obligation names) keep their code: the
/// check is the very one `gen-doc` runs.
fn as_ck01(f: Finding) -> Finding {
    match f.code {
        "GD01" | "GD02" | "GD05" => Finding::new("CK01", f.message),
        _ => f,
    }
}

/// Load and judge the inputs of the contract crate at `crate_arg`.  `Ok(Err(findings))` means
/// the inputs are wrong; a path or size problem is a [`NoVerdict`].
pub fn load_inputs(
    crate_arg: &Path,
    root_arg: Option<&Path>,
) -> Result<Result<ContractInputs, Vec<Finding>>, NoVerdict> {
    let layout = resolve_layout(crate_arg, root_arg)?;
    ensure_plain_dirs(&layout.root, &["code", "packages", "rust"])?;
    // `absolute_plain` refused a symlink in the path the user typed, but the crate directory
    // and the directories under it are read and written below, so each is checked again here,
    // at the point of use: a symlinked `warrant/` would carry the one write of `gen-contract`
    // outside the crate.  Fail closed (CK01), before anything is read.
    let mut f: Vec<Finding> = Vec::new();
    for (rel, shown) in [
        ("", "the crate directory"),
        ("warrant", "warrant"),
        (MUTANTS_DIR, MUTANTS_DIR),
    ] {
        let dir = layout.impl_dir.join(rel);
        if let Ok(md) = std::fs::symlink_metadata(&dir) {
            if md.file_type().is_symlink() {
                f.push(Finding::new(
                    "CK01",
                    format!(
                        "{shown}: is a symlink, and a contract crate is read and written in place"
                    ),
                ));
            }
        }
    }
    if !f.is_empty() {
        return Ok(Err(f));
    }
    // The shared half: vos.toml kind, contract.vos, the tier files, the admission of every part.
    let pkg = load_contract_package(&layout, &layout.impl_pkg.clone(), &mut f)?;
    let Some(pkg) = pkg else {
        return Ok(Err(f.into_iter().map(as_ck01).collect()));
    };
    if pkg.lemmas_part.is_none() && f.iter().all(|x| !x.message.contains("lemmas.part")) {
        f.push(Finding::new(
            "CK01",
            "warrant/lemmas.part: not found (a contract crate requires one, VOS04 5.6)",
        ));
    }

    // The mutants: ordinary admitted parts, one per lemma.
    let dir = layout.impl_dir.join(MUTANTS_DIR);
    let mut mutants: Vec<(String, Part)> = Vec::new();
    for name in list_names(&dir)? {
        let Some(stem) = name.strip_suffix(".part") else {
            f.push(Finding::new(
                "CK05",
                format!("{MUTANTS_DIR}/{}: not a `<lemma>.part` file", esc(&name)),
            ));
            continue;
        };
        let Some(bytes) = read_regular_opt(&dir.join(&name), vos_manifest::caps::PART_MAX_BYTES)?
        else {
            continue; // vanished between the listing and the read: nothing to judge
        };
        if let Some(part) = admit(&bytes, &format!("{MUTANTS_DIR}/{name}"), &mut f)? {
            mutants.push((stem.to_string(), part));
        }
    }
    mutants.sort_by(|a, b| a.0.cmp(&b.0));

    let table = read_text(&layout.impl_dir.join(TABLE_FILE), TABLE_MAX_BYTES)?;
    // The Rust enums (CK10): the shape checker's own hardened reader, so a symlink, a FIFO, a
    // case-trap file name or an oversized source is refused here exactly as it is there.
    let sources = vos_shape_check::loader::load_sources(&layout.impl_dir).map_err(|e| {
        NoVerdict::new(format!(
            "cannot read the sources of the contract crate: {e}"
        ))
    })?;

    // CK11's inputs.  The `sat` key is read leniently: a `vos.toml` that fails to parse was
    // already reported above (GD05), so here an error just means "no key".
    let sat = match read_regular_opt(&layout.impl_dir.join("vos.toml"), VOS_TOML_MAX_BYTES)? {
        Some(bytes) => String::from_utf8(bytes)
            .ok()
            .and_then(|t| parse_vos_toml(&t).ok())
            .and_then(|v| v.sat),
        None => None,
    };
    let mut builds = Vec::new();
    for name in ["BUILD", "BUILD_windows"] {
        builds.push((
            name,
            read_text(&layout.impl_dir.join(name), BUILD_MAX_BYTES)?,
        ));
    }
    let sat_table = read_text(&layout.impl_dir.join(SAT_TABLE_FILE), TABLE_MAX_BYTES)?;

    let findings: Vec<Finding> = f.into_iter().map(as_ck01).collect();
    match (findings.is_empty(), pkg.contract_part, pkg.lemmas_part) {
        (true, Some(contract_part), Some(lemmas_part)) => Ok(Ok(ContractInputs {
            layout,
            contract: pkg.contract,
            contract_part,
            lemmas_part,
            mutants,
            table,
            sources,
            sat,
            builds,
            sat_table,
        })),
        _ => Ok(Err(findings)),
    }
}

// ---------------------------------------------------------------------------------------
// The evaluation document
// ---------------------------------------------------------------------------------------

/// The Warrant type, literal suffix and largest value of the smallest unsigned type holding `v`.
fn config_type(v: u64) -> (&'static str, &'static str, u64) {
    if v <= u64::from(u8::MAX) {
        ("U8", "u8", u64::from(u8::MAX))
    } else if v <= u64::from(u16::MAX) {
        ("U16", "u16", u64::from(u16::MAX))
    } else if v <= u64::from(u32::MAX) {
        ("U32", "u32", u64::from(u32::MAX))
    } else {
        ("U64", "u64", u64::MAX)
    }
}

/// The generated models of the `config` rows, one line each, in key order.  `overrides` replaces
/// the value of one key (a config mutant); the TYPE stays that of the real value, so the mutated
/// document type-checks wherever the real one does.
///
/// ```text
///   config N 8     ->     "  model cfg_N() -> U8 = 8u8\n"
/// ```
pub fn config_preamble(contract: &Contract, overrides: Option<(&str, u64)>) -> String {
    let mut s = String::new();
    for c in &contract.config {
        let (ty, suffix, _) = config_type(c.value);
        let v = match overrides {
            Some((key, v)) if key == c.key => v,
            _ => c.value,
        };
        s.push_str(&format!("  model cfg_{}() -> {ty} = {v}{suffix}\n", c.key));
    }
    s
}

/// Assemble the evaluation document: header, the generated config models, the contract block,
/// the lemmas.  A part is admitted, so it is LF-terminated ASCII and the pieces join cleanly.
pub fn evaluation_document(
    contract: &Contract,
    contract_text: &str,
    lemmas_text: &str,
    overrides: Option<(&str, u64)>,
) -> String {
    let name = document_name(&contract.package).unwrap_or_else(|_| "vos_contract".to_string());
    format!(
        "warrant {name} v1\n{}{contract_text}{lemmas_text}",
        config_preamble(contract, overrides)
    )
}

/// `contract_text` with the byte range `start..end` replaced by `with`.
pub fn splice(contract_text: &str, start: usize, end: usize, with: &str) -> String {
    format!("{}{with}{}", &contract_text[..start], &contract_text[end..])
}

// ---------------------------------------------------------------------------------------
// The evaluator: every binder over its whole domain
// ---------------------------------------------------------------------------------------

/// What one lemma did over its binders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LemmaRun {
    /// The product of the binder domains, before refinement guards (the number the budget
    /// bounds).
    pub product: u64,
    /// Cases whose guards held and whose body was evaluated.
    pub admitted: u64,
    /// Admitted cases whose body was not `true`.
    pub failed: u64,
    /// The binder values of the first failing case, for the finding.
    pub first_failure: Option<String>,
}

/// What a job produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Run {
    /// The lemma ran over its whole domain.
    Done(LemmaRun),
    /// The lemma is not one this tool can enumerate (CK03): the reason.
    Refused(String),
    /// The document did not parse, type-check or evaluate: the reason.
    Broke(String),
}

/// The values of a complete binder type: `Bool` and `U8` (VOS01 section 5.6: every binder of
/// a contract lemma is complete).  Anything else is refused, not approximated.
fn domain(ty: &TypeExpr) -> Option<Vec<EvalValue>> {
    match ty {
        TypeExpr::Named { name: "Bool", .. } => {
            Some(vec![EvalValue::Bool(false), EvalValue::Bool(true)])
        }
        TypeExpr::Named { name: "U8", .. } => Some((0..=255u8).map(EvalValue::U8).collect()),
        _ => None,
    }
}

/// Parse `src`, then run `f` over the file (the AST borrows the source).
pub(crate) fn with_file<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> Result<R, String> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default())
        .map_err(|e| format!("{e}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default()).map_err(|e| format!("{e}"))?;
    let file =
        parse_document(&layout, &doc, ParserLimits::default()).map_err(|e| format!("{e}"))?;
    Ok(f(&file))
}

/// Parse and type-check `src`; `Ok(())` or the reason.
fn type_check(src: &str) -> Result<(), String> {
    with_file(src, |file| {
        check_warrant(file)
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    })?
}

/// Evaluate lemma `lemma` of `file` over every value of its binders.
fn evaluate_lemma(file: &WarrantFile, lemma: &str) -> Run {
    if let Err(e) = check_warrant(file) {
        return Run::Broke(format!("type check failed: {e:?}"));
    }
    let Some(claim) = file.declarations.iter().find_map(|d| match d {
        Declaration::Claim(c) if c.name == lemma => Some(c),
        _ => None,
    }) else {
        return Run::Broke(format!("no claim named {lemma}"));
    };
    let Expression::Quantified {
        is_forall: true,
        binders,
        body,
        ..
    } = &claim.body
    else {
        return Run::Refused("is not a `forall` claim".to_string());
    };
    let mut domains: Vec<Vec<EvalValue>> = Vec::new();
    for b in binders {
        match domain(&b.param_type) {
            Some(d) => domains.push(d),
            None => {
                return Run::Refused(format!(
                    "binder `{}` has no complete domain here (only Bool and U8 are enumerated)",
                    b.name
                ))
            }
        }
    }
    let product = domains
        .iter()
        .fold(1u64, |p, d| p.saturating_mul(d.len() as u64));
    if product > LEMMA_BUDGET {
        return Run::Refused(format!(
            "binder domains multiply to {product}, over the budget of {LEMMA_BUDGET}"
        ));
    }

    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let (mut admitted, mut failed) = (0u64, 0u64);
    let mut first_failure = None;
    let mut index = vec![0usize; domains.len()];
    'cases: loop {
        let mut env: HashMap<&str, EvalValue> = HashMap::new();
        for (k, binder) in binders.iter().enumerate() {
            env.insert(binder.name, domains[k][index[k]].clone());
        }
        // A refinement guard decides whether the case is admitted.
        evaluator.reset_work();
        let mut guarded = true;
        for b in binders {
            if let Some(guard) = &b.refinement {
                match evaluator.eval(guard, &env) {
                    Ok(EvalValue::Bool(true)) => {}
                    Ok(_) => guarded = false,
                    Err(e) => return Run::Broke(format!("a guard did not evaluate: {e}")),
                }
            }
        }
        if guarded {
            admitted += 1;
            evaluator.reset_work();
            match evaluator.eval(body, &env) {
                Ok(EvalValue::Bool(true)) => {}
                Ok(_) => {
                    failed += 1;
                    if first_failure.is_none() {
                        let shown: Vec<String> = binders
                            .iter()
                            .map(|b| format!("{}={:?}", b.name, env[b.name]))
                            .collect();
                        first_failure = Some(shown.join(", "));
                    }
                }
                Err(e) => return Run::Broke(format!("did not evaluate: {e}")),
            }
        }
        // The odometer: the last binder is the fastest digit.
        let mut pos = domains.len();
        loop {
            if pos == 0 {
                break 'cases;
            }
            pos -= 1;
            index[pos] += 1;
            if index[pos] < domains[pos].len() {
                break;
            }
            index[pos] = 0;
        }
    }
    Run::Done(LemmaRun {
        product,
        admitted,
        failed,
        first_failure,
    })
}

/// One evaluation: lemma `lemma` over document `src`.
pub fn run_lemma(src: &str, lemma: &str) -> Run {
    match with_file(src, |file| evaluate_lemma(file, lemma)) {
        Ok(r) => r,
        Err(e) => Run::Broke(format!("the document does not parse: {e}")),
    }
}

/// Run `(document index, lemma)` jobs on at most [`MAX_WORKERS`] threads and return the results
/// **in job order**.  Each worker has the stack the Warrant checker needs.
fn run_jobs(docs: &[String], jobs: &[(usize, String)]) -> Result<Vec<Run>, NoVerdict> {
    let results: Mutex<Vec<Option<Run>>> = Mutex::new(vec![None; jobs.len()]);
    let next = AtomicUsize::new(0);
    let workers = jobs.len().clamp(1, MAX_WORKERS);
    let outcome: Result<(), NoVerdict> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..workers {
            let h = std::thread::Builder::new()
                .name("vos-link-lemmas".to_string())
                .stack_size(WARRANT_STACK_BYTES)
                .spawn_scoped(scope, || loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some((doc, lemma)) = jobs.get(i) else {
                        break;
                    };
                    let run = run_lemma(&docs[*doc], lemma);
                    if let Ok(mut r) = results.lock() {
                        r[i] = Some(run);
                    }
                })
                .map_err(|e| NoVerdict::new(format!("cannot start an evaluation thread: {e}")))?;
            handles.push(h);
        }
        for h in handles {
            h.join()
                .map_err(|_| NoVerdict::new("an evaluation thread panicked"))?;
        }
        Ok(())
    });
    outcome?;
    let done = results
        .into_inner()
        .map_err(|_| NoVerdict::new("an evaluation thread panicked"))?;
    done.into_iter()
        .map(|r| r.ok_or_else(|| NoVerdict::new("an evaluation job produced no result")))
        .collect()
}

/// Run `f` on a thread with the stack the Warrant tools need and return what it returns.
fn on_big_stack<R: Send>(f: impl FnOnce() -> R + Send) -> Result<R, NoVerdict> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("vos-link-warrant".to_string())
            .stack_size(WARRANT_STACK_BYTES)
            .spawn_scoped(scope, f)
            .map_err(|e| NoVerdict::new(format!("cannot start the Warrant thread: {e}")))?
            .join()
            .map_err(|_| NoVerdict::new("the Warrant thread panicked"))
    })
}

// ---------------------------------------------------------------------------------------
// The check
// ---------------------------------------------------------------------------------------

/// What the check found, and the table it would write if it found nothing.
#[derive(Debug, Default)]
pub struct Checked {
    /// `CK02` to `CK08` and `CK10`, in order (`CK09` and `CK11` are the caller's: the table comparison and the model-SAT files).
    pub findings: Vec<Finding>,
    /// The generated `warrant/lemmas.table`, present only when `findings` is empty.
    pub table: Option<String>,
}

impl Checked {
    fn find(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding::new(code, msg));
    }
}

/// One lemma's row of the table, before it is printed.
struct Row {
    name: String,
    run: LemmaRun,
    mutant_model: String,
    mutant_sha: String,
    mutant_failed: u64,
}

/// One config mutant's row.
struct ConfigRow {
    key: String,
    value: u64,
    kills: Vec<String>,
}

/// A mutant that was built: the model it replaces, the mutated contract text, its digest.
struct Mutated {
    model: String,
    text: String,
    sha: String,
}

/// Run every check of `check-contract` except the comparison with the table on disk.
pub fn check_inputs(inp: &ContractInputs) -> Result<Checked, NoVerdict> {
    let mut out = Checked::default();
    let contract_text = inp.contract_part.text();
    let lemmas_text = inp.lemmas_part.text();

    // --- CK02: the lemmas part is claims only, at least one -------------------------------
    let lemma_names: Vec<String> = match claims_of_block(lemmas_text) {
        Ok(claims) => claims.into_iter().map(|c| c.name).collect(),
        Err(e) => {
            out.find("CK02", format!("warrant/lemmas.part: {e}"));
            return Ok(out);
        }
    };
    // (A block with no declaration does not parse at all, so an empty lemma list cannot reach
    // here: `claims_of_block` has already said CK02.)

    // --- CK03: a bounded number of lemmas, so the work is bounded -------------------------
    if lemma_names.len() > MAX_LEMMAS {
        out.find(
            "CK03",
            format!(
                "warrant/lemmas.part holds {} lemmas, over the limit of {MAX_LEMMAS}",
                lemma_names.len()
            ),
        );
        return Ok(out);
    }

    // --- CK02: the evaluation document type-checks ----------------------------------------
    let base = evaluation_document(&inp.contract, contract_text, lemmas_text, None);
    if let Err(e) = on_big_stack(|| type_check(&base))? {
        out.find(
            "CK02",
            format!(
                "contract.part, the generated `cfg_` models and lemmas.part do not type-check: {}",
                esc(&e)
            ),
        );
        return Ok(out);
    }

    // --- CK10: the Rust enums and the Warrant variants, generated and compared ---------------
    let (enum_findings, variant_lines) = check_enums(inp, &base)?;
    out.findings.extend(enum_findings);

    // --- CK05: the mutant set equals the lemma set, both ways ------------------------------
    let have: BTreeSet<&str> = inp.mutants.iter().map(|(s, _)| s.as_str()).collect();
    let want: BTreeSet<&str> = lemma_names.iter().map(String::as_str).collect();
    for l in want.difference(&have) {
        out.find(
            "CK05",
            format!("lemma `{l}` has no mutant: {MUTANTS_DIR}/{l}.part is missing"),
        );
    }
    for m in have.difference(&want) {
        out.find(
            "CK05",
            // The stem is an author-controlled file name: escaped, like every name printed.
            format!(
                "{MUTANTS_DIR}/{}.part names no lemma of lemmas.part",
                esc(m)
            ),
        );
    }

    // --- CK06: each mutant is exactly one model that replaces one of the contract's -------
    let models = match on_big_stack(|| declarations_of_block(contract_text))? {
        Ok(d) => d,
        Err(e) => {
            out.find("CK02", format!("warrant/contract.part: {e}"));
            return Ok(out);
        }
    };
    let mut mutated: Vec<Option<Mutated>> = Vec::new();
    for lemma in &lemma_names {
        mutated.push(build_mutant(&mut out, inp, contract_text, &models, lemma)?);
    }

    // --- config: which lemmas call each key ---------------------------------------------------
    let keys: Vec<String> = inp
        .contract
        .config
        .iter()
        .map(|c| format!("cfg_{}", c.key))
        .collect();
    let calls = match on_big_stack(|| claim_calls(&base, &keys))? {
        Ok(c) => c,
        Err(e) => {
            out.find("CK02", format!("lemmas.part: {e}"));
            return Ok(out);
        }
    };
    // (config row index, mutated value) for each tool-generated mutant that can exist.
    let mut config_mutants: Vec<(usize, u64)> = Vec::new();
    let mut callers_of: Vec<Vec<String>> = Vec::new();
    for (ci, c) in inp.contract.config.iter().enumerate() {
        let model = format!("cfg_{}", c.key);
        let callers: Vec<String> = calls
            .iter()
            .filter(|(_, called)| called.contains(&model))
            .map(|(name, _)| name.clone())
            .collect();
        if callers.is_empty() {
            out.find(
                "CK08",
                format!(
                    "config {} is called by no lemma: nothing ties `{model}` (contract.vos) to the text, so a wrong value passes (VOS04 5.6)",
                    c.key
                ),
            );
        } else {
            let (_, _, max) = config_type(c.value);
            if c.value > 0 {
                config_mutants.push((ci, c.value - 1));
            }
            if c.value < max {
                config_mutants.push((ci, c.value + 1));
            }
        }
        callers_of.push(callers);
    }

    // --- no evaluation of a contract that is already refused --------------------------------
    // A CK02, CK05 or CK06 finding means the documents are not the ones the lemmas were written
    // for; running every lemma on them would spend up to `MAX_LEMMAS` x `LEMMA_BUDGET`
    // evaluations to add findings nobody can act on first.  Return what is known.
    if out
        .findings
        .iter()
        .any(|x| matches!(x.code, "CK02" | "CK05" | "CK06"))
    {
        return Ok(out);
    }

    // --- the jobs, run together -------------------------------------------------------------
    // docs[0] is the real document; every other document is one mutation of it.
    let mut docs: Vec<String> = vec![base];
    let mut jobs: Vec<(usize, String)> = Vec::new();
    for l in &lemma_names {
        jobs.push((0, l.clone()));
    }
    let mut mutant_job_of: Vec<Option<usize>> = vec![None; lemma_names.len()];
    for (i, l) in lemma_names.iter().enumerate() {
        if let Some(m) = &mutated[i] {
            docs.push(evaluation_document(
                &inp.contract,
                &m.text,
                lemmas_text,
                None,
            ));
            mutant_job_of[i] = Some(jobs.len());
            jobs.push((docs.len() - 1, l.clone()));
        }
    }
    let config_job_start = jobs.len();
    for (ci, v) in &config_mutants {
        let key = &inp.contract.config[*ci].key;
        docs.push(evaluation_document(
            &inp.contract,
            contract_text,
            lemmas_text,
            Some((key, *v)),
        ));
        let d = docs.len() - 1;
        for l in &callers_of[*ci] {
            jobs.push((d, l.clone()));
        }
    }
    let results = run_jobs(&docs, &jobs)?;

    // --- CK03 / CK04: the base run ----------------------------------------------------------
    let mut rows: Vec<Row> = Vec::new();
    for (i, l) in lemma_names.iter().enumerate() {
        match &results[i] {
            Run::Refused(why) => out.find("CK03", format!("lemma `{l}` {why}")),
            Run::Broke(why) => out.find("CK04", format!("lemma `{l}` {why}")),
            Run::Done(r) if r.admitted == 0 => out.find(
                "CK03",
                format!(
                    "lemma `{l}` admits no case ({} candidates, every refinement guard false): it says nothing",
                    r.product
                ),
            ),
            Run::Done(r) if r.failed > 0 => out.find(
                "CK04",
                format!(
                    "lemma `{l}` fails on {} of {} cases; the first: {}",
                    r.failed,
                    r.admitted,
                    r.first_failure.as_deref().unwrap_or("(none recorded)")
                ),
            ),
            Run::Done(r) => rows.push(Row {
                name: l.clone(),
                run: r.clone(),
                mutant_model: String::new(),
                mutant_sha: String::new(),
                mutant_failed: 0,
            }),
        }
    }

    // --- CK06 / CK07: the mutant run ----------------------------------------------------------
    for (i, l) in lemma_names.iter().enumerate() {
        let (Some(j), Some(m)) = (mutant_job_of[i], &mutated[i]) else {
            continue; // CK05 or CK06 reported it
        };
        match &results[j] {
            Run::Done(r) if r.failed > 0 => {
                if let Some(row) = rows.iter_mut().find(|r| &r.name == l) {
                    row.mutant_model = m.model.clone();
                    row.mutant_sha = m.sha.clone();
                    row.mutant_failed = r.failed;
                }
            }
            Run::Done(_) => out.find(
                "CK07",
                format!("lemma `{l}` survives its mutant (model `{}` replaced by {MUTANTS_DIR}/{l}.part): it fails on no case, so the lemma does not constrain that model", m.model),
            ),
            Run::Refused(why) | Run::Broke(why) => out.find(
                "CK06",
                format!("{MUTANTS_DIR}/{l}.part: lemma `{l}` {why} on the mutated document (an evaluation error is not a kill)"),
            ),
        }
    }

    // --- CK08: the config mutants --------------------------------------------------------------
    let mut config_rows: Vec<ConfigRow> = Vec::new();
    let mut cursor = config_job_start;
    for (ci, v) in &config_mutants {
        let key = inp.contract.config[*ci].key.clone();
        let mut kills: Vec<String> = Vec::new();
        for l in &callers_of[*ci] {
            match &results[cursor] {
                Run::Done(r) if r.failed > 0 => kills.push(l.clone()),
                Run::Done(_) => {}
                Run::Refused(why) | Run::Broke(why) => out.find(
                    "CK08",
                    format!(
                        "config {key} = {v}: lemma `{l}` {why} (an evaluation error is not a kill)"
                    ),
                ),
            }
            cursor += 1;
        }
        if kills.is_empty() {
            out.find(
                "CK08",
                format!(
                    "config {key} mutated to {v}: no lemma that calls cfg_{key} fails, so `contract.vos` and the Warrant text are not tied (VOS04 5.6)"
                ),
            );
        }
        config_rows.push(ConfigRow {
            key,
            value: *v,
            kills,
        });
    }

    if out.findings.is_empty() {
        out.table = Some(table_text(inp, &rows, &config_rows, &variant_lines));
    }
    Ok(out)
}

/// Judge the mutant part of `lemma` (CK06) and, if it is well formed, splice it into the
/// contract text by the span of the model it replaces.
fn build_mutant(
    out: &mut Checked,
    inp: &ContractInputs,
    contract_text: &str,
    models: &[vos_manifest::warrant::DeclFact],
    lemma: &str,
) -> Result<Option<Mutated>, NoVerdict> {
    let Some((_, part)) = inp.mutants.iter().find(|(s, _)| s == lemma) else {
        return Ok(None); // CK05 reported it
    };
    let at = |m: String| format!("{MUTANTS_DIR}/{lemma}.part: {m}");
    let decls = match on_big_stack(|| declarations_of_block(part.text()))? {
        Ok(d) => d,
        Err(e) => {
            out.find("CK06", at(format!("does not parse: {e}")));
            return Ok(None);
        }
    };
    let [one] = decls.as_slice() else {
        out.find(
            "CK06",
            at(format!(
                "holds {} declarations, and a mutant is exactly one `model`",
                decls.len()
            )),
        );
        return Ok(None);
    };
    if one.kind != DeclKind::Model || !one.has_body {
        out.find(
            "CK06",
            at("its one declaration is not a `model` with a body".to_string()),
        );
        return Ok(None);
    }
    let Some(target) = models
        .iter()
        .find(|d| d.kind == DeclKind::Model && d.name == one.name)
    else {
        out.find(
            "CK06",
            at(format!(
                "model `{}` is not declared by contract.part, so it replaces nothing",
                one.name
            )),
        );
        return Ok(None);
    };
    let model_text = &part.text()[one.start..one.end];
    Ok(Some(Mutated {
        model: one.name.clone(),
        text: splice(contract_text, target.start, target.end, model_text),
        sha: sha256_hex(part.text().as_bytes()),
    }))
}

/// CK10 (VOS-039b-2a): render each Rust enum of the crate and the Warrant variant of the same
/// name with one function each (`enums`) and compare the bytes.  Returns the findings and, in
/// source order, the Rust lines (which equal the Warrant lines when there is no finding): the
/// `variant` rows of the table.  A crate with no `src/` has no enum to compare.
fn check_enums(inp: &ContractInputs, base: &str) -> Result<(Vec<Finding>, Vec<String>), NoVerdict> {
    if inp.sources.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let rust = match enums::rust_enums(&inp.sources) {
        Ok(r) => r,
        Err(problems) => {
            let findings = problems
                .into_iter()
                .map(|p| Finding::new("CK10", p))
                .collect();
            return Ok((findings, Vec::new()));
        }
    };
    // `base` type-checked a moment ago, so it parses.
    // One map built once (enum name to its brace-written constructors), so a constructor
    // lookup is a map probe and not a scan of every enum (an enum with many constructors
    // would otherwise cost enums x constructors).
    let mut named_ctors: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> =
        std::collections::BTreeMap::new();
    for r in &rust {
        named_ctors
            .entry(r.name.as_str())
            .or_default()
            .extend(r.named.iter().map(String::as_str));
    }
    let is_named = |e: &str, c: &str| named_ctors.get(e).is_some_and(|s| s.contains(c));
    let warrant = on_big_stack(|| enums::warrant_variants(base, &is_named))?
        .map_err(|e| NoVerdict::new(format!("the checked document no longer parses: {e}")))?;
    Ok(enums::correspond(&rust, &warrant))
}

/// The generated table (`vos-lemmas 2`), the bytes `gen-contract` writes.  Never parsed.
fn table_text(
    inp: &ContractInputs,
    rows: &[Row],
    config_rows: &[ConfigRow],
    variant_lines: &[String],
) -> String {
    let mut t = String::from("vos-lemmas 2\n");
    t.push_str(&format!(
        "contract-part {}\n",
        sha256_hex(inp.contract_part.text().as_bytes())
    ));
    t.push_str(&format!(
        "lemmas-part {}\n",
        sha256_hex(inp.lemmas_part.text().as_bytes())
    ));
    for c in &inp.contract.config {
        t.push_str(&format!("config {} {}\n", c.key, c.value));
    }
    for v in variant_lines {
        t.push_str(v);
        t.push('\n');
    }
    for r in rows {
        t.push_str(&format!(
            "lemma {} product {} admitted {} mutant {} mutant-part {} failed {}\n",
            r.name, r.run.product, r.run.admitted, r.mutant_model, r.mutant_sha, r.mutant_failed
        ));
    }
    for c in config_rows {
        t.push_str(&format!(
            "config-mutant {} {} kills {}\n",
            c.key,
            c.value,
            c.kills.join(",")
        ));
    }
    t.push_str("end\n");
    t
}

// ---------------------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------------------

fn notes() -> Vec<String> {
    vec!["check-contract: every lemma holds over the whole domain of its binders, every lemma fails on its own mutant (a parsed, span-replaced model), every `config` key is called by a lemma and breaks one when moved by one, and `warrant/lemmas.table` equals what this run writes, byte for byte. When `vos.toml` says `sat = \"model\"`, CK11 checks the model-SAT lane's files only (the BUILD line, and a well-formed `warrant/lemmas.sat` current against the parts with one row per lemma per width and at least one counted obligation); the BUILD line runs the lane itself. Every Rust `enum` of the crate renders to the same `variant ...` line as its Warrant variant (CK10; a tuple variant's field names are not compared). Not checked: that a mutant is the BEST control (the exact set of claims it breaks, VOS-D109), the evidence bindings' `enum ... ctors=` lines against the contract, `Sched::N` in the trait and an implementation's `n_is_eight` literal against `contract.vos`, and bound rows against ceilings (VOS-D055, VOS-039b-2b). The command-type rule is `vos-shape-check` SH19, run by the contract crate's BUILD.".to_string()]
}

/// A line of `text`, escaped and cut, for a finding.
fn show_line(text: &str, n: usize) -> String {
    let l = text
        .lines()
        .nth(n.saturating_sub(1))
        .unwrap_or("(end of file)");
    let cut: String = l.chars().take(200).collect();
    esc(&cut)
}

/// `vos-link check-contract <contract-crate-dir>`.
pub fn check_contract(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let mut out = Outcome::default();
    let inp = match load_inputs(crate_arg, root_arg)? {
        Ok(i) => i,
        Err(findings) => {
            out.findings = findings;
            return Ok(out);
        }
    };
    out.header = format!(
        "contract crate {}, {} mutant parts",
        inp.layout.impl_pkg,
        inp.mutants.len()
    );
    let checked = check_inputs(&inp)?;
    out.findings = checked.findings;
    // CK11 (VOS31 10.2): the model-SAT lane's files.  Needs the claim names, so it runs only
    // when `lemmas.part` parses (CK02 reports it otherwise).
    if let Ok(claims) = claims_of_block(inp.lemmas_part.text()) {
        let names: Vec<String> = claims.into_iter().map(|c| c.name).collect();
        out.findings
            .extend(crate::sat_lane::check_sat_lane(&inp, &names));
    }
    if let Some(want) = checked.table {
        match &inp.table {
            None => out.findings.push(Finding::new(
                "CK09",
                format!("{TABLE_FILE}: not found (run `vos-link gen-contract`)"),
            )),
            Some(found) if *found != want => {
                let n = first_difference_line(found, &want);
                out.findings.push(Finding::new(
                    "CK09",
                    format!(
                        "{TABLE_FILE}:{n}: differs from the table this run writes; expected {}, found {}",
                        show_line(&want, n),
                        show_line(found, n)
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    out.notes = notes();
    Ok(out)
}

/// `vos-link gen-contract <contract-crate-dir>`: run the check, and write the table only if it
/// found nothing (a generator that refuses to write what `check-contract` would reject).
pub fn gen_contract(crate_arg: &Path, root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let mut out = Outcome::default();
    let inp = match load_inputs(crate_arg, root_arg)? {
        Ok(i) => i,
        Err(findings) => {
            out.findings = findings;
            return Ok(out);
        }
    };
    out.header = format!("contract crate {}", inp.layout.impl_pkg);
    let checked = check_inputs(&inp)?;
    out.findings = checked.findings;
    let Some(table) = checked.table else {
        return Ok(out);
    };
    if inp.table.as_deref() == Some(table.as_str()) {
        out.action = Some(format!("unchanged {TABLE_FILE}"));
    } else {
        write_replacing(
            &inp.layout.impl_dir.join("warrant"),
            "lemmas.table",
            table.as_bytes(),
        )?;
        out.action = Some(format!("wrote {TABLE_FILE}"));
    }
    out.notes = notes();
    Ok(out)
}
