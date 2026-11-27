//! # The Warrant evaluator as an oracle (read-only)
//!
//! The cross-checks of VOS31 section 6 compare the encoder with **the Warrant evaluator**, the
//! only implementation of `contract.part`'s meaning that is not hand-written here.  This module
//! is the one place that talks to it.  It edits nothing in `warrant-grammar` (a recorded input,
//! VOS31 section 3); it only parses, type-checks and evaluates.
//!
//! ```text
//!   code/packages/rust/vos-sched-contract/            read through vos-link's library surface
//!     contract.vos, warrant/contract.part,            (`load_inputs`: size caps, no symlinks)
//!     warrant/lemmas.part, warrant/mutants/*.part
//!                     |
//!                     v
//!   base document  = evaluation_document(contract.part, lemmas.part)       <- what check-contract runs
//!   mutant document = the same with ONE model replaced BY SPAN by mutants/<lemma>.part
//!                     |
//!                     v   (a thread with a 256 MiB stack, as the Warrant tools need)
//!   per lemma : the SET of failing cases        <- not just a count: VOS31 6.1
//!   per probe : the value of a model function on every case of its domain
//! ```
//!
//! ## A set, not a count
//!
//! `vos-link`'s `run_lemma` reports how many cases of a lemma failed.  Comparing counts would let
//! an encoder that fails on the *wrong* 448 cases pass.  [`run_document`] therefore evaluates
//! the same way `run_lemma` does (every binder over its whole domain) but records WHICH cases
//! failed, numbered as in [`crate::lemmas`].
//!
//! ## Cost
//!
//! The evaluator is an interpreter: a lemma is 65,536 evaluations.  Callers that need the same
//! document more than once should keep the [`DocResults`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, TypeExpr, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use vos_link::contract_check::{evaluation_document, load_inputs, splice};
use vos_manifest::warrant::{declarations_of_block, DeclKind};
use vos_manifest::Contract;

use crate::probes::{Probe, Ty};
use crate::types::Val;

/// The stack the Warrant checker and evaluator need (the same figure `vos-link` uses).
pub const WARRANT_STACK_BYTES: usize = 256 * 1024 * 1024;

/// The most cases one evaluated lemma or probe may have: the evaluator route's own X1 budget
/// (`vos-link`'s `LEMMA_BUDGET`, 400,000, VOS04 5.6), so a probe can never be an unbounded loop.
pub const CASE_BUDGET: u64 = 400_000;

/// The inputs of a contract crate, as text.
#[derive(Debug)]
pub struct ContractFiles {
    /// `contract.vos` (for the `config` rows).
    pub contract: Contract,
    /// `warrant/contract.part`.
    pub contract_text: String,
    /// `warrant/lemmas.part`.
    pub lemmas_text: String,
    /// `warrant/mutants/<stem>.part`: `(stem, text)`.
    pub mutants: Vec<(String, String)>,
    /// `warrant/lemmas.table` as committed, if present (the counts of section 6.1 are read from it).
    pub table: Option<String>,
    /// The contract crate directory (absolute), where `warrant/lemmas.sat` lives.
    pub dir: PathBuf,
    /// `vos.toml` as read (bounded, no symlink), if present: `sat = "model"` is read from it.
    pub vos_toml: Option<String>,
    /// `warrant/lemmas.sat` as committed, if present (VOS31 section 8).
    pub sat_table: Option<String>,
}

/// Why a contract crate could not be loaded.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The inputs are wrong (`vos-link` findings, or an unreadable small file).
    Inputs(String),
    /// A path or size problem the loader reports as "no verdict".
    NoVerdict(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Inputs(s) | LoadError::NoVerdict(s) => write!(f, "{s}"),
        }
    }
}

/// The most bytes of `vos.toml` and of `lemmas.sat` that are read.
pub const SMALL_FILE_MAX_BYTES: usize = 1024 * 1024;

/// Read the contract crate at `crate_dir` (`root` is the repository root, or `None` to find it).
/// Every failure is a message, never a panic.
pub fn load(crate_dir: &Path, root: Option<&Path>) -> Result<ContractFiles, String> {
    load_detailed(crate_dir, root).map_err(|e| e.to_string())
}

