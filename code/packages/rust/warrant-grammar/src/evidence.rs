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
use std::path::Path;

use crate::binding::{
    is_rust_identifier, validate_bindings, BindingManifest, HostShape, HostSpec, HostTy,
};
use crate::checker::CheckedWarrantModule;
use crate::codegen::{
    find_operation_in_expr, hex_decode, CodegenError, CodegenErrorKind, DiagnosticCode,
};
use crate::fixed::{self, FixedTy};
use crate::vectors::{self, LoadedTable, Shape as ColShape};
use crate::parser::{
    valid_table_path, BinaryOp, ClaimDecl, Declaration, Expression, Literal, OpDecl, Parameter, Pattern, ProjKey,
    TypeExpr,
    UnaryOp, Version, WarrantFile, Width,
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

/// The allocation cap of the WX-H observer, per observed adapter call (VER04j
/// section 6.3: a tool-profile value, 64 MiB by default).  It is echoed into the
/// generated `OBSERVER_CAP_BYTES` constant of a document that has an `output` or
/// `heap` bound, and into no other.
pub const OBSERVER_CAP_BYTES: u64 = 64 * 1024 * 1024;

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
    /// WX-A host composites (`struct`/`enum` lines, `ret=`, `errors=`); empty
    /// for a package with no composite operation.
    pub hosts: HostSpec,
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
        | Expression::Grouped { span, .. }
        | Expression::Tuple { span, .. }
        | Expression::Project { span, .. } => *span,
    }
}

// ---------------------------------------------------------------------------
// Binder types and domain sizes
// ---------------------------------------------------------------------------

/// Generator-side binder type (mirrors `Ty` in the prelude).
///
/// The first nine variants are the scalar types of WX-B; the rest are the WX-A
/// composites, built from component types (VER04j section 4.7).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty {
    Bool,
    U8,
    U64,
    Nat,
    Bytes,
    BytesN(u64),
    /// v1: a fixed-width binder at its default domain (full for `U8 I8 U16
    /// I16`, the edge set for the wider types).
    Fx(FixedTy),
    /// v1: a `U16`/`I16` binder demoted to its edge set by the budget rule.
    FxEdge(FixedTy),
    /// v1: a `Nat` that flows directly into a shift count or bit index.
    NatCount,
    /// WX-A: `(T1, ..., Tn)`.
    Tuple(Vec<Ty>),
    /// WX-A: a record (one constructor named like the type) or a variant: the
    /// constructors and their field types, in declaration order.
    Data {
        name: String,
        ctors: Vec<(String, Vec<Ty>)>,
    },
    /// WX-A: `Option<T>`.
    Opt(Box<Ty>),
    /// WX-A: `Result<T, E>` with the error cases of `E` in declaration order.
    Res(Box<Ty>, Vec<String>),
    /// WX-A: `List<T>`.
    List(Box<Ty>),
}

impl Ty {
    /// The prelude's `Ty` value as Rust source.  Composites borrow `'static`
    /// slices, which constant promotion allows in a `const`.
    fn rust(&self) -> String {
        match self {
            Ty::Fx(f) => format!("Ty::Fx(F::{})", f.name()),
            Ty::FxEdge(f) => format!("Ty::FxEdge(F::{})", f.name()),
            Ty::NatCount => "Ty::NatCount".into(),
            Ty::Bool => "Ty::Bool".into(),
            Ty::U8 => "Ty::U8".into(),
            Ty::U64 => "Ty::U64".into(),
            Ty::Nat => "Ty::Nat".into(),
            Ty::Bytes => "Ty::Bytes".into(),
            Ty::BytesN(n) => format!("Ty::BytesN({n})"),
            Ty::Tuple(cs) => {
                let items: Vec<String> = cs.iter().map(Ty::rust).collect();
                format!("Ty::Tuple(&[{}])", items.join(", "))
            }
            Ty::Data { name, ctors } => {
                let items: Vec<String> = ctors
                    .iter()
                    .map(|(c, fs)| {
                        let fields: Vec<String> = fs.iter().map(Ty::rust).collect();
                        format!("(\"{c}\", &[{}])", fields.join(", "))
                    })
                    .collect();
                format!("Ty::Data(\"{name}\", &[{}])", items.join(", "))
            }
            Ty::Opt(t) => format!("Ty::Opt(&{})", t.rust()),
            Ty::Res(t, cases) => {
                let items: Vec<String> = cases.iter().map(|c| format!("\"{c}\"")).collect();
                format!("Ty::Res(&{}, &[{}])", t.rust(), items.join(", "))
            }
            Ty::List(t) => format!("Ty::List(&{})", t.rust()),
        }
    }

    /// Closed-form size of the domain the prelude enumerates (VER04h section 3
    /// and, for composites, VER04j section 4.7).
    ///
    /// `None` means "does not fit in a `u64`" and callers must treat it as
    /// over budget.  (Arithmetic here is checked on purpose: a wrapped
    /// product of eight `U8` binders is `2^64 = 0`, which used to pass the
    /// budget and send the enumeration into an effectively endless loop.)
    ///
    /// ```text
    /// (U64, Nat)            377 * 65 = 24,505     product, lexicographic
    /// variant A(Bool)|B     2 + 1 = 3             concatenation of products
    /// Option<T>             1 + |T|
    /// Result<T, E>          |T| + #cases
    /// List<T>               1 + d + d^2 + e^3     d = min(16,|T|), e = min(4,|T|)
    /// ```
    fn domain_size(&self) -> Option<u64> {
        match self {
            Ty::Bool => Some(2),
            Ty::U8 => Some(256),
            Ty::Nat => Some(65),
            Ty::U64 => Some(256 + 56 + 1 + 64),
            Ty::NatCount => Some(65 + 8),
            Ty::Fx(f) if f.bits() <= 16 => Some(1u64 << f.bits()),
            Ty::Fx(f) | Ty::FxEdge(f) => Some(fixed::edge_domain(*f).len() as u64),
            Ty::Bytes => Some(1 + 256 + 16 + 64 + 10),
            Ty::BytesN(n) if *n > MAX_FIXED_WIDTH => None,
            Ty::BytesN(0) => Some(1),
            Ty::BytesN(1) => Some(256),
            Ty::BytesN(n) if *n <= 3 => 4u64.checked_pow(*n as u32),
            Ty::BytesN(n) => 3u64.checked_mul(*n)?.checked_add(4),
            Ty::Tuple(cs) => product_size(cs),
            Ty::Data { ctors, .. } => {
                let mut total = 0u64;
                for (_, fields) in ctors {
                    total = total.checked_add(product_size(fields)?)?;
                }
                Some(total)
            }
            Ty::Opt(t) => t.domain_size()?.checked_add(1),
            Ty::Res(t, cases) => t.domain_size()?.checked_add(cases.len() as u64),
            Ty::List(t) => {
                let n = t.domain_size()?;
                let (d, e) = (n.min(16), n.min(4));
                1u64.checked_add(d)?.checked_add(d * d)?.checked_add(e * e * e)
            }
        }
    }

    /// The most bytes one element of the domain can occupy: every node costs
    /// [`VALUE_OVERHEAD`] plus, for `Bytes`, its payload (VER04j section 14.1,
    /// item 20).  For a scalar this is the exact `payload + overhead` of VER04h.
    fn elem_cost(&self) -> Option<u64> {
        let node = VALUE_OVERHEAD;
        match self {
            Ty::Bytes => node.checked_add(8),
            Ty::BytesN(n) => node.checked_add(*n),
            Ty::Tuple(cs) => {
                let mut c = node;
                for t in cs {
                    c = c.checked_add(t.elem_cost()?)?;
                }
                Some(c)
            }
            Ty::Data { ctors, .. } => {
                let mut worst = 0u64;
                for (_, fields) in ctors {
                    let mut c = 0u64;
                    for t in fields {
                        c = c.checked_add(t.elem_cost()?)?;
                    }
                    worst = worst.max(c);
                }
                node.checked_add(worst)
            }
            Ty::Opt(t) => node.checked_add(t.elem_cost()?),
            Ty::Res(t, _) => node.checked_add(t.elem_cost()?.max(2 * node)),
            Ty::List(t) => node.checked_add(t.elem_cost()?.checked_mul(3)?),
            _ => Some(node),
        }
    }

    /// Upper bound on the bytes the enumerated domain occupies.
    fn domain_bytes(&self) -> Option<u64> {
        self.domain_size()?.checked_mul(self.elem_cost()?)
    }
}

/// Size of the Cartesian product of component domains (`None` on overflow).
fn product_size(cs: &[Ty]) -> Option<u64> {
    let mut n = 1u64;
    for c in cs {
        n = n.checked_mul(c.domain_size()?)?;
    }
    Some(n)
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

/// The fields of a record or constructor: name and declared type.
type FieldTys<'a> = Vec<(&'a str, TypeExpr<'a>)>;

/// What the generator needs to know about the document's type declarations to
/// turn a `TypeExpr` into a [`Ty`]: aliases, records, variants, error types
/// and the constructor table.
#[derive(Default)]
struct TyEnv<'a> {
    aliases: HashMap<&'a str, TypeExpr<'a>>,
    records: HashMap<&'a str, FieldTys<'a>>,
    variants: HashMap<&'a str, Vec<(&'a str, FieldTys<'a>)>>,
    errors: HashMap<&'a str, Vec<&'a str>>,
    /// constructor name -> (type name, index, arity)
    ctors: HashMap<String, (String, usize, usize)>,
    /// record type name -> field names in order
    fields: BTreeMap<String, Vec<String>>,
}

impl<'a> TyEnv<'a> {
    fn new(file: &WarrantFile<'a>) -> TyEnv<'a> {
        let mut env = TyEnv {
            aliases: HashMap::new(),
            records: HashMap::new(),
            variants: HashMap::new(),
            errors: HashMap::new(),
            ctors: HashMap::new(),
            fields: BTreeMap::new(),
        };
        for d in &file.declarations {
            match d {
                Declaration::Model(m) if m.body.is_none() => {
                    env.aliases.insert(m.name, m.target_type.clone());
                }
                Declaration::Error(e) => {
                    env.errors.insert(e.name, e.variants.clone());
                }
                Declaration::Record(r) => {
                    env.ctors
                        .insert(r.name.to_string(), (r.name.to_string(), 0, r.fields.len()));
                    env.fields.insert(
                        r.name.to_string(),
                        r.fields.iter().map(|f| f.name.to_string()).collect(),
                    );
                    env.records.insert(
                        r.name,
                        r.fields.iter().map(|f| (f.name, f.ty.clone())).collect(),
                    );
                }
                Declaration::Variant(v) => {
                    let mut list = Vec::new();
                    for (i, c) in v.ctors.iter().enumerate() {
                        env.ctors
                            .insert(c.name.to_string(), (v.name.to_string(), i, c.fields.len()));
                        list.push((
                            c.name,
                            c.fields.iter().map(|f| (f.name, f.ty.clone())).collect(),
                        ));
                    }
                    env.variants.insert(v.name, list);
                }
                _ => {}
            }
        }
        env
    }

    /// Does the declared type mention a composite (tuple, record, variant)?
    fn has_composite(&self, t: &TypeExpr<'_>, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        match t {
            TypeExpr::Tuple { .. } => true,
            TypeExpr::Named { name, .. } => {
                self.records.contains_key(name)
                    || self.variants.contains_key(name)
                    || self
                        .aliases
                        .get(name)
                        .is_some_and(|a| self.has_composite(a, depth + 1))
            }
            TypeExpr::Option { inner, .. } | TypeExpr::List { inner, .. } => {
                self.has_composite(inner, depth + 1)
            }
            TypeExpr::Result { ok_type, .. } => self.has_composite(ok_type, depth + 1),
            TypeExpr::Bytes { .. } => false,
        }
    }
}

/// Resolve a declared type to the generator's [`Ty`] (aliases, records,
/// variants and error types looked up in `env`).  Fixed-width names are `Fx`
/// only in a v1 document.
fn ty_of_type<'a>(
    t: &TypeExpr<'a>,
    env: &TyEnv<'a>,
    v1: bool,
    span: (usize, usize),
    depth: usize,
) -> Result<Ty, CodegenError> {
    let nodes = std::cell::Cell::new(0usize);
    ty_of_type_in(t, env, v1, span, depth, &nodes)
}

/// Most nodes a binder or result type may expand to.  Records and variants are
/// inlined by value, so a diamond (`Li(a: L(i-1), b: L(i-1))`) is exponential in
/// its depth although it is a few lines of source; the cap refuses it before any
/// size computation walks the expanded tree (W0706).
pub const MAX_TYPE_NODES: usize = 4096;

