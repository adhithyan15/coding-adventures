//! # Warrant-derived evidence harness generator (VER04h / VER-P030)
//!
//! Turns a checked Warrant document into one Rust test target (to be run with
//! `harness = false` on `coding_adventures_test_bootstrap`) containing, for
//! every `example` and every `claim`:
//!
//! * the example as an L0 case,
//! * a bounded-exhaustive suite over a finite domain,
//! * an L2 property suite with relation-biased generators,
//! * a coverage-guided fuzz suite,
//! * negative controls (the same suites re-run with an injected adapter fault
//!   must fail),
//!
//! and writes an evidence manifest when the target runs.
//!
//! ## The oracle is the claim
//!
//! Earlier generators hard-coded `a == b` as the expected value.  Here the
//! claim body and the `model` functions are *compiled*: every supported
//! expression form is translated to Rust over a small dynamic value type `V`
//! (see `evidence_prelude.rs.txt`).  An expression form the translator does not
//! know is a generation error (`W0701`), never a silent skip.
//!
//! ## Determinism
//!
//! The output is a pure function of the checked document, the binding
//! manifest and [`EvidenceOptions`]; the BUILD regeneration check compares the
//! checked-in file with this function's output byte for byte.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

use crate::binding::{validate_bindings, BindingManifest};
use crate::checker::CheckedWarrantModule;
use crate::codegen::{
    find_operation_in_expr, hex_decode, CodegenError, CodegenErrorKind, DiagnosticCode,
};
use crate::parser::{
    BinaryOp, Declaration, Expression, Literal, OpDecl, Parameter, TypeExpr, UnaryOp,
    WarrantFile, Width,
};

/// The runtime prelude embedded verbatim in every generated harness.
const PRELUDE: &str = include_str!("evidence_prelude.rs.txt");

/// Maximum size of the Cartesian product of binder domains (VER04h section 3).
pub const EXHAUSTIVE_BUDGET: u64 = 400_000;

/// Generation parameters. Every value is echoed into the manifest or the
/// generated constants, so changing one changes the checked-in file.
#[derive(Clone, Debug)]
pub struct EvidenceOptions {
    pub package_crate: String,
    pub spec_path: String,
    pub spec_sha256: String,
    pub seed: u64,
    pub property_cases: u64,
    pub fuzz_iterations: u64,
    pub negctl_fuzz_iterations: u64,
}

fn unsupported(msg: impl Into<String>, span: (usize, usize)) -> CodegenError {
    CodegenError {
        code: DiagnosticCode::W0701,
        kind: CodegenErrorKind::UnsupportedTarget(msg.into()),
        span,
    }
}

fn span_of(e: &Expression<'_>) -> (usize, usize) {
    match e {
        Expression::Quantified { span, .. }
        | Expression::Conditional { span, .. }
        | Expression::Match { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Literal { span, .. }
        | Expression::List { span, .. }
        | Expression::Reference { span, .. }
        | Expression::Grouped { span, .. } => *span,
    }
}

// ---------------------------------------------------------------------------
// Binder types and domain sizes
// ---------------------------------------------------------------------------

/// Generator-side binder type (mirrors `Ty` in the prelude).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ty {
    Bool,
    U8,
    U64,
    Nat,
    Bytes,
    BytesN(u64),
}

impl Ty {
    fn rust(self) -> String {
        match self {
            Ty::Bool => "Ty::Bool".into(),
            Ty::U8 => "Ty::U8".into(),
            Ty::U64 => "Ty::U64".into(),
            Ty::Nat => "Ty::Nat".into(),
            Ty::Bytes => "Ty::Bytes".into(),
            Ty::BytesN(n) => format!("Ty::BytesN({n})"),
        }
    }

    /// Closed-form size of the domain the prelude enumerates (VER04h section 3).
    fn domain_size(self) -> u64 {
        match self {
            Ty::Bool => 2,
            Ty::U8 => 256,
            Ty::Nat => 65,
            Ty::U64 => 256 + 56 + 1 + 64,
            Ty::Bytes => 1 + 256 + 16 + 64 + 10,
            Ty::BytesN(0) => 1,
            Ty::BytesN(1) => 256,
            Ty::BytesN(n) if n <= 3 => 4u64.pow(n as u32),
            Ty::BytesN(n) => 4 + 3 * n,
        }
    }
}