/// Read a small regular file (no symlink, bounded); `None` when it does not exist.
fn read_small(path: &Path) -> Result<Option<String>, LoadError> {
    match vos_link::fsio::read_regular_opt(path, SMALL_FILE_MAX_BYTES) {
        Ok(None) => Ok(None),
        Ok(Some(b)) => String::from_utf8(b)
            .map(Some)
            .map_err(|_| LoadError::Inputs(format!("{}: not UTF-8", path.display()))),
        Err(e) => Err(LoadError::Inputs(e.to_string())),
    }
}

/// [`load`], telling a wrong input apart from a path problem.
pub fn load_detailed(crate_dir: &Path, root: Option<&Path>) -> Result<ContractFiles, LoadError> {
    let inputs = match load_inputs(crate_dir, root) {
        Ok(Ok(i)) => i,
        Ok(Err(findings)) => {
            let msgs: Vec<String> = findings
                .iter()
                .map(|f| format!("{}: {}", f.code, f.message))
                .collect();
            return Err(LoadError::Inputs(msgs.join("; ")));
        }
        Err(no_verdict) => return Err(LoadError::NoVerdict(format!("{no_verdict:?}"))),
    };
    let dir = inputs.layout.impl_dir.clone();
    let vos_toml = read_small(&dir.join("vos.toml"))?;
    let sat_table = read_small(&dir.join("warrant").join("lemmas.sat"))?;
    Ok(ContractFiles {
        dir,
        vos_toml,
        sat_table,
        contract: inputs.contract,
        table: inputs.table.clone(),
        contract_text: inputs.contract_part.text().to_string(),
        lemmas_text: inputs.lemmas_part.text().to_string(),
        mutants: inputs
            .mutants
            .iter()
            .map(|(s, p)| (s.clone(), p.text().to_string()))
            .collect(),
    })
}

impl ContractFiles {
    /// The document `check-contract` evaluates for the real contract.
    pub fn base_document(&self) -> String {
        evaluation_document(&self.contract, &self.contract_text, &self.lemmas_text, None)
    }

    /// The document with `lemma`'s mutant spliced in: the one model of
    /// `warrant/mutants/<lemma>.part` replaces the model of the same name in `contract.part`, by
    /// the parser's span (never a text search), exactly as `check-contract` does.
    pub fn mutant_document(&self, lemma: &str) -> Result<String, String> {
        let part = self
            .mutants
            .iter()
            .find(|(s, _)| s == lemma)
            .map(|(_, t)| t.as_str())
            .ok_or_else(|| format!("no mutant part for lemma `{lemma}`"))?;
        let contract_text = self.contract_text.clone();
        let part_text = part.to_string();
        let (models, decls) = on_big_stack(move || {
            (
                declarations_of_block(&contract_text),
                declarations_of_block(&part_text),
            )
        })?;
        let models = models.map_err(|e| format!("contract.part: {e}"))?;
        let decls = decls.map_err(|e| format!("mutant of {lemma}: {e}"))?;
        let [one] = decls.as_slice() else {
            return Err(format!(
                "mutant of {lemma} holds {} declarations, not exactly one",
                decls.len()
            ));
        };
        if one.kind != DeclKind::Model || !one.has_body {
            return Err(format!("mutant of {lemma} is not a `model` with a body"));
        }
        let target = models
            .iter()
            .find(|d| d.kind == DeclKind::Model && d.name == one.name)
            .ok_or_else(|| {
                format!(
                    "mutant of {lemma}: model `{}` is not in contract.part",
                    one.name
                )
            })?;
        let replacement = &part[one.start..one.end];
        let spliced = splice(&self.contract_text, target.start, target.end, replacement);
        Ok(evaluation_document(
            &self.contract,
            &spliced,
            &self.lemmas_text,
            None,
        ))
    }
}

/// Run `f` on a thread with the stack the Warrant tools need.
pub fn on_big_stack<R: Send>(f: impl FnOnce() -> R + Send) -> Result<R, String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("vos-model-sat-warrant".to_string())
            .stack_size(WARRANT_STACK_BYTES)
            .spawn_scoped(scope, f)
            .map_err(|e| format!("cannot start the Warrant thread: {e}"))?
            .join()
            .map_err(|_| "the Warrant thread panicked".to_string())
    })
}

