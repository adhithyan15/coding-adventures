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

/// A parsed `evidence.bindings` file: the binding manifest plus every
/// generation parameter, so one data file (not Rust code) tells the evidence
/// generator how to bind a package (VER04 section 6, VER04h section 1).
///
/// ```text
/// # comment
/// package coding_adventures_ct_compare
/// revision 0.1.0
/// spec code/specs/ct-compare.warrant
/// output code/packages/rust/ct-compare/tests/generated/warrant_evidence.rs
/// seed 0x57415252414e5421
/// property_cases 4096
/// fuzz_iterations 5000
/// negctl_fuzz_iterations 1000
/// exhaustive_budget 400000
/// op eq export=ct_eq faults=invert,ignore-last-byte
/// op eq_fixed export=ct_eq_fixed static=N faults=invert,ignore-last-byte
/// op select_bytes export=ct_select_bytes fresh error=CT_SELECT_LENGTH_MISMATCH faults=swap-choice,missing-error
/// op power export=power args=u8,u32 faults=exp-mod-256
/// ```
///
/// Keys other than `op` appear once.  `op` options: `export=` (required),
/// `static=A,B`, bare `fresh`, `error=CASE` (panic-to-error normalization),
/// `args=u8,u32` (Rust parameter types of a scalar export), `faults=a,b`
/// (required, at least one).  Anything unknown is an error: a typo must not
/// silently weaken the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBindingFile {
    pub manifest: BindingManifest,
    pub spec: String,
    pub output: String,
    pub seed: u64,
    pub property_cases: u64,
    pub fuzz_iterations: u64,
    pub negctl_fuzz_iterations: u64,
    pub exhaustive_budget: u64,
    pub arg_types: BTreeMap<String, Vec<String>>,
}

fn parse_u64(text: &str) -> Result<u64, String> {
    let t = text.replace('_', "");
    let parsed = match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => t.parse::<u64>(),
    };
    parsed.map_err(|_| format!("`{text}` is not an unsigned integer"))
}

/// Parse an `evidence.bindings` file (see [`EvidenceBindingFile`]).
pub fn parse_evidence_bindings(text: &str) -> Result<EvidenceBindingFile, String> {
    let mut package = None;
    let mut revision = None;
    let mut spec = None;
    let mut output = None;
    let mut seed = None;
    let mut property_cases = None;
    let mut fuzz_iterations = None;
    let mut negctl_fuzz_iterations = None;
    let mut exhaustive_budget = None;
    let mut bindings = BTreeMap::new();
    let mut arg_types = BTreeMap::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |m: String| format!("line {}: {m}", n + 1);
        let (key, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        let once = |slot: &mut Option<String>| -> Result<(), String> {
            if slot.is_some() || rest.is_empty() {
                return Err(at(format!("`{key}` needs exactly one value, once")));
            }
            *slot = Some(rest.to_string());
            Ok(())
        };
        let num = |slot: &mut Option<u64>| -> Result<(), String> {
            if slot.is_some() {
                return Err(at(format!("`{key}` appears twice")));
            }
            *slot = Some(parse_u64(rest).map_err(at)?);
            Ok(())
        };
        match key {
            "package" => once(&mut package)?,
            "revision" => once(&mut revision)?,
            "spec" => once(&mut spec)?,
            "output" => once(&mut output)?,
            "seed" => num(&mut seed)?,
            "property_cases" => num(&mut property_cases)?,
            "fuzz_iterations" => num(&mut fuzz_iterations)?,
            "negctl_fuzz_iterations" => num(&mut negctl_fuzz_iterations)?,
            "exhaustive_budget" => num(&mut exhaustive_budget)?,
            "op" => {
                let mut words = rest.split_whitespace();
                let name = words.next().ok_or_else(|| at("`op` needs a name".into()))?;
                let mut export = None;
                let mut static_params = Vec::new();
                let mut fresh = false;
                let mut error = None;
                let mut faults = Vec::new();
                let mut args = None;
                for w in words {
                    let (k, v) = w.split_once('=').unwrap_or((w, ""));
                    let list = || -> Vec<String> {
                        v.split(',').filter(|s| !s.is_empty()).map(String::from).collect()
                    };
                    match k {
                        "export" if !v.is_empty() => export = Some(v.to_string()),
                        "static" if !v.is_empty() => static_params = list(),
                        "fresh" if v.is_empty() => fresh = true,
                        "error" if !v.is_empty() => error = Some(v.to_string()),
                        "args" if !v.is_empty() => args = Some(list()),
                        "faults" if !v.is_empty() => faults = list(),
                        _ => return Err(at(format!("op `{name}`: unknown or malformed option `{w}`"))),
                    }
                }
                let export = export.ok_or_else(|| at(format!("op `{name}` has no export=")))?;
                if faults.is_empty() {
                    return Err(at(format!("op `{name}` needs faults= (at least one)")));
                }
                if let Some(a) = args {
                    arg_types.insert(name.to_string(), a);
                }
                let dup = bindings.insert(
                    name.to_string(),
                    OpBinding {
                        export,
                        static_params,
                        fresh,
                        error_normalization: error.map(|c| ErrorNormalization {
                            kind: "panic_to_error".into(),
                            error_case: c,
                        }),
                        negative_controls: faults,
                    },
                );
                if dup.is_some() {
                    return Err(at(format!("op `{name}` is bound twice")));
                }
            }
            other => return Err(at(format!("unknown key `{other}`"))),
        }
    }
    let need = |v: Option<String>, k: &str| v.ok_or_else(|| format!("missing `{k}`"));
    let need_n = |v: Option<u64>, k: &str| v.ok_or_else(|| format!("missing `{k}`"));
    if bindings.is_empty() {
        return Err("no `op` lines".into());
    }
    Ok(EvidenceBindingFile {
        manifest: BindingManifest {
            package: need(package, "package")?,
            revision: need(revision, "revision")?,
            bindings,
        },
        spec: need(spec, "spec")?,
        output: need(output, "output")?,
        seed: need_n(seed, "seed")?,
        property_cases: need_n(property_cases, "property_cases")?,
        fuzz_iterations: need_n(fuzz_iterations, "fuzz_iterations")?,
        negctl_fuzz_iterations: need_n(negctl_fuzz_iterations, "negctl_fuzz_iterations")?,
        exhaustive_budget: exhaustive_budget.unwrap_or(400_000),
        arg_types,
    })
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