fn ty_of(p: &Parameter<'_>) -> Result<Ty, CodegenError> {
    match &p.param_type {
        TypeExpr::Bytes { width: None, .. } => Ok(Ty::Bytes),
        TypeExpr::Bytes {
            width: Some(Width::Literal(n)),
            ..
        } => Ok(Ty::BytesN(*n)),
        TypeExpr::Named { name, .. } => match *name {
            "Bytes" => Ok(Ty::Bytes),
            "Bool" => Ok(Ty::Bool),
            "U8" => Ok(Ty::U8),
            "U64" => Ok(Ty::U64),
            "Nat" => Ok(Ty::Nat),
            other => Err(unsupported(format!("binder type {other}"), p.span)),
        },
        _ => Err(unsupported(
            format!("binder `{}` has a type the generator cannot enumerate", p.name),
            p.span,
        )),
    }
}

// ---------------------------------------------------------------------------
// Expression translation
// ---------------------------------------------------------------------------

struct Cx<'c> {
    binders: HashSet<String>,
    models: &'c HashSet<String>,
    ops: &'c HashMap<String, usize>,
    errors: &'c HashSet<String>,
    allow_ops: bool,
    widths: &'c mut BTreeMap<String, BTreeSet<u64>>,
}

fn emit(e: &Expression<'_>, cx: &mut Cx<'_>) -> Result<String, CodegenError> {
    let sp = span_of(e);
    Ok(match e {
        Expression::Grouped { inner, .. } => emit(inner, cx)?,
        Expression::Literal { lit, .. } => match lit {
            Literal::Bool(b) => format!("V::Bool({b})"),
            Literal::U64(n) => format!("V::U64({n}u64)"),
            Literal::U8(n) => format!("V::Nat({n}u64)"),
            Literal::Int(n) => format!("V::Nat({n}u64)"),
            Literal::Bytes(hex) => {
                let bytes = hex_decode(hex, sp)?;
                if bytes.is_empty() {
                    "V::Bytes(Vec::new())".into()
                } else {
                    let items: Vec<String> = bytes.iter().map(|b| format!("0x{b:02x}")).collect();
                    format!("V::Bytes(vec![{}])", items.join(", "))
                }
            }
        },
        Expression::Unary { op, operand, .. } => match op {
            UnaryOp::Not => format!("V::Bool(!{}.truthy())", emit(operand, cx)?),
            UnaryOp::Neg => return Err(unsupported("unary minus", sp)),
        },
        Expression::Binary {
            op, left, right, ..
        } => {
            let l = emit(left, cx)?;
            let r = emit(right, cx)?;
            match op {
                BinaryOp::Eq => format!("V::Bool({l} == {r})"),
                BinaryOp::NotEq => format!("V::Bool({l} != {r})"),
                BinaryOp::Lt => format!("V::compare(&{l}, &{r}, \"<\")"),
                BinaryOp::Gt => format!("V::compare(&{l}, &{r}, \">\")"),
                BinaryOp::Le => format!("V::compare(&{l}, &{r}, \"<=\")"),
                BinaryOp::Ge => format!("V::compare(&{l}, &{r}, \">=\")"),
                BinaryOp::Add => format!("V::arith(&{l}, &{r}, '+')"),
                BinaryOp::Sub => format!("V::arith(&{l}, &{r}, '-')"),
                BinaryOp::Mul => format!("V::arith(&{l}, &{r}, '*')"),
                BinaryOp::And => format!("V::Bool({l}.truthy() && {r}.truthy())"),
                BinaryOp::Or => format!("V::Bool({l}.truthy() || {r}.truthy())"),
                BinaryOp::Implies => format!("V::Bool(!{l}.truthy() || {r}.truthy())"),
            }
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => format!(
            "(if {}.truthy() {{ {} }} else {{ {} }})",
            emit(condition, cx)?,
            emit(then_branch, cx)?,
            emit(else_branch, cx)?
        ),
        Expression::Reference {
            name, widths, args, ..
        } => {
            let segs = &name.segments;
            if segs.len() == 2 && cx.errors.contains(segs[0]) && args.is_none() {
                return Ok(format!("V::Name(\"{}\".into())", segs[1]));
            }
            if segs.len() != 1 {
                return Err(unsupported(format!("reference {}", segs.join(".")), sp));
            }
            let n = segs[0];
            match args {
                None => {
                    if cx.binders.contains(n) {
                        format!("b_{n}.clone()")
                    } else {
                        return Err(unsupported(format!("unbound name `{n}`"), sp));
                    }
                }
                Some(list) => {
                    let mut parts = Vec::new();
                    for a in list {
                        parts.push(emit(a, cx)?);
                    }
                    match n {
                        "Ok" if parts.len() == 1 => format!("V::Ok(Box::new({}))", parts[0]),
                        "Err" if parts.len() == 1 => format!("V::err_of({})", parts[0]),
                        "len" if parts.len() == 1 => format!("{}.length()", parts[0]),
                        _ if cx.models.contains(n) => {
                            format!("model_{n}({})", parts.join(", "))
                        }
                        _ if cx.ops.contains_key(n) => {
                            if !cx.allow_ops {
                                return Err(unsupported("operation call inside a model", sp));
                            }
                            let mut ws = Vec::new();
                            for w in widths {
                                match w {
                                    Width::Literal(v) => {
                                        ws.push(format!("{v}u64"));
                                        cx.widths.entry(n.to_string()).or_default().insert(*v);
                                    }
                                    Width::Named(_) => {
                                        return Err(unsupported("symbolic width in a call", sp))
                                    }
                                }
                            }
                            format!("op_{n}(&[{}], &[{}], fault)", ws.join(", "), parts.join(", "))
                        }
                        _ => return Err(unsupported(format!("call to `{n}`"), sp)),
                    }
                }
            }
        }
        Expression::Quantified { .. } | Expression::Match { .. } | Expression::List { .. } => {
            return Err(unsupported("quantifier, match or list inside a body", sp));
        }
    })
}

// ---------------------------------------------------------------------------
// Operation adapters
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    BytesEq,
    FixedEq,
    U64Eq,
    Select,
}

