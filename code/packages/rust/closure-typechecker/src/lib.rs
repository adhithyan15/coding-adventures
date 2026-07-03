//! JavaScript type checker (Closure-style).
//!
//! Consumes a [`Program`] from `javascript-ast` and a [`Sidecar`] from
//! `type-sidecar` and produces typed judgments per
//! [CLOC06](../../../specs/CLOC06-pass-interface-contract.md). Diagnostic
//! reporting follows
//! [CLOC08](../../../specs/CLOC08-closurec-cli-surface.md)'s severity
//! model (`Error` / `Warning` / `Note` + `DiagnosticGroup`).
//!
//! # Scope (v1)
//!
//! v1 is **passthrough**: each AST node that carries a CV ID and has a
//! matching sidecar [`Record`] gets that record's `ty` copied into the
//! returned [`CheckResult::judgments`] map. No actual inference yet —
//! the API surface is what's being established. Once `javascript-ast`
//! grows `Statement` / `Expression` variants (deferred from CLOC02
//! Phase 1), the inference engine slots in here.
//!
//! Even without inference, this scaffolding does real work:
//!
//! 1. Establishes the [`check`] API the future
//!    `closurec` CLI will call.
//! 2. Plumbs CV [`Contribution`]s per CLOC03 §"Stage 3 — Typechecker"
//!    so every judged node carries a `"judged"` tag in the log.
//! 3. Pins the [`Diagnostic`] / [`Severity`] / [`DiagnosticGroup`]
//!    types so passes and the CLI can be written against them now.

use std::collections::HashMap;

use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::Program;
use coding_adventures_type_sidecar::{Sidecar, Type};

/// Result of a [`check`] call.
#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    /// One entry per AST node we have a type for, keyed by the node's
    /// correlation-vector ID. v1 fills this with `ty` values copied
    /// from the input [`Sidecar`].
    pub judgments: HashMap<String, Type>,

    /// Diagnostics surfaced during checking. v1 emits nothing here —
    /// the inference engine is deferred — but the type is pinned so
    /// downstream consumers (the CLI per CLOC08) can be written
    /// against it.
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    /// Construct an empty result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is `cv` known to the checker?
    pub fn has_judgment(&self, cv: &str) -> bool {
        self.judgments.contains_key(cv)
    }

    /// The resolved type for `cv`, if any.
    pub fn judgment(&self, cv: &str) -> Option<&Type> {
        self.judgments.get(cv)
    }
}

/// One diagnostic emitted by the checker. Shape matches CLOC08's
/// severity model; the future `closurec` CLI will render these into
/// the standard rustc-style output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Correlation-vector ID of the AST node the diagnostic is about.
    pub cv: String,
    /// How serious it is.
    pub severity: Severity,
    /// Which diagnostic group it belongs to (used by
    /// `--strict-group` / `--allow-group` / `--quiet-group` per
    /// CLOC08).
    pub group: DiagnosticGroup,
    /// Human-readable one-line message.
    pub message: String,
}

/// Diagnostic severity per CLOC08.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Compile-stopping.
    Error,
    /// Surfaced but doesn't fail the compile.
    Warning,
    /// Informational — attached to other diagnostics for context.
    Note,
}

/// Diagnostic group name. Free-form for now (free strings let the
/// inference engine introduce groups without coordinating). Once the
/// set stabilises we can switch to an enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticGroup(pub String);

