//! Canonical formula-execution audit used by the `adj-formula-audit` binary.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adj_lang::ast::Term as AstTerm;
use adj_lang::{
    compile_with_imports, formula_provenance, program_source_map, ImportLimits, ImportProvider,
    ProgramSourceMap, SourceSpan,
};
use coding_adventures_sha256::sha256_hex;
use logic_engine::compute::ExactRational;
use logic_engine::{
    verify_derived, ComputationFailure, ComputationStatus, ComputeError, ComputeExpr, ContentHash,
    DerivationNode, FactId, KnowledgeBase, Provenance, QuoteMiss, QuoteStatus, RoundSpec,
    RoundingMode, SnapshotStore, TrustTier, UnverifiedReason,
};
use serde::Serialize;

use crate::FsProvider;

const MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug)]
enum Failure {
    Audit(String),
    Usage(String),
}

#[derive(Debug, Clone)]
struct ResolvedImport {
    importer: String,
    literal: String,
    resolved: String,
}

struct RecordingProvider {
    inner: FsProvider,
    imports: RefCell<Vec<ResolvedImport>>,
    sources: RefCell<BTreeMap<String, String>>,
}

impl ImportProvider for RecordingProvider {
    fn resolve(&self, importer: &str, literal: &str) -> Result<String, String> {
        let resolved = self.inner.resolve(importer, literal)?;
        self.imports.borrow_mut().push(ResolvedImport {
            importer: importer.to_string(),
            literal: literal.to_string(),
            resolved: resolved.clone(),
        });
        Ok(resolved)
    }