impl Shape {
    fn faults(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Shape::BytesEq | Shape::FixedEq => {
                &[("invert", "Fault::Invert"), ("ignore-last-byte", "Fault::IgnoreLastByte")]
            }
            Shape::U64Eq => &[("invert", "Fault::Invert"), ("ignore-high-bit", "Fault::IgnoreHighBit")],
            Shape::Select => &[
                ("swap-choice", "Fault::SwapChoice"),
                ("missing-error", "Fault::MissingError"),
            ],
        }
    }
}

fn shape_of(op: &OpDecl<'_>) -> Result<Shape, CodegenError> {
    let tys: Result<Vec<Ty>, _> = op.params.iter().map(ty_of_op_param).collect();
    let tys = tys?;
    let returns_bool = matches!(&op.return_type, TypeExpr::Named { name: "Bool", .. });
    let returns_result = matches!(&op.return_type, TypeExpr::Result { .. });
    match (op.static_params.len(), tys.as_slice(), returns_bool, returns_result) {
        (0, [Ty::Bytes, Ty::Bytes], true, _) => Ok(Shape::BytesEq),
        (1, [Ty::BytesN(_), Ty::BytesN(_)], true, _) => Ok(Shape::FixedEq),
        (0, [Ty::U64, Ty::U64], true, _) => Ok(Shape::U64Eq),
        (0, [Ty::Bytes, Ty::Bytes, Ty::Bool], _, true) if op.is_fresh => Ok(Shape::Select),
        _ => Err(unsupported(
            format!("operation `{}` has a signature with no adapter shape", op.name),
            op.span,
        )),
    }
}

/// Operation parameters may carry a *named* width (`Bytes<N>`); treat it as
/// `BytesN(0)` for shape matching only.
fn ty_of_op_param(p: &Parameter<'_>) -> Result<Ty, CodegenError> {
    match &p.param_type {
        TypeExpr::Bytes {
            width: Some(Width::Named(_)),
            ..
        } => Ok(Ty::BytesN(0)),
        _ => ty_of(p),
    }
}

