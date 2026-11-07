//! # Warrant Rust Binding Schema and Validator (VER04d §5 / VER-P007)
//!
//! Enforces that real public exports correspond to abstract Warrant operations,
//! validating signature compatibility, freshness assertions, negative controls,
//! error normalizations, and closed failure codes (`W0501`, `W0502`).

use std::collections::BTreeMap;

use crate::checker::{CheckedWarrantModule, CheckerError, CheckerErrorKind, DiagnosticCode, Type};

/// Closed error normalization rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorNormalization {
    pub kind: String, // e.g. "panic_to_error"
    pub error_case: String,
}

/// Binding specification for a single Warrant public operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpBinding {
    pub export: String,
    pub static_params: Vec<String>,
    pub fresh: bool,
    pub error_normalization: Option<ErrorNormalization>,
    pub negative_controls: Vec<String>,
}

/// Complete host binding manifest for a Warrant module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingManifest {
    pub package: String,
    pub revision: String,
    pub bindings: BTreeMap<String, OpBinding>,
}

/// Validates that a host binding manifest correctly and completely covers a checked Warrant module.
pub fn validate_bindings(
    module: &CheckedWarrantModule,
    manifest: &BindingManifest,
) -> Result<(), CheckerError> {
    // 1. Every op declared in the Warrant document must have a binding (W0501)
    for (op_name, op) in &module.operations {
        let binding = manifest.bindings.get(op_name).ok_or_else(|| CheckerError {
            code: DiagnosticCode::W0501,
            kind: CheckerErrorKind::MissingBinding(format!(
                "Op {op_name} is declared in Warrant but has no binding in manifest"
            )),
            span: op.span,
        })?;

        // 2. Freshness requirement: if op declared fresh, binding must assert fresh
        if op.is_fresh && !binding.fresh {
            return Err(CheckerError {
                code: DiagnosticCode::W0502,
                kind: CheckerErrorKind::MissingFreshRequirement(format!(
                    "Op {op_name} is declared fresh in Warrant, but binding does not specify fresh: true"
                )),
                span: op.span,
            });
        }

        // 3. Every bound op must specify at least one negative control (W0502)
        if binding.negative_controls.is_empty() {
            return Err(CheckerError {
                code: DiagnosticCode::W0502,
                kind: CheckerErrorKind::MissingNegativeControl(format!(
                    "Binding for {op_name} must declare at least one negative control"
                )),
                span: op.span,
            });
        }

        // 4. Static parameters must match
        if binding.static_params.len() != op.static_params.len() {
            return Err(CheckerError {
                code: DiagnosticCode::W0501,
                kind: CheckerErrorKind::InvalidContract(format!(
                    "Binding for {op_name} static param count ({}) != op static params ({})",
                    binding.static_params.len(),
                    op.static_params.len()
                )),
                span: op.span,
            });
        }

        // 5. Error normalization validation
        if let Some(norm) = &binding.error_normalization {
            match &op.return_type {
                Type::Result(_, err_name) => {
                    let err_decl = module.errors.get(err_name).ok_or_else(|| CheckerError {
                        code: DiagnosticCode::W0501,
                        kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                            "Op {op_name} return error type {err_name} is not defined"
                        )),
                        span: op.span,
                    })?;

                    if !err_decl.variants.contains(&norm.error_case) {
                        return Err(CheckerError {
                            code: DiagnosticCode::W0501,
                            kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                                "Normalized error case {} is not a variant of error type {err_name}",
                                norm.error_case
                            )),
                            span: op.span,
                        });
                    }
                }
                _ => {
                    return Err(CheckerError {
                        code: DiagnosticCode::W0501,
                        kind: CheckerErrorKind::InvalidErrorNormalization(format!(
                            "Op {op_name} does not return Result, cannot declare error normalization"
                        )),
                        span: op.span,
                    });
                }
            }
        }
    }

    // 6. Manifest bindings cannot refer to unknown ops (W0501)
    for binding_op_name in manifest.bindings.keys() {
        if !module.operations.contains_key(binding_op_name) {
            return Err(CheckerError {
                code: DiagnosticCode::W0501,
                kind: CheckerErrorKind::UnknownBoundOperation(format!(
                    "Binding manifest specifies op {binding_op_name} not found in Warrant module"
                )),
                span: (0, 0),
            });
        }
    }

    Ok(())
}
