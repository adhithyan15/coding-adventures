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

/// Default maximum size of the Cartesian product of binder domains (VER04h
/// section 3).  A package may raise it in [`EvidenceOptions::exhaustive_budget`]
/// when it can justify the wall-clock cost (gf256 does so for `U8^3`, 2^24).
pub const EXHAUSTIVE_BUDGET: u64 = 400_000;

/// Largest fixed width (`Bytes<N>` binder or `op<N>` call width) the generator
/// accepts.  The enumerated domain of `Bytes<N>` holds `4 + 3N` values of `N`
/// bytes each, i.e. about `3N^2` bytes, and every width becomes a monomorphised
/// const-generic arm in the adapter; both must stay small (W0707).
pub const MAX_FIXED_WIDTH: u64 = 4096;

/// Memory budget for all binder domains of one claim, in bytes.  The element
/// budget alone is not enough: `Bytes<100000>` has only ~300k elements but
/// would materialise ~30 GB (VER04h section 3, audit A2).
pub const DOMAIN_BYTES_BUDGET: u64 = 64 * 1024 * 1024;

/// Upper bound on the heap/stack footprint of one `V` value besides its
/// payload bytes.
pub const VALUE_OVERHEAD: u64 = 32;

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
    /// Largest Cartesian product of binder domains one claim may enumerate;
    /// echoed into the generated `EXHAUSTIVE_BUDGET` constant.
    pub exhaustive_budget: u64,
    /// Rust parameter types of each operation's export, by operation name
    /// (`u8`, `u16`, `u32`, `u64`).  Only the scalar adapter consults it; an
    /// absent entry means the Rust type matching the Warrant type (`U8` is
    /// `u8`, `U64` is `u64`, `Nat` is `u32`).
    pub arg_types: BTreeMap<String, Vec<String>>,
}

fn unsupported(msg: impl Into<String>, span: (usize, usize)) -> CodegenError {
    CodegenError {
        code: DiagnosticCode::W0701,
        kind: CodegenErrorKind::UnsupportedTarget(msg.into()),
        span,
    }
}

