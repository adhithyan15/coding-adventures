//! # The parity view of a lowered document
//!
//! The independent oracle (`python/warrant-oracle`) does not write a v2 `corpus.json` yet,
//! so "the lowering agrees with the oracle" has to be stated on something both can print.
//! That something is this **parity view**: every lowered model, operation, suite and
//! example, as canonical JSON, plus the worst-path costs.  The Rust side renders it with
//! [`oracle_view`]; `tests/fixtures/dump_oracle_lowering.py` renders the same view from the
//! oracle's own `lower_document`, and the test compares the two byte for byte.
//!
//! ```text
//!   {
//!     "document": "v0", "module": "ct_compare", "schema": "warrant-lowering-parity-v0",
//!     "errors":  {"SelectError": ["CT_SELECT_LENGTH_MISMATCH"]},
//!     "ops":     {name: {"params": [[n,T]..], "ret": T, ["width_params"], ["fresh"], ["admit"]}},
//!     "model_order": [names...],
//!     "models":  {name: {"params": [[n,T]..], "ret": T, "body": IR, "ir_cost": N}},
//!     "suites":  [{"id","claim","kind","binders","admit","body","subject","expected",
//!                  "result_type","ops","ir_cost"}...],
//!     "examples":[{"name","body","subject","expected","result_type"}...]
//!   }
//! ```
//!
//! It is a *view*, not the `corpus.json` of slice S3c: it has no digests.  The `v0` view has no
//! `requires` either; `admit` is the op's refinements as IR, lowered by the oracle's own typer
//! in the dump script.
//!
//! The `types` key is present **only** for a document that declares a record or a variant (slice S11b,
//! R59); see [`oracle_view_v1`].
//!
//! ## The `v1` view (slice S10b)
//!
//! [`oracle_view_v1`] is the schema `warrant-lowering-parity-v1` of the oracle's `dump-lowering`
//! (`python/warrant-oracle`, `lowering_dump.py`): the same view with the computed `requires`
//! (VER04r 16.10 R9) on every op row and every suite, for documents that use the `ir.fixed`
//! built-ins.  `tests/fixed_parity.rs` renders the Rust lowering of the oracle's committed
//! `fixed_forms_v1.warrant` and demands it equal the committed dump byte for byte.

use crate::cost::{suite_cost, ModelCosts};
use crate::error::{code, GenError};
use crate::ir::{Ir, MAX_IR_DEPTH};
use crate::json::Json;
use crate::lower::{Lowered, LoweredType, SuiteKind};

/// The `schema` string of the `v0` view.
pub const VIEW_SCHEMA: &str = "warrant-lowering-parity-v0";

/// The `schema` string of the `v1` view (adds `requires`, S10b).
pub const VIEW_SCHEMA_V1: &str = "warrant-lowering-parity-v1";

fn params_json(params: &[(String, crate::types::Ty)]) -> Json {
    Json::arr(
        params
            .iter()
            .map(|(n, t)| Json::arr([Json::str(n.as_str()), Json::str(t.name())])),
    )
}

/// Render one tree, refusing one deeper than the IR depth limit (rendering recurses).
fn ir_json(ir: &Ir) -> Result<Json, GenError> {
    if ir.depth() > MAX_IR_DEPTH {
        return Err(GenError::new(
            code::LIMIT,
            "",
            "IR nesting exceeds 64 levels",
        ));
    }
    Ok(ir.to_json())
}

/// Render the parity view of `lowered`, with costs computed against `limit`.
///
/// # Errors
///
/// A cost-limit or depth refusal, as [`ModelCosts::compute`] and [`suite_cost`].
pub fn oracle_view(lowered: &Lowered, limit: u64) -> Result<Json, GenError> {
    view(lowered, limit, false)
}

/// The `v1` parity view of `lowered` (see the module documentation): [`oracle_view`] plus
/// `requires` on every op row and every suite.  `lowered` may hold `ir.fixed` units (obtain it
/// from [`crate::lower::lower_document_with_fixed`]); their `ir_cost` is `1 +` the operands per
/// built-in, as the oracle's analysing `Program` computes it.
///
/// # Errors
///
/// A cost-limit or depth refusal, as [`ModelCosts::compute`] and [`suite_cost`].
#[doc(hidden)]
pub fn oracle_view_v1(lowered: &Lowered, limit: u64) -> Result<Json, GenError> {
    view(lowered, limit, true)
}