/// What the evaluator says about one lemma over its whole domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LemmaFailures {
    /// The number of cases (the product of the binder domains).
    pub product: u64,
    /// `failing[c]` is true iff case `c` (the numbering of [`crate::lemmas`]) did not evaluate
    /// to `true`.
    pub failing: Vec<bool>,
}

impl LemmaFailures {
    /// How many cases failed (the `failed` column of `lemmas.table`).
    pub fn failed(&self) -> u64 {
        self.failing.iter().filter(|&&f| f).count() as u64
    }
}

/// Everything evaluated on one document.
#[derive(Debug, Default)]
pub struct DocResults {
    /// Per requested lemma.
    pub lemmas: HashMap<String, LemmaFailures>,
    /// Per requested probe: the value on every case, in case order.
    pub probes: HashMap<&'static str, Vec<Val>>,
}

/// Parse `src` and run `f` on the file (the AST borrows the source).
pub fn with_file<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> Result<R, String> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default())
        .map_err(|e| format!("{e}"))?;
    let layout = capture_layout(&doc, LayoutLimits::default()).map_err(|e| format!("{e}"))?;
    let file =
        parse_document(&layout, &doc, ParserLimits::default()).map_err(|e| format!("{e}"))?;
    Ok(f(&file))
}

/// The bit width of a binder type, or `None` for a type this crate does not enumerate.
fn binder_bits(ty: &TypeExpr) -> Option<usize> {
    match ty {
        TypeExpr::Named { name: "Bool", .. } => Some(1),
        TypeExpr::Named { name: "U8", .. } => Some(8),
        _ => None,
    }
}

/// The value of one binder from its bits.
fn binder_value(ty: &TypeExpr, bits: u64) -> EvalValue {
    match ty {
        TypeExpr::Named { name: "Bool", .. } => EvalValue::Bool(bits == 1),
        _ => EvalValue::U8(bits as u8),
    }
}

/// Evaluate the `forall` claim `name` (a lemma or a probe) on every case, calling `on_case(case,
/// value-of-the-body-or-of-its-left-operand)`.  `left_operand` selects a probe.
fn each_case<'a>(
    file: &'a WarrantFile<'a>,
    name: &str,
    left_operand: bool,
    mut on_case: impl FnMut(u64, EvalValue),
) -> Result<u64, String> {
    let claim = file
        .declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Claim(c) if c.name == name => Some(c),
            _ => None,
        })
        .ok_or_else(|| format!("no claim named {name}"))?;
    let Expression::Quantified {
        is_forall: true,
        binders,
        body,
        ..
    } = &claim.body
    else {
        return Err(format!("{name} is not a `forall` claim"));
    };
    let mut widths = Vec::new();
    for b in binders {
        if b.refinement.is_some() {
            return Err(format!(
                "{name}: binder `{}` has a refinement guard",
                b.name
            ));
        }
        widths.push(
            binder_bits(&b.param_type)
                .ok_or_else(|| format!("{name}: binder `{}` is not Bool or U8", b.name))?,
        );
    }
    let total_bits: usize = widths.iter().sum();
    if total_bits >= 40 || (1u64 << total_bits) > CASE_BUDGET {
        return Err(format!(
            "{name}: 2^{total_bits} cases is over the budget of {CASE_BUDGET}"
        ));
    }
    let mut target: &Expression = body;
    while let Expression::Grouped { inner, .. } = target {
        target = inner;
    }
    if left_operand {
        let Expression::Binary { left, .. } = target else {
            return Err(format!("{name}: a probe body must be `a == a`"));
        };
        target = left;
    }
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    let total: u64 = 1 << total_bits;
    for case in 0..total {
        let mut env: HashMap<&str, EvalValue> = HashMap::new();
        let mut shift = 0;
        for (b, w) in binders.iter().zip(&widths) {
            let bits = (case >> shift) & ((1u64 << w) - 1);
            shift += w;
            env.insert(b.name, binder_value(&b.param_type, bits));
        }
        evaluator.reset_work();
        let v = evaluator
            .eval(target, &env)
            .map_err(|e| format!("{name} did not evaluate on case {case}: {e}"))?;
        on_case(case, v);
    }
    Ok(total)
}