    fn load(&self, canonical: &str) -> Result<String, String> {
        let path = Path::new(canonical);
        if !path.starts_with(&self.inner.root) {
            return Err(format!("{canonical} escapes the import root"));
        }
        if !fs::symlink_metadata(path)
            .map_err(|error| format!("{canonical}: {error}"))?
            .file_type()
            .is_file()
        {
            return Err(format!("source is not a regular file: {canonical}"));
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|error| format!("{canonical}: {error}"))?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("{canonical}: {error}"))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(format!(
                "source exceeds {MAX_BYTES} byte limit: {canonical}"
            ));
        }
        let source = String::from_utf8(bytes)
            .map_err(|error| format!("source is not UTF-8, {canonical}: {error}"))?;
        let mut sources = self.sources.borrow_mut();
        if let Some(previous) = sources.get(canonical) {
            if previous != &source {
                return Err(format!("source changed during audit: {canonical}"));
            }
        } else {
            sources.insert(canonical.to_string(), source.clone());
        }
        Ok(source)
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SpanDto {
    end: usize,
    sha256: String,
    start: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FormulaIdentityDto {
    body: SpanDto,
    declaration: SpanDto,
    formulabook: String,
    name: String,
    parameters: Vec<String>,
    source_sha256: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CorroborationDto {
    locator: String,
    source: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct QuoteIdentityDto {
    byte_len: usize,
    byte_offset: Option<usize>,
    snapshot_sha256: Option<String>,
    text_sha256: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProvenanceDto {
    corroborations: Vec<CorroborationDto>,
    locator: Option<String>,
    quote: Option<QuoteIdentityDto>,
    source: String,
    trust: &'static str,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FactIdentityDto {
    provenance: ProvenanceDto,
    term: String,
}

#[derive(Serialize)]
struct ImportDto {
    declaration: SpanDto,
    imported_source_sha256: String,
    importer_source_sha256: String,
    literal: String,
}

#[derive(Serialize)]
struct QuestionDto {
    declaration: SpanDto,
    name: String,
    source_sha256: String,
}

#[derive(Serialize)]
struct RationalDto {
    denominator: String,
    numerator: String,
}

#[derive(Serialize)]
struct ResultDto {
    dimension: String,
    exact_rational: Option<RationalDto>,
    f64_bits: String,
}

#[derive(Serialize)]
struct ScopeDto {
    derived_limit: usize,
    fact_limit: u64,
}

#[derive(Serialize)]
struct PlanDto {
    expression: PlanExprDto,
    is_query_answer: bool,
    scope: ScopeDto,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PlanExprDto {
    Aggregate {
        operator: &'static str,
        slot: String,
    },
    Binary {
        left: Box<PlanExprDto>,
        operator: &'static str,
        right: Box<PlanExprDto>,
    },
    Literal {
        f64_bits: String,
    },
    Reference {
        name: String,
    },
    Round {
        expression: Box<PlanExprDto>,
        mode: &'static str,
        precision: PrecisionDto,
    },
    ToCurrency {
        code: String,
        expression: Box<PlanExprDto>,
        mode: &'static str,
        places: u32,
    },
    ToPercent {
        expression: Box<PlanExprDto>,
        mode: &'static str,
        places: u32,
    },
    ToScientific {
        expression: Box<PlanExprDto>,
        figures: u32,
        mode: &'static str,
    },
    Unary {
        expression: Box<PlanExprDto>,
        operator: &'static str,
    },
}

#[derive(Serialize)]
struct PrecisionDto {
    kind: &'static str,
    value: u32,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TreeDto {
    DerivedReference {
        f64_bits: String,
        name: String,
    },
    Leaf {
        fact: FactIdentityDto,
        f64_bits: String,
        slot: String,
    },
    Literal {
        f64_bits: String,
    },
    Operation {
        f64_bits: String,
        operands: Vec<TreeDto>,
        operator: &'static str,
    },
    Round {
        f64_bits: String,
        mode: &'static str,
        operand: Box<TreeDto>,
        operand_exact: Option<RationalDto>,
        precision: PrecisionDto,
    },
    ToCurrency {
        code: String,
        f64_bits: String,
        mode: &'static str,
        operand: Box<TreeDto>,
        operand_exact: Option<RationalDto>,
        places: u32,
        rendered: String,
    },
    ToPercent {
        f64_bits: String,
        mode: &'static str,
        operand: Box<TreeDto>,
        operand_exact: Option<RationalDto>,
        places: u32,
        rendered: String,
    },
    ToScientific {
        f64_bits: String,
        figures: u32,
        mode: &'static str,
        operand: Box<TreeDto>,
        operand_exact: Option<RationalDto>,
        rendered: String,
    },
}

#[derive(Serialize)]
struct InputDto {
    identity: FactIdentityDto,
    quote: QuoteStatusDto,
}

#[derive(Serialize)]
struct FormulaCheckDto {
    identity: FormulaIdentityDto,
    provenance: ProvenanceDto,
    quote: QuoteStatusDto,
}

#[derive(Serialize)]
struct QuoteStatusDto {
    byte_len: Option<usize>,
    byte_offset: Option<usize>,
    reason: Option<&'static str>,
    status: &'static str,
}

#[derive(Serialize)]
struct ComputationStatusDto {
    reason: Option<&'static str>,
    recomputed_f64_bits: Option<String>,
    recorded_f64_bits: Option<String>,
    status: &'static str,
}

#[derive(Serialize)]
struct VerificationDto {
    computation: ComputationStatusDto,
    formula_quotes: Vec<FormulaCheckDto>,
    fully_verified: bool,
    input_quotes: Vec<InputDto>,
    is_query_answer: bool,
    passed: bool,
}

#[derive(Serialize)]
struct DerivationDto {
    export: FormulaIdentityDto,
    formula_sequence: Vec<FormulaIdentityDto>,
    inputs: Vec<FactIdentityDto>,
    plan: PlanDto,
    question: QuestionDto,
    result: ResultDto,
    tree: TreeDto,
    verification: VerificationDto,
}

#[derive(Serialize)]
struct AuditDto {
    contract: &'static str,
    derivations: Vec<DerivationDto>,
    imports: Vec<ImportDto>,
    kind: &'static str,
    root_source_sha256: String,
    schema_version: u8,
}

struct LoadedSource {
    hash: String,
    map: ProgramSourceMap,
    source: String,
}

#[derive(Clone)]
struct ExportRecord {
    identity: FormulaIdentityDto,
    provenance: Provenance,
}

struct DirSnapshots {
    root: PathBuf,
    cache: RefCell<BTreeMap<String, Option<Vec<u8>>>>,
}

impl SnapshotStore for DirSnapshots {
    fn get(&self, hash: &ContentHash) -> Option<Vec<u8>> {
        if let Some(found) = self.cache.borrow().get(hash.as_hex()) {
            return found.clone();
        }
        let path = self.root.join(hash.as_hex());
        let value = (|| {
            if !fs::symlink_metadata(&path).ok()?.file_type().is_file() {
                return None;
            }
            let mut bytes = Vec::new();
            fs::File::open(path)
                .ok()?
                .take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() as u64 > MAX_BYTES || !hash.matches(&bytes) {
                return None;
            }
            Some(bytes)
        })();
        self.cache
            .borrow_mut()
            .insert(hash.as_hex().to_string(), value.clone());
        value
    }
}

struct EmptySnapshots;

impl SnapshotStore for EmptySnapshots {
    fn get(&self, _hash: &ContentHash) -> Option<Vec<u8>> {
        None
    }
}

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn rational(value: &ExactRational) -> RationalDto {
    RationalDto {
        denominator: value.denominator().to_string(),
        numerator: value.numerator().to_string(),
    }
}

fn span(source: &[u8], value: SourceSpan) -> SpanDto {
    SpanDto {
        end: value.end,
        sha256: sha256_hex(&source[value.start..value.end]),
        start: value.start,
    }
}

fn trust_name(value: TrustTier) -> &'static str {
    match value {
        TrustTier::Consensus => "consensus",
        TrustTier::Authoritative => "authoritative",
        TrustTier::Empirical => "empirical",
        TrustTier::Inferred => "inferred",
        TrustTier::Unattributed => "unattributed",
    }
}

fn provenance_dto(value: &Provenance) -> ProvenanceDto {
    ProvenanceDto {
        corroborations: value
            .corroborations
            .iter()
            .map(|citation| CorroborationDto {
                locator: citation.locator.clone(),
                source: citation.source.clone(),
            })
            .collect(),
        locator: value.locator.clone(),
        quote: value.quote.text().map(|text| QuoteIdentityDto {
            byte_len: text.len(),
            byte_offset: value.quote.byte_offset(),
            snapshot_sha256: value
                .snapshot
                .as_ref()
                .map(|hash| hash.as_hex().to_string()),
            text_sha256: sha256_hex(text.as_bytes()),
        }),
        source: value.source.clone(),
        trust: trust_name(value.trust_tier),
    }
}

fn fact_identity(kb: &KnowledgeBase, id: FactId) -> Result<FactIdentityDto, Failure> {
    let fact = kb
        .fact(id)
        .ok_or_else(|| Failure::Audit(format!("verification references absent fact {}", id.0)))?;
    Ok(FactIdentityDto {
        provenance: provenance_dto(&fact.provenance),
        term: fact.term.to_string(),
    })
}

fn mode_name(mode: RoundingMode) -> &'static str {
    match mode {
        RoundingMode::Down => "down",
        RoundingMode::Up => "up",
        RoundingMode::Floor => "floor",
        RoundingMode::Ceiling => "ceiling",
        RoundingMode::HalfUp => "half_up",
        RoundingMode::HalfDown => "half_down",
        RoundingMode::HalfEven => "half_even",
    }
}

fn precision(spec: RoundSpec) -> PrecisionDto {
    match spec {
        RoundSpec::Places(value) => PrecisionDto {
            kind: "places",
            value,
        },
        RoundSpec::SigFigures(value) => PrecisionDto {
            kind: "significant_figures",
            value,
        },
    }
}

fn plan_expr(value: &ComputeExpr) -> PlanExprDto {
    match value {
        ComputeExpr::Ref(name) => PlanExprDto::Reference { name: name.clone() },
        ComputeExpr::Lit(value) => PlanExprDto::Literal {
            f64_bits: bits(*value),
        },
        ComputeExpr::Bin(op, left, right) => PlanExprDto::Binary {
            left: Box::new(plan_expr(left)),
            operator: op.symbol(),
            right: Box::new(plan_expr(right)),
        },
        ComputeExpr::Unary(op, expression) => PlanExprDto::Unary {
            expression: Box::new(plan_expr(expression)),
            operator: op.symbol(),
        },
        ComputeExpr::Agg(op, slot) => PlanExprDto::Aggregate {
            operator: op.symbol(),
            slot: slot.clone(),
        },
        ComputeExpr::Round { spec, mode, expr } => PlanExprDto::Round {
            expression: Box::new(plan_expr(expr)),
            mode: mode_name(*mode),
            precision: precision(*spec),
        },
        ComputeExpr::ToScientific {
            figures,
            mode,
            expr,
        } => PlanExprDto::ToScientific {
            expression: Box::new(plan_expr(expr)),
            figures: *figures,
            mode: mode_name(*mode),
        },
        ComputeExpr::ToPercent { places, mode, expr } => PlanExprDto::ToPercent {
            expression: Box::new(plan_expr(expr)),
            mode: mode_name(*mode),
            places: *places,
        },
        ComputeExpr::ToCurrency {
            code,
            places,
            mode,
            expr,
        } => PlanExprDto::ToCurrency {
            code: code.clone(),
            expression: Box::new(plan_expr(expr)),
            mode: mode_name(*mode),
            places: *places,
        },
    }
}

fn tree(value: &DerivationNode, kb: &KnowledgeBase) -> Result<TreeDto, Failure> {
    Ok(match value {
        DerivationNode::Leaf {
            slot,
            value,
            fact_id,
        } => TreeDto::Leaf {
            fact: fact_identity(kb, *fact_id)?,
            f64_bits: bits(*value),
            slot: slot.clone(),
        },
        DerivationNode::DerivedRef { name, value } => TreeDto::DerivedReference {
            f64_bits: bits(*value),
            name: name.clone(),
        },
        DerivationNode::Lit { value } => TreeDto::Literal {
            f64_bits: bits(*value),
        },
        DerivationNode::Op {
            op,
            operands,
            result,
        } => TreeDto::Operation {
            f64_bits: bits(*result),
            operands: operands
                .iter()
                .map(|operand| tree(operand, kb))
                .collect::<Result<_, _>>()?,
            operator: op.symbol(),
        },
        DerivationNode::Round {
            spec,
            mode,
            operand,
            operand_exact,
            result,
        } => TreeDto::Round {
            f64_bits: bits(*result),
            mode: mode_name(*mode),
            operand: Box::new(tree(operand, kb)?),
            operand_exact: operand_exact.as_ref().map(rational),
            precision: precision(*spec),
        },
        DerivationNode::ToScientific {
            figures,
            mode,
            rendered,
            operand,
            operand_exact,
            result,
        } => TreeDto::ToScientific {
            f64_bits: bits(*result),
            figures: *figures,
            mode: mode_name(*mode),
            operand: Box::new(tree(operand, kb)?),
            operand_exact: operand_exact.as_ref().map(rational),
            rendered: rendered.clone(),
        },
        DerivationNode::ToPercent {
            places,
            mode,
            rendered,
            operand,
            operand_exact,
            result,
        } => TreeDto::ToPercent {
            f64_bits: bits(*result),
            mode: mode_name(*mode),
            operand: Box::new(tree(operand, kb)?),
            operand_exact: operand_exact.as_ref().map(rational),
            places: *places,
            rendered: rendered.clone(),
        },
        DerivationNode::ToCurrency {
            code,
            places,
            mode,
            rendered,
            operand,
            operand_exact,
            result,
        } => TreeDto::ToCurrency {
            code: code.clone(),
            f64_bits: bits(*result),
            mode: mode_name(*mode),
            operand: Box::new(tree(operand, kb)?),
            operand_exact: operand_exact.as_ref().map(rational),
            places: *places,
            rendered: rendered.clone(),
        },
    })
}

fn quote_status(value: &QuoteStatus) -> QuoteStatusDto {
    match value {
        QuoteStatus::Verified {
            byte_offset,
            byte_len,
        } => QuoteStatusDto {
            byte_len: Some(*byte_len),
            byte_offset: Some(*byte_offset),
            reason: None,
            status: "verified",
        },
        QuoteStatus::QuoteMissing(reason) => QuoteStatusDto {
            byte_len: None,
            byte_offset: None,
            reason: Some(match reason {
                QuoteMiss::BlankSpan => "blank_span",
                QuoteMiss::RangeOutOfBounds { .. } => "range_out_of_bounds",
                QuoteMiss::NotACharBoundary { .. } => "not_a_char_boundary",
                QuoteMiss::TextDiffers { .. } => "text_differs",
            }),
            status: "quote_missing",
        },
        QuoteStatus::Unverified(reason) => QuoteStatusDto {
            byte_len: None,
            byte_offset: None,
            reason: Some(match reason {
                UnverifiedReason::Unmigrated => "unmigrated",
                UnverifiedReason::NoSnapshotPinned => "no_snapshot_pinned",
                UnverifiedReason::SnapshotUnavailable => "snapshot_unavailable",
                UnverifiedReason::NoByteOffset => "no_byte_offset",
                UnverifiedReason::NoProvenance => "no_provenance",
            }),
            status: "unverified",
        },
        QuoteStatus::SourceDrifted => QuoteStatusDto {
            byte_len: None,
            byte_offset: None,
            reason: None,
            status: "source_drifted",
        },
        QuoteStatus::SourceUnreachable => QuoteStatusDto {
            byte_len: None,
            byte_offset: None,
            reason: None,
            status: "source_unreachable",
        },
        QuoteStatus::NotApplicable => QuoteStatusDto {
            byte_len: None,
            byte_offset: None,
            reason: None,
            status: "not_applicable",
        },
    }
}

fn status_matches_quote_identity(provenance: &Provenance, status: &QuoteStatus) -> bool {
    match status {
        QuoteStatus::Verified {
            byte_offset,
            byte_len,
        } => {
            provenance.quote.byte_offset() == Some(*byte_offset)
                && provenance.quote.text().map(str::len) == Some(*byte_len)
                && provenance.snapshot.is_some()
        }
        _ => true,
    }
}

fn compute_error_reason(value: &ComputeError) -> &'static str {
    match value {
        ComputeError::UnknownSlot { .. } => "unknown_slot",
        ComputeError::EmptyAggregation { .. } => "empty_aggregation",
        ComputeError::DivisionByZero => "division_by_zero",
        ComputeError::MalformedExpr { .. } => "malformed_expression",
        ComputeError::TooDeep { .. } => "too_deep",
        ComputeError::NonFinite { .. } => "non_finite",
        ComputeError::DimensionMismatch { .. } => "dimension_mismatch",
    }
}

fn computation_status(value: &ComputationStatus) -> ComputationStatusDto {
    let mut result = ComputationStatusDto {
        reason: None,
        recomputed_f64_bits: None,
        recorded_f64_bits: None,
        status: "rechecked",
    };
    match value {
        ComputationStatus::ReChecked => {}
        ComputationStatus::Unverifiable(_) => {
            result.status = "unverifiable";
            result.reason = Some("inexact_narrowing_source");
        }
        ComputationStatus::Failed(failure) => {
            result.status = "failed";
            result.reason = Some(match failure {
                ComputationFailure::PlanUnavailable => "plan_unavailable",
                ComputationFailure::ArtifactDoesNotMatchPlan => "artifact_does_not_match_plan",
                ComputationFailure::ScopeUnavailable => "scope_unavailable",
                ComputationFailure::EvaluationFailed(error) => compute_error_reason(error),
                ComputationFailure::ValueDiffers {
                    recorded,
                    recomputed,
                } => {
                    result.recorded_f64_bits = Some(bits(*recorded));
                    result.recomputed_f64_bits = Some(bits(*recomputed));
                    "value_differs"
                }
                ComputationFailure::ExactValueDiffers => "exact_value_differs",
                ComputationFailure::DimensionDiffers => "dimension_differs",
                ComputationFailure::TreeDiffers => "tree_differs",
                ComputationFailure::ReferencedDerivedDiffers(_) => "referenced_derived_differs",
            });
        }
    }
    result
}

fn formula_name_arity(term: &AstTerm) -> Option<(&str, usize)> {
    match term {
        AstTerm::Compound { functor, args } => Some((functor, args.len())),
        _ => None,
    }
}

fn collect_fact_ids(
    index: usize,
    kb: &KnowledgeBase,
    visiting: &mut BTreeSet<usize>,
    output: &mut BTreeSet<FactId>,
) -> Result<(), Failure> {
    if !visiting.insert(index) {
        return Err(Failure::Audit("cycle in derived references".to_string()));
    }
    let derived = &kb.derived_bindings()[index];
    let plan = kb
        .computation_plan_for(derived)
        .ok_or_else(|| Failure::Audit(format!("{} has no trusted plan", derived.name)))?;
    fn walk(
        node: &DerivationNode,
        before: usize,
        kb: &KnowledgeBase,
        visiting: &mut BTreeSet<usize>,
        output: &mut BTreeSet<FactId>,
    ) -> Result<(), Failure> {
        match node {
            DerivationNode::Leaf { fact_id, .. } => {
                output.insert(*fact_id);
            }
            DerivationNode::DerivedRef { name, .. } => {
                let dependency = kb.derived_bindings()[..before]
                    .iter()
                    .rposition(|candidate| candidate.name == *name)
                    .ok_or_else(|| {
                        Failure::Audit(format!(
                            "derived reference {name} has no unique predecessor"
                        ))
                    })?;
                collect_fact_ids(dependency, kb, visiting, output)?;
            }
            DerivationNode::Op { operands, .. } => {
                for operand in operands {
                    walk(operand, before, kb, visiting, output)?;
                }
            }
            DerivationNode::Round { operand, .. }
            | DerivationNode::ToScientific { operand, .. }
            | DerivationNode::ToPercent { operand, .. }
            | DerivationNode::ToCurrency { operand, .. } => {
                walk(operand, before, kb, visiting, output)?;
            }
            DerivationNode::Lit { .. } => {}
        }
        Ok(())
    }
    walk(
        &derived.tree,
        plan.scope.derived_limit,
        kb,
        visiting,
        output,
    )?;
    visiting.remove(&index);
    Ok(())
}

fn build_sources(raw: BTreeMap<String, String>) -> Result<BTreeMap<String, LoadedSource>, Failure> {
    raw.into_iter()
        .map(|(canonical, source)| {
            if source.len() as u64 > MAX_BYTES {
                return Err(Failure::Audit(format!(
                    "source exceeds {MAX_BYTES} byte limit: {canonical}"
                )));
            }
            let hash = sha256_hex(source.as_bytes());
            let map = program_source_map(&source).map_err(|error| {
                Failure::Audit(format!("source map failed for {hash}: {error:?}"))
            })?;
            Ok((canonical, LoadedSource { hash, map, source }))
        })
        .collect()
}

fn build_exports(sources: &BTreeMap<String, LoadedSource>) -> Result<Vec<ExportRecord>, Failure> {
    let mut names = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut exports = Vec::new();
    for loaded in sources.values() {
        for mapped in &loaded.map.formulas {
            let identity = FormulaIdentityDto {
                body: span(loaded.source.as_bytes(), mapped.body_span),
                declaration: span(loaded.source.as_bytes(), mapped.declaration_span),
                formulabook: mapped.formulabook.clone(),
                name: mapped.formula.name.clone(),
                parameters: mapped.formula.params.clone(),
                source_sha256: loaded.hash.clone(),
            };
            if !names.insert(identity.name.clone()) {
                return Err(Failure::Audit(format!(
                    "duplicate formula export name: {}",
                    identity.name
                )));
            }
            if !identities.insert(identity.clone()) {
                return Err(Failure::Audit(format!(
                    "duplicate formula export identity: {}",
                    identity.name
                )));
            }
            let provenance = formula_provenance(&mapped.formula).map_err(|error| {
                Failure::Audit(format!(
                    "formula {} provenance failed: {error:?}",
                    identity.name
                ))
            })?;
            exports.push(ExportRecord {
                identity,
                provenance,
            });
        }
    }
    exports.sort_by(|left, right| left.identity.cmp(&right.identity));
    Ok(exports)
}

fn unique_export<'a>(
    provenance: &Provenance,
    exports: &'a [ExportRecord],
) -> Result<&'a ExportRecord, Failure> {
    let matches: Vec<_> = exports
        .iter()
        .filter(|candidate| candidate.provenance == *provenance)
        .collect();
    match matches.as_slice() {
        [only] => Ok(*only),
        [] => Err(Failure::Audit(format!(
            "formula provenance has no parser-backed export: {}",
            provenance.source
        ))),
        _ => Err(Failure::Audit(format!(
            "formula provenance maps ambiguously to {} exports: {}",
            matches.len(),
            provenance.source
        ))),
    }
}

fn build_imports(
    sources: &BTreeMap<String, LoadedSource>,
    edges: &[ResolvedImport],
) -> Result<Vec<ImportDto>, Failure> {
    let mut by_key: BTreeMap<(&str, &str), &ResolvedImport> = BTreeMap::new();
    for edge in edges {
        if by_key
            .insert((&edge.importer, &edge.literal), edge)
            .is_some()
        {
            return Err(Failure::Audit(format!(
                "ambiguous repeated import {:?} in one source",
                edge.literal
            )));
        }
    }
    let mut output = Vec::new();
    for (canonical, loaded) in sources {
        for import in &loaded.map.imports {
            let edge = by_key
                .get(&(canonical.as_str(), import.literal.as_str()))
                .ok_or_else(|| {
                    Failure::Audit(format!(
                        "parser import {:?} has no resolver edge",
                        import.literal
                    ))
                })?;
            let imported = sources.get(&edge.resolved).ok_or_else(|| {
                Failure::Audit(format!("resolved import was not loaded: {}", edge.resolved))
            })?;
            output.push(ImportDto {
                declaration: span(loaded.source.as_bytes(), import.declaration_span),
                imported_source_sha256: imported.hash.clone(),
                importer_source_sha256: loaded.hash.clone(),
                literal: import.literal.clone(),
            });
        }
    }
    output.sort_by(|left, right| {
        (&left.importer_source_sha256, left.declaration.start)
            .cmp(&(&right.importer_source_sha256, right.declaration.start))
    });
    Ok(output)
}

fn build_audit(
    root_id: &str,
    provider: &RecordingProvider,
    snapshots: &dyn SnapshotStore,
) -> Result<AuditDto, Failure> {
    let lowered = compile_with_imports(root_id, provider, ImportLimits::default())
        .map_err(|error| Failure::Audit(format!("compile failed: {error:?}")))?;
    let sources = build_sources(provider.sources.borrow().clone())?;
    let root = sources
        .get(root_id)
        .ok_or_else(|| Failure::Audit("root source was not recorded".to_string()))?;
    let exports = build_exports(&sources)?;
    let imports = build_imports(&sources, &provider.imports.borrow())?;

    let mut derivations = Vec::new();
    let mut used_questions = BTreeSet::new();
    for (index, derived) in lowered.kb.derived_bindings().iter().enumerate() {
        let plan = lowered
            .kb
            .computation_plan_for(derived)
            .ok_or_else(|| Failure::Audit(format!("{} has no trusted plan", derived.name)))?;
        if !plan.is_query_answer {
            continue;
        }
        let question_matches: Vec<_> = root
            .map
            .queries
            .iter()
            .filter(|question| {
                formula_name_arity(&question.conclusion).is_some_and(|(name, arity)| {
                    name == derived.name
                        && exports.iter().any(|candidate| {
                            candidate.identity.name == name
                                && candidate.identity.parameters.len() == arity
                        })
                })
            })
            .collect();
        let question = match question_matches.as_slice() {
            [only] => *only,
            _ => {
                return Err(Failure::Audit(format!(
                    "query result {} maps to {} source questions",
                    derived.name,
                    question_matches.len()
                )))
            }
        };
        if !used_questions.insert(question.declaration_span.start) {
            return Err(Failure::Audit(format!(
                "source question for {} was consumed twice",
                derived.name
            )));
        }

        let export_matches: Vec<_> = exports
            .iter()
            .filter(|candidate| {
                formula_name_arity(&question.conclusion).is_some_and(|(name, arity)| {
                    candidate.identity.name == name && candidate.identity.parameters.len() == arity
                })
            })
            .collect();
        let export = match export_matches.as_slice() {
            [only] => *only,
            _ => {
                return Err(Failure::Audit(format!(
                    "question {} maps to {} exports",
                    derived.name,
                    export_matches.len()
                )))
            }
        };

        let checked = verify_derived(derived, &lowered.kb, snapshots);
        if checked.name != derived.name || checked.is_query_answer != plan.is_query_answer {
            return Err(Failure::Audit(format!(
                "verification identity differs from trusted plan for {}",
                derived.name
            )));
        }
        if checked.formula_sources.len() != checked.formula_quotes.len() {
            return Err(Failure::Audit(format!(
                "formula identity/status count differs for {}",
                derived.name
            )));
        }
        if checked
            .formula_sources
            .iter()
            .zip(&checked.formula_quotes)
            .any(|(provenance, status)| !status_matches_quote_identity(provenance, status))
        {
            return Err(Failure::Audit(format!(
                "verified formula quote identity differs from its provenance for {}",
                derived.name
            )));
        }
        let formula_sequence: Vec<_> = checked
            .formula_sources
            .iter()
            .map(|provenance| unique_export(provenance, &exports).map(|item| item.identity.clone()))
            .collect::<Result<_, _>>()?;
        if formula_sequence.first() != Some(&export.identity) {
            return Err(Failure::Audit(format!(
                "outer formula identity differs from question export for {}",
                derived.name
            )));
        }

        let verified_ids: BTreeSet<_> = checked
            .input_quotes
            .iter()
            .map(|item| item.fact_id)
            .collect();
        if verified_ids.len() != checked.input_quotes.len() {
            return Err(Failure::Audit(format!(
                "duplicate input verification identity for {}",
                derived.name
            )));
        }
        let mut tree_ids = BTreeSet::new();
        collect_fact_ids(index, &lowered.kb, &mut BTreeSet::new(), &mut tree_ids)?;
        if tree_ids != verified_ids {
            return Err(Failure::Audit(format!(
                "derivation/verification input sets differ for {}",
                derived.name
            )));
        }
        let inputs: Vec<_> = checked
            .input_quotes
            .iter()
            .map(|item| fact_identity(&lowered.kb, item.fact_id))
            .collect::<Result<_, _>>()?;
        if checked.input_quotes.iter().any(|item| {
            lowered
                .kb
                .fact(item.fact_id)
                .is_none_or(|fact| !status_matches_quote_identity(&fact.provenance, &item.quote))
        }) {
            return Err(Failure::Audit(format!(
                "verified input quote identity differs from its provenance for {}",
                derived.name
            )));
        }
        let unique_inputs: BTreeSet<_> = inputs.iter().cloned().collect();
        if unique_inputs.len() != inputs.len() {
            return Err(Failure::Audit(format!(
                "consumed facts have ambiguous stable identities for {}",
                derived.name
            )));
        }

        let formula_quotes = checked
            .formula_sources
            .iter()
            .zip(formula_sequence.iter().cloned())
            .zip(checked.formula_quotes.iter().map(quote_status))
            .map(|((provenance, identity), quote)| FormulaCheckDto {
                identity,
                provenance: provenance_dto(provenance),
                quote,
            })
            .collect();
        let input_quotes = inputs
            .iter()
            .cloned()
            .zip(
                checked
                    .input_quotes
                    .iter()
                    .map(|item| quote_status(&item.quote)),
            )
            .map(|(identity, quote)| InputDto { identity, quote })
            .collect();
        derivations.push(DerivationDto {
            export: export.identity.clone(),
            formula_sequence,
            inputs,
            plan: PlanDto {
                expression: plan_expr(plan.expr),
                is_query_answer: plan.is_query_answer,
                scope: ScopeDto {
                    derived_limit: plan.scope.derived_limit,
                    fact_limit: plan.scope.fact_limit,
                },
            },
            question: QuestionDto {
                declaration: span(root.source.as_bytes(), question.declaration_span),
                name: derived.name.clone(),
                source_sha256: root.hash.clone(),
            },
            result: ResultDto {
                dimension: derived.dim.tag(),
                exact_rational: derived.exact.as_ref().map(rational),
                f64_bits: bits(derived.value),
            },
            tree: tree(&derived.tree, &lowered.kb)?,
            verification: VerificationDto {
                computation: computation_status(&checked.computation),
                formula_quotes,
                fully_verified: checked.fully_verified(),
                input_quotes,
                is_query_answer: checked.is_query_answer,
                passed: checked.passed(),
            },
        });
    }
    if derivations.is_empty() {
        return Err(Failure::Audit(
            "program produced no formula query answers".to_string(),
        ));
    }
    Ok(AuditDto {
        contract: "adj-lang/formula_audit/v1",
        derivations,
        imports,
        kind: "formula_execution_audit",
        root_source_sha256: root.hash.clone(),
        schema_version: 1,
    })
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>), Failure> {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "adj-formula-audit".to_string());
    let usage = || format!("usage: {program} [--snapshots DIR] PROGRAM.adj");
    let mut path = None;
    let mut snapshots = None;
    while let Some(arg) = args.next() {
        if arg == "--snapshots" {
            if snapshots.is_some() {
                return Err(Failure::Usage("--snapshots may appear once".to_string()));
            }
            snapshots = Some(args.next().ok_or_else(|| Failure::Usage(usage()))?.into());
        } else if path.replace(PathBuf::from(arg)).is_some() {
            return Err(Failure::Usage(usage()));
        }
    }
    Ok((path.ok_or_else(|| Failure::Usage(usage()))?, snapshots))
}

