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
//! It is a *view*, not the `corpus.json` of slice S3c: it has no digests and no `requires`
//! and it prints the operation rows without the S3 addition `requires` (the oracle has
//! none); `admit` is the op's refinements as IR, lowered by the oracle's own typer in the
//! dump script.

use crate::cost::{suite_cost, ModelCosts};
use crate::error::{code, GenError};
use crate::ir::{Ir, MAX_IR_DEPTH};
use crate::json::Json;
use crate::lower::{Lowered, SuiteKind};

/// The `schema` string of the view.
pub const VIEW_SCHEMA: &str = "warrant-lowering-parity-v0";

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
        suites.push(Json::obj([
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
        ]));
    }
    let mut examples = Vec::new();
    for x in &lowered.examples {
        let at = |e: GenError| e.at(&x.name);
        let r = &x.reference;
        let (subject, expected, result) = (
            ir_json(&r.subject).map_err(at)?,
            ir_json(&r.expected).map_err(at)?,
            r.result.name(),
        );
        examples.push(Json::obj([
            ("name", Json::str(x.name.as_str())),
            ("body", ir_json(&x.body).map_err(at)?),
            ("subject", subject),
            ("expected", expected),
            ("result_type", Json::str(result)),
        ]));
    }
    Ok(Json::obj([
        ("schema", Json::str(VIEW_SCHEMA)),
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
    ]))
}