/// Read an evaluator value as a [`Val`] of type `ty`.
pub fn to_val(v: &EvalValue, ty: Ty) -> Option<Val> {
    match (ty, v) {
        (Ty::Bool, EvalValue::Bool(b)) => Some(Val::Bool(*b)),
        (Ty::Ready, EvalValue::Con { fields, .. }) => {
            let mut mask = 0u64;
            for (i, f) in fields.iter().enumerate() {
                mask |= u64::from(f.as_bool()?) << i;
            }
            Some(Val::Ready(mask))
        }
        (Ty::Upd, EvalValue::Con { index, .. }) => u8::try_from(*index).ok().map(Val::Upd),
        (Ty::Pick, EvalValue::Con { ctor, fields, .. }) => match ctor.as_str() {
            "Idle" => Some(Val::Pick(None)),
            "Run" => Some(Val::Pick(Some(fields.first()?.as_u64()? as u8))),
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate `lemmas` and `probes` on the document `src`.  `src` must already contain each
/// probe's claim ([`with_probe_claims`]).  Runs on a big-stack thread.
pub fn run_document(src: &str, lemmas: &[&str], probes: &[Probe]) -> Result<DocResults, String> {
    let src = src.to_string();
    let lemmas: Vec<String> = lemmas.iter().map(|s| s.to_string()).collect();
    let probes: Vec<Probe> = probes.to_vec();
    on_big_stack(move || -> Result<DocResults, String> {
        with_file(&src, |file| -> Result<DocResults, String> {
            check_warrant(file).map_err(|e| format!("type check failed: {e:?}"))?;
            let mut out = DocResults::default();
            for l in &lemmas {
                let mut failing: Vec<bool> = Vec::new();
                let product = each_case(file, l, false, |case, v| {
                    debug_assert_eq!(case as usize, failing.len());
                    failing.push(v != EvalValue::Bool(true));
                })?;
                out.lemmas
                    .insert(l.clone(), LemmaFailures { product, failing });
            }
            for p in &probes {
                let mut values: Vec<Val> = Vec::new();
                let mut bad: Option<String> = None;
                each_case(
                    file,
                    &format!("probe_{}", p.name),
                    true,
                    |case, v| match to_val(&v, p.ty) {
                        Some(x) => values.push(x),
                        None => {
                            bad.get_or_insert(format!(
                                "probe {} case {case}: unexpected value {v:?}",
                                p.name
                            ));
                            values.push(Val::Bool(false));
                        }
                    },
                )?;
                if let Some(e) = bad {
                    return Err(e);
                }
                out.probes.insert(p.name, values);
            }
            Ok(out)
        })?
    })?
}

/// `doc` with every probe's claim appended.
pub fn with_probe_claims(doc: &str, probes: &[Probe]) -> String {
    let mut s = doc.to_string();
    for p in probes {
        s.push_str(&p.claim_text());
    }
    s
}

/// Every `claim` of `src` with whether its body is, at the top level, an implication
/// `P => Q`, in file order (VOS31 7.2 rule 4).  Read from the **Warrant parser's AST**, never
/// from the encoder, so an encoder cannot declare its own lemma exempt.  Runs on a big-stack
/// thread.
pub fn claims_of(src: &str) -> Result<Vec<(String, bool)>, String> {
    use coding_adventures_warrant_grammar::parser::BinaryOp;
    let src = src.to_string();
    on_big_stack(move || {
        with_file(&src, |file| {
            let mut out = Vec::new();
            for d in &file.declarations {
                let Declaration::Claim(c) = d else { continue };
                let mut body: &Expression = &c.body;
                if let Expression::Quantified {
                    is_forall: true,
                    body: inner,
                    ..
                } = body
                {
                    body = inner;
                }
                while let Expression::Grouped { inner, .. } = body {
                    body = inner;
                }
                let implies = matches!(
                    body,
                    Expression::Binary {
                        op: BinaryOp::Implies,
                        ..
                    }
                );
                out.push((c.name.to_string(), implies));
            }
            out
        })
    })?
}