fn width_too_large(n: u64, span: (usize, usize)) -> CodegenError {
    CodegenError {
        code: DiagnosticCode::W0707,
        kind: CodegenErrorKind::UnsupportedTarget(format!(
            "fixed width {n} exceeds the cap {MAX_FIXED_WIDTH}"
        )),
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
    ///
    /// `None` means "does not fit in a `u64`" and callers must treat it as
    /// over budget.  (Arithmetic here is checked on purpose: a wrapped
    /// product of eight `U8` binders is `2^64 = 0`, which used to pass the
    /// budget and send the enumeration into an effectively endless loop.)
    fn domain_size(self) -> Option<u64> {
        match self {
            Ty::Bool => Some(2),
            Ty::U8 => Some(256),
            Ty::Nat => Some(65),
            Ty::U64 => Some(256 + 56 + 1 + 64),
            Ty::Bytes => Some(1 + 256 + 16 + 64 + 10),
            Ty::BytesN(n) if n > MAX_FIXED_WIDTH => None,
            Ty::BytesN(0) => Some(1),
            Ty::BytesN(1) => Some(256),
            Ty::BytesN(n) if n <= 3 => 4u64.checked_pow(n as u32),
            Ty::BytesN(n) => 3u64.checked_mul(n)?.checked_add(4),
        }
    }

    /// Upper bound on the bytes the enumerated domain occupies: every element
    /// costs [`VALUE_OVERHEAD`] plus its payload.
    fn domain_bytes(self) -> Option<u64> {
        let payload = match self {
            Ty::Bytes => 8,
            Ty::BytesN(n) => n,
            _ => 0,
        };
        self.domain_size()?.checked_mul(payload.checked_add(VALUE_OVERHEAD)?)
    }
}

/// Cartesian-product size and total domain bytes of a binder list, or `None`
/// when either overflows a `u64` or a width is over the cap.
fn claim_domain(tys: &[Ty]) -> Option<(u64, u64)> {
    let mut product = 1u64;
    let mut bytes = 0u64;
    for t in tys {
        product = product.checked_mul(t.domain_size()?)?;
        bytes = bytes.checked_add(t.domain_bytes()?)?;
    }
    Some((product, bytes))
}

fn ty_of(p: &Parameter<'_>) -> Result<Ty, CodegenError> {
    match &p.param_type {
        TypeExpr::Bytes { width: None, .. } => Ok(Ty::Bytes),
        TypeExpr::Bytes {
            width: Some(Width::Literal(n)),
            ..
        } => {
            if *n > MAX_FIXED_WIDTH {
                return Err(width_too_large(*n, p.span));
            }
            Ok(Ty::BytesN(*n))
        }
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
                                return Err(unsupported(
                                    "operation call inside a model or binder refinement (an admission predicate may use models only, so a fault cannot change which cases are admitted)",
                                    sp,
                                ));
                            }
                            let mut ws = Vec::new();
                            for w in widths {
                                match w {
                                    Width::Literal(v) => {
                                        if *v > MAX_FIXED_WIDTH {
                                            return Err(width_too_large(*v, sp));
                                        }
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
// Fault registry
// ---------------------------------------------------------------------------

/// Every fault a binding may name, in the order the generated `Fault` enum
/// lists the ones a document uses.  The first five are consumed by the
/// fixed-shape (byte-slice / `U64`) adapters; the rest by the scalar adapter
/// (see [`SCALAR_FAULTS`]).  A package's generated enum holds only the faults
/// its bindings reference, so adding a registry entry never changes the output
/// of a package that does not use it.
const FAULTS: &[(&str, &str)] = &[
    ("invert", "Invert"),
    ("ignore-last-byte", "IgnoreLastByte"),
    ("ignore-high-bit", "IgnoreHighBit"),
    ("swap-choice", "SwapChoice"),
    ("missing-error", "MissingError"),
    ("or-for-xor", "OrForXor"),
    ("wrapping-sub", "WrappingSub"),
    ("mul-as-add", "MulAsAdd"),
    ("alt-poly", "AltPoly"),
    ("generator-shift", "GeneratorShift"),
    ("exp-mod-256", "ExpMod256"),
];

fn fault_variant(name: &str) -> Option<&'static str> {
    FAULTS.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

/// How a scalar-adapter fault rewrites the call.  Templates are Rust source
/// over `{k}` (the package crate), `{e}` (the export name) and `{args}` (the
/// converted arguments `p0, p1, ...`).
#[derive(Clone, Copy)]
enum ScalarFaultKind {
    /// The call is replaced by the expression.
    Replace(&'static str),
    /// The result `r: u8` is rewritten by the expression.
    Post(&'static str),
    /// The last argument is rewritten by the expression over `p`.
    PreLast(&'static str),
    /// A panic of a fallible operation becomes `Ok(0)` instead of the error.
    MissingError,
}

/// Faults the scalar adapter understands (VER04h section 6, field-arithmetic
/// family).  Each models a mistake a plausible refactor of the real crate
/// could introduce, injected at the adapter so the crate is never modified.
const SCALAR_FAULTS: &[(&str, ScalarFaultKind)] = &[
    // addition done with OR instead of XOR (forgets that 1 + 1 = 0).
    ("or-for-xor", ScalarFaultKind::Replace("p0 | p1")),
    // integer wrapping subtraction instead of XOR.
    ("wrapping-sub", ScalarFaultKind::Replace("p0.wrapping_sub(p1)")),
    // multiplication that returns the (XOR) sum.
    ("mul-as-add", ScalarFaultKind::Replace("p0 ^ p1")),
    // the operation computed in the AES field (reduction polynomial 0x11B)
    // instead of the Reed-Solomon one (0x11D): still a field, wrong tables.
    ("alt-poly", ScalarFaultKind::Replace("{k}::Field::new(0x11B).{e}({args})")),
    // the antilog index is off by one: every non-zero result is multiplied by
    // the generator 2 (ALOG[i + 1] instead of ALOG[i]).
    (
        "generator-shift",
        ScalarFaultKind::Post("if r == 0 { r } else { {k}::multiply(r, 2) }"),
    ),
    // the exponent is reduced modulo 256 instead of the group order 255.
    ("exp-mod-256", ScalarFaultKind::PreLast("p % 256")),
    ("missing-error", ScalarFaultKind::MissingError),
];

fn scalar_fault(name: &str) -> Option<ScalarFaultKind> {
    SCALAR_FAULTS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

/// The generated `Fault` enum, restricted to `used` faults.  For a package
/// that uses exactly the original five, this is byte-for-byte the text the
/// prelude carried before faults became data (ct-compare's checked-in harness
/// proves it).
fn fault_block(used: &BTreeSet<String>) -> String {
    let list: Vec<(&str, &str)> = FAULTS
        .iter()
        .copied()
        .filter(|(n, _)| used.contains(*n))
        .collect();
    let mut s = String::new();
    s.push_str("/// Deliberate adapter faults (VER04h section 6). `None` is the real adapter.\n");
    s.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum Fault {\n    None,\n");
    for (_, v) in &list {
        let _ = writeln!(s, "    {v},");
    }
    s.push_str(
        "}\n\nimpl Fault {\n    pub fn name(self) -> &'static str {\n        match self {\n            Fault::None => \"none\",\n",
    );
    for (n, v) in &list {
        let _ = writeln!(s, "            Fault::{v} => \"{n}\",");
    }
    s.push_str(
        "        }\n    }\n    pub fn parse(text: &str) -> Option<Fault> {\n        [\n            Fault::None,\n",
    );
    for (_, v) in &list {
        let _ = writeln!(s, "            Fault::{v},");
    }
    s.push_str("        ]\n        .into_iter()\n        .find(|f| f.name() == text)\n    }\n}\n");
    s
}

/// Expand the prelude: `//@@FAULTS@@` becomes the generated fault block and
/// `//@if <feature>` ... `//@endif` regions are kept only when the feature is
/// enabled (marker lines never reach the output).
fn expand_prelude(faults: &str, features: &BTreeSet<&str>) -> String {
    let mut out = String::new();
    let mut keep = true;
    for line in PRELUDE.lines() {
        if let Some(f) = line.strip_prefix("//@if ") {
            keep = features.contains(f.trim());
            continue;
        }
        if line == "//@endif" {
            keep = true;
            continue;
        }
        if !keep {
            continue;
        }
        if line == "//@@FAULTS@@" {
            out.push_str(faults);
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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
    /// Scalar-parameter operation returning `U8` (`fallible == false`) or
    /// `Result<U8, E>` through panic normalization (`fallible == true`).
    Scalar {
        fallible: bool,
    },
}

impl Shape {
    /// Faults a fixed shape owns.  The scalar shape owns none: its faults are
    /// the ones the operation's binding names (`negative_controls`).
    fn faults(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Shape::Scalar { .. } => &[],
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
        (0, ps, _, _)
            if !ps.is_empty()
                && ps.iter().all(|t| matches!(t, Ty::U8 | Ty::U64 | Ty::Nat))
                && !op.is_fresh
                && (returns_u8(&op.return_type) || returns_result_u8(&op.return_type)) =>
        {
            Ok(Shape::Scalar {
                fallible: returns_result_u8(&op.return_type),
            })
        }
        _ => Err(unsupported(
            format!("operation `{}` has a signature with no adapter shape", op.name),
            op.span,
        )),
    }
}

fn returns_u8(t: &TypeExpr<'_>) -> bool {
    matches!(t, TypeExpr::Named { name: "U8", .. })
}

fn returns_result_u8(t: &TypeExpr<'_>) -> bool {
    matches!(t, TypeExpr::Result { ok_type, .. } if returns_u8(ok_type))
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
        // Scalar operations are emitted by `scalar_adapter`.
        Shape::Scalar { .. } => {}
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

/// Adapter for a scalar-parameter operation returning `U8` or `Result<U8, E>`.
///
/// Arguments are converted from the dynamic value to the export's Rust types
/// (a value that does not fit is a harness bug and panics outside any claim),
/// the real export is called, and the `u8` result is wrapped as `V::Nat` (or
/// `Ok`/`Err` for a fallible operation whose declared panic becomes `err_case`).
/// Each fault the binding lists adds one `match fault` arm; with `Fault::None`
/// the adapter is exactly `export(p0, p1, ...)`.
fn scalar_adapter(
    op: &OpDecl<'_>,
    fallible: bool,
    export: &str,
    err_case: Option<&str>,
    faults: &[String],
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    let n = op.name;
    let tys: Vec<Ty> = op
        .params
        .iter()
        .map(ty_of_op_param)
        .collect::<Result<_, _>>()?;
    let given = options.arg_types.get(n);
    if let Some(g) = given {
        if g.len() != tys.len() {
            return Err(unsupported(
                format!("operation `{n}` arg_types has {} entries for {} parameters", g.len(), tys.len()),
                op.span,
            ));
        }
    }
    let mut conv = String::new();
    let mut names = Vec::new();
    for (i, t) in tys.iter().enumerate() {
        let default = match t {
            Ty::U8 => "u8",
            Ty::U64 => "u64",
            _ => "u32",
        };
        let rt = given.map(|g| g[i].as_str()).unwrap_or(default);
        if !["u8", "u16", "u32", "u64"].contains(&rt) {
            return Err(unsupported(format!("argument type `{rt}` of `{n}`"), op.span));
        }
        if rt == "u64" {
            let _ = writeln!(conv, "    let p{i} = args[{i}].u64v();");
        } else {
            let _ = writeln!(
                conv,
                "    let p{i} = {rt}::try_from(args[{i}].u64v()).expect(\"argument {i} of {n} fits {rt}\");"
            );
        }
        names.push(format!("p{i}"));
    }
    let arglist = names.join(", ");
    let subst = |t: &str| {
        t.replace("{k}", &options.package_crate)
            .replace("{e}", export)
            .replace("{args}", &arglist)
    };
    let mut pre = String::new();
    let mut replace_arms = String::new();
    let mut post_arms = String::new();
    let mut missing = false;
    let last = tys.len() - 1;
    for f in faults {
        let (Some(variant), Some(kind)) = (fault_variant(f), scalar_fault(f)) else {
            return Err(unsupported(
                format!("operation `{n}` names unknown scalar fault `{f}`"),
                op.span,
            ));
        };
        match kind {
            ScalarFaultKind::Replace(t) => {
                let _ = writeln!(replace_arms, "            Fault::{variant} => {},", subst(t));
            }
            ScalarFaultKind::Post(t) => {
                let _ = writeln!(post_arms, "            Fault::{variant} => {},", subst(t));
            }
            ScalarFaultKind::PreLast(t) => {
                let _ = writeln!(
                    pre,
                    "    let p{last} = if fault == Fault::{variant} {{ let p = p{last}; {} }} else {{ p{last} }};",
                    subst(t)
                );
            }
            ScalarFaultKind::MissingError => {
                if !fallible {
                    return Err(unsupported(
                        format!("fault `missing-error` needs a fallible operation, `{n}` is total"),
                        op.span,
                    ));
                }
                missing = true;
            }
        }
    }
    let mut compute = String::new();
    let _ = write!(
        compute,
        "let r: u8 = match fault {{\n{replace_arms}            _ => {export}({arglist}),\n        }};\n"
    );
    if !post_arms.is_empty() {
        let _ = write!(compute, "        let r: u8 = match fault {{\n{post_arms}            _ => r,\n        }};\n");
    }
    // The adapter obeys a fault only when it is the attacked operation (or no
    // operation is singled out, as in a global WARRANT_EVIDENCE_FAULT run).
    let _ = writeln!(conv, "    let fault = fault_for(\"{n}\", fault);");
    let mut s = String::new();
    if fallible {
        let case = err_case.ok_or_else(|| CodegenError {
            code: DiagnosticCode::W0704,
            kind: CodegenErrorKind::MissingBinding(format!(
                "binding for `{n}` has no error normalization"
            )),
            span: op.span,
        })?;
        let _ = write!(
            s,
            "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n{conv}{pre}\
             \x20   match contained(|| {{\n        {compute}        r\n    }}) {{\n\
             \x20       Some(r) => V::Ok(Box::new(V::Nat(u64::from(r)))),\n"
        );
        if missing {
            s.push_str("        None if fault == Fault::MissingError => V::Ok(Box::new(V::Nat(0))),\n");
        }
        let _ = write!(s, "        None => V::Err(\"{case}\".into()),\n    }}\n}}\n\n");
    } else {
        let _ = write!(
            s,
            "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n{conv}{pre}\
             \x20   {compute}    V::Nat(u64::from(r))\n}}\n\n"
        );
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
    let mut features: BTreeSet<&str> = BTreeSet::new();

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
        let (shown, over) = match claim_domain(&tys) {
            Some((p, b)) => (
                format!("{p} ({b} bytes)"),
                p > options.exhaustive_budget || b > DOMAIN_BYTES_BUDGET,
            ),
            None => ("overflowing u64".to_string(), true),
        };
        if over {
            return Err(CodegenError {
                code: DiagnosticCode::W0706,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "claim {} exhaustive domain {shown} exceeds budget {} cases / {DOMAIN_BYTES_BUDGET} bytes",
                    c.name, options.exhaustive_budget
                )),
                span: *span,
            });
        }
        if tys.contains(&Ty::U8) {
            features.insert("u8");
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
        // Admission predicates may use models only: an operation inside one
        // would be evaluated by the faulted adapter and so change which cases
        // a negative control admits.
        cx.allow_ops = false;
        for p in binders {
            if let Some(r) = &p.refinement {
                let _ = write!(admit, " && {}.truthy()", emit(r, &mut cx)?);
            }
        }
        cx.allow_ops = true;
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
    // The faults each operation is attacked with, as (name, Rust variant).
    let mut op_faults: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut used_faults: BTreeSet<String> = BTreeSet::new();
    let mut adapters = String::new();
    for op in &op_decls {
        let binding = &manifest.bindings[op.name];
        let shape = shape_of(op)?;
        let empty = BTreeSet::new();
        let err_case = binding.error_normalization.as_ref().map(|n| n.error_case.as_str());
        if let Shape::Scalar { fallible } = shape {
            features.insert("scoped");
            adapters.push_str(&scalar_adapter(
                op,
                fallible,
                &binding.export,
                err_case,
                &binding.negative_controls,
                options,
            )?);
            let mut list = Vec::new();
            for f in &binding.negative_controls {
                let variant = fault_variant(f).ok_or_else(|| {
                    unsupported(format!("unknown fault `{f}` for `{}`", op.name), op.span)
                })?;
                list.push((f.clone(), format!("Fault::{variant}")));
            }
            op_faults.insert(op.name.to_string(), list);
        } else {
            adapters.push_str(&adapter(
                op,
                shape,
                &binding.export,
                err_case,
                widths.get(op.name).unwrap_or(&empty),
            )?);
            op_faults.insert(
                op.name.to_string(),
                shape
                    .faults()
                    .iter()
                    .map(|(n, v)| (n.to_string(), v.to_string()))
                    .collect(),
            );
        }
        for (n, _) in &op_faults[op.name] {
            used_faults.insert(n.clone());
        }
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
    for op in shapes.keys() {
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
            let scoped = matches!(shapes[op], Shape::Scalar { .. });
            for (fname, frust) in &op_faults[op] {
                if scoped {
                    // The fault attacks this operation only: another scalar
                    // adapter that lists the same fault must stay honest, or a
                    // fault applied consistently to every operation (the AES
                    // polynomial, say) would hide from every relational claim.
                    // The control stops at the first claim (document order)
                    // that catches the fault, so a 2^24-case claim that cannot
                    // catch it is not run to completion; `caught_by` names
                    // that first claim.
                    let _ = write!(
                        suite,
                        "    reg(&mut suite, Meta {{ fault: \"{fname}\".into(), target: \"{kind}\".into(), ..Meta::new(\"{idn}\", \"negctl\", \"{op}\", \"\") }}, || {{\n\
                         \x20       let runs: [(&str, fn(Fault) -> Verdict); {len}] = [{members}];\n\
                         \x20       let caught: Vec<&str> = attacking(\"{op}\", || runs.iter().find(|(_, f)| is_detected(&f({frust}))).map(|(n, _)| *n).into_iter().collect());\n\
                         \x20       negctl_result(caught, \"{fname}\")\n    }});\n",
                        idn = id("negctl", &format!("{kind}/{op}/{fname}")),
                        len = members.len(),
                        members = members.join(", "),
                    );
                    continue;
                }
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
         pub const EXHAUSTIVE_BUDGET: u64 = {budget};\n\
         pub const DOMAIN_BYTES_BUDGET: u64 = {bbudget};\n\
         pub const MAX_FIXED_WIDTH: u64 = {maxw};\n\
         pub const VALUE_OVERHEAD: u64 = {vo};\n\n",
        spec = options.spec_path,
        sha = options.spec_sha256,
        krate = options.package_crate,
        seed = options.seed,
        pc = options.property_cases,
        fi = options.fuzz_iterations,
        nfi = options.negctl_fuzz_iterations,
        budget = options.exhaustive_budget,
        bbudget = DOMAIN_BYTES_BUDGET,
        maxw = MAX_FIXED_WIDTH,
        vo = VALUE_OVERHEAD,
    );
    out.push_str(&expand_prelude(&fault_block(&used_faults), &features));
    out.push_str("\n// ---------------------------------------------------------------------------\n// Generated from the Warrant document\n// ---------------------------------------------------------------------------\n\n");
    out.push_str(&body);
    out.push_str(&adapters);
    out.push_str(&neg_wrappers);
    out.push_str(&suite);
    Ok(out)
}