fn ty_of_type_in<'a>(
    t: &TypeExpr<'a>,
    env: &TyEnv<'a>,
    v1: bool,
    span: (usize, usize),
    depth: usize,
    nodes: &std::cell::Cell<usize>,
) -> Result<Ty, CodegenError> {
    if depth > 64 {
        return Err(unsupported("type nests too deeply or is recursive", span));
    }
    nodes.set(nodes.get() + 1);
    if nodes.get() > MAX_TYPE_NODES {
        return Err(CodegenError {
            code: DiagnosticCode::W0706,
            kind: CodegenErrorKind::UnsupportedTarget(format!(
                "type expands to more than {MAX_TYPE_NODES} nodes"
            )),
            span,
        });
    }
    let rec = |t: &TypeExpr<'a>| ty_of_type_in(t, env, v1, span, depth + 1, nodes);
    match t {
        TypeExpr::Bytes { width: None, .. } => Ok(Ty::Bytes),
        TypeExpr::Bytes {
            width: Some(Width::Literal(n)),
            ..
        } => {
            if *n > MAX_FIXED_WIDTH {
                return Err(width_too_large(*n, span));
            }
            Ok(Ty::BytesN(*n))
        }
        TypeExpr::Bytes { .. } => Err(unsupported("a symbolic width in a binder type", span)),
        TypeExpr::Named { name, .. } => {
            if v1 {
                if let Some(f) = FixedTy::from_name(name) {
                    return Ok(Ty::Fx(f));
                }
            }
            match *name {
                "Bytes" => Ok(Ty::Bytes),
                "Bool" => Ok(Ty::Bool),
                "U8" => Ok(Ty::U8),
                "U64" => Ok(Ty::U64),
                "Nat" => Ok(Ty::Nat),
                other if v1 => {
                    if let Some(a) = env.aliases.get(other) {
                        return rec(a);
                    }
                    if let Some(fields) = env.records.get(other) {
                        let fs: Result<Vec<Ty>, _> = fields.iter().map(|(_, t)| rec(t)).collect();
                        return Ok(Ty::Data {
                            name: other.to_string(),
                            ctors: vec![(other.to_string(), fs?)],
                        });
                    }
                    if let Some(ctors) = env.variants.get(other) {
                        let mut out = Vec::new();
                        for (c, fields) in ctors {
                            let fs: Result<Vec<Ty>, _> =
                                fields.iter().map(|(_, t)| rec(t)).collect();
                            out.push((c.to_string(), fs?));
                        }
                        return Ok(Ty::Data {
                            name: other.to_string(),
                            ctors: out,
                        });
                    }
                    Err(unsupported(format!("binder type {other}"), span))
                }
                other => Err(unsupported(format!("binder type {other}"), span)),
            }
        }
        _ if !v1 => Err(unsupported(
            "a binder type the generator cannot enumerate",
            span,
        )),
        TypeExpr::Tuple { elems, .. } => {
            let cs: Result<Vec<Ty>, _> = elems.iter().map(rec).collect();
            Ok(Ty::Tuple(cs?))
        }
        TypeExpr::Option { inner, .. } => Ok(Ty::Opt(Box::new(rec(inner)?))),
        TypeExpr::List { inner, .. } => Ok(Ty::List(Box::new(rec(inner)?))),
        TypeExpr::Result {
            ok_type, err_name, ..
        } => {
            let cases = env
                .errors
                .get(err_name)
                .ok_or_else(|| unsupported(format!("error type {err_name}"), span))?;
            Ok(Ty::Res(
                Box::new(rec(ok_type)?),
                cases.iter().map(|c| c.to_string()).collect(),
            ))
        }
    }
}

fn ty_of<'a>(p: &Parameter<'a>, v1: bool, env: &TyEnv<'a>) -> Result<Ty, CodegenError> {
    ty_of_type(&p.param_type, env, v1, p.span, 0).map_err(|e| {
        if e.code == DiagnosticCode::W0701
            && matches!(&e.kind, CodegenErrorKind::UnsupportedTarget(m)
                if m.starts_with("a binder type the generator cannot enumerate"))
        {
            unsupported(
                format!("binder `{}` has a type the generator cannot enumerate", p.name),
                p.span,
            )
        } else {
            e
        }
    })
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
    /// The document is a v1 document (WX-B/WX-C built-ins and types).
    v1: bool,
    /// WX-A constructors: name -> (type name, index, arity).
    ctors: &'c HashMap<String, (String, usize, usize)>,
}