fn view(lowered: &Lowered, limit: u64, v1: bool) -> Result<Json, GenError> {
    let costs = ModelCosts::compute(&lowered.models, limit)?;
    let errors = Json::obj(
        lowered
            .errors
            .iter()
            .map(|e| (e.name.clone(), Json::strs(e.cases.iter()))),
    );
    let mut ops = Vec::new();
    for op in &lowered.ops {
        let mut row = vec![
            ("params".to_string(), params_json(&op.params)),
            ("ret".to_string(), Json::str(op.ret.name())),
        ];
        if !op.statics.is_empty() {
            row.push(("width_params".into(), Json::strs(op.statics.iter())));
        }
        if op.fresh {
            row.push(("fresh".into(), Json::Bool(true)));
        }
        if let Some(admit) = &op.admit {
            row.push(("admit".into(), ir_json(admit).map_err(|e| e.at(&op.name))?));
        }
        if v1 {
            row.push(("requires".into(), Json::strs(op.requires.iter())));
        }
        ops.push((op.name.clone(), Json::Obj(row)));
    }
    let mut models = Vec::new();
    for m in &lowered.models {
        models.push((
            m.name.clone(),
            Json::obj([
                ("params", params_json(&m.params)),
                ("ret", Json::str(m.ret.name())),
                ("body", ir_json(&m.body).map_err(|e| e.at(&m.name))?),
                ("ir_cost", Json::Int(costs.cost(&m.name).unwrap_or(0))),
            ]),
        ));
    }
    let mut suites = Vec::new();
    for s in &lowered.suites {
        let at = |e: GenError| e.at(&s.claim);
        let sc = suite_cost(s, &costs, limit)?;
        let (kind, subject, expected, result) = match &s.kind {
            SuiteKind::Reference {
                subject,
                expected,
                result,
            } => (
                "reference",
                ir_json(subject).map_err(at)?,
                ir_json(expected).map_err(at)?,
                result.name(),
            ),
            SuiteKind::Predicate => (
                "predicate",
                Json::Arr(vec![]),
                Json::Arr(vec![]),
                "Bool".to_string(),
            ),
        };
        // The oracle writes `null` for the absent sides of a predicate; the canonical
        // writer has no null, so both sides write an empty array instead.
        let mut row = vec![
            ("id", Json::str(s.id.as_str())),
            ("claim", Json::str(s.claim.as_str())),
            ("kind", Json::str(kind)),
            ("binders", params_json(&s.binders)),
            ("admit", ir_json(&s.admit).map_err(at)?),
            ("body", ir_json(&s.body).map_err(at)?),
            ("subject", subject),
            ("expected", expected),
            ("result_type", Json::str(result)),
            ("ops", Json::strs(s.ops.iter())),
            ("ir_cost", Json::Int(sc.total)),
        ];
        if v1 {
            row.push(("requires", Json::strs(s.requires.iter())));
        }
        suites.push(Json::obj(row));
    }
    let mut examples = Vec::new();
    for x in &lowered.examples {
        let at = |e: GenError| e.at(&x.name);
        // An example the profile omits (VER04r 8.4) has no decomposition: the oracle writes an
        // empty array for each absent side (the canonical writer has no null) and `""` for the
        // type.  Only a document lowered with `omit_example` lines has one (S10t-2).
        let (subject, expected, result) = match &x.reference {
            Some(r) => (
                ir_json(&r.subject).map_err(at)?,
                ir_json(&r.expected).map_err(at)?,
                r.result.name(),
            ),
            None => (Json::Arr(vec![]), Json::Arr(vec![]), String::new()),
        };
        examples.push(Json::obj([
            ("name", Json::str(x.name.as_str())),
            ("body", ir_json(&x.body).map_err(at)?),
            ("subject", subject),
            ("expected", expected),
            ("result_type", Json::str(result)),
        ]));
    }
    let mut root = vec![
        (
            "schema",
            Json::str(if v1 { VIEW_SCHEMA_V1 } else { VIEW_SCHEMA }),
        ),
        ("module", Json::str(lowered.module.as_str())),
        ("document", Json::str(lowered.document())),
        ("errors", errors),
        ("ops", Json::Obj(ops)),
        (
            "model_order",
            Json::strs(lowered.models.iter().map(|m| m.name.as_str())),
        ),
        ("models", Json::Obj(models)),
        ("suites", Json::arr(suites)),
        ("examples", Json::arr(examples)),
    ];
    // The corpus `types` rows of the records and variants (VER04r 2.2), **only for a document that
    // declares one** (R59), so every older dump keeps its bytes.  No error type is here (those stay under
    // `errors`); the writer sorts the keys, the order inside a row is the declared one.
    if !lowered.types.is_empty() {
        root.push((
            "types",
            Json::obj(lowered.types.iter().map(|t| (t.name().to_string(), type_row(t)))),
        ));
    }
    Ok(Json::obj(root))
}

/// One `types` row: `{"kind": "record", "fields": [[name, type], ..]}` or
/// `{"kind": "variant", "ctors": [[ctor, [type, ..]], ..]}`.
pub(crate) fn type_row(t: &LoweredType) -> Json {
    match t {
        LoweredType::Record { fields, .. } => Json::obj([
            ("kind", Json::str("record")),
            (
                "fields",
                Json::arr(
                    fields
                        .iter()
                        .map(|(n, ty)| Json::arr([Json::str(n.as_str()), Json::str(ty.name())])),
                ),
            ),
        ]),
        LoweredType::Variant { ctors, .. } => Json::obj([
            ("kind", Json::str("variant")),
            (
                "ctors",
                Json::arr(ctors.iter().map(|(c, tys)| {
                    Json::arr([
                        Json::str(c.as_str()),
                        Json::arr(tys.iter().map(|ty| Json::str(ty.name()))),
                    ])
                })),
            ),
        ]),
    }
}