fn adapter(
    op: &OpDecl<'_>,
    shape: Shape,
    export: &str,
    err_case: Option<&str>,
    widths: &BTreeSet<u64>,
) -> Result<String, CodegenError> {
    let n = op.name;
    let mut s = String::new();
    match shape {
        Shape::BytesEq => {
            let _ = write!(
                s,
                "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n\
                 \x20   let (mut a, mut b) = (args[0].bytes().to_vec(), args[1].bytes().to_vec());\n\
                 \x20   if fault == Fault::IgnoreLastByte {{\n\
                 \x20       a = mask_last_byte(&a);\n\
                 \x20       b = mask_last_byte(&b);\n\
                 \x20   }}\n\
                 \x20   let r = {export}(&a, &b);\n\
                 \x20   V::Bool(if fault == Fault::Invert {{ !r }} else {{ r }})\n\
                 }}\n\n"
            );
        }
        Shape::FixedEq => {
            let _ = write!(
                s,
                "fn fixed_{n}<const N: usize>(a: &[u8], b: &[u8]) -> bool {{\n\
                 \x20   let a: &[u8; N] = a.try_into().expect(\"left width\");\n\
                 \x20   let b: &[u8; N] = b.try_into().expect(\"right width\");\n\
                 \x20   {export}(a, b)\n\
                 }}\n\n\
                 fn op_{n}(w: &[u64], args: &[V], fault: Fault) -> V {{\n\
                 \x20   let (mut a, mut b) = (args[0].bytes().to_vec(), args[1].bytes().to_vec());\n\
                 \x20   if fault == Fault::IgnoreLastByte {{\n\
                 \x20       a = mask_last_byte(&a);\n\
                 \x20       b = mask_last_byte(&b);\n\
                 \x20   }}\n\
                 \x20   let r = match w[0] {{\n"
            );
            for w in widths {
                let _ = writeln!(s, "        {w} => fixed_{n}::<{w}>(&a, &b),");
            }
            s.push_str(
                "        _ => panic!(\"no generated arm for this width\"),\n    };\n\
                 \x20   V::Bool(if fault == Fault::Invert { !r } else { r })\n}\n\n",
            );
        }
        Shape::U64Eq => {
            let _ = write!(
                s,
                "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n\
                 \x20   let (mut a, mut b) = (args[0].u64v(), args[1].u64v());\n\
                 \x20   if fault == Fault::IgnoreHighBit {{\n\
                 \x20       a &= !(1u64 << 63);\n\
                 \x20       b &= !(1u64 << 63);\n\
                 \x20   }}\n\
                 \x20   let r = {export}(a, b);\n\
                 \x20   V::Bool(if fault == Fault::Invert {{ !r }} else {{ r }})\n\
                 }}\n\n"
            );
        }
        Shape::Select => {
            let case = err_case.ok_or_else(|| CodegenError {
                code: DiagnosticCode::W0704,
                kind: CodegenErrorKind::MissingBinding(format!(
                    "binding for `{n}` has no error normalization"
                )),
                span: op.span,
            })?;
            let _ = write!(
                s,
                "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n\
                 \x20   let a = args[0].bytes().to_vec();\n\
                 \x20   let b = args[1].bytes().to_vec();\n\
                 \x20   let mut choice = args[2].boolv();\n\
                 \x20   if fault == Fault::SwapChoice {{\n\
                 \x20       choice = !choice;\n\
                 \x20   }}\n\
                 \x20   match contained(|| {export}(&a, &b, choice)) {{\n\
                 \x20       Some(mut out) => {{\n\
                 \x20           let got = out.clone();\n\
                 \x20           if let Some(first) = out.first_mut() {{\n\
                 \x20               *first ^= 0xff;\n\
                 \x20           }}\n\
                 \x20           let alias = !out.is_empty()\n\
                 \x20               && (out.as_ptr() == a.as_ptr() || out.as_ptr() == b.as_ptr());\n\
                 \x20           let inputs_intact = a == args[0].bytes() && b == args[1].bytes();\n\
                 \x20           if alias || !inputs_intact {{\n\
                 \x20               V::Err(\"FRESHNESS_VIOLATION\".into())\n\
                 \x20           }} else {{\n\
                 \x20               V::Ok(Box::new(V::Bytes(got)))\n\
                 \x20           }}\n\
                 \x20       }}\n\
                 \x20       None if fault == Fault::MissingError => V::Ok(Box::new(V::Bytes(a))),\n\
                 \x20       None => V::Err(\"{case}\".into()),\n\
                 \x20   }}\n\
                 }}\n\n"
            );
        }
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// The generator
// ---------------------------------------------------------------------------

/// Compile `file` into the evidence harness source (see module docs).
pub fn compile_rust_evidence_harness(
    file: &WarrantFile<'_>,
    checked: &CheckedWarrantModule,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    validate_bindings(checked, manifest).map_err(|e| CodegenError {
        code: DiagnosticCode::W0704,
        kind: CodegenErrorKind::MissingBinding(format!("{:?}", e.kind)),
        span: e.span,
    })?;
    generate_evidence_harness(file, manifest, options)
}

/// [`compile_rust_evidence_harness`] without the binding/contract validation
/// step: translation only.  Exposed so the translator's refusals can be tested
/// on documents that the type checker would reject earlier.
pub fn generate_evidence_harness(
    file: &WarrantFile<'_>,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    for d in &file.declarations {
        if let Declaration::Op(o) = d {
            if !manifest.bindings.contains_key(o.name) {
                return Err(CodegenError {
                    code: DiagnosticCode::W0704,
                    kind: CodegenErrorKind::MissingBinding(format!(
                        "operation {} has no binding",
                        o.name
                    )),
                    span: o.span,
                });
            }
        }
    }
    let module = file.name;

    let mut models = HashSet::new();
    let mut errors = HashSet::new();
    let mut ops: HashMap<String, usize> = HashMap::new();
    let mut op_decls: Vec<&OpDecl<'_>> = Vec::new();
    for d in &file.declarations {
        match d {
            Declaration::Model(m) if m.body.is_some() => {
                models.insert(m.name.to_string());
            }
            Declaration::Error(e) => {
                errors.insert(e.name.to_string());
            }
            Declaration::Op(o) => {
                ops.insert(o.name.to_string(), o.static_params.len());
                op_decls.push(o);
            }
            _ => {}
        }
    }
    let op_names: HashSet<&str> = op_decls.iter().map(|o| o.name).collect();

    let mut widths: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut body = String::new();

    // Model functions.
    for d in &file.declarations {
        if let Declaration::Model(m) = d {
            if let Some(b) = &m.body {
                let mut cx = Cx {
                    binders: m.params.iter().map(|p| p.name.to_string()).collect(),
                    models: &models,
                    ops: &ops,
                    errors: &errors,
                    allow_ops: false,
                    widths: &mut widths,
                };
                let params: Vec<String> =
                    m.params.iter().map(|p| format!("b_{}: V", p.name)).collect();
                let _ = write!(
                    body,
                    "fn model_{}({}) -> V {{\n    {}\n}}\n\n",
                    m.name,
                    params.join(", "),
                    emit(b, &mut cx)?
                );
            }
        }
    }

    // Examples.
    struct Ex {
        name: String,
        op: String,
    }
    let mut examples: Vec<Ex> = Vec::new();
    for d in &file.declarations {
        if let Declaration::Example(ex) = d {
            let op = find_operation_in_expr(&ex.body, &op_names).ok_or_else(|| {
                unsupported(format!("example {} calls no operation", ex.name), ex.span)
            })?;
            let mut cx = Cx {
                binders: HashSet::new(),
                models: &models,
                ops: &ops,
                errors: &errors,
                allow_ops: true,
                widths: &mut widths,
            };
            let _ = write!(
                body,
                "#[allow(unused_variables)]\npub fn example_{}(fault: Fault) -> bool {{\n    {}.truthy()\n}}\n\n",
                ex.name,
                emit(&ex.body, &mut cx)?
            );
            examples.push(Ex {
                name: ex.name.to_string(),
                op: op.to_string(),
            });
        }
    }

    // Claims.
    struct Cl {
        name: String,
        op: String,
    }
    let mut claims: Vec<Cl> = Vec::new();
    for d in &file.declarations {
        let Declaration::Claim(c) = d else { continue };
        let Expression::Quantified {
            is_forall: true,
            binders,
            body: cbody,
            span,
        } = &c.body
        else {
            return Err(unsupported(
                format!("claim {} must be `forall`", c.name),
                c.span,
            ));
        };
        let op = find_operation_in_expr(cbody, &op_names)
            .ok_or_else(|| unsupported(format!("claim {} calls no operation", c.name), *span))?;
        let tys: Vec<Ty> = binders.iter().map(ty_of).collect::<Result<_, _>>()?;
        let product: u64 = tys.iter().map(|t| t.domain_size()).product();
        if product > EXHAUSTIVE_BUDGET {
            return Err(CodegenError {
                code: DiagnosticCode::W0706,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "claim {} exhaustive domain {product} exceeds budget {EXHAUSTIVE_BUDGET}",
                    c.name
                )),
                span: *span,
            });
        }
        let mut cx = Cx {
            binders: binders.iter().map(|p| p.name.to_string()).collect(),
            models: &models,
            ops: &ops,
            errors: &errors,
            allow_ops: true,
            widths: &mut widths,
        };
        let binds: String = binders
            .iter()
            .enumerate()
            .map(|(i, p)| format!("    let b_{} = env[{i}].clone();\n", p.name))
            .collect();
        let mut admit = String::from("true");
        for p in binders {
            if let Some(r) = &p.refinement {
                let _ = write!(admit, " && {}.truthy()", emit(r, &mut cx)?);
            }
        }
        let holds = emit(cbody, &mut cx)?;
        let tys_src: Vec<String> = tys.iter().map(|t| t.rust()).collect();
        let _ = write!(
            body,
            "const CLAIM_{up}_TYS: &[Ty] = &[{tys}];\n\n\
             #[allow(unused_variables)]\nfn claim_{n}_admit(env: &[V]) -> bool {{\n{binds}    {admit}\n}}\n\n\
             #[allow(unused_variables)]\nfn claim_{n}_holds(env: &[V], fault: Fault) -> bool {{\n{binds}    {holds}.truthy()\n}}\n\n\
             pub fn exh_{n}(fault: Fault) -> (Verdict, Stats) {{\n    exhaustive(CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault)\n}}\n\n\
             pub fn prop_{n}(fault: Fault) -> (Verdict, Stats) {{\n    property(\"claim/{module}/{n}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault)\n}}\n\n\
             pub fn fuz_{n}(fault: Fault, iterations: u64) -> (Verdict, Stats) {{\n    fuzz_claim(\"fuzz/{module}/{n}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault, iterations)\n}}\n\n",
            up = c.name.to_uppercase(),
            n = c.name,
            tys = tys_src.join(", "),
        );
        claims.push(Cl {
            name: c.name.to_string(),
            op: op.to_string(),
        });
    }
    if claims.is_empty() {
        return Err(unsupported("document has no claims to generate evidence for", file.span));
    }

    // Adapters (after expression translation so fixed widths are known).
    let mut shapes: BTreeMap<String, Shape> = BTreeMap::new();
    let mut adapters = String::new();
    for op in &op_decls {
        let binding = &manifest.bindings[op.name];
        let shape = shape_of(op)?;
        let empty = BTreeSet::new();
        adapters.push_str(&adapter(
            op,
            shape,
            &binding.export,
            binding.error_normalization.as_ref().map(|n| n.error_case.as_str()),
            widths.get(op.name).unwrap_or(&empty),
        )?);
        shapes.insert(op.name.to_string(), shape);
    }

    // Suite registration.
    let id = |kind: &str, rest: &str| format!("warrant/{module}/{kind}/{rest}");
    let mut suite = String::new();
    suite.push_str("pub fn build_suite(fault: Fault) -> Suite {\n    let mut suite = Suite::new();\n");
    for ex in &examples {
        let _ = write!(
            suite,
            "    reg(&mut suite, Meta::new(\"{}\", \"example\", \"{}\", \"\"), move || {{\n        (example_verdict(example_{}(fault)), Stats::default(), String::new())\n    }});\n",
            id("example", &ex.name),
            ex.op,
            ex.name
        );
    }
    for c in &claims {
        let _ = write!(
            suite,
            "    reg(&mut suite, Meta {{ domain: describe(CLAIM_{up}_TYS), ..Meta::new(\"{id1}\", \"exhaustive\", \"{op}\", \"{n}\") }}, move || {{\n        let (v, s) = exh_{n}(fault);\n        (v, s, String::new())\n    }});\n\
             \x20   reg(&mut suite, Meta {{ seed: seed_of(\"claim/{module}/{n}\").to_string(), ..Meta::new(\"{id2}\", \"property\", \"{op}\", \"{n}\") }}, move || {{\n        let (v, s) = prop_{n}(fault);\n        (v, s, String::new())\n    }});\n\
             \x20   reg(&mut suite, Meta {{ seed: seed_of(\"fuzz/{module}/{n}\").to_string(), ..Meta::new(\"{id3}\", \"fuzz\", \"{op}\", \"{n}\") }}, move || {{\n        let (v, s) = fuz_{n}(fault, FUZZ_ITERATIONS);\n        (v, s, String::new())\n    }});\n",
            up = c.name.to_uppercase(),
            n = c.name,
            op = c.op,
            id1 = id("exhaustive", &c.name),
            id2 = id("property", &c.name),
            id3 = id("fuzz", &c.name),
        );
    }
    // Negative controls, per (suite kind, op, fault).
    let mut neg_wrappers = String::new();
    for ex in &examples {
        let _ = write!(
            neg_wrappers,
            "fn neg_ex_{n}(fault: Fault) -> Verdict {{\n    example_verdict(example_{n}(fault))\n}}\n\n",
            n = ex.name
        );
    }
    for c in &claims {
        let _ = write!(
            neg_wrappers,
            "fn neg_exh_{n}(fault: Fault) -> Verdict {{\n    exh_{n}(fault).0\n}}\n\n\
             fn neg_prop_{n}(fault: Fault) -> Verdict {{\n    prop_{n}(fault).0\n}}\n\n\
             fn neg_fuz_{n}(fault: Fault) -> Verdict {{\n    fuz_{n}(fault, NEGCTL_FUZZ_ITERATIONS).0\n}}\n\n",
            n = c.name
        );
    }
    for (op, shape) in &shapes {
        for kind in ["example", "exhaustive", "property", "fuzz"] {
            let members: Vec<String> = if kind == "example" {
                examples
                    .iter()
                    .filter(|e| &e.op == op)
                    .map(|e| format!("(\"{n}\", neg_ex_{n})", n = e.name))
                    .collect()
            } else {
                let pre = match kind {
                    "exhaustive" => "neg_exh",
                    "property" => "neg_prop",
                    _ => "neg_fuz",
                };
                claims
                    .iter()
                    .filter(|c| &c.op == op)
                    .map(|c| format!("(\"{n}\", {pre}_{n})", n = c.name))
                    .collect()
            };
            if members.is_empty() {
                continue;
            }
            for (fname, frust) in shape.faults() {
                let _ = write!(
                    suite,
                    "    reg(&mut suite, Meta {{ fault: \"{fname}\".into(), target: \"{kind}\".into(), ..Meta::new(\"{idn}\", \"negctl\", \"{op}\", \"\") }}, || {{\n\
                     \x20       let runs: [(&str, fn(Fault) -> Verdict); {len}] = [{members}];\n\
                     \x20       let caught: Vec<&str> = runs.iter().filter(|(_, f)| is_detected(&f({frust}))).map(|(n, _)| *n).collect();\n\
                     \x20       negctl_result(caught, \"{fname}\")\n    }});\n",
                    idn = id("negctl", &format!("{kind}/{op}/{fname}")),
                    len = members.len(),
                    members = members.join(", "),
                );
            }
        }
    }
    suite.push_str("    suite\n}\n");

    // Assemble.
    let mut out = String::new();
    let _ = write!(
        out,
        "// GENERATED BY coding_adventures_warrant_grammar::evidence (VER04h). DO NOT EDIT MANUALLY.\n\
         // Source: {spec}\n\
         // Source-SHA256: {sha}\n\
         // Target: Rust evidence harness (examples, bounded-exhaustive, property, fuzz, negative controls)\n\
         // Regenerate: cargo run -p coding_adventures_warrant_grammar --example gen_evidence\n\n\
         use {krate}::*;\n\n\
         pub const MODULE: &str = \"{module}\";\n\
         pub const SPEC_PATH: &str = \"{spec}\";\n\
         pub const SPEC_SHA256: &str = \"{sha}\";\n\
         pub const EVIDENCE_SEED: u64 = {seed};\n\
         pub const PROPERTY_CASES: u64 = {pc};\n\
         pub const FUZZ_ITERATIONS: u64 = {fi};\n\
         pub const NEGCTL_FUZZ_ITERATIONS: u64 = {nfi};\n\
         pub const EXHAUSTIVE_BUDGET: u64 = {budget};\n\n",
        spec = options.spec_path,
        sha = options.spec_sha256,
        krate = options.package_crate,
        seed = options.seed,
        pc = options.property_cases,
        fi = options.fuzz_iterations,
        nfi = options.negctl_fuzz_iterations,
        budget = EXHAUSTIVE_BUDGET,
    );
    out.push_str(PRELUDE);
    out.push_str("\n// ---------------------------------------------------------------------------\n// Generated from the Warrant document\n// ---------------------------------------------------------------------------\n\n");
    out.push_str(&body);
    out.push_str(&adapters);
    out.push_str(&neg_wrappers);
    out.push_str(&suite);
    Ok(out)
}