fn run() -> Result<(), Failure> {
    let (path, snapshots) = parse_args()?;
    let root_path = fs::canonicalize(&path)
        .map_err(|error| Failure::Usage(format!("cannot open {:?}: {error}", path)))?;
    if !fs::symlink_metadata(&root_path)
        .map_err(|error| Failure::Usage(format!("cannot inspect {:?}: {error}", path)))?
        .file_type()
        .is_file()
    {
        return Err(Failure::Usage(format!(
            "program is not a regular file: {:?}",
            path
        )));
    }
    let root_dir = root_path
        .parent()
        .ok_or_else(|| Failure::Usage(format!("program has no parent: {:?}", path)))?
        .to_path_buf();
    let root_id = root_path.to_string_lossy().into_owned();
    let provider = RecordingProvider {
        inner: FsProvider { root: root_dir },
        imports: RefCell::new(Vec::new()),
        sources: RefCell::new(BTreeMap::new()),
    };
    let empty = EmptySnapshots;
    let directory;
    let store: &dyn SnapshotStore = if let Some(root) = snapshots {
        directory = DirSnapshots {
            root,
            cache: RefCell::new(BTreeMap::new()),
        };
        &directory
    } else {
        &empty
    };
    let audit = build_audit(&root_id, &provider, store)?;
    let canonical = serde_json::to_value(&audit)
        .map_err(|error| Failure::Audit(format!("cannot construct JSON: {error}")))?;
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    serde_json::to_writer_pretty(&mut writer, &canonical)
        .map_err(|error| Failure::Usage(format!("cannot write JSON: {error}")))?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|error| Failure::Usage(format!("cannot write JSON: {error}")))?;
    Ok(())
}

/// Process entry point kept in the library so focused tests can exercise it.
pub fn main_entry() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Audit(error)) => {
            eprintln!("adj-formula-audit: {error}");
            ExitCode::from(1)
        }
        Err(Failure::Usage(error)) => {
            eprintln!("adj-formula-audit: {error}");
            ExitCode::from(2)
        }
    }
}