fn emit(e: &Expression<'_>, cx: &mut Cx<'_>) -> Result<String, CodegenError> {
    let sp = span_of(e);
    Ok(match e {
        Expression::Grouped { inner, .. } => emit(inner, cx)?,
        Expression::Literal { lit, .. } => match lit {
            Literal::Bool(b) => format!("V::Bool({b})"),
            Literal::U64(n) if cx.v1 => format!("V::Fixed(F::U64, {n}i128)"),
            Literal::U8(n) if cx.v1 => format!("V::Fixed(F::U8, {n}i128)"),
            Literal::Int(n) if cx.v1 => format!("V::Int({n}i128)"),
            Literal::Fixed(f, v) => format!("V::Fixed(F::{}, {v}i128)", f.name()),
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
            UnaryOp::Neg if cx.v1 => format!("V::neg(&{})", emit(operand, cx)?),
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
            // WX-A: `x.f.g` with a binder head is projection and wins over an
            // error constructor of the same spelling (VER04j section 4.3).
            if cx.v1 && segs.len() >= 2 && args.is_none() && cx.binders.contains(segs[0]) {
                let mut out = format!("b_{}", segs[0]);
                for f in &segs[1..] {
                    let _ = write!(out, ".field(\"{f}\")");
                }
                return Ok(out);
            }
            // WX-A constructors: `C(e1, ..., en)` and the bare nullary `C`.
            if segs.len() == 1 && !cx.binders.contains(segs[0]) {
                if let Some((ty, idx, arity)) = cx.ctors.get(segs[0]) {
                    let mut parts = Vec::new();
                    for a in args.iter().flat_map(|l| l.iter()) {
                        parts.push(emit(a, cx)?);
                    }
                    if parts.len() != *arity || !widths.is_empty() {
                        return Err(unsupported(
                            format!("constructor `{}` expects {arity} arguments", segs[0]),
                            sp,
                        ));
                    }
                    return Ok(format!(
                        "V::Con(\"{ty}\", {idx}, \"{}\", vec![{}])",
                        segs[0],
                        parts.join(", ")
                    ));
                }
            }
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
                    } else if cx.v1 && n == "None" {
                        "V::Opt(None)".to_string()
                    } else {
                        return Err(unsupported(format!("unbound name `{n}`"), sp));
                    }
                }
                Some(list) => {
                    let mut parts = Vec::new();
                    for a in list {
                        parts.push(emit(a, cx)?);
                    }
                    let v1_call = if cx.v1 && !cx.models.contains(n) && !cx.ops.contains_key(n) {
                        emit_v1_call(n, widths, &parts, sp)?
                    } else {
                        None
                    };
                    match n {
                        _ if v1_call.is_some() => v1_call.unwrap(),
                        "Ok" if parts.len() == 1 => format!("V::Ok(Box::new({}))", parts[0]),
                        "Err" if parts.len() == 1 => format!("V::err_of({})", parts[0]),
                        "len" if parts.len() == 1 => format!("{}.length()", parts[0]),
                        // WX-H: byte-string `concat`, so a claim can build a
                        // well-formed stream for the operation under test.
                        "concat" if cx.v1 && parts.len() == 2 => {
                            format!("V::concat(&{}, &{})", parts[0], parts[1])
                        }
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
        Expression::Tuple { elements, .. } => {
            let mut parts = Vec::new();
            for el in elements {
                parts.push(emit(el, cx)?);
            }
            format!("V::Tup(vec![{}])", parts.join(", "))
        }
        Expression::Project { base, key, .. } => {
            let b = emit(base, cx)?;
            match key {
                ProjKey::Index(i) => format!("{b}.proj({i})"),
                ProjKey::Field(f) => format!("{b}.field(\"{f}\")"),
            }
        }
        // WX-A `match`: a closure tries each arm's pattern in order; the checker
        // has proved one matches (VER04j sections 4.3 and 4.5).
        Expression::Match { target, arms, .. } => {
            let mut out = format!("(|| -> V {{\nlet s = {};\n", emit(target, cx)?);
            for arm in arms {
                let mut names = Vec::new();
                let pat = emit_pattern(&arm.pattern, cx, &mut names, sp)?;
                let added: Vec<String> = names
                    .iter()
                    .filter(|n| cx.binders.insert((*n).clone()))
                    .cloned()
                    .collect();
                let body = emit(&arm.body, cx)?;
                for n in &added {
                    cx.binders.remove(n);
                }
                let mut lets = String::new();
                if !names.is_empty() {
                    lets.push_str("let mut it = bs.into_iter(); ");
                    for n in &names {
                        let _ = write!(lets, "let b_{n} = it.next().expect(\"binder\"); ");
                    }
                }
                let _ = writeln!(
                    out,
                    "{{ let mut bs: Vec<V> = Vec::new(); if pat_match(&{pat}, &s, &mut bs) {{ {lets}return {body}; }} }}"
                );
            }
            out.push_str("panic!(\"non-exhaustive match\")\n})()");
            out
        }
        Expression::Quantified { .. } | Expression::List { .. } => {
            return Err(unsupported("quantifier or list inside a body", sp));
        }
    })
}

/// A pattern as a prelude `Pat` value expression; binder names are appended to
/// `names` in the order `pat_match` pushes their values (left to right).  A bare
/// NAME that is a declared constructor is a constructor pattern, any other NAME
/// a binder (VER04j section 4.3).
fn emit_pattern(
    p: &Pattern<'_>,
    cx: &Cx<'_>,
    names: &mut Vec<String>,
    span: (usize, usize),
) -> Result<String, CodegenError> {
    Ok(match p {
        Pattern::Wild => "Pat::Wild".into(),
        Pattern::Bool(b) => format!("Pat::Bool({b})"),
        Pattern::None => "Pat::None".into(),
        Pattern::Some(q) => format!("Pat::Some(&{})", emit_pattern(q, cx, names, span)?),
        Pattern::Ok(q) => format!("Pat::Ok(&{})", emit_pattern(q, cx, names, span)?),
        Pattern::Err(q) => format!("Pat::Err(&{})", emit_pattern(q, cx, names, span)?),
        Pattern::Tuple(ps) => {
            let mut items = Vec::new();
            for q in ps {
                items.push(emit_pattern(q, cx, names, span)?);
            }
            format!("Pat::Tup(&[{}])", items.join(", "))
        }
        Pattern::Ctor { name, args } => {
            let segs = &name.segments;
            if segs.len() == 2 {
                return Ok(format!("Pat::Case(\"{}\")", segs[1]));
            }
            if let Some((_, idx, arity)) = cx.ctors.get(segs[0]) {
                if args.len() != *arity {
                    return Err(unsupported(
                        format!("pattern `{}` has the wrong number of fields", segs[0]),
                        span,
                    ));
                }
                let mut items = Vec::new();
                for q in args {
                    items.push(emit_pattern(q, cx, names, span)?);
                }
                return Ok(format!("Pat::Con({idx}, &[{}])", items.join(", ")));
            }
            if !args.is_empty() {
                return Err(unsupported(format!("unknown constructor `{}`", segs[0]), span));
            }
            names.push(segs[0].to_string());
            "Pat::Bind".into()
        }
    })
}

// ---------------------------------------------------------------------------
// WX-B / WX-C expression translation (v1 documents only)
// ---------------------------------------------------------------------------

/// The static type argument of `widen<T>(x)` and friends, as the prelude's
/// `F::T` constant.
fn static_fixed(widths: &[Width<'_>], span: (usize, usize)) -> Result<String, CodegenError> {
    match widths {
        [Width::Named(n)] if FixedTy::from_name(n).is_some() => Ok(format!("F::{n}")),
        _ => Err(unsupported("static argument is not a fixed-width type", span)),
    }
}

/// Translate a call to a v1 built-in over already translated operands, or
/// `None` when `name` is not one (a user model or operation).
fn emit_v1_call(
    name: &str,
    widths: &[Width<'_>],
    p: &[String],
    span: (usize, usize),
) -> Result<Option<String>, CodegenError> {
    let arity = match name {
        "max_value" | "min_value" => 0,
        "widen" | "wrap" | "try" | "to_int" | "to_nat" | "bit_not" | "popcount" | "clz" | "ctz"
        | "be_bytes" | "le_bytes" | "from_be" | "from_le" | "Some" => 1,
        "checked_add" | "checked_sub" | "checked_mul" | "div" | "rem" | "div_trunc"
        | "rem_trunc" | "unwrap_or" | "imin" | "imax" | "bit_and" | "bit_or" | "bit_xor" | "shl" | "shr" | "sar"
        | "rotl" | "rotr" | "bit" => 2,
        _ => return Ok(None),
    };
    if p.len() != arity {
        return Err(unsupported(format!("`{name}` expects {arity} operands"), span));
    }
    let t = || static_fixed(widths, span);
    Ok(Some(match name {
        "Some" => format!("V::Opt(Some(Box::new({})))", p[0]),
        "max_value" => format!("V::max_value({})", t()?),
        "min_value" => format!("V::min_value({})", t()?),
        "widen" => format!("V::widen({}, &{})", t()?, p[0]),
        "wrap" => format!("V::wrap({}, &{})", t()?, p[0]),
        "try" => format!("V::try_({}, &{})", t()?, p[0]),
        "to_int" => format!("V::to_int(&{})", p[0]),
        "to_nat" => format!("V::to_nat(&{})", p[0]),
        "checked_add" => format!("V::checked('+', &{}, &{})", p[0], p[1]),
        "checked_sub" => format!("V::checked('-', &{}, &{})", p[0], p[1]),
        "checked_mul" => format!("V::checked('*', &{}, &{})", p[0], p[1]),
        "div" => format!("V::divrem(true, &{}, &{})", p[0], p[1]),
        "rem" => format!("V::divrem(false, &{}, &{})", p[0], p[1]),
        "div_trunc" => format!("V::divrem_trunc(true, &{}, &{})", p[0], p[1]),
        "rem_trunc" => format!("V::divrem_trunc(false, &{}, &{})", p[0], p[1]),
        "unwrap_or" => format!("V::unwrap_or(&{}, &{})", p[0], p[1]),
        "imin" => format!("V::imin(&{}, &{})", p[0], p[1]),
        "imax" => format!("V::imax(&{}, &{})", p[0], p[1]),
        "bit_and" => format!("V::bitop('&', &{}, &{})", p[0], p[1]),
        "bit_or" => format!("V::bitop('|', &{}, &{})", p[0], p[1]),
        "bit_xor" => format!("V::bitop('^', &{}, &{})", p[0], p[1]),
        "bit_not" => format!("V::bit_not(&{})", p[0]),
        "shl" | "shr" | "sar" | "rotl" | "rotr" => {
            format!("V::shift(\"{name}\", &{}, &{})", p[0], p[1])
        }
        "popcount" | "clz" | "ctz" => format!("V::bitcount(\"{name}\", &{})", p[0]),
        "bit" => format!("V::bit(&{}, &{})", p[0], p[1]),
        "be_bytes" => format!("V::endian_bytes(true, &{})", p[0]),
        "le_bytes" => format!("V::endian_bytes(false, &{})", p[0]),
        "from_be" => format!("V::from_endian(true, {}, &{})", t()?, p[0]),
        _ => format!("V::from_endian(false, {}, &{})", t()?, p[0]),
    }))
}

/// Names of binders that flow *directly* as the count (second) argument of a
/// shift, rotate or `bit` call (VER04j section 3.6): those `Nat` binders get
/// the count-aware domain.
fn count_binders(e: &Expression<'_>, out: &mut HashSet<String>) {
    match e {
        Expression::Reference { name, args, .. } => {
            if let Some(list) = args {
                if name.segments.len() == 1
                    && matches!(name.segments[0], "shl" | "shr" | "sar" | "rotl" | "rotr" | "bit")
                {
                    if let Some(Expression::Reference {
                        name: n, args: None, ..
                    }) = list.get(1)
                    {
                        if n.segments.len() == 1 {
                            out.insert(n.segments[0].to_string());
                        }
                    }
                }
                for a in list {
                    count_binders(a, out);
                }
            }
        }
        Expression::Binary { left, right, .. } => {
            count_binders(left, out);
            count_binders(right, out);
        }
        Expression::Unary { operand, .. } => count_binders(operand, out),
        Expression::Grouped { inner, .. } => count_binders(inner, out),
        Expression::Project { base, .. } => count_binders(base, out),
        Expression::Tuple { elements, .. } => {
            for x in elements {
                count_binders(x, out);
            }
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            count_binders(condition, out);
            count_binders(then_branch, out);
            count_binders(else_branch, out);
        }
        Expression::Quantified { body, .. } => count_binders(body, out),
        Expression::Match { target, arms, .. } => {
            count_binders(target, out);
            for a in arms {
                count_binders(&a.body, out);
            }
        }
        Expression::List { elements, .. } => {
            for x in elements {
                count_binders(x, out);
            }
        }
        Expression::Literal { .. } => {}
    }
}

/// Demote the leftmost full `U16`/`I16` leaf of `ty` (depth first) to its edge
/// set, recording its component path: a tuple component or record field index,
/// and for a variant the constructor name before the field index (`a.Q.0`).
fn demote_first(ty: &mut Ty, path: &mut Vec<String>) -> bool {
    match ty {
        Ty::Fx(f) if f.bits() == 16 => {
            *ty = Ty::FxEdge(*f);
            true
        }
        Ty::Tuple(cs) => cs.iter_mut().enumerate().any(|(i, c)| {
            path.push(i.to_string());
            let hit = demote_first(c, path);
            if !hit {
                path.pop();
            }
            hit
        }),
        Ty::Data { ctors, .. } => {
            let several = ctors.len() > 1;
            ctors.iter_mut().any(|(cname, fs)| {
                if several {
                    path.push(cname.clone());
                }
                let hit = fs.iter_mut().enumerate().any(|(i, c)| {
                    path.push(i.to_string());
                    let hit = demote_first(c, path);
                    if !hit {
                        path.pop();
                    }
                    hit
                });
                if !hit && several {
                    path.pop();
                }
                hit
            })
        }
        Ty::Opt(t) | Ty::List(t) | Ty::Res(t, _) => demote_first(t, path),
        _ => false,
    }
}

/// VER04j section 2.6, the budget rule.  Every binder starts at its default
/// domain (`full` where it has one); while the Cartesian product (or the byte
/// budget) is over `budget`, demote the *leftmost* `full` `U16`/`I16` leaf (for a
/// composite binder, depth first inside it) to its edge set.  Returns the
/// binder index and component path of each demotion, in order.  `U8`/`I8`
/// leaves are never demoted.  If nothing is left to demote and the claim is
/// still over budget the caller reports W0706 -- no silent sampling.
fn demote_for_budget(tys: &mut [Ty], budget: u64) -> Vec<(usize, Vec<String>)> {
    let mut demoted = Vec::new();
    loop {
        let over = match claim_domain(tys) {
            Some((p, b)) => p > budget || b > DOMAIN_BYTES_BUDGET,
            None => true,
        };
        if !over {
            break;
        }
        let mut done = false;
        for (i, t) in tys.iter_mut().enumerate() {
            let mut path = Vec::new();
            if demote_first(t, &mut path) {
                demoted.push((i, path));
                done = true;
                break;
            }
        }
        if !done {
            break;
        }
    }
    demoted
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

/// The faults of VER04j section 4.8: rewrites of a composite operation's result.
/// They apply to the operation they are bound to only.
const WXA_FAULTS: &[(&str, &str)] = &[
    ("drop-consumed", "DropConsumed"),
    ("swap-components", "SwapComponents"),
    ("field-zero", "FieldZero"),
    ("wrong-ctor", "WrongCtor"),
    ("ok-as-err", "OkAsErr"),
    ("off-by-one-consumed", "OffByOneConsumed"),
];

/// Faults for an operation whose result is a byte string (a digest, a codec
/// output): rewrites of the composite adapter's result, so a pinned table (WX-E)
/// has something to fail.  They apply to the operation they are bound to only.
const WXE_FAULTS: &[(&str, &str)] = &[
    ("flip-result-bit", "FlipResultBit"),
    ("drop-last-result-byte", "DropLastResultByte"),
];

/// The faults of VER04j section 6.4 (WX-H), rewrites of a composite adapter's
/// limit parameter or result.  They apply to the operation they are bound to only.
const WXH_FAULTS: &[(&str, &str)] = &[
    ("ignore-limit", "IgnoreLimit"),
    ("off-by-one-limit", "OffByOneLimit"),
    ("limit-after-alloc", "LimitAfterAlloc"),
    ("truncate-silently", "TruncateSilently"),
];

fn fault_variant(name: &str) -> Option<&'static str> {
    FAULTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .or_else(|| {
            V1_FAULTS
                .iter()
                .find(|(n, _, _)| *n == name)
                .map(|(_, v, _)| *v)
        })
        .or_else(|| {
            WXA_FAULTS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| *v)
        })
        .or_else(|| {
            WXH_FAULTS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| *v)
        })
        .or_else(|| {
            WXE_FAULTS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| *v)
        })
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
        .chain(V1_FAULTS.iter().map(|(n, v, _)| (*n, *v)))
        .chain(WXA_FAULTS.iter().copied())
        .chain(WXH_FAULTS.iter().copied())
        .chain(WXE_FAULTS.iter().copied())
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
/// enabled (marker lines never reach the output).  Regions nest: a line is kept
/// only if every enclosing region is.  A document enables exactly one of the
/// features `v0` and `v1`, so a v0 document's output is the v0 text unchanged.
fn expand_prelude(faults: &str, features: &BTreeSet<&str>) -> String {
    let mut out = String::new();
    let mut stack: Vec<bool> = Vec::new();
    for line in PRELUDE.lines() {
        if let Some(f) = line.strip_prefix("//@if ") {
            let outer = stack.last().copied().unwrap_or(true);
            stack.push(outer && features.contains(f.trim()));
            continue;
        }
        if line == "//@endif" {
            stack.pop();
            continue;
        }
        if !stack.last().copied().unwrap_or(true) {
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
    /// v1: scalar parameters (fixed-width integers, `Nat`) and a fixed-width,
    /// `Bool`, `Nat`, `Option` or `Result` result.
    ScalarV1(RetV1),
    /// WX-A: a signature that mentions a tuple, record or variant, converted to
    /// and from the host through the bindings' `struct`/`enum`/`ret=` data.
    Composite,
}

impl Shape {
    /// Does the shape's adapter obey a fault only while its operation is the
    /// one under attack (the `scoped` mechanism)?
    fn scoped(self) -> bool {
        matches!(
            self,
            Shape::Scalar { .. } | Shape::ScalarV1(_) | Shape::Composite
        )
    }
}

impl Shape {
    /// Faults a fixed shape owns.  The scalar shape owns none: its faults are
    /// the ones the operation's binding names (`negative_controls`).
    fn faults(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Shape::Scalar { .. } | Shape::ScalarV1(_) | Shape::Composite => &[],
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

fn shape_of<'a>(op: &OpDecl<'a>, v1: bool, env: &TyEnv<'a>) -> Result<Shape, CodegenError> {
    if v1
        && (op.params.iter().any(|p| env.has_composite(&p.param_type, 0))
            || env.has_composite(&op.return_type, 0))
    {
        return Ok(Shape::Composite);
    }
    let tys: Result<Vec<Ty>, _> = op
        .params
        .iter()
        .map(|p| ty_of_op_param(p, v1, env))
        .collect();
    let tys = tys?;
    if v1 {
        let scalar_params = !tys.is_empty()
            && tys
                .iter()
                .all(|t| matches!(t, Ty::Fx(_) | Ty::Nat | Ty::NatCount));
        if scalar_params && op.static_params.is_empty() && !op.is_fresh {
            if let Some(ret) = v1_ret(&op.return_type) {
                return Ok(Shape::ScalarV1(ret));
            }
        }
    }
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
        // WX-H (VER04j section 14.2 item 35): any other v1 signature (a `Bytes`
        // result, a `Bytes` parameter beside a number, ...) goes through the
        // composite adapter, whose host conversions say what is supported.
        _ if v1 => Ok(Shape::Composite),
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
fn ty_of_op_param<'a>(p: &Parameter<'a>, v1: bool, env: &TyEnv<'a>) -> Result<Ty, CodegenError> {
    match &p.param_type {
        TypeExpr::Bytes {
            width: Some(Width::Named(_)),
            ..
        } => Ok(Ty::BytesN(0)),
        _ => ty_of(p, v1, env),
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
        // Scalar operations are emitted by `scalar_adapter`/`scalar_adapter_v1`.
        Shape::Scalar { .. } | Shape::ScalarV1(_) | Shape::Composite => {}
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
        .map(|p| ty_of_op_param(p, false, &TyEnv::default()))
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
// WX-B / WX-C scalar adapters and faults (v1 documents only)
// ---------------------------------------------------------------------------

/// What a v1 scalar export returns, as the Warrant return type declares it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RetV1 {
    /// A fixed-width integer (`U32`, `I8`, ...).
    Fixed(FixedTy),
    Bool,
    /// A `Nat` count (`popcount`, `clz`, `ctz`).
    Nat,
    /// `Option<T>` (the `try<T>` shape).
    Opt(FixedTy),
    /// `Result<T, E>` through panic normalisation.
    Res(FixedTy),
}

/// How a v1 fault rewrites the adapter.  Templates are Rust source over:
/// `{k}` the package crate, `{e}` the export, `{args}` the converted arguments
/// `p0, p1, ...`, `{t}` the Rust type of the first argument, `{u}` its unsigned
/// counterpart, `{w}` its width in bits, `{rt}`/`{ru}`/`{half}` the result type,
/// its unsigned counterpart and the type one width narrower, and `{pt}` (only
/// in [`V1Fault::PreAll`]) the Rust type of the argument being rewritten.
#[derive(Clone, Copy)]
enum V1Fault {
    /// The call is replaced by the expression.
    Replace(&'static str),
    /// The result `r` is rewritten by the expression.
    Post(&'static str),
    /// The last argument `p` is rewritten.
    PreLast(&'static str),
    /// Every signed argument `p` is rewritten.
    PreAll(&'static str),
}

/// The faults of VER04j sections 2.7 and 3.7.  The arithmetic-shaped ones
/// carry the operator in their name (`saturate-add`), because an adapter is
/// told what the operation *is* only through its binding; the rest need no
/// operator.
const V1_FAULTS: &[(&str, &str, V1Fault)] = &[
    // saturating instead of wrapping arithmetic
    ("saturate-add", "SaturateAdd", V1Fault::Replace("p0.saturating_add(p1)")),
    ("saturate-sub", "SaturateSub", V1Fault::Replace("p0.saturating_sub(p1)")),
    ("saturate-mul", "SaturateMul", V1Fault::Replace("p0.saturating_mul(p1)")),
    // overflow returns 0
    ("trap-as-zero-add", "TrapAsZeroAdd", V1Fault::Replace("p0.checked_add(p1).unwrap_or(0)")),
    ("trap-as-zero-sub", "TrapAsZeroSub", V1Fault::Replace("p0.checked_sub(p1).unwrap_or(0)")),
    ("trap-as-zero-mul", "TrapAsZeroMul", V1Fault::Replace("p0.checked_mul(p1).unwrap_or(0)")),
    // the adapter computes one width narrower
    ("narrow-width", "NarrowWidth", V1Fault::Post("{narrow}")),
    // a signed value compared as unsigned: x ^ MIN maps signed order to
    // unsigned order, so the honest export then orders the unsigned values
    ("sign-confuse", "SignConfuse", V1Fault::PreAll("p ^ {pt}::MIN")),
    // a `try`-shaped op accepts max + 1 and min - 1
    (
        "off-by-one-range",
        "OffByOneRange",
        V1Fault::Post("if r.is_none() && (p0 as i128 == {rt}::MAX as i128 + 1 || p0 as i128 == {rt}::MIN as i128 - 1) { Some(p0 as {rt}) } else { r }"),
    ),
    // shift count reduced modulo the width (x86, wrapping_shl)
    ("shift-mask", "ShiftMask", V1Fault::PreLast("p % {w}")),
    // a count >= w returns the value unshifted (a C undefined-behaviour outcome)
    ("shift-ub", "ShiftUb", V1Fault::Replace("if p1 >= {w} { p0 } else { {e}({args}) }")),
    // rotation computed as a shift: zero fill
    ("rotate-as-shift-left", "RotateAsShiftLeft", V1Fault::Replace("if p1 >= {w} { 0 } else { p0 << p1 }")),
    (
        "rotate-as-shift-right",
        "RotateAsShiftRight",
        V1Fault::Replace("if p1 >= {w} { 0 } else { ((p0 as {u}) >> p1) as {t} }"),
    ),
    // arithmetic right shift computed as a logical one
    ("sar-as-shr", "SarAsShr", V1Fault::Replace("if p1 >= {w} { 0 } else { ((p0 as {u}) >> p1) as {t} }")),
    // popcount that counts only the low w - 1 bits
    ("popcount-off", "PopcountOff", V1Fault::Replace("((p0 as {u}) & ({u}::MAX >> 1)).count_ones()")),
];

fn v1_fault(name: &str) -> Option<V1Fault> {
    V1_FAULTS.iter().find(|(n, _, _)| *n == name).map(|(_, _, k)| *k)
}

/// The Rust name of a parameter's native type: the binding's `args=` entry, or
/// the type matching the Warrant type (`Nat` is `u32`).
fn rust_arg_type(t: &Ty, given: Option<&str>) -> Result<String, String> {
    let default = match t {
        Ty::Fx(f) | Ty::FxEdge(f) => f.suffix(),
        _ => "u32",
    };
    let rt = given.unwrap_or(default);
    if FixedTy::from_suffix(rt).is_none() {
        return Err(rt.to_string());
    }
    Ok(rt.to_string())
}

/// `{u}` for a Rust integer type name (`i16` becomes `u16`).
fn unsigned_of(rt: &str) -> String {
    format!("u{}", &rt[1..])
}

fn v1_ret(t: &TypeExpr<'_>) -> Option<RetV1> {
    let fixed = |t: &TypeExpr<'_>| match t {
        TypeExpr::Named { name, .. } => FixedTy::from_name(name),
        _ => None,
    };
    match t {
        TypeExpr::Named { name: "Bool", .. } => Some(RetV1::Bool),
        TypeExpr::Named { name: "Nat", .. } => Some(RetV1::Nat),
        TypeExpr::Option { inner, .. } => fixed(inner).map(RetV1::Opt),
        TypeExpr::Result { ok_type, .. } => fixed(ok_type).map(RetV1::Res),
        other => fixed(other).map(RetV1::Fixed),
    }
}

/// Adapter for a scalar-parameter operation of a v1 document: parameters are
/// fixed-width integers or `Nat`s, the result is a fixed-width integer, `Bool`,
/// `Nat`, `Option<T>` or `Result<T, E>`.  Arguments are range-checked into the
/// export's Rust types (a value that does not fit is a harness bug and panics
/// outside any claim); every fault the binding lists adds one `match fault` arm,
/// so with `Fault::None` the adapter is exactly `export(p0, p1, ...)`.
fn scalar_adapter_v1(
    op: &OpDecl<'_>,
    ret: RetV1,
    export: &str,
    err_case: Option<&str>,
    faults: &[String],
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    let n = op.name;
    let tys: Vec<Ty> = op
        .params
        .iter()
        .map(|p| ty_of_op_param(p, true, &TyEnv::default()))
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
    let mut rts = Vec::new();
    for (i, t) in tys.iter().enumerate() {
        let rt = rust_arg_type(t, given.map(|g| g[i].as_str()))
            .map_err(|bad| unsupported(format!("argument type `{bad}` of `{n}`"), op.span))?;
        let _ = writeln!(
            conv,
            "    let p{i} = {rt}::try_from(args[{i}].int()).expect(\"argument {i} of {n} fits {rt}\");"
        );
        names.push(format!("p{i}"));
        rts.push(rt);
    }
    let arglist = names.join(", ");
    let t0 = rts[0].clone();
    let result_rt = match ret {
        RetV1::Fixed(f) | RetV1::Opt(f) | RetV1::Res(f) => f.suffix().to_string(),
        RetV1::Nat | RetV1::Bool => "u32".to_string(),
    };
    let half = match result_rt.as_str() {
        "u16" => "u8".to_string(),
        "i16" => "i8".to_string(),
        r if r.len() == 3 => format!("{}{}", &r[..1], r[1..].parse::<u32>().unwrap_or(8) / 2),
        _ => "u8".to_string(),
    };
    let narrow = if result_rt.ends_with('8') {
        "r & 0x0f".to_string()
    } else {
        format!("(r as {half}) as {result_rt}")
    };
    let subst = |t: &str| {
        t.replace("{narrow}", &narrow)
            .replace("{k}", &options.package_crate)
            .replace("{e}", export)
            .replace("{args}", &arglist)
            .replace("{t}", &t0)
            .replace("{u}", &unsigned_of(&t0))
            .replace("{w}", &t0[1..])
            .replace("{rt}", &result_rt)
            .replace("{ru}", &unsigned_of(&result_rt))
            .replace("{half}", &half)
    };
    let mut pre = String::new();
    let mut replace_arms = String::new();
    let mut post_arms = String::new();
    let mut missing = false;
    let last = tys.len() - 1;
    for f in faults {
        if f == "missing-error" {
            if !matches!(ret, RetV1::Res(_)) {
                return Err(unsupported(
                    format!("fault `missing-error` needs a fallible operation, `{n}` is total"),
                    op.span,
                ));
            }
            missing = true;
            continue;
        }
        let (Some(variant), Some(kind)) = (fault_variant(f), v1_fault(f)) else {
            return Err(unsupported(
                format!("operation `{n}` names unknown v1 fault `{f}`"),
                op.span,
            ));
        };
        match kind {
            V1Fault::Replace(t) => {
                let _ = writeln!(replace_arms, "            Fault::{variant} => {},", subst(t));
            }
            V1Fault::Post(t) => {
                let _ = writeln!(post_arms, "            Fault::{variant} => {},", subst(t));
            }
            V1Fault::PreLast(t) => {
                let _ = writeln!(
                    pre,
                    "    let p{last} = if fault == Fault::{variant} {{ let p = p{last}; {} }} else {{ p{last} }};",
                    subst(t)
                );
            }
            V1Fault::PreAll(t) => {
                let mut any = false;
                for (i, rt) in rts.iter().enumerate() {
                    if !rt.starts_with('i') {
                        continue;
                    }
                    any = true;
                    let body = subst(t).replace("{pt}", rt);
                    let _ = writeln!(
                        pre,
                        "    let p{i} = if fault == Fault::{variant} {{ let p = p{i}; {body} }} else {{ p{i} }};"
                    );
                }
                if !any {
                    return Err(unsupported(
                        format!("fault `{f}` needs a signed argument, `{n}` has none"),
                        op.span,
                    ));
                }
            }
        }
    }
    let mut compute = String::new();
    let _ = write!(
        compute,
        "let r = match fault {{\n{replace_arms}            _ => {export}({arglist}),\n        }};\n"
    );
    if !post_arms.is_empty() {
        let _ = write!(compute, "        let r = match fault {{\n{post_arms}            _ => r,\n        }};\n");
    }
    let _ = writeln!(conv, "    let fault = fault_for(\"{n}\", fault);");
    let wrap = |expr: &str, fty: FixedTy| format!("V::Fixed(F::{}, {expr} as i128)", fty.name());
    let mut s = String::new();
    match ret {
        RetV1::Res(f) => {
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
                 \x20       Some(r) => V::Ok(Box::new({})),\n",
                wrap("r", f)
            );
            if missing {
                let _ = writeln!(
                    s,
                    "        None if fault == Fault::MissingError => V::Ok(Box::new({})),",
                    wrap("0", f)
                );
            }
            let _ = write!(s, "        None => V::Err(\"{case}\".into()),\n    }}\n}}\n\n");
        }
        _ => {
            let result = match ret {
                RetV1::Fixed(f) => wrap("r", f),
                RetV1::Bool => "V::Bool(r)".to_string(),
                RetV1::Nat => "V::Int(r as i128)".to_string(),
                RetV1::Opt(f) => format!(
                    "V::Opt(r.map(|x| Box::new(V::Fixed(F::{}, x as i128))))",
                    f.name()
                ),
                RetV1::Res(_) => unreachable!(),
            };
            let _ = write!(
                s,
                "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{\n{conv}{pre}\
                 \x20   {compute}    {result}\n}}\n\n"
            );
        }
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// WX-A composite adapters (VER04j sections 4.6 and 4.8)
// ---------------------------------------------------------------------------

/// The host type a Warrant type maps to when the bindings do not say
/// (scalars, tuples of them, `Option`, `List`); records and variants must be
/// declared with a `struct` or `enum` line.
fn default_host(t: &Ty) -> Result<HostTy, String> {
    Ok(match t {
        Ty::Fx(f) | Ty::FxEdge(f) => HostTy::Int(f.suffix().to_string()),
        Ty::Nat | Ty::NatCount => HostTy::Int("u32".into()),
        Ty::Bool => HostTy::Bool,
        Ty::Bytes | Ty::BytesN(_) => HostTy::Bytes,
        Ty::Tuple(cs) => HostTy::Tuple(cs.iter().map(default_host).collect::<Result<_, _>>()?),
        Ty::Opt(t) => HostTy::Opt(Box::new(default_host(t)?)),
        Ty::List(t) => HostTy::List(Box::new(default_host(t)?)),
        Ty::Data { name, .. } => HostTy::Named(name.clone()),
        other => return Err(format!("no default host type for {other:?}")),
    })
}

/// State of one conversion-code generation: the declared host types and a
/// counter that makes every generated variable name unique.
struct HostCx<'h> {
    /// The operation being adapted (named in a length-check message).
    op: String,
    spec: &'h HostSpec,
    /// Warrant record name -> field names, to bind host struct fields by name.
    fields: &'h BTreeMap<String, Vec<String>>,
    ctr: usize,
}

/// A host struct's field names must be the record's field names (or the
/// positional indexes of a tuple struct): binding by position alone would let
/// `fields=b:u8,a:u8` silently swap two fields.
fn check_field_names(
    record: &str,
    host: &[crate::binding::HostField],
    hc: &HostCx<'_>,
) -> Result<(), String> {
    let Some(names) = hc.fields.get(record) else {
        return Ok(());
    };
    for (i, (hf, want)) in host.iter().zip(names).enumerate() {
        if hf.name != *want && hf.name != i.to_string() {
            return Err(format!(
                "struct `{record}` field {i} is `{}`, the record's field is `{want}`",
                hf.name
            ));
        }
    }
    Ok(())
}

impl HostCx<'_> {
    fn fresh(&mut self, prefix: &str) -> String {
        self.ctr += 1;
        format!("{prefix}{}", self.ctr)
    }
}

fn is_numeric(t: &Ty) -> bool {
    matches!(t, Ty::Fx(_) | Ty::FxEdge(_) | Ty::Nat | Ty::NatCount)
}

/// Rust source converting the model value at `place` (an expression of type
/// `&V`) into host type `h`.  A scalar the host integer cannot hold raises
/// `Unreachable` (see `host`), never narrows.  `top` marks a parameter, which
/// is passed to the export by reference where the host type is `bytes`.
fn to_host(
    t: &Ty,
    h: &HostTy,
    place: &str,
    top: bool,
    hc: &mut HostCx<'_>,
) -> Result<String, String> {
    match (t, h) {
        (Ty::Fx(f) | Ty::FxEdge(f), HostTy::Int(rt)) if rt == f.suffix() => {
            Ok(format!("host::<{rt}>({place}.int())"))
        }
        (Ty::Nat | Ty::NatCount, HostTy::Int(rt)) if rt.starts_with('u') => {
            Ok(format!("host::<{rt}>({place}.int())"))
        }
        (Ty::Bool, HostTy::Bool) => Ok(format!("{place}.boolv()")),
        (Ty::Bytes | Ty::BytesN(_), HostTy::Bytes) => Ok(if top {
            format!("{place}.bytes()")
        } else {
            format!("{place}.bytes().to_vec()")
        }),
        // WX-N: the model value narrows to `[u8; N]` by `try_from`; a value of any
        // other length is unreachable by type (the harness's own signal), never
        // truncated or padded.  A top-level parameter is passed by reference, as
        // `bytes` is.
        (Ty::BytesN(n), HostTy::BytesN(m)) if n == m => {
            let arr = format!(
                "<[u8; {n}]>::try_from({place}.bytes()).unwrap_or_else(|_| std::panic::panic_any(Unreachable))"
            );
            Ok(if top { format!("&{arr}") } else { arr })
        }
        (Ty::Tuple(cs), HostTy::Tuple(hs)) if cs.len() == hs.len() => {
            let mut parts = Vec::new();
            for (i, (c, h)) in cs.iter().zip(hs).enumerate() {
                parts.push(to_host(c, h, &format!("{place}.tup()[{i}]"), false, hc)?);
            }
            Ok(format!("({})", parts.join(", ")))
        }
        (Ty::Data { name, ctors }, HostTy::Named(hn)) if hn == name => {
            let decl = hc
                .spec
                .types
                .get(hn)
                .ok_or_else(|| format!("host type `{hn}` is not declared"))?;
            let path = decl.path.clone();
            match &decl.shape {
                HostShape::Struct(hfields) => {
                    if ctors.len() != 1 || ctors[0].1.len() != hfields.len() {
                        return Err(format!("struct `{hn}` does not fit the record's fields"));
                    }
                    check_field_names(name, hfields, hc)?;
                    let mut parts = Vec::new();
                    for (i, (c, hf)) in ctors[0].1.iter().zip(hfields).enumerate() {
                        let v = to_host(c, &hf.ty, &format!("{place}.fields()[{i}]"), false, hc)?;
                        parts.push(format!("{}: {v}", hf.name));
                    }
                    Ok(format!("{path} {{ {} }}", parts.join(", ")))
                }
                HostShape::Enum(hctors) => {
                    if ctors.len() != hctors.len() {
                        return Err(format!("enum `{hn}` has a different number of constructors"));
                    }
                    let mut arms = String::new();
                    for (i, ((cname, cfields), (hname, hfields))) in
                        ctors.iter().zip(hctors).enumerate()
                    {
                        if cname != hname || cfields.len() != hfields.len() {
                            return Err(format!(
                                "enum `{hn}` constructor `{hname}` does not fit `{cname}`"
                            ));
                        }
                        let mut parts = Vec::new();
                        for (j, (c, hf)) in cfields.iter().zip(hfields).enumerate() {
                            let v =
                                to_host(c, &hf.ty, &format!("{place}.fields()[{j}]"), false, hc)?;
                            parts.push(format!("{}: {v}", hf.name));
                        }
                        let _ = write!(arms, "{i} => {path}::{hname} {{ {} }}, ", parts.join(", "));
                    }
                    Ok(format!(
                        "match {place}.ctor() {{ {arms}_ => unreachable!(\"constructor index\") }}"
                    ))
                }
            }
        }
        (Ty::Opt(c), HostTy::Opt(hh)) => {
            let x = hc.fresh("x");
            let inner = to_host(c, hh, &x, false, hc)?;
            Ok(format!("{place}.some_ref().map(|{x}| {inner})"))
        }
        (Ty::List(c), HostTy::List(hh)) => {
            let x = hc.fresh("x");
            let inner = to_host(c, hh, &x, false, hc)?;
            Ok(format!(
                "{place}.list().iter().map(|{x}| {inner}).collect::<Vec<_>>()"
            ))
        }
        _ => Err(format!("host type {h:?} does not fit the Warrant type {t:?}")),
    }
}

/// Rust source converting the host value `expr` (evaluated exactly once) into
/// a model value of type `t`.
fn from_host(t: &Ty, h: &HostTy, expr: &str, hc: &mut HostCx<'_>) -> Result<String, String> {
    match (t, h) {
        (Ty::Fx(f) | Ty::FxEdge(f), HostTy::Int(rt)) if rt == f.suffix() => {
            Ok(format!("V::Fixed(F::{}, ({expr}) as i128)", f.name()))
        }
        (Ty::Nat | Ty::NatCount, HostTy::Int(rt)) if rt.starts_with('u') => {
            Ok(format!("V::Int(({expr}) as i128)"))
        }
        (Ty::Bool, HostTy::Bool) => Ok(format!("V::Bool({expr})")),
        // WX-N: a `Bytes<N>` result must be exactly `N` bytes.  The check sits
        // outside the panic containment, so a wrong-length export fails the
        // harness instead of being compared as a (different) model value.
        (Ty::BytesN(n), HostTy::Bytes | HostTy::BytesN(_)) => {
            if matches!(h, HostTy::BytesN(m) if m != n) {
                return Err(format!("host type {h:?} does not fit the Warrant type {t:?}"));
            }
            let op = hc.op.clone();
            let v = hc.fresh("b");
            Ok(format!(
                "{{ let {v}: Vec<u8> = ({expr}).to_vec(); if {v}.len() != {n} {{ panic!(\"operation `{op}`: the export returned {{}} bytes for a Bytes<{n}> result, which is not {n} bytes\", {v}.len()); }} V::Bytes({v}) }}"
            ))
        }
        (Ty::Bytes, HostTy::Bytes) => Ok(format!("V::Bytes(({expr}).to_vec())")),
        (Ty::Tuple(cs), HostTy::Tuple(hs)) if cs.len() == hs.len() => {
            let v = hc.fresh("t");
            let mut parts = Vec::new();
            for (i, (c, h)) in cs.iter().zip(hs).enumerate() {
                parts.push(from_host(c, h, &format!("{v}.{i}"), hc)?);
            }
            Ok(format!("{{ let {v} = {expr}; V::Tup(vec![{}]) }}", parts.join(", ")))
        }
        (Ty::Data { name, ctors }, HostTy::Named(hn)) if hn == name => {
            let decl = hc
                .spec
                .types
                .get(hn)
                .ok_or_else(|| format!("host type `{hn}` is not declared"))?;
            let path = decl.path.clone();
            match &decl.shape {
                HostShape::Struct(hfields) => {
                    if ctors.len() != 1 || ctors[0].1.len() != hfields.len() {
                        return Err(format!("struct `{hn}` does not fit the record's fields"));
                    }
                    check_field_names(name, hfields, hc)?;
                    let v = hc.fresh("s");
                    let mut parts = Vec::new();
                    for (c, hf) in ctors[0].1.iter().zip(hfields) {
                        parts.push(from_host(c, &hf.ty, &format!("{v}.{}", hf.name), hc)?);
                    }
                    Ok(format!(
                        "{{ let {v} = {expr}; V::Con(\"{name}\", 0, \"{}\", vec![{}]) }}",
                        ctors[0].0,
                        parts.join(", ")
                    ))
                }
                HostShape::Enum(hctors) => {
                    if ctors.len() != hctors.len() {
                        return Err(format!("enum `{hn}` has a different number of constructors"));
                    }
                    let mut arms = String::new();
                    for (i, ((cname, cfields), (hname, hfields))) in
                        ctors.iter().zip(hctors).enumerate()
                    {
                        if cname != hname || cfields.len() != hfields.len() {
                            return Err(format!(
                                "enum `{hn}` constructor `{hname}` does not fit `{cname}`"
                            ));
                        }
                        let (mut pats, mut vals) = (Vec::new(), Vec::new());
                        for (c, hf) in cfields.iter().zip(hfields) {
                            let var = hc.fresh("y");
                            pats.push(format!("{}: {var}", hf.name));
                            vals.push(from_host(c, &hf.ty, &var, hc)?);
                        }
                        let _ = write!(
                            arms,
                            "{path}::{hname} {{ {} }} => V::Con(\"{name}\", {i}, \"{cname}\", vec![{}]), ",
                            pats.join(", "),
                            vals.join(", ")
                        );
                    }
                    Ok(format!("match {expr} {{ {arms}}}"))
                }
            }
        }
        (Ty::Opt(c), HostTy::Opt(hh)) => {
            let x = hc.fresh("x");
            let inner = from_host(c, hh, &x, hc)?;
            Ok(format!(
                "match {expr} {{ None => V::Opt(None), Some({x}) => V::Opt(Some(Box::new({inner}))) }}"
            ))
        }
        (Ty::List(c), HostTy::List(hh)) => {
            let x = hc.fresh("x");
            let inner = from_host(c, hh, &x, hc)?;
            Ok(format!(
                "V::List(({expr}).into_iter().map(|{x}| {inner}).collect())"
            ))
        }
        _ => Err(format!("host type {h:?} does not fit the Warrant type {t:?}")),
    }
}

/// The adapter of an operation whose signature mentions a tuple, record or
/// variant (VER04j sections 4.6 and 4.8).
///
/// ```text
/// fn op_decode(_w, args: &[V], fault) -> V {
///     let fault = fault_for("decode", fault);
///     let p0 = args[0].bytes();                  // bytes  -> &[u8]
///     let p1 = host::<usize>(args[1].int());     // Nat    -> usize, or Unreachable
///     let v = match contained(|| decode(p0, p1)) {          // panic -> Err(CASE)
///         Some(r) => V::Ok(Box::new({ let t = r; V::Tup(vec![..t.0.., ..t.1..]) })),
///         None => V::Err("TRUNCATED".into()),
///     };
///     match fault { Fault::OffByOneConsumed => fault_off_by_one_consumed(v), _ => v }
/// }
/// ```
///
/// An export that returns a host `Result` is bound with `errors=` and its error
/// variants are mapped to Warrant cases instead.  Every fault of the binding is
/// a rewrite of the final value and is checked, at generation, to apply to the
/// declared result type (W0701 otherwise).
fn composite_adapter<'a>(
    op: &OpDecl<'a>,
    env: &TyEnv<'a>,
    export: &str,
    err_case: Option<&str>,
    faults: &[String],
    options: &EvidenceOptions,
    limit: Option<usize>,
) -> Result<String, CodegenError> {
    let n = op.name;
    let bad = |m: String| unsupported(format!("operation `{n}`: {m}"), op.span);
    // A `fresh` result is satisfied by construction here: a host `Vec<u8>` or
    // `Result<Vec<u8>, _>` is owned and cannot alias the input (WX-H, the
    // section 6.2 example is `fresh`).
    if !op.static_params.is_empty() {
        return Err(bad(
            "static widths are not supported on a composite operation".into(),
        ));
    }
    let host = options.hosts.ops.get(n);
    if options.arg_types.contains_key(n) && host.and_then(|h| h.args.as_ref()).is_none() {
        return Err(bad("an args= entry is not a host type".into()));
    }
    let ptys: Vec<Ty> = op
        .params
        .iter()
        .map(|p| ty_of_op_param(p, true, env))
        .collect::<Result<_, _>>()?;
    let hargs: Vec<HostTy> = match host.and_then(|h| h.args.clone()) {
        Some(a) if a.len() == ptys.len() => a,
        Some(a) => {
            return Err(bad(format!(
                "args= has {} entries for {} parameters",
                a.len(),
                ptys.len()
            )))
        }
        None => ptys
            .iter()
            .map(default_host)
            .collect::<Result<_, _>>()
            .map_err(bad)?,
    };
    let full = ty_of_type(&op.return_type, env, true, op.span, 0)?;
    let (payload, cases, is_result) = match &full {
        Ty::Res(t, cases) => ((**t).clone(), cases.clone(), true),
        other => (other.clone(), Vec::new(), false),
    };
    let hret = match host.and_then(|h| h.ret.clone()) {
        Some(r) => r,
        None => default_host(&payload).map_err(bad)?,
    };
    // Which faults make sense for this result (checked, never silently inert).
    // A fault rewrites the composite *inside* an `Ok` or a `Some`.
    let core = match &payload {
        Ty::Opt(t) => (**t).clone(),
        other => other.clone(),
    };
    for f in faults {
        let ok = match f.as_str() {
            "drop-consumed" | "off-by-one-consumed" => {
                matches!(&core, Ty::Tuple(cs) if cs.len() >= 2 && is_numeric(&cs[1]))
            }
            "swap-components" => {
                matches!(&core, Ty::Tuple(cs) if cs.len() >= 2 && cs.iter().all(|c| *c == cs[0]))
            }
            "field-zero" => matches!(&core, Ty::Data { ctors, .. }
                if ctors.len() == 1 && ctors[0].1.iter().any(is_numeric)),
            "wrong-ctor" => matches!(&core, Ty::Data { ctors, .. } if ctors.len() >= 2),
            "ok-as-err" => {
                is_result && matches!(&core, Ty::Tuple(_) | Ty::Data { .. }) && !cases.is_empty()
            }
            "missing-error" => is_result,
            // WX-H: the limit faults rewrite the one numeric parameter the
            // operation's `output` bound mentions.
            "flip-result-bit" | "drop-last-result-byte" => {
                matches!(&core, Ty::Bytes | Ty::BytesN(_))
            }
            "ignore-limit" | "off-by-one-limit" | "limit-after-alloc" => limit.is_some(),
            "truncate-silently" => {
                limit.is_some()
                    && is_result
                    && payload == Ty::Bytes
                    && ptys.contains(&Ty::Bytes)
            }
            _ => {
                return Err(bad(format!("fault `{f}` is not a composite-operation fault")));
            }
        };
        if !ok {
            return Err(bad(format!(
                "fault `{f}` does not apply to the result type of the operation"
            )));
        }
    }
    let mut hc = HostCx {
        op: n.to_string(),
        spec: &options.hosts,
        fields: &env.fields,
        ctr: 0,
    };
    let mut s = String::new();
    let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
    let _ = writeln!(s, "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{");
    let _ = writeln!(s, "    let fault = fault_for(\"{n}\", fault);");
    for (i, (t, h)) in ptys.iter().zip(&hargs).enumerate() {
        let v = to_host(t, h, &format!("args[{i}]"), true, &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let p{i} = {v};");
    }
    if let Some(l) = limit {
        let HostTy::Int(rt) = &hargs[l] else {
            return Err(bad("the limit parameter needs an integer host type".into()));
        };
        let mut arms = String::new();
        if faults.iter().any(|f| f == "ignore-limit") {
            let _ = write!(arms, "Fault::IgnoreLimit => {rt}::MAX, ");
        }
        if faults.iter().any(|f| f == "off-by-one-limit") {
            let _ = write!(arms, "Fault::OffByOneLimit => p{l}.saturating_add(1), ");
        }
        if !arms.is_empty() {
            let _ = writeln!(s, "    let p{l} = match fault {{ {arms}_ => p{l} }};");
        }
        if faults.iter().any(|f| f == "limit-after-alloc") {
            let _ = writeln!(
                s,
                "    let hog = (fault == Fault::LimitAfterAlloc).then(|| vec![0u8; p{l} as usize]);"
            );
        }
    }
    let arglist: Vec<String> = (0..ptys.len()).map(|i| format!("p{i}")).collect();
    let call = format!("{export}({})", arglist.join(", "));
    let minimal = format!("domain_take({}, 1).remove(0)", payload.rust());
    if !is_result {
        let v = from_host(&payload, &hret, &call, &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let v: V = {v};");
    } else if let Some(case) = host.and_then(|h| h.err_any.as_deref()) {
        // WX-N `err_any=CASE`: the export returns `Result<T, E>` for an opaque
        // `E` (`&'static str`, `()`); every `Err(_)` is one Warrant case.
        if !cases.contains(&case.to_string()) && !env_cases_of(&op.return_type, env).contains(&case.to_string()) {
            return Err(bad(format!("err_any= names `{case}`, which is not an error case")));
        }
        // Soundness: `Err(_)` carries no information about WHICH error the
        // export returned, so it can only stand for a type with one case.
        // With two or more, an export that returns the wrong error would still
        // map to `CASE` and a claim `f(..) == Err(CASE)` would pass vacuously.
        let declared = env_cases_of(&op.return_type, env);
        let n_cases = if declared.is_empty() { cases.len() } else { declared.len() };
        if n_cases != 1 {
            return Err(CodegenError {
                code: DiagnosticCode::W0704,
                kind: CodegenErrorKind::MissingBinding(format!(
                    "err_any= is incompatible with `{n}`: its error type has {n_cases} cases, \
                     and err_any= needs exactly one (an opaque Err(_) cannot say which case; \
                     use errors= to map each host error variant)"
                )),
                span: op.span,
            });
        }
        let ok = from_host(&payload, &hret, "r", &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let v: V = match {call} {{");
        let _ = writeln!(s, "        Ok(r) => V::Ok(Box::new({ok})),");
        if faults.iter().any(|f| f == "missing-error") {
            let _ = writeln!(
                s,
                "        Err(_) if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
            );
        }
        let _ = writeln!(s, "        Err(_) => V::Err(\"{case}\".into()),\n    }};");
    } else if let Some(he) = host.and_then(|h| h.errors.as_ref()) {
        let decl_cases = env_cases_of(&op.return_type, env);
        for (_, case) in &he.map {
            if !cases.contains(case) && !decl_cases.contains(case) {
                return Err(bad(format!("errors= names `{case}`, which is not an error case")));
            }
        }
        let ok = from_host(&payload, &hret, "r", &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let v: V = match {call} {{");
        let _ = writeln!(s, "        Ok(r) => V::Ok(Box::new({ok})),");
        if faults.iter().any(|f| f == "missing-error") {
            let _ = writeln!(
                s,
                "        Err(_) if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
            );
        }
        let _ = writeln!(s, "        Err(e) => match e {{");
        for (variant, case) in &he.map {
            let _ = writeln!(
                s,
                "            {}::{variant} {{ .. }} => V::Err(\"{case}\".into()),",
                he.path
            );
        }
        let _ = writeln!(
            s,
            "            #[allow(unreachable_patterns)]\n            _ => panic!(\"unmapped host error\"),\n        }},\n    }};"
        );
    } else {
        let case = err_case.ok_or_else(|| CodegenError {
            code: DiagnosticCode::W0704,
            kind: CodegenErrorKind::MissingBinding(format!(
                "binding for `{n}` has neither error normalization nor errors="
            )),
            span: op.span,
        })?;
        let ok = from_host(&payload, &hret, "r", &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let v: V = match contained(|| {call}) {{");
        let _ = writeln!(s, "        Some(r) => V::Ok(Box::new({ok})),");
        if faults.iter().any(|f| f == "missing-error") {
            let _ = writeln!(
                s,
                "        None if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
            );
        }
        let _ = writeln!(s, "        None => V::Err(\"{case}\".into()),\n    }};");
    }
    if limit.is_some() && faults.iter().any(|f| f == "limit-after-alloc") {
        // The hog must live across the call (and not be optimised away).
        s.push_str("    std::hint::black_box(&hog);\n");
    }
    let mut arms = String::new();
    for f in faults {
        let rhs = match f.as_str() {
            "flip-result-bit" => "fault_flip_result_bit(v)".to_string(),
            "drop-last-result-byte" => "fault_drop_last_result_byte(v)".to_string(),
            "truncate-silently" => {
                let b = ptys.iter().position(|t| *t == Ty::Bytes).unwrap_or(0);
                let l = limit.unwrap_or(0);
                format!("fault_truncate_silently(v, args[{b}].bytes(), args[{l}].int())")
            }
            "drop-consumed" => "fault_drop_consumed(v)".to_string(),
            "swap-components" => "fault_swap_components(v)".to_string(),
            "field-zero" => "fault_field_zero(v)".to_string(),
            "off-by-one-consumed" => "fault_off_by_one_consumed(v)".to_string(),
            "wrong-ctor" => format!("fault_wrong_ctor(v, {})", core.rust()),
            "ok-as-err" => format!("fault_ok_as_err(v, \"{}\")", cases[0]),
            _ => continue,
        };
        let variant = fault_variant(f).expect("registered fault");
        let _ = writeln!(arms, "        Fault::{variant} => {rhs},");
    }
    if arms.is_empty() {
        s.push_str("    v\n}\n\n");
    } else {
        let _ = write!(s, "    match fault {{\n{arms}        _ => v,\n    }}\n}}\n\n");
    }
    Ok(s)
}

/// The declared error cases of the `Result` return type of an operation.
fn env_cases_of(t: &TypeExpr<'_>, env: &TyEnv<'_>) -> Vec<String> {
    match t {
        TypeExpr::Result { err_name, .. } => env
            .errors
            .get(err_name)
            .map(|v| v.iter().map(|c| c.to_string()).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Does the document need the WX-A part of the prelude (composite values,
/// patterns, composite domains)?  Only a v1 document that declares a record or
/// variant, writes a tuple, a projection or a `match`, or quantifies over an
/// `Option`/`Result`/`List`/composite binder.  Every other v1 document keeps
/// the WX-B/WX-C output byte for byte.
fn uses_composites<'a>(file: &WarrantFile<'a>, env: &TyEnv<'a>) -> bool {
    fn non_scalar<'a>(t: &TypeExpr<'a>, env: &TyEnv<'a>, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        match t {
            TypeExpr::Tuple { .. }
            | TypeExpr::Option { .. }
            | TypeExpr::Result { .. }
            | TypeExpr::List { .. } => true,
            TypeExpr::Named { name, .. } => {
                env.records.contains_key(name)
                    || env.variants.contains_key(name)
                    || env
                        .aliases
                        .get(name)
                        .is_some_and(|a| non_scalar(a, env, depth + 1))
            }
            TypeExpr::Bytes { .. } => false,
        }
    }
    fn walk<'a>(e: &Expression<'a>, env: &TyEnv<'a>) -> bool {
        match e {
            Expression::Tuple { .. } | Expression::Project { .. } | Expression::Match { .. } => true,
            Expression::Quantified { binders, body, .. } => {
                binders.iter().any(|b| non_scalar(&b.param_type, env, 0)) || walk(body, env)
            }
            Expression::Binary { left, right, .. } => walk(left, env) || walk(right, env),
            Expression::Unary { operand, .. } => walk(operand, env),
            Expression::Grouped { inner, .. } => walk(inner, env),
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => walk(condition, env) || walk(then_branch, env) || walk(else_branch, env),
            Expression::Reference { args, .. } => args
                .iter()
                .flat_map(|l| l.iter())
                .any(|a| walk(a, env)),
            Expression::List { elements, .. } => elements.iter().any(|a| walk(a, env)),
            Expression::Literal { .. } => false,
        }
    }
    file.declarations.iter().any(|d| match d {
        Declaration::Record(_) | Declaration::Variant(_) => true,
        Declaration::Model(m) => {
            m.params.iter().any(|p| env.has_composite(&p.param_type, 0))
                || env.has_composite(&m.target_type, 0)
                || m.body.as_ref().is_some_and(|b| walk(b, env))
        }
        Declaration::Op(o) => {
            o.params.iter().any(|p| env.has_composite(&p.param_type, 0))
                || env.has_composite(&o.return_type, 0)
        }
        Declaration::Example(ex) => walk(&ex.body, env),
        Declaration::Claim(c) => walk(&c.body, env),
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// WX-H helpers
// ---------------------------------------------------------------------------

/// The limit parameter of an operation's `output` row: the one unsigned numeric
/// parameter (`U16`, `U32`, `U64` or `Nat`) the bound mentions.  `Ok(None)` when
/// it mentions none or several, in which case the limit faults are refused.
fn limit_param<'a>(
    op: &OpDecl<'a>,
    output: &Expression<'a>,
    v1: bool,
    env: &TyEnv<'a>,
) -> Result<Option<usize>, CodegenError> {
    let names: HashSet<&str> = op.params.iter().map(|p| p.name).collect();
    let mut seen = BTreeSet::new();
    output.mentioned_names(&names, &mut seen);
    let mut found = None;
    let mut count = 0;
    for (i, p) in op.params.iter().enumerate() {
        if !seen.contains(p.name) {
            continue;
        }
        let numeric = match ty_of(p, v1, env) {
            Ok(Ty::Fx(f)) => !f.signed() && f.bits() >= 16,
            Ok(Ty::Nat) => true,
            _ => false,
        };
        if numeric {
            count += 1;
            found = Some(i);
        }
    }
    Ok(if count == 1 { found } else { None })
}

// ---------------------------------------------------------------------------
// The generator
// ---------------------------------------------------------------------------

/// The verified `vectors` tables of a document and where the generated harness
/// finds their checked-in copies (VER04j section 5.3 item 1).
///
/// ```text
/// <package>/tests/generated/warrant_evidence.rs        the harness
/// <package>/tests/generated/tables/<name>.tfv          dir = "tests/generated/tables"
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TableSet<'t> {
    /// The copies' directory, relative to the package root; it obeys the path
    /// rules of a table source (so it can be embedded in a string literal).
    pub dir: &'t str,
    /// Tables returned by [`load_document_tables`], one per `vectors` declaration.
    pub tables: &'t [LoadedTable],
}

impl TableSet<'_> {
    /// No tables: valid only for a document that declares none.
    pub const NONE: TableSet<'static> = TableSet {
        dir: "tests/generated/tables",
        tables: &[],
    };

    /// The files the driver must write (or, with `--check`, compare): the
    /// package-relative path of each copy and its exact bytes.
    pub fn files(&self) -> Vec<(String, &[u8])> {
        self.tables
            .iter()
            .map(|t| (format!("{}/{}.tfv", self.dir, t.name), t.bytes.as_slice()))
            .collect()
    }
}

/// The shape of values a column of generator type `t` carries in a TFV row.
fn shape_of_ty(t: &Ty) -> ColShape {
    match t {
        Ty::Bool => ColShape::Bool,
        Ty::U8 => ColShape::fixed(FixedTy::U8),
        Ty::U64 => ColShape::fixed(FixedTy::U64),
        Ty::Nat | Ty::NatCount => ColShape::nat(),
        Ty::Fx(f) | Ty::FxEdge(f) => ColShape::fixed(*f),
        Ty::Bytes => ColShape::Bytes,
        Ty::BytesN(n) => ColShape::BytesN(*n),
        Ty::Tuple(cs) => ColShape::Tuple(cs.iter().map(shape_of_ty).collect()),
        Ty::Data { name, ctors } => {
            // A record has one constructor (named like the type) and is its
            // field list; a variant has at least two.
            if ctors.len() == 1 && ctors[0].0 == *name {
                ColShape::Tuple(ctors[0].1.iter().map(shape_of_ty).collect())
            } else {
                ColShape::Variant(
                    ctors
                        .iter()
                        .map(|(c, fs)| (c.clone(), fs.iter().map(shape_of_ty).collect()))
                        .collect(),
                )
            }
        }
        Ty::Opt(t) => ColShape::Opt(Box::new(shape_of_ty(t))),
        Ty::List(t) => ColShape::List(Box::new(shape_of_ty(t))),
        Ty::Res(t, cases) => ColShape::Result(Box::new(shape_of_ty(t)), cases.clone()),
    }
}

/// The generator types of a table's columns, in declaration order.
fn column_tys<'a>(
    v: &crate::parser::VectorsDecl<'a>,
    env: &TyEnv<'a>,
) -> Result<Vec<Ty>, CodegenError> {
    v.columns
        .iter()
        .map(|c| ty_of_type(&c.ty, env, true, c.span, 0))
        .collect()
}

/// Load and verify every `vectors` table of a v1 document from the fixtures
/// root `root` (see [`crate::vectors`] for the order of the checks): the pin is
/// compared before the file is parsed, sizes and row counts are capped, symlinks
/// are refused, and every row is decoded against its column types.  Nothing is
/// fetched from anywhere else.
pub fn load_document_tables(
    file: &WarrantFile<'_>,
    root: &Path,
) -> Result<Vec<LoadedTable>, CodegenError> {
    let env = TyEnv::new(file);
    let mut out: Vec<LoadedTable> = Vec::new();
    let table_count = file
        .declarations
        .iter()
        .filter(|d| matches!(d, Declaration::Vectors(_)))
        .count();
    if table_count > vectors::MAX_DOCUMENT_TABLES {
        return Err(CodegenError {
            code: DiagnosticCode::W0602,
            kind: CodegenErrorKind::UnsupportedTarget(format!(
                "{table_count} tables exceed the cap of {} per document",
                vectors::MAX_DOCUMENT_TABLES
            )),
            span: file.span,
        });
    }
    let mut held_bytes: u64 = 0;
    // The row budget of the whole document is checked before any file is
    // touched: refusing 250,001 declared rows must not first read 100,000-row
    // files.  (Saturating, so a hostile `rows` cannot wrap the sum.)
    let mut total: u64 = 0;
    for d in &file.declarations {
        let Declaration::Vectors(v) = d else { continue };
        total = total.saturating_add(v.rows);
        if v.rows > crate::checker::MAX_TABLE_ROWS || total > crate::checker::MAX_DOCUMENT_ROWS {
            return Err(CodegenError {
                code: DiagnosticCode::W0602,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "table {} exceeds the row budget ({} per table, {} per document)",
                    v.name,
                    crate::checker::MAX_TABLE_ROWS,
                    crate::checker::MAX_DOCUMENT_ROWS
                )),
                span: v.span,
            });
        }
    }
    for d in &file.declarations {
        let Declaration::Vectors(v) = d else { continue };
        let shapes: Vec<(String, ColShape)> = column_tys(v, &env)?
            .iter()
            .zip(&v.columns)
            .map(|(t, c)| (c.name.to_string(), shape_of_ty(t)))
            .collect();
        let table = vectors::load_table(v, &shapes, root)?;
        held_bytes = held_bytes.saturating_add(table.bytes.len() as u64);
        if held_bytes > vectors::MAX_DOCUMENT_BYTES {
            return Err(CodegenError {
                code: DiagnosticCode::W0602,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "the tables of this document hold more than {} bytes together",
                    vectors::MAX_DOCUMENT_BYTES
                )),
                span: v.span,
            });
        }
        out.push(table);
    }
    Ok(out)
}

/// Compile `file` into the evidence harness source (see module docs).
pub fn compile_rust_evidence_harness(
    file: &WarrantFile<'_>,
    checked: &CheckedWarrantModule,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    compile_rust_evidence_harness_with_tables(file, checked, manifest, options, &TableSet::NONE)
}

/// [`compile_rust_evidence_harness`] for a document with `vectors` tables.
pub fn compile_rust_evidence_harness_with_tables(
    file: &WarrantFile<'_>,
    checked: &CheckedWarrantModule,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
    tables: &TableSet<'_>,
) -> Result<String, CodegenError> {
    validate_bindings(checked, manifest).map_err(|e| CodegenError {
        code: DiagnosticCode::W0704,
        kind: CodegenErrorKind::MissingBinding(format!("{:?}", e.kind)),
        span: e.span,
    })?;
    generate_evidence_harness_with_tables(file, manifest, options, tables)
}

/// [`compile_rust_evidence_harness`] without the binding/contract validation
/// step: translation only.  Exposed so the translator's refusals can be tested
/// on documents that the type checker would reject earlier.
pub fn generate_evidence_harness(
    file: &WarrantFile<'_>,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    generate_evidence_harness_with_tables(file, manifest, options, &TableSet::NONE)
}

/// [`generate_evidence_harness`] for a document with `vectors` tables.
pub fn generate_evidence_harness_with_tables(
    file: &WarrantFile<'_>,
    manifest: &BindingManifest,
    options: &EvidenceOptions,
    table_set: &TableSet<'_>,
) -> Result<String, CodegenError> {
    // Everything spliced into generated Rust source as code must be an
    // identifier; the spec path lands inside a string literal and must not be
    // able to close it.  The bindings parser checks the same things, but a
    // caller may build a manifest by hand, so the generator checks again.
    if !is_rust_identifier(&options.package_crate) {
        return Err(unsupported(
            format!("package `{}` is not a Rust identifier", options.package_crate),
            (0, 0),
        ));
    }
    if options.spec_path.contains(['"', '\\', '\n', '\r']) {
        return Err(unsupported(format!("spec path {:?} cannot be embedded", options.spec_path), (0, 0)));
    }
    for (op, b) in &manifest.bindings {
        if !is_rust_identifier(&b.export) {
            return Err(unsupported(
                format!("export `{}` of operation `{op}` is not a Rust identifier", b.export),
                (0, 0),
            ));
        }
        if let Some(n) = &b.error_normalization {
            if !is_rust_identifier(&n.error_case) {
                return Err(unsupported(
                    format!("error case `{}` of operation `{op}` is not an identifier", n.error_case),
                    (0, 0),
                ));
            }
        }
    }
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
    let v1 = file.version == Version::V1;
    let env = TyEnv::new(file);

    // WX-E: every declared table must have been loaded (and verified) by the
    // caller; the copies' directory is embedded in a string literal, so it must
    // obey the path rules.
    let mut table_decls: Vec<(&crate::parser::VectorsDecl<'_>, &LoadedTable, Vec<Ty>)> = Vec::new();
    for d in &file.declarations {
        let Declaration::Vectors(v) = d else { continue };
        if !v1 {
            return Err(unsupported("a `vectors` table in a v0 document", v.span));
        }
        let Some(t) = table_set.tables.iter().find(|t| t.name == v.name) else {
            return Err(CodegenError {
                code: DiagnosticCode::W0708,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "table {} was not loaded: use load_document_tables and the *_with_tables entry points",
                    v.name
                )),
                span: v.span,
            });
        };
        if t.sha256 != v.sha256 || t.source != v.source || t.rows.len() as u64 != v.rows {
            return Err(CodegenError {
                code: DiagnosticCode::W0708,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "table {}: the loaded file is not the one the document pins",
                    v.name
                )),
                span: v.span,
            });
        }
        // A table no constant function fails can never be failed by a fault of
        // VER04h section 6 either (VER04j section 5.3 item 4).
        if vectors::is_constant_output(t, &v.holds) {
            return Err(CodegenError {
                code: DiagnosticCode::W0601,
                kind: CodegenErrorKind::UnsupportedTarget(format!(
                    "table {} is not discriminating: its expected column is the same in every row",
                    v.name
                )),
                span: v.span,
            });
        }
        table_decls.push((v, t, column_tys(v, &env)?));
    }
    if !table_decls.is_empty() && !valid_table_path(table_set.dir) {
        return Err(unsupported(
            format!("table directory {:?} breaks the path rules", table_set.dir),
            file.span,
        ));
    }
    {
        let mut upper = BTreeSet::new();
        for (v, _, _) in &table_decls {
            if !upper.insert(v.name.to_uppercase()) {
                return Err(unsupported(
                    format!("table name `{}` collides with another table when upper-cased", v.name),
                    v.span,
                ));
            }
        }
    }
    let plain_column = |t: &Ty| {
        matches!(
            t,
            Ty::Bool | Ty::Fx(_) | Ty::FxEdge(_) | Ty::Nat | Ty::NatCount | Ty::Bytes | Ty::BytesN(_)
        )
    };
    let tables_need_comp = table_decls
        .iter()
        .any(|(_, _, tys)| tys.iter().any(|t| !plain_column(t)));
    let comp = v1
        && (tables_need_comp
            || uses_composites(file, &env)
            || file.declarations.iter().any(|d| {
                matches!(d, Declaration::Op(o) if matches!(shape_of(o, v1, &env), Ok(Shape::Composite)))
            }));

    // WX-H: the `heap` and `output` rows of each operation's `bound` (v1 only).
    // A document with at least one of them gets the allocation observer.
    #[derive(Default)]
    struct ObsOp<'f, 'a> {
        heap: Option<&'f Expression<'a>>,
        output: Option<&'f Expression<'a>>,
    }
    let mut obs_ops: BTreeMap<&str, ObsOp<'_, '_>> = BTreeMap::new();
    if v1 {
        for d in &file.declarations {
            if let Declaration::Bound(b) = d {
                for r in &b.rows {
                    match r.name {
                        "heap" => obs_ops.entry(b.name).or_default().heap = Some(&r.bound),
                        "output" => obs_ops.entry(b.name).or_default().output = Some(&r.bound),
                        _ => {}
                    }
                }
            }
        }
    }
    let obs = !obs_ops.is_empty();

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
    for name in obs_ops.keys() {
        if !op_names.contains(name) {
            return Err(unsupported(format!("bound of unknown operation `{name}`"), file.span));
        }
    }

    let mut widths: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut body = String::new();
    let mut features: BTreeSet<&str> = BTreeSet::new();
    features.insert(if v1 { "v1" } else { "v0" });
    features.insert(if comp { "comp" } else { "plain" });
    if obs {
        features.insert("obs");
    }
    if !table_decls.is_empty() {
        features.insert("vec");
    }

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
                    v1,
                    ctors: &env.ctors,
                };
                let params: Vec<String> =
                    m.params.iter().map(|p| format!("b_{}: V", p.name)).collect();
                let attr = if comp {
                    "#[allow(unused_variables, unused_mut)]\n"
                } else {
                    ""
                };
                let _ = write!(
                    body,
                    "{attr}fn model_{}({}) -> V {{\n    {}\n}}\n\n",
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
                v1,
                ctors: &env.ctors,
            };
            if obs {
                // An observed instance that the observer fails is a failed
                // example, not a panic (VER04j section 14.2 item 34).
                let _ = write!(
                    body,
                    "#[allow(unused_variables)]\npub fn example_{}(fault: Fault) -> bool {{\n    match catch_unwind(AssertUnwindSafe(|| {}.truthy())) {{\n        Ok(v) => v,\n        Err(p) => obs_failed(p),\n    }}\n}}\n\n",
                    ex.name,
                    emit(&ex.body, &mut cx)?
                );
            } else {
                let _ = write!(
                    body,
                    "#[allow(unused_variables)]\npub fn example_{}(fault: Fault) -> bool {{\n    {}.truthy()\n}}\n\n",
                    ex.name,
                    emit(&ex.body, &mut cx)?
                );
            }
            examples.push(Ex {
                name: ex.name.to_string(),
                op: op.to_string(),
            });
        }
    }

    // Claims.
    //
    // `name` is the label that ends up in ids, seeds and the manifest, `ident`
    // the Rust identifier the claim's functions are named after (they differ for
    // the synthesized `<op>/output-bound` claims of WX-H, section 14.2 item 32).
    struct Cl {
        name: String,
        ident: String,
        op: String,
    }
    enum Job<'f, 'a> {
        Real(&'f ClaimDecl<'a>),
        OutputBound(&'f OpDecl<'a>),
    }
    /// One claim to generate: a written one, or a synthesized `output-bound`.
    struct Work<'f, 'a> {
        label: String,
        ident: String,
        binders: &'f [Parameter<'a>],
        cbody: Option<&'f Expression<'a>>,
        span: (usize, usize),
        op: String,
    }
    let mut jobs: Vec<Job<'_, '_>> = Vec::new();
    for d in &file.declarations {
        if let Declaration::Claim(c) = d {
            jobs.push(Job::Real(c));
        }
    }
    for o in &op_decls {
        if obs_ops.get(o.name).is_some_and(|b| b.output.is_some()) {
            jobs.push(Job::OutputBound(o));
        }
    }
    let mut claims: Vec<Cl> = Vec::new();
    for job in &jobs {
        let Work {
            label,
            ident,
            binders,
            cbody,
            span,
            op,
        } = match job {
            Job::Real(c) => {
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
                let op = find_operation_in_expr(cbody, &op_names).ok_or_else(|| {
                    unsupported(format!("claim {} calls no operation", c.name), *span)
                })?;
                Work {
                    label: c.name.to_string(),
                    ident: c.name.to_string(),
                    binders: binders.as_slice(),
                    cbody: Some(&**cbody),
                    span: *span,
                    op: op.to_string(),
                }
            }
            Job::OutputBound(o) => {
                if !o.static_params.is_empty() {
                    return Err(unsupported(
                        format!(
                            "operation `{}`: an `output` bound on an operation with static widths is not supported",
                            o.name
                        ),
                        o.span,
                    ));
                }
                Work {
                    label: format!("{}/output-bound", o.name),
                    ident: format!("{}_output_bound", o.name),
                    binders: o.params.as_slice(),
                    cbody: None,
                    span: o.span,
                    op: o.name.to_string(),
                }
            }
        };
        if claims.iter().any(|c| c.ident == ident) {
            return Err(unsupported(
                format!("generated claim name `{ident}` is already taken"),
                span,
            ));
        }
        let mut tys: Vec<Ty> = binders
            .iter()
            .map(|b| ty_of(b, v1, &env))
            .collect::<Result<_, _>>()?;
        let mut demoted: Vec<(usize, Vec<String>)> = Vec::new();
        if v1 {
            // A Nat that flows directly into a shift count gets the
            // count-aware domain (VER04j section 3.6).
            let mut counts = HashSet::new();
            if let Some(cbody) = cbody {
                count_binders(cbody, &mut counts);
            }
            for (t, b) in tys.iter_mut().zip(binders.iter()) {
                if *t == Ty::Nat && counts.contains(b.name) {
                    *t = Ty::NatCount;
                }
            }
            // The budget rule of section 2.6.
            demoted = demote_for_budget(&mut tys, options.exhaustive_budget);
        }
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
                    "claim {label} exhaustive domain {shown} exceeds budget {} cases / {DOMAIN_BYTES_BUDGET} bytes",
                    options.exhaustive_budget
                )),
                span,
            });
        }
        if tys.contains(&Ty::U8) {
            features.insert("u8");
        }
        let names_src: Vec<String> = binders.iter().map(|b| format!("\"{}\"", b.name)).collect();
        let demoted_src: Vec<String> = demoted
            .iter()
            .map(|(i, path)| {
                let mut name = binders[*i].name.to_string();
                for seg in path {
                    let _ = write!(name, ".{seg}");
                }
                format!("\"{name}\"")
            })
            .collect();
        let mut cx = Cx {
            binders: binders.iter().map(|p| p.name.to_string()).collect(),
            models: &models,
            ops: &ops,
            errors: &errors,
            allow_ops: true,
            widths: &mut widths,
            v1,
            ctors: &env.ctors,
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
        for p in binders.iter() {
            if let Some(r) = &p.refinement {
                let _ = write!(admit, " && {}.truthy()", emit(r, &mut cx)?);
            }
        }
        cx.allow_ops = true;
        let holds = match cbody {
            Some(cbody) => emit(cbody, &mut cx)?,
            // The WX-H property (section 6.3): `size(op(args)) <= bound`.  The
            // bound is evaluated first, for every instance, so a bound that is
            // negative on the admitted domain fails even where the operation
            // answers `Err`.
            None => {
                let args: Vec<String> = binders.iter().map(|p| format!("b_{}.clone()", p.name)).collect();
                format!(
                    "V::Bool({{ let bound = op_{op}_output(env); obs_output_ok(op_{op}(&[], &[{}], fault), bound) }})",
                    args.join(", ")
                )
            }
        };
        let tys_src: Vec<String> = tys.iter().map(|t| t.rust()).collect();
        if v1 {
            let _ = write!(
                body,
                "const CLAIM_{up}_NAMES: &[&str] = &[{names}];\n\n\
                 const CLAIM_{up}_DEMOTED: &[&str] = &[{dem}];\n\n",
                up = ident.to_uppercase(),
                names = names_src.join(", "),
                dem = demoted_src.join(", "),
            );
        }
        let up = ident.to_uppercase();
        let n = ident.as_str();
        let _ = write!(body, "const CLAIM_{up}_TYS: &[Ty] = &[{}];\n\n", tys_src.join(", "));
        let _ = write!(
            body,
            "#[allow(unused_variables)]\nfn claim_{n}_admit(env: &[V]) -> bool {{\n{binds}    {admit}\n}}\n\n"
        );
        // WX-H: an instance the observer fails is a false claim, not a panic.
        let obs_arm = if obs {
            "        Err(p) if p.is::<ObsFailure>() => obs_failed(p),\n"
        } else {
            ""
        };
        if comp {
            // A value a host integer cannot hold raises `Unreachable`: the case
            // is skipped and counted, never narrowed (VER04j section 4.6).
            let _ = write!(
                body,
                "#[allow(unused_variables, unused_mut)]\nfn claim_{n}_holds(env: &[V], fault: Fault) -> bool {{\n{binds}    match catch_unwind(AssertUnwindSafe(|| {holds}.truthy())) {{\n        Ok(v) => v,\n        Err(p) if p.is::<Unreachable>() => {{\n            count_unreach();\n            true\n        }}\n{obs_arm}        Err(p) => std::panic::resume_unwind(p),\n    }}\n}}\n\n"
            );
            let _ = write!(
                body,
                "pub fn exh_{n}(fault: Fault) -> (Verdict, Stats) {{\n    take_unreach();\n    let (v, mut s) = exhaustive(CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault);\n    s.unreachable = take_unreach();\n    (v, s)\n}}\n\n\
                 pub fn prop_{n}(fault: Fault) -> (Verdict, Stats) {{\n    take_unreach();\n    let (v, mut s) = property(\"claim/{module}/{label}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault);\n    s.unreachable = take_unreach();\n    (v, s)\n}}\n\n\
                 pub fn fuz_{n}(fault: Fault, iterations: u64) -> (Verdict, Stats) {{\n    take_unreach();\n    let (v, mut s) = fuzz_claim(\"fuzz/{module}/{label}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault, iterations);\n    s.unreachable = take_unreach();\n    (v, s)\n}}\n\n"
            );
        } else {
            let holds_fn = if obs {
                format!("    match catch_unwind(AssertUnwindSafe(|| {holds}.truthy())) {{\n        Ok(v) => v,\n        Err(p) => obs_failed(p),\n    }}")
            } else {
                format!("    {holds}.truthy()")
            };
            let _ = write!(
                body,
                "#[allow(unused_variables)]\nfn claim_{n}_holds(env: &[V], fault: Fault) -> bool {{\n{binds}{holds_fn}\n}}\n\n\
                 pub fn exh_{n}(fault: Fault) -> (Verdict, Stats) {{\n    exhaustive(CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault)\n}}\n\n\
                 pub fn prop_{n}(fault: Fault) -> (Verdict, Stats) {{\n    property(\"claim/{module}/{label}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault)\n}}\n\n\
                 pub fn fuz_{n}(fault: Fault, iterations: u64) -> (Verdict, Stats) {{\n    fuzz_claim(\"fuzz/{module}/{label}\", CLAIM_{up}_TYS, claim_{n}_admit, claim_{n}_holds, fault, iterations)\n}}\n\n"
            );
        }
        claims.push(Cl {
            name: label,
            ident,
            op,
        });
    }
    if claims.is_empty() {
        return Err(unsupported("document has no claims to generate evidence for", file.span));
    }

    // WX-E: one function per table (the embedded copy, its pin, its columns,
    // and `holds` as a function of one row).
    struct Tb {
        name: String,
        op: String,
    }
    let mut tabs: Vec<Tb> = Vec::new();
    for (v, t, tys) in &table_decls {
        let op = find_operation_in_expr(&v.holds, &op_names)
            .ok_or_else(|| unsupported(format!("table {} calls no operation", v.name), v.span))?;
        let mut cx = Cx {
            binders: v.columns.iter().map(|c| c.name.to_string()).collect(),
            models: &models,
            ops: &ops,
            errors: &errors,
            allow_ops: true,
            widths: &mut widths,
            v1,
            ctors: &env.ctors,
        };
        let holds = emit(&v.holds, &mut cx)?;
        let binds: String = v
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| format!("    let b_{} = env[{i}].clone();\n", c.name))
            .collect();
        let cols: Vec<String> = v
            .columns
            .iter()
            .zip(tys)
            .map(|(c, ty)| format!("(\"{}\", {})", c.name, ty.rust()))
            .collect();
        let obs_arm = if obs {
            "        Err(p) if p.is::<ObsFailure>() => obs_failed(p),\n"
        } else {
            ""
        };
        let (n, up) = (v.name, v.name.to_uppercase());
        let _ = write!(
            body,
            "const TABLE_{up}_BYTES: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{dir}/{n}.tfv\"));\n\
             const TABLE_{up}_SHA256: &str = \"{sha}\";\n\
             const TABLE_{up}_ROWS: usize = {rows};\n\
             const TABLE_{up}_COLUMNS: &[(&str, Ty)] = &[{cols}];\n\n\
             #[allow(unused_variables, unused_mut)]\nfn table_{n}_holds(env: &[V], fault: Fault) -> bool {{\n{binds}    match catch_unwind(AssertUnwindSafe(|| {holds}.truthy())) {{\n        Ok(v) => v,\n{obs_arm}        Err(_) => false,\n    }}\n}}\n\n\
             pub fn tab_{n}(fault: Fault) -> (Verdict, TableStats) {{\n    run_table(\"{n}\", TABLE_{up}_BYTES, TABLE_{up}_SHA256, TABLE_{up}_ROWS, TABLE_{up}_COLUMNS, table_{n}_holds, fault)\n}}\n\n",
            dir = table_set.dir,
            sha = t.sha256,
            rows = t.rows.len(),
            cols = cols.join(", "),
        );
        tabs.push(Tb {
            name: v.name.to_string(),
            op: op.to_string(),
        });
    }

    // WX-H: the `heap` and `output` rows as functions of the call's arguments.
    for (opn, rows) in &obs_ops {
        let decl = op_decls
            .iter()
            .find(|o| o.name == *opn)
            .expect("checked above");
        for (metric, expr) in [("heap", rows.heap), ("output", rows.output)] {
            let Some(expr) = expr else { continue };
            let mut cx = Cx {
                binders: decl.params.iter().map(|p| p.name.to_string()).collect(),
                models: &models,
                ops: &ops,
                errors: &errors,
                // A bound is a function of the arguments: it may call models
                // but never the operation it bounds.
                allow_ops: false,
                widths: &mut widths,
                v1,
                ctors: &env.ctors,
            };
            let binds: String = decl
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| format!("        let b_{} = args[{i}].clone();\n", p.name))
                .collect();
            let _ = write!(
                body,
                "#[allow(unused_variables)]\nfn op_{opn}_{metric}(args: &[V]) -> i128 {{\n    obs_bound(\"{metric} bound of `{opn}`\", || {{\n{binds}        {}\n    }})\n}}\n\n",
                emit(expr, &mut cx)?
            );
        }
    }

    // WX-A: the position of a record field, looked up by name at run time (the
    // translator has no static types, so `r.f` is resolved dynamically).
    if comp {
        body.push_str("#[allow(unused_variables)]\nfn field_pos(ty: &str, f: &str) -> usize {\n    match (ty, f) {\n");
        for (ty, fields) in &env.fields {
            for (i, f) in fields.iter().enumerate() {
                let _ = writeln!(body, "        (\"{ty}\", \"{f}\") => {i},");
            }
        }
        body.push_str("        _ => panic!(\"no such field\"),\n    }\n}\n\n");
    }

    // Adapters (after expression translation so fixed widths are known).
    let mut shapes: BTreeMap<String, Shape> = BTreeMap::new();
    // The faults each operation is attacked with, as (name, Rust variant).
    let mut op_faults: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut used_faults: BTreeSet<String> = BTreeSet::new();
    let mut adapters = String::new();
    for op in &op_decls {
        let binding = &manifest.bindings[op.name];
        let shape = shape_of(op, v1, &env)?;
        let adapters_before = adapters.len();
        // WX-H: the numeric parameter an `output` row mentions is the limit the
        // four limit faults rewrite (VER04j section 14.2 item 36).
        let limit = match obs_ops.get(op.name).and_then(|r| r.output) {
            Some(expr) => limit_param(op, expr, v1, &env)?,
            None => None,
        };
        for f in &binding.negative_controls {
            if WXH_FAULTS.iter().any(|(n, _)| n == f) && shape != Shape::Composite {
                return Err(unsupported(
                    format!("fault `{f}` of operation `{}` needs a composite-adapter operation", op.name),
                    op.span,
                ));
            }
        }
        let empty = BTreeSet::new();
        let err_case = binding.error_normalization.as_ref().map(|n| n.error_case.as_str());
        if shape == Shape::Composite {
            features.insert("scoped");
            adapters.push_str(&composite_adapter(
                op,
                &env,
                &binding.export,
                err_case,
                &binding.negative_controls,
                options,
                limit,
            )?);
            let mut list = Vec::new();
            for f in &binding.negative_controls {
                let variant = fault_variant(f).ok_or_else(|| {
                    unsupported(format!("unknown fault `{f}` for `{}`", op.name), op.span)
                })?;
                list.push((f.clone(), format!("Fault::{variant}")));
            }
            op_faults.insert(op.name.to_string(), list);
        } else if let Shape::ScalarV1(ret) = shape {
            features.insert("scoped");
            adapters.push_str(&scalar_adapter_v1(
                op,
                ret,
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
        } else if let Shape::Scalar { fallible } = shape {
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
        if let Some(rows) = obs_ops.get(op.name) {
            // The adapter just emitted becomes `op_<n>_raw`; `op_<n>` runs it on
            // the observer worker inside an allocation window and compares the
            // peak with the `heap` row (VER04j section 6.3).
            let emitted = adapters.split_off(adapters_before);
            let from = format!("fn op_{}(", op.name);
            let to = format!("fn op_{}_raw(", op.name);
            if !emitted.contains(&from) {
                return Err(unsupported(
                    format!("operation `{}` has no observable adapter", op.name),
                    op.span,
                ));
            }
            adapters.push_str(&emitted.replacen(&from, &to, 1));
            let n = op.name;
            let resolve = if shape.scoped() {
                format!("    let fault = fault_for(\"{n}\", fault);\n")
            } else {
                String::new()
            };
            let heap = if rows.heap.is_some() {
                format!("    let heap = Some(op_{n}_heap(args));\n")
            } else {
                "    let heap: Option<i128> = None;\n".to_string()
            };
            let _ = write!(
                adapters,
                "#[allow(unused_variables)]\nfn op_{n}(w: &[u64], args: &[V], fault: Fault) -> V {{\n{resolve}{heap}    let (w, a) = (w.to_vec(), args.to_vec());\n    obs_observe(\"{n}\", heap, move || op_{n}_raw(&w, &a, fault))\n}}\n\n"
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
            "    reg(&mut suite, Meta {{ domain: {describe}, {demoted}..Meta::new(\"{id1}\", \"exhaustive\", \"{op}\", \"{label}\") }}, move || {{\n        let (v, s) = exh_{n}(fault);\n        (v, s, String::new())\n    }});\n\
             \x20   reg(&mut suite, Meta {{ seed: seed_of(\"claim/{module}/{label}\").to_string(), ..Meta::new(\"{id2}\", \"property\", \"{op}\", \"{label}\") }}, move || {{\n        let (v, s) = prop_{n}(fault);\n        (v, s, String::new())\n    }});\n\
             \x20   reg(&mut suite, Meta {{ seed: seed_of(\"fuzz/{module}/{label}\").to_string(), ..Meta::new(\"{id3}\", \"fuzz\", \"{op}\", \"{label}\") }}, move || {{\n        let (v, s) = fuz_{n}(fault, FUZZ_ITERATIONS);\n        (v, s, String::new())\n    }});\n",
            n = c.ident,
            label = c.name,
            op = c.op,
            describe = if v1 {
                format!("describe(CLAIM_{}_TYS, CLAIM_{}_NAMES)", c.ident.to_uppercase(), c.ident.to_uppercase())
            } else {
                format!("describe(CLAIM_{}_TYS)", c.ident.to_uppercase())
            },
            demoted = if v1 {
                format!(
                    "demoted: CLAIM_{}_DEMOTED.iter().map(|s| s.to_string()).collect(), ",
                    c.ident.to_uppercase()
                )
            } else {
                String::new()
            },
            id1 = id("exhaustive", &c.name),
            id2 = id("property", &c.name),
            id3 = id("fuzz", &c.name),
        );
    }
    for t in &tabs {
        let _ = write!(
            suite,
            "    reg(&mut suite, Meta::new(\"{id}\", \"table\", \"{op}\", \"{n}\"), move || {{\n        let (v, _) = tab_{n}(fault);\n        (v, Stats::default(), String::new())\n    }});\n",
            id = id("table", &t.name),
            op = t.op,
            n = t.name,
        );
    }
    // Negative controls, per (suite kind, op, fault).
    let mut neg_wrappers = String::new();
    for t in &tabs {
        let _ = write!(
            neg_wrappers,
            "fn neg_tab_{n}(fault: Fault) -> Verdict {{\n    tab_{n}(fault).0\n}}\n\n",
            n = t.name
        );
    }
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
            n = c.ident
        );
    }
    for op in shapes.keys() {
        for kind in ["example", "exhaustive", "property", "fuzz", "table"] {
            let members: Vec<String> = if kind == "table" {
                tabs.iter()
                    .filter(|t| &t.op == op)
                    .map(|t| format!("(\"{n}\", neg_tab_{n})", n = t.name))
                    .collect()
            } else if kind == "example" {
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
                    .map(|c| format!("(\"{}\", {pre}_{})", c.name, c.ident))
                    .collect()
            };
            if members.is_empty() {
                continue;
            }
            let scoped = shapes[op].scoped();
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

    // WX-H: the `imin`/`imax` helpers are emitted only when some expression
    // uses them, so every other harness stays byte-identical.
    if body.contains("V::imin(") || body.contains("V::imax(") {
        features.insert("minmax");
    }
    if body.contains("V::concat(") {
        features.insert("concat");
    }
    if WXE_FAULTS.iter().any(|(n, _)| used_faults.contains(*n)) {
        features.insert("bytefaults");
    }
    // WX-N: a v1 document with a fixed-width `Bytes<N>` anywhere labels that
    // domain `full` or `sample` instead of `set`.  Gated like the helpers above,
    // so no other harness changes by a byte (VER04j section 14.4).
    if v1 && body.contains("Ty::BytesN(") {
        features.insert("bytesn");
    }

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
         pub const VALUE_OVERHEAD: u64 = {vo};\n{obs_cap}\n",
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
        obs_cap = if obs {
            format!("pub const OBSERVER_CAP_BYTES: u64 = {OBSERVER_CAP_BYTES};\n")
        } else {
            String::new()
        },
    );
    out.push_str(&expand_prelude(&fault_block(&used_faults), &features));
    out.push_str("\n// ---------------------------------------------------------------------------\n// Generated from the Warrant document\n// ---------------------------------------------------------------------------\n\n");
    out.push_str(&body);
    out.push_str(&adapters);
    out.push_str(&neg_wrappers);
    out.push_str(&suite);
    Ok(out)
}