impl DiagnosticGroup {
    /// Construct from a static or owned string.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Check `program` against `sidecar`, appending CV contributions to
/// `cv`. Returns a [`CheckResult`] containing the judgments and any
/// diagnostics.
///
/// v1 is passthrough:
///
/// - The `Program`'s root CV is looked up in the sidecar.
/// - If a record exists and has a concrete `ty`, it lands in
///   [`CheckResult::judgments`] and a `"judged"` `Contribution` is
///   appended to the CV log per CLOC03 §"Stage 3 — Typechecker".
/// - Records with `ty = None` are not copied (no judgment).
///
/// Once `javascript-ast` grows `Statement` / `Expression` variants,
/// this routine extends to walk every node, doing real inference
/// against the sidecar.
pub fn check(program: &Program, sidecar: &Sidecar, cv: &mut CVLog) -> CheckResult {
    let mut result = CheckResult::new();
    // The Program's cv field is optional (CLOC09 amendment). If the
    // caller constructed the Program with tracing disabled, there's no
    // CV id to look up in the sidecar and therefore no judgment to
    // make on the program root — skip silently.
    if let Some(ref node_cv) = program.cv {
        judge_node(node_cv, sidecar, &mut result, cv);
    }
    result
}

/// Look up `node_cv` in the sidecar and, if it carries a typed record,
/// add the judgment to `result` and append a CV contribution.
fn judge_node(node_cv: &str, sidecar: &Sidecar, result: &mut CheckResult, cv: &mut CVLog) {
    let Some(record) = sidecar.get(&node_cv.to_string()) else {
        return;
    };
    let Some(ty) = record.ty.clone() else {
        // Producer explicitly has no opinion. Nothing to judge.
        return;
    };

    // Record the judgment for downstream consumers.
    result.judgments.insert(node_cv.to_string(), ty.clone());

    // Per CLOC03 §"Stage 3 — Typechecker", append a tagged
    // contribution. We use `tag = "judged"` and stash the resolved
    // type name in meta for debuggability.
    let mut meta = std::collections::HashMap::new();
    meta.insert(
        "type".to_string(),
        serde_json::Value::String(type_label(&ty)),
    );
    // The error path (Err) only fires when the entity is deleted; we
    // just created the typechecker judgment so this can't happen here.
    let _ = cv.contribute(node_cv, "typechecker", "judged", meta);
}

/// Human-readable label for a [`Type`] used in `Contribution.meta`.
fn type_label(ty: &Type) -> String {
    match ty {
        Type::Never => "Never".into(),
        Type::Unknown => "Unknown".into(),
        Type::Any => "Any".into(),
        Type::Undefined => "Undefined".into(),
        Type::Null => "Null".into(),
        Type::Boolean => "Boolean".into(),
        Type::Number => "Number".into(),
        Type::BigInt => "BigInt".into(),
        Type::String => "String".into(),
        Type::Symbol => "Symbol".into(),
        Type::Opaque { raw } => format!("Opaque({raw})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_ast::SourceType;
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::{Attributes, EvidenceStep, ProducerId, Provenance, Record};

    fn program_with_cv(cv: &str) -> Program {
        Program::new(cv.to_string(), EsVersion::Es2025, SourceType::Module)
    }

    fn record(cv: &str, ty: Option<Type>) -> Record {
        Record {
            cv: cv.to_string(),
            ty,
            attributes: Attributes::default(),
            provenance: Provenance {
                producer: ProducerId::new("test"),
                producer_version: "0.0.0".into(),
                source_file: None,
                source_location: None,
                generated_at: None,
                evidence: vec![EvidenceStep {
                    stage: "test".into(),
                    note: "test fixture".into(),
                    at: None,
                }],
            },
        }
    }

    #[test]
    fn empty_sidecar_yields_no_judgments() {
        let program = program_with_cv("program.1");
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let result = check(&program, &sidecar, &mut cv);
        assert!(result.judgments.is_empty());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn record_with_ty_produces_judgment() {
        let program = program_with_cv("program.1");
        let mut sidecar = Sidecar::new();
        sidecar.insert(record("program.1", Some(Type::Number)));
        let mut cv = CVLog::new(true);

        let result = check(&program, &sidecar, &mut cv);

        assert!(result.has_judgment("program.1"));
        assert_eq!(result.judgment("program.1"), Some(&Type::Number));
    }

    #[test]
    fn record_with_no_ty_produces_no_judgment() {
        let program = program_with_cv("program.1");
        let mut sidecar = Sidecar::new();
        sidecar.insert(record("program.1", None));
        let mut cv = CVLog::new(true);

        let result = check(&program, &sidecar, &mut cv);

        assert!(!result.has_judgment("program.1"));
    }

    #[test]
    fn judged_node_gets_cv_contribution() {
        // CLOC03 §Stage 3 contract: every judged node must get a
        // "judged" Contribution in the CV log so downstream
        // source-map and debug tooling can explain the type.
        //
        // The CV crate generates IDs from origins; we can't pin
        // `program.1` directly. Instead: mint a real ID with
        // `cv.create()`, use that ID for both the Program and the
        // sidecar Record, then assert the contribution lands on it.
        let mut cv = CVLog::new(true);
        let prog_cv = cv.create(None);

        let program = program_with_cv(&prog_cv);
        let mut sidecar = Sidecar::new();
        sidecar.insert(record(&prog_cv, Some(Type::String)));

        let _result = check(&program, &sidecar, &mut cv);

        let history = cv.history(&prog_cv);
        assert!(
            history
                .iter()
                .any(|c| c.source == "typechecker" && c.tag == "judged"),
            "expected a typechecker/judged contribution; got {history:?}"
        );
        // And the meta payload should carry the resolved type name.
        let judged = history
            .iter()
            .find(|c| c.source == "typechecker" && c.tag == "judged")
            .unwrap();
        assert_eq!(judged.meta.get("type").and_then(|v| v.as_str()), Some("String"));
    }

    #[test]
    fn no_judgment_when_sidecar_missing_cv() {
        let program = program_with_cv("program.1");
        let mut sidecar = Sidecar::new();
        sidecar.insert(record("other.1", Some(Type::Number)));
        let mut cv = CVLog::new(true);

        let result = check(&program, &sidecar, &mut cv);
        assert!(result.judgments.is_empty());
    }

    #[test]
    fn check_result_accessors_work() {
        let mut r = CheckResult::new();
        assert!(!r.has_judgment("x"));
        assert!(r.judgment("x").is_none());

        r.judgments.insert("x".into(), Type::Boolean);
        assert!(r.has_judgment("x"));
        assert_eq!(r.judgment("x"), Some(&Type::Boolean));
    }

    #[test]
    fn diagnostic_types_are_buildable() {
        // Pin the Diagnostic API surface so downstream consumers can
        // be written against it before inference fills it in.
        let d = Diagnostic {
            cv: "x.1".into(),
            severity: Severity::Warning,
            group: DiagnosticGroup::new("missing-types"),
            message: "implicit any in parameter `id`".into(),
        };
        assert_eq!(d.severity, Severity::Warning);
        assert_eq!(d.group, DiagnosticGroup::new("missing-types"));
        assert!(d.message.contains("any"));
    }

    #[test]
    fn type_label_renders_primitives() {
        assert_eq!(type_label(&Type::Number), "Number");
        assert_eq!(type_label(&Type::String), "String");
        assert_eq!(type_label(&Type::Undefined), "Undefined");
    }

    #[test]
    fn type_label_renders_opaque_with_raw() {
        assert_eq!(
            type_label(&Type::Opaque {
                raw: "Foo<Bar>".into()
            }),
            "Opaque(Foo<Bar>)"
        );
    }

    #[test]
    fn check_with_disabled_cv_log_still_works() {
        // CLOC03 production fast path: log disabled, but checker
        // shouldn't panic and should still produce judgments.
        let program = program_with_cv("p.1");
        let mut sidecar = Sidecar::new();
        sidecar.insert(record("p.1", Some(Type::Boolean)));
        let mut cv = CVLog::new(false);

        let result = check(&program, &sidecar, &mut cv);
        assert_eq!(result.judgment("p.1"), Some(&Type::Boolean));
    }
}
