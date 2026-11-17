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
//!     "ops":     {name: {"params": [[n,T]..], "ret": T, ["width_params"], ["fresh"]}},
//!     "model_order": [names...],
//!     "models":  {name: {"params": [[n,T]..], "ret": T, "body": IR, "ir_cost": N}},
//!     "suites":  [{"id","claim","kind","binders","admit","body","subject","expected",
//!                  "result_type","ops","ir_cost"}...],
//!     "examples":[{"name","body","subject","expected","result_type"}...]
//!   }
//! ```
//!
//! It is a *view*, not the `corpus.json` of slice S3c: it has no digests and no `requires`
//! and it prints the oracle's operation rows (without the S3 additions `requires` and
//! `admit`), which is the part the two readers share.

use crate::cost::{suite_cost, ModelCosts};
use crate::error::GenError;
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

/// Render the parity view of `lowered`, with costs computed against `limit`.
///
/// # Errors
///
/// A cost-limit refusal, as [`ModelCosts::compute`] and [`suite_cost`].
pub fn oracle_view(lowered: &Lowered, limit: u64) -> Result<Json, GenError> {
    let costs = ModelCosts::compute(&lowered.models, limit)?;
    let errors = Json::obj(
        lowered
            .errors
            .iter()
            .map(|e| (e.name.clone(), Json::strs(e.cases.iter()))),
    );
    let ops = Json::obj(lowered.ops.iter().map(|op| {
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
        (op.name.clone(), Json::Obj(row))
    }));
    let models = Json::obj(lowered.models.iter().map(|m| {
        (
            m.name.clone(),
            Json::obj([
                ("params", params_json(&m.params)),
                ("ret", Json::str(m.ret.name())),
                ("body", m.body.to_json()),
                ("ir_cost", Json::Int(costs.cost(&m.name).unwrap_or(0))),
            ]),
        )
    }));
    let mut suites = Vec::new();
    for s in &lowered.suites {
        let sc = suite_cost(s, &costs, limit)?;
        let (kind, subject, expected, result) = match &s.kind {
            SuiteKind::Reference {
                subject,
                expected,
                result,
            } => (
                "reference",
                subject.to_json(),
                expected.to_json(),
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
            ("admit", s.admit.to_json()),
            ("body", s.body.to_json()),
            ("subject", subject),
            ("expected", expected),
            ("result_type", Json::str(result)),
            ("ops", Json::strs(s.ops.iter())),
            ("ir_cost", Json::Int(sc.total)),
        ]));
    }
    let examples = Json::arr(lowered.examples.iter().map(|x| {
        let (subject, expected, result) = match &x.reference {
            Some(r) => (r.subject.to_json(), r.expected.to_json(), r.result.name()),
            None => (Json::Arr(vec![]), Json::Arr(vec![]), String::new()),
        };
        Json::obj([
            ("name", Json::str(x.name.as_str())),
            ("body", x.body.to_json()),
            ("subject", subject),
            ("expected", expected),
            ("result_type", Json::str(result)),
        ])
    }));
    Ok(Json::obj([
        ("schema", Json::str(VIEW_SCHEMA)),
        ("module", Json::str(lowered.module.as_str())),
        ("document", Json::str(lowered.document())),
        ("errors", errors),
        ("ops", ops),
        (
            "model_order",
            Json::strs(lowered.models.iter().map(|m| m.name.as_str())),
        ),
        ("models", models),
        ("suites", Json::arr(suites)),
        ("examples", examples),
    ]))
}
