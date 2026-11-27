//! # Faults, controls and the witness search (VER04r 5.6, 8)
//!
//! A *fault* is data: a deliberate bug the lanes inject around one operation to prove the
//! corpus can notice it.  There are three kinds (VER04r 8.1):
//!
//! ```text
//!   pre      rewrites parameters before the call:  rewrites = [{param, expr}, ..]
//!            every expr reads the ORIGINAL parameters; all rewrites apply simultaneously
//!   post     rewrites the result:                  expr over the parameters and `result`
//!   replace  supplies the result (the call is still made and counted): same shape as post
//! ```
//!
//! `result` is the one reserved name for the call's result.  The templates below are the
//! portable names of VER04r 8.2, instantiated **by signature**: `invert` needs a `Bool`
//! result, `swap-choice` a `Bool` parameter called `choice`, and so on.  A name the registry
//! does not know, or a template the operation's signature cannot take, is a generation error
//! (`fault`): nothing is skipped.
//!
//! ## Levels: data the generator cannot evaluate, and the gate
//!
//! A fault expression belongs to feature levels.  `ignore-high-bit` and `xor-lsb` use
//! `bit_and`/`bit_xor`, which are `ir.fixed` (VER04r 8.2, [D9]); `ok-as-err`, `drop-consumed`,
//! `swap-components` and the tuple-payload `missing-error` are `ir.tuple` (they are written with
//! `match` on a `Result` and the tuple literal, [D35]); `off-by-one-consumed` ([D36]; its `Nat` form
//! uses `rem` and `unwrap_or`, its fixed-width form `bit_xor`) is `ir.fixed` plus `ir.tuple`; the `Ok` forms of `flip-result-bit` and
//! `drop-last-result-byte` are `match` over the `Bytes` forms and so name two levels; `saturate-add`, `saturate-sub`
//! and `shift-mask` ([D51], S11f-2) are `ir.fixed` (`checked_*`, `max_value`, `rem`, `unwrap_or`), and the scalar
//! `missing-error` ([D50]) is `ir.core`.  A fault
//! row's `requires` is the **sorted union of the template's own levels and the levels of the op's
//! parameter and result types** (R9, S10t-1 review): a fault on an operation with a tuple
//! parameter needs `ir.tuple` even where the template names no tuple tag.
//!
//! The **fault-evaluation gate** is the set of levels whose faults the witness search evaluates.
//! It is [`SHIPPED_GATE`]: `ir.core`, `ir.fixed` and `ir.tuple`, since the atomic flip (S10c, [D29]).
//! A caller may pass a narrower gate (the closed gate is `ir.core` only): a fault whose
//! `requires` names any level the gate lacks is emitted **as data** and each of its candidate claims that
//! has a stand-in is `deferred`, naming the **first level of the fault's `requires`, in sorted
//! order, that the gate does not implement** (R12); no witness search runs.  This is true even for
//! a fault whose expression this crate *can* evaluate (every one, since S10c-2 evaluates the
//! `ir.fixed` built-ins): no suite or chunk digest depends on a deferred fault (VER04r 8.2).  The gate is a **library parameter** only
//! ([`crate::emit::generate_gated`]); there is no command-line or environment switch.
//!
//! ## The three claim states
//!
//! For one fault and one *candidate* claim (a claim whose body calls the fault's op):
//!
//! ```text
//!   stand-in?   the claim is a `reference` suite whose subject side is exactly
//!               `op(b1, .., bk)` with every argument a plain binder: the reference
//!               (`expected`) can then stand in for the subject.
//!   no          -> unproven                       (a predicate, or a computed argument)
//!   yes, level not implemented -> deferred         (S10 will search)
//!   yes, level implemented     -> search every admitted case, in order:
//!                                   some case where faulted != expected -> witnessed (chunk)
//!                                   none                                -> blind
//! ```
//!
//! The search is exact: for a `pre` fault `expected` is re-evaluated at the rewritten
//! binders, for `post`/`replace` the fault expression is evaluated with `result` bound to
//! `expected`.  The witness is the **lowest** admitted case, and its chunk joins the slim set.

use crate::error::{code, GenError};
use coding_adventures_warrant_grammar::fixed::FixedTy;

use crate::eval::{Program, Stats, Vars};
use crate::ir::{Arith, Cmp, Ir, Pat};
use crate::json::Json;
use crate::lower::{ErrorType, LoweredOp, LoweredSuite, SuiteKind};
use crate::suite::SuitePlan;
use crate::types::{Decls, Ty, Width};
use crate::value::Value;

/// A feature level of a fault expression.  The derived order is the order of the level names
/// (`ir.core` < `ir.data` < `ir.fixed` < `ir.tuple`), which is the sorted order a `requires` list is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// `ir.core`: the 26 tags of Appendix A.
    Core,
    /// `ir.data`: `some`, `none`, `ctor`, `field` and the data patterns (S11c-2).
    Data,
    /// `ir.fixed`: the fixed-width built-ins (`bit_and`, `bit_xor`, ...).
    Fixed,
    /// `ir.tuple`: the tuple literal, `project` and `match` on a `Result`.
    Tuple,
}

impl Level {
    /// The name written to `requires`.
    pub fn name(self) -> &'static str {
        match self {
            Level::Core => "ir.core",
            Level::Data => "ir.data",
            Level::Fixed => "ir.fixed",
            Level::Tuple => "ir.tuple",
        }
    }
}

/// The shipped fault-evaluation gate: `ir.core`, `ir.data`, `ir.fixed` and `ir.tuple`, every level this crate
/// evaluates (`ir.fixed` and `ir.tuple` opened by the atomic flip of S10c, [D29]; `ir.data` by S11c-2, [D52]).  A caller that wants the closed gate passes
/// `&[Level::Core]` to [`crate::emit::generate_gated`] explicitly.
pub const SHIPPED_GATE: [Level; 4] = [Level::Core, Level::Data, Level::Fixed, Level::Tuple];

/// The first level of `requires` (sorted) that `gate` does not implement, or `None` when every
/// level is implemented (the fault is then searched, not deferred).
pub fn first_unimplemented(requires: &[Level], gate: &[Level]) -> Option<Level> {
    requires.iter().copied().find(|l| !gate.contains(l))
}

/// A fault expression: IR the generator evaluates.  Since S10c-2 every level the templates use
/// (`ir.core`, `ir.fixed`, `ir.tuple`) is evaluable, so there is no longer a "data only" form; whether
/// a fault is *searched* is the gate's decision (see the module documentation), not the
/// expression's.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expr {
    /// A tree of `ir.core`, `ir.fixed` and `ir.tuple` nodes.
    Core(Ir),
}

impl Expr {
    fn json(&self) -> Json {
        match self {
            Expr::Core(ir) => ir.to_json(),
        }
    }
}

/// The three kinds of VER04r 8.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Parameter rewrites, applied simultaneously.
    Pre(Vec<(String, Expr)>),
    /// A rewrite of the result.
    Post(Expr),
    /// A replacement of the result.
    Replace(Expr),
}

/// One fault instantiated for one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The portable fault name.
    pub name: String,
    /// The operation it perturbs.
    pub op: String,
    /// What it does.
    pub kind: Kind,
    /// The sorted levels the fault needs: its template's and its op's types' (R9).
    pub requires: Vec<Level>,
}

impl Fault {
    /// The `faults` row of `corpus.json` (VER04r 8.1).
    pub fn to_json(&self) -> Json {
        let mut row = vec![
            ("fault".to_string(), Json::str(self.name.as_str())),
            ("op".to_string(), Json::str(self.op.as_str())),
            (
                "requires".to_string(),
                Json::strs(self.requires.iter().map(|l| l.name())),
            ),
        ];
        match &self.kind {
            Kind::Pre(rewrites) => {
                row.push(("kind".into(), Json::str("pre")));
                row.push((
                    "rewrites".into(),
                    Json::arr(rewrites.iter().map(|(p, e)| {
                        Json::obj([("param", Json::str(p.as_str())), ("expr", e.json())])
                    })),
                ));
            }
            Kind::Post(e) => {
                row.push(("kind".into(), Json::str("post")));
                row.push(("expr".into(), e.json()));
            }
            Kind::Replace(e) => {
                row.push(("kind".into(), Json::str("replace")));
                row.push(("expr".into(), e.json()));
            }
        }
        Json::Obj(row)
    }
}

/// Every fault name of the registry (VER04r 8.2, the generic table).
pub const KNOWN_FAULTS: [&str; 18] = [
    "invert",
    "missing-error",
    "swap-choice",
    "off-by-one",
    "truncate-last",
    "ignore-last-byte",
    "xor-lsb",
    "ignore-high-bit",
    "flip-result-bit",
    "drop-last-result-byte",
    "ok-as-err",
    "drop-consumed",
    "swap-components",
    "off-by-one-consumed",
    "saturate-add",
    "saturate-sub",
    "shift-mask",
    "field-zero",
];

/// The name a `match` binds the payload of an `Ok` to in the `Ok` forms (VER04r 8.2); an op
/// parameter of that name would be shadowed, a corpus error, so the templates refuse it.
pub const OK_PAYLOAD: &str = "ok_payload";

/// The reserved name of the call's result in a fault expression.
pub const RESULT: &str = "result";

fn var(n: &str) -> Box<Ir> {
    Box::new(Ir::Var(n.to_string()))
}

fn nat(v: i128) -> Box<Ir> {
    Box::new(Ir::Int {
        ty: Ty::Nat,
        value: v,
    })
}

/// `len(x) - 1` (an `Int`: `Nat - Nat`).
fn len_minus_one(x: &str) -> Box<Ir> {
    Box::new(Ir::Arith(
        Arith::Sub,
        Ty::Int,
        Box::new(Ir::Len(var(x))),
        nat(1),
    ))
}

/// `len(x) > 0`.
fn nonempty(x: &str) -> Box<Ir> {
    Box::new(Ir::Cmp(Cmp::Gt, Box::new(Ir::Len(var(x))), nat(0)))
}

fn refuse(op: &LoweredOp, fault: &str, why: &str) -> GenError {
    GenError::new(
        code::FAULT,
        format!("{}/{fault}", op.name),
        format!("the fault cannot be instantiated on this operation: {why}"),
    )
}

/// `U8` literal.
fn u8_lit(v: i128) -> Ir {
    Ir::Int {
        ty: Ty::Fixed(coding_adventures_warrant_grammar::fixed::FixedTy::U8),
        value: v,
    }
}

/// An `ir.fixed` node of the templates: `at` is the width a built-in without a static type works at.
fn fixed_node(tag: &str, at: Option<FixedTy>, args: Vec<Ir>) -> Ir {
    Ir::Fixed {
        tag: crate::ir::fixed_tag(tag).unwrap_or("bit_xor"),
        ty: None,
        at,
        args,
    }
}

/// `build(ty)` of VER04r 8.2 (the one rule of [D50], R31): a template for a value of type `ty`.
///
/// * a **tuple** is the tuple of `build` of its components and a **record** the `ctor` of `build` of its
///   fields: a composite is always **built**, never taken whole from a parameter of its type, and one that is
///   itself a tuple or a record is built in turn (R95 (e): the old refusal of a nested component is gone);
/// * a **leaf** (`Bool`, `Nat`, a fixed-width type, `Bytes`, `Bytes<N>`, `Option<T>` or a variant) is the
///   first parameter, in parameter order, whose type is **exactly** `ty`, else the **zero** of `ty` (`false`,
///   `0`, an empty or all-zero `Bytes`, `["none","T"]`); a variant has no zero;
/// * anything else (a `Result`, a list) has no template, and the one wording of every position is
///   `missing-error has no template for this payload type`.
fn build(ty: &Ty, op: &LoweredOp, fault: &str, decls: &Decls) -> Result<Ir, GenError> {
    let no_template = || refuse(op, fault, "missing-error has no template for this payload type");
    match ty {
        Ty::Tuple(parts) => {
            return Ok(Ir::Tuple(
                parts
                    .iter()
                    .map(|p| build(p, op, fault, decls))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        Ty::Named(name) if decls.is_record(name) => {
            let fields = decls.fields(name, name).ok_or_else(no_template)?;
            return Ok(Ir::Ctor {
                name: name.clone(),
                args: fields
                    .iter()
                    .map(|f| build(f, op, fault, decls))
                    .collect::<Result<Vec<_>, _>>()?,
            });
        }
        Ty::Bool | Ty::Nat | Ty::Bytes(_) | Ty::Option(_) | Ty::Fixed(_) | Ty::Named(_) => {}
        _ => return Err(no_template()),
    }
    if let Some((n, _)) = op.params.iter().find(|(_, t)| t == ty) {
        return Ok(Ir::Var(n.clone()));
    }
    match ty {
        // a variant has no zero (its default value is the decision `wrong-ctor` needs, [D49])
        Ty::Named(_) => Err(no_template()),
        Ty::Option(inner) => {
            if has_named_width(inner) {
                return Err(refuse(op, fault, "a named width has no zero literal"));
            }
            Ok(Ir::None((**inner).clone()))
        }
        other => zero(other, op, fault),
    }
}

/// Does a type mention a named width (`Bytes<N>`)?  A static width is a parameter of the op, not a type of
/// the corpus, so an IR `none` cannot carry it.
fn has_named_width(ty: &Ty) -> bool {
    match ty {
        Ty::Bytes(Some(Width::Param(_))) => true,
        Ty::Option(t) | Ty::Result(t, _) | Ty::List(t) => has_named_width(t),
        Ty::Tuple(parts) => parts.iter().any(has_named_width),
        _ => false,
    }
}

/// The template levels of a `build` result (8.2): `ir.core` for a bare `var` or `lit`, else `ir.tuple` if it
/// holds a `tuple` node together with `ir.data` if it holds a `ctor` or `none` node, sorted.
fn build_levels(built: &Ir) -> Vec<Level> {
    let (mut tuple, mut data) = (false, false);
    built.walk(&mut |n| match n {
        Ir::Tuple(_) => tuple = true,
        Ir::Ctor { .. } | Ir::None(_) => data = true,
        _ => {}
    });
    let mut out = Vec::new();
    if data {
        out.push(Level::Data);
    }
    if tuple {
        out.push(Level::Tuple);
    }
    if out.is_empty() {
        out.push(Level::Core);
    }
    out
}

/// The zero literal of a type (VER04r 16.10 R3): `0`, `false` or zero bytes.
fn zero(ty: &Ty, op: &LoweredOp, fault: &str) -> Result<Ir, GenError> {
    Ok(match ty {
        Ty::Bool => Ir::Bool(false),
        Ty::Nat | Ty::Fixed(_) => Ir::Int {
            ty: ty.clone(),
            value: 0,
        },
        Ty::Bytes(None) => Ir::Bytes(String::new()),
        Ty::Bytes(Some(Width::Lit(n))) => Ir::Bytes("00".repeat(usize::try_from(*n).unwrap_or(0))),
        Ty::Bytes(Some(Width::Param(_))) => {
            return Err(refuse(op, fault, "a named width has no zero literal"))
        }
        _ => {
            return Err(refuse(
                op,
                fault,
                "a component of this type has no zero literal",
            ))
        }
    })
}

/// The `Ok` form (VER04r 8.2): rewrite the payload of an `Ok`, leave an `Err` alone:
/// `["match", R, [[["ok",["bind","ok_payload"]], ["ok", F]], [["err_any"], R]]]`, `F` being
/// `body` applied to `["var","ok_payload"]`.
fn ok_form(op: &LoweredOp, fault: &str, body: impl Fn(Ir) -> Ir) -> Result<Ir, GenError> {
    if op.params.iter().any(|(p, _)| p == OK_PAYLOAD) {
        return Err(refuse(op, fault, "an operation parameter is named ok_payload"));
    }
    Ok(Ir::Match {
        scrutinee: var(RESULT),
        arms: vec![
            (
                Pat::Ok(Box::new(Pat::Bind(OK_PAYLOAD.to_string()))),
                Ir::Ok(Box::new(body(Ir::Var(OK_PAYLOAD.to_string())))),
            ),
            (Pat::ErrAny, Ir::Var(RESULT.to_string())),
        ],
        // a fault template matches on the op's `Result`: `ir.tuple`
        on_result: true,
    })
}

/// The `Bytes` form of `flip-result-bit` over `src` (VER04r 8.2, exact JSON):
/// `if len(src) > 0 then concat(be_bytes(bit_xor(unwrap_or(at(src, 0), 0u8), 1u8)), drop(src, 1))
/// else b"01"`.
fn flip_expr(src: &Ir) -> Ir {
    Ir::If(
        Box::new(Ir::Cmp(
            Cmp::Gt,
            Box::new(Ir::Len(Box::new(src.clone()))),
            nat(0),
        )),
        Box::new(Ir::Concat(
            Box::new(fixed_node(
                "be_bytes",
                Some(FixedTy::U8),
                vec![fixed_node(
                    "bit_xor",
                    Some(FixedTy::U8),
                    vec![
                        fixed_node(
                            "unwrap_or",
                            None,
                            vec![
                                fixed_node("at", None, vec![src.clone(), *nat(0)]),
                                u8_lit(0),
                            ],
                        ),
                        u8_lit(1),
                    ],
                )],
            )),
            Box::new(Ir::Drop(Box::new(src.clone()), nat(1))),
        )),
        Box::new(Ir::Bytes("01".to_string())),
    )
}

/// The `Bytes` form of `drop-last-result-byte` over `src` (VER04r 8.2, exact JSON):
/// `if len(src) > 0 then take(src, len(src) - 1) else b"00"`.
fn drop_last_expr(src: &Ir) -> Ir {
    Ir::If(
        Box::new(Ir::Cmp(
            Cmp::Gt,
            Box::new(Ir::Len(Box::new(src.clone()))),
            nat(0),
        )),
        Box::new(Ir::Take(
            Box::new(src.clone()),
            Box::new(Ir::Arith(
                Arith::Sub,
                Ty::Int,
                Box::new(Ir::Len(Box::new(src.clone()))),
                nat(1),
            )),
        )),
        Box::new(Ir::Bytes("00".to_string())),
    )
}

/// The tuple type a composite fault targets and whether it sits inside an `Ok`.
fn tuple_target<'o>(
    op: &'o LoweredOp,
    fault: &str,
) -> Result<(&'o [Ty], bool), GenError> {
    match &op.ret {
        Ty::Tuple(parts) => Ok((parts, false)),
        Ty::Result(ok, _) => match &**ok {
            Ty::Tuple(parts) => Ok((parts, true)),
            _ => Err(refuse(
                op,
                fault,
                &format!("{fault} needs a tuple or Result<tuple, E> result"),
            )),
        },
        _ => Err(refuse(
            op,
            fault,
            &format!("{fault} needs a tuple or Result<tuple, E> result"),
        )),
    }
}

fn project(src: &Ir, index: usize) -> Ir {
    Ir::Project(Box::new(src.clone()), index)
}

/// An expression of a template (`ir.core`, `ir.fixed` and `ir.tuple` nodes, all evaluable).
fn expr_of(ir: Ir) -> Expr {
    Expr::Core(ir)
}

/// Instantiate the template `name` on `op`'s signature, for a document with no error types
/// ([`instantiate_in`] with none; `ok-as-err` needs them).
///
/// # Errors
///
/// As [`instantiate_in`].
pub fn instantiate(op: &LoweredOp, name: &str) -> Result<Fault, GenError> {
    instantiate_in(op, name, &[])
}

/// Instantiate the template `name` on `op`'s signature.  `errors` are the document's error types
/// (the cases of an `ok-as-err` fault come from them).
///
/// The row's `requires` is the sorted union of the template's own levels and the levels of the
/// op's parameter and result types (R9).
///
/// # Errors
///
/// `fault` for an unknown name or a signature the template cannot take; `W0701` for an op
/// with a parameter called `result` (the reserved name, VER04r 8.1).
pub fn instantiate_in(
    op: &LoweredOp,
    name: &str,
    errors: &[ErrorType],
) -> Result<Fault, GenError> {
    instantiate_with(op, name, errors, &Decls::default())
}

/// [`instantiate_in`] for a document that declares records and variants (`decls`): `field-zero` and the
/// record, `Option` and variant payloads of `missing-error` read them (VER04r 8.2, [D49], [D50], R31, R32).
/// A document that declares none gives empty tables, which is not a second mode (R95 (d)): `field-zero` then
/// has no record to name, and `build` meets only tuples and scalars.
///
/// # Errors
///
/// As [`instantiate_in`].
pub fn instantiate_with(
    op: &LoweredOp,
    name: &str,
    errors: &[ErrorType],
    decls: &Decls,
) -> Result<Fault, GenError> {
    check_reserved(op)?;
    let bytes_params: Vec<&str> = op
        .params
        .iter()
        .filter(|(_, t)| t.is_bytes())
        .map(|(n, _)| n.as_str())
        .collect();
    let (kind, levels): (Kind, Vec<Level>) = match name {
        "invert" => {
            if op.ret != Ty::Bool {
                return Err(refuse(op, name, "invert needs a Bool result"));
            }
            (
                Kind::Post(Expr::Core(Ir::Not(var(RESULT)))),
                vec![Level::Core],
            )
        }
        // Harness citation (`V1_FAULTS` in warrant-grammar/src/evidence.rs): the Rust harness builds
        // the `Ok` payload as `Ok(0)` / the first domain element; here it is the first parameter of
        // that type ([D50]), a value difference accepted as in [D37].
        // `missing-error` (VER04r 8.2, [D50]): `if is_err(result) then Ok(<payload>) else result`, one
        // rule for every payload.  A tuple payload is the tuple of `ci` (R3); a `Bytes` payload is the
        // first `Bytes` parameter, else the zero `Bytes` literal; a scalar payload (`Bool`, `Nat`,
        // fixed-width, S11f-2) is the first parameter whose type is *exactly* the payload type, else
        // the type's zero literal.  The order of refusals is the oracle's.
        "missing-error" => {
            let Ty::Result(ok, _) = &op.ret else {
                return Err(refuse(op, name, "missing-error needs a Result result"));
            };
            // A `Bytes` payload (any `Bytes<M>`) is the first `Bytes` parameter, matched loosely (the rule
            // since S3c, kept for byte parity), else the zero literal.  Every other payload is `build` of
            // VER04r 8.2 ([D50], R31): a composite is always built, a leaf is the first parameter of exactly
            // its type, else its zero.
            let (leaf, levels) = if ok.is_bytes() {
                (
                    match bytes_params.first() {
                        Some(p) => Ir::Var((*p).to_string()),
                        None => zero(ok, op, name)?,
                    },
                    vec![Level::Core],
                )
            } else {
                let built = build(ok, op, name, decls)?;
                let levels = build_levels(&built);
                (built, levels)
            };
            let expr = Ir::If(
                Box::new(Ir::IsErr(var(RESULT))),
                Box::new(Ir::Ok(Box::new(leaf))),
                var(RESULT),
            );
            return finish(op, name, Kind::Replace(expr_of(expr)), levels);
        }
        // `field-zero` ([D49], R32): the first numeric field of a record result becomes `0` (8.2), inside an
        // `Ok` for a `Result`; every other field is read back unchanged.  `F(S) = ["ctor","N",[["field",S,0],..,
        // ["lit",{"type":"T","value":"0"}],..,["field",S,n-1]]]`.  Kind `post`; `ir.data`, and `ir.tuple` as well
        // in the `Ok` form.
        "field-zero" => {
            let (inside, target) = match &op.ret {
                Ty::Result(ok, _) => (true, &**ok),
                other => (false, other),
            };
            let Ty::Named(rname) = target else {
                return Err(refuse(op, name, "field-zero needs a record or Result<record, E> result"));
            };
            let Some(fields) = decls.is_record(rname).then(|| decls.fields(rname, rname)).flatten() else {
                return Err(refuse(op, name, "field-zero needs a record or Result<record, E> result"));
            };
            let Some(k) = fields.iter().position(|t| matches!(t, Ty::Nat) || t.is_fixed()) else {
                return Err(refuse(op, name, "field-zero needs a record with a numeric field"));
            };
            let zero_k = Ir::Int {
                ty: fields[k].clone(),
                value: 0,
            };
            let count = fields.len();
            let rewrite = |src: Ir| Ir::Ctor {
                name: rname.clone(),
                args: (0..count)
                    .map(|i| {
                        if i == k {
                            zero_k.clone()
                        } else {
                            Ir::Field(Box::new(src.clone()), i)
                        }
                    })
                    .collect(),
            };
            let expr = if inside {
                ok_form(op, name, rewrite)?
            } else {
                rewrite(Ir::Var(RESULT.to_string()))
            };
            (
                Kind::Post(expr_of(expr)),
                if inside { vec![Level::Data, Level::Tuple] } else { vec![Level::Data] },
            )
        }
        "swap-choice" => {
            if !op.params.iter().any(|(n, t)| n == "choice" && *t == Ty::Bool) {
                return Err(refuse(op, name, "swap-choice needs a Bool parameter `choice`"));
            }
            (
                Kind::Pre(vec![(
                    "choice".to_string(),
                    Expr::Core(Ir::Not(var("choice"))),
                )]),
                vec![Level::Core],
            )
        }
        "off-by-one" => {
            let (arith_ty, lit_ty) = match &op.ret {
                Ty::Nat => (Ty::Int, Ty::Nat),
                t if t.is_fixed() => (t.clone(), t.clone()),
                _ => return Err(refuse(op, name, "off-by-one needs a Nat or fixed-width result")),
            };
            (
                Kind::Post(Expr::Core(Ir::Arith(
                    Arith::Add,
                    arith_ty,
                    var(RESULT),
                    Box::new(Ir::Int {
                        ty: lit_ty,
                        value: 1,
                    }),
                ))),
                vec![Level::Core],
            )
        }
        "truncate-last" => {
            if !op.ret.is_bytes() {
                return Err(refuse(op, name, "truncate-last needs a Bytes result"));
            }
            (
                Kind::Post(Expr::Core(Ir::If(
                    nonempty(RESULT),
                    Box::new(Ir::Take(var(RESULT), len_minus_one(RESULT))),
                    var(RESULT),
                ))),
                vec![Level::Core],
            )
        }
        "ignore-last-byte" => {
            if bytes_params.is_empty() {
                return Err(refuse(op, name, "ignore-last-byte needs a Bytes parameter"));
            }
            let rewrites = bytes_params
                .iter()
                .map(|p| {
                    (
                        (*p).to_string(),
                        Expr::Core(Ir::If(
                            nonempty(p),
                            Box::new(Ir::Concat(
                                Box::new(Ir::Take(var(p), len_minus_one(p))),
                                Box::new(Ir::Bytes("00".to_string())),
                            )),
                            var(p),
                        )),
                    )
                })
                .collect();
            (Kind::Pre(rewrites), vec![Level::Core])
        }
        "xor-lsb" => {
            let Some(t) = op.ret.fixed() else {
                return Err(refuse(op, name, "xor-lsb needs a fixed-width result"));
            };
            (
                Kind::Post(Expr::Core(fixed_node(
                    "bit_xor",
                    Some(t),
                    vec![
                        Ir::Var(RESULT.to_string()),
                        Ir::Int {
                            ty: Ty::Fixed(t),
                            value: 1,
                        },
                    ],
                ))),
                vec![Level::Fixed],
            )
        }
        "ignore-high-bit" => {
            let u64_params: Vec<&str> = op
                .params
                .iter()
                .filter(|(_, t)| t.fixed() == Some(coding_adventures_warrant_grammar::fixed::FixedTy::U64))
                .map(|(n, _)| n.as_str())
                .collect();
            if u64_params.is_empty() {
                return Err(refuse(op, name, "ignore-high-bit needs a U64 parameter"));
            }
            let rewrites = u64_params
                .iter()
                .map(|p| {
                    (
                        (*p).to_string(),
                        Expr::Core(fixed_node(
                            "bit_and",
                            Some(FixedTy::U64),
                            vec![
                                Ir::Var((*p).to_string()),
                                Ir::Int {
                                    ty: Ty::Fixed(FixedTy::U64),
                                    value: 9_223_372_036_854_775_807,
                                },
                            ],
                        )),
                    )
                })
                .collect();
            (Kind::Pre(rewrites), vec![Level::Fixed])
        }
        // `flip-result-bit` [D27, D35]: a fixed-width, `Bytes`/`Bytes<N>` or `Result` of those.
        "flip-result-bit" => match &op.ret {
            ret if ret.is_fixed() => {
                let t = ret.fixed();
                (
                    Kind::Post(Expr::Core(fixed_node(
                        "bit_xor",
                        t,
                        vec![
                            Ir::Var(RESULT.to_string()),
                            Ir::Int {
                                ty: ret.clone(),
                                value: 1,
                            },
                        ],
                    ))),
                    vec![Level::Fixed],
                )
            }
            ret if ret.is_bytes() => (
                Kind::Post(expr_of(flip_expr(&Ir::Var(RESULT.to_string())))),
                vec![Level::Fixed],
            ),
            Ty::Result(ok, _) if ok.is_bytes() => (
                Kind::Post(expr_of(ok_form(op, name, |src| flip_expr(&src))?)),
                vec![Level::Fixed, Level::Tuple],
            ),
            Ty::Result(ok, _) if ok.is_fixed() => {
                let t = (**ok).clone();
                (
                    Kind::Post(expr_of(ok_form(op, name, |src| {
                        fixed_node(
                            "bit_xor",
                            t.fixed(),
                            vec![
                                src,
                                Ir::Int {
                                    ty: t.clone(),
                                    value: 1,
                                },
                            ],
                        )
                    })?)),
                    vec![Level::Fixed, Level::Tuple],
                )
            }
            _ => {
                return Err(refuse(
                    op,
                    name,
                    "flip-result-bit needs a fixed-width, Bytes or Result of those result",
                ))
            }
        },
        // `drop-last-result-byte` [D27, D35]: `Bytes`/`Bytes<N>` or a `Result` of one.
        "drop-last-result-byte" => match &op.ret {
            ret if ret.is_bytes() => (
                Kind::Post(expr_of(drop_last_expr(&Ir::Var(RESULT.to_string())))),
                vec![Level::Core],
            ),
            Ty::Result(ok, _) if ok.is_bytes() => (
                Kind::Post(expr_of(ok_form(op, name, |src| drop_last_expr(&src))?)),
                vec![Level::Core, Level::Tuple],
            ),
            _ => {
                return Err(refuse(
                    op,
                    name,
                    "drop-last-result-byte needs a Bytes or Result<Bytes, E> result",
                ))
            }
        },
        // The second component replaced by `0` (VER04j 4.8), inside an `Ok` for a `Result`.
        "drop-consumed" => {
            let (components, inside) = tuple_target(op, name)?;
            let second = &components[1];
            if !(second.is_fixed() || *second == Ty::Nat) {
                return Err(refuse(op, name, "drop-consumed needs a numeric second component"));
            }
            let zero_second = zero(second, op, name)?;
            let n = components.len();
            let rewrite = |src: Ir| {
                let mut parts = vec![project(&src, 0), zero_second.clone()];
                parts.extend((2..n).map(|i| project(&src, i)));
                Ir::Tuple(parts)
            };
            let expr = if inside {
                ok_form(op, name, rewrite)?
            } else {
                rewrite(Ir::Var(RESULT.to_string()))
            };
            (Kind::Post(expr_of(expr)), vec![Level::Tuple])
        }
        // The second component `n` exchanged with its pair partner: `n + 1` when `n` is even and
        // `n - 1` when it is odd (VER04r 8.2 [D36], VER04j 4.8), inside an `Ok` for a `Result`.
        // `Nat` component: `if unwrap_or(rem(C, 2), 0) == 0 then C + 1 else C - 1` (`Int` literals
        // in the test, `Nat` literals in the arms; `add`/`sub` on a `Nat` are exact and `Int`
        // typed, and the value is always a `Nat` value).  Fixed-width component: `bit_xor(C, 1)`.
        // The numeric check comes before the `ok_payload` clash, as in the oracle.
        "off-by-one-consumed" => {
            let (components, inside) = tuple_target(op, name)?;
            let second = &components[1];
            if !(second.is_fixed() || *second == Ty::Nat) {
                return Err(refuse(
                    op,
                    name,
                    "off-by-one-consumed needs a numeric second component",
                ));
            }
            let n = components.len();
            let rewrite = |src: Ir| {
                let c = project(&src, 1);
                let x = if let Some(t) = second.fixed() {
                    fixed_node(
                        "bit_xor",
                        Some(t),
                        vec![
                            c,
                            Ir::Int {
                                ty: Ty::Fixed(t),
                                value: 1,
                            },
                        ],
                    )
                } else {
                    let int = |v: i128| {
                        Box::new(Ir::Int {
                            ty: Ty::Int,
                            value: v,
                        })
                    };
                    let rem = fixed_node(
                        "rem",
                        None,
                        vec![
                            c.clone(),
                            Ir::Int {
                                ty: Ty::Int,
                                value: 2,
                            },
                        ],
                    );
                    let even = fixed_node(
                        "unwrap_or",
                        None,
                        vec![
                            rem,
                            Ir::Int {
                                ty: Ty::Int,
                                value: 0,
                            },
                        ],
                    );
                    Ir::If(
                        Box::new(Ir::Cmp(Cmp::Eq, Box::new(even), int(0))),
                        Box::new(Ir::Arith(Arith::Add, Ty::Int, Box::new(c.clone()), nat(1))),
                        Box::new(Ir::Arith(Arith::Sub, Ty::Int, Box::new(c), nat(1))),
                    )
                };
                let mut parts = vec![project(&src, 0), x];
                parts.extend((2..n).map(|i| project(&src, i)));
                Ir::Tuple(parts)
            };
            let expr = if inside {
                ok_form(op, name, rewrite)?
            } else {
                rewrite(Ir::Var(RESULT.to_string()))
            };
            (Kind::Post(expr_of(expr)), vec![Level::Fixed, Level::Tuple])
        }
        // The components of a homogeneous tuple exchanged (reversed, so a pair is swapped).
        "swap-components" => {
            let (components, inside) = tuple_target(op, name)?;
            if components.iter().any(|c| *c != components[0]) {
                return Err(refuse(op, name, "swap-components needs a homogeneous tuple"));
            }
            let n = components.len();
            let rewrite = |src: Ir| Ir::Tuple((0..n).rev().map(|i| project(&src, i)).collect());
            let expr = if inside {
                ok_form(op, name, rewrite)?
            } else {
                rewrite(Ir::Var(RESULT.to_string()))
            };
            (Kind::Post(expr_of(expr)), vec![Level::Tuple])
        }
        // Success reported as the first error case of the error type (VER04j 4.8).
        "ok-as-err" => {
            let Ty::Result(ok, err_name) = &op.ret else {
                return Err(refuse(op, name, "ok-as-err needs a Result<tuple, E> result"));
            };
            if !matches!(&**ok, Ty::Tuple(_)) {
                return Err(refuse(op, name, "ok-as-err needs a Result<tuple, E> result"));
            }
            let Some(first) = errors
                .iter()
                .find(|e| e.name == *err_name)
                .and_then(|e| e.cases.first())
            else {
                return Err(refuse(op, name, "ok-as-err needs the error type's cases"));
            };
            let expr = Ir::Match {
                scrutinee: var(RESULT),
                arms: vec![
                    (Pat::Ok(Box::new(Pat::Wild)), Ir::Err(first.clone())),
                    (Pat::ErrAny, Ir::Var(RESULT.to_string())),
                ],
                on_result: true,
            };
            (Kind::Post(expr_of(expr)), vec![Level::Tuple])
        }
        // `saturate-add` / `saturate-sub` (VER04r 8.2, [D51]; kind `replace`): the saturating sum or
        // difference of the first two parameters, which share one fixed-width type `T`; the result is
        // `T` or `Result<T,E>` (then wrapped in `Ok`).  Unsigned add: `unwrap_or(checked_add(A,B),
        // max_value(T))`; unsigned sub: `.. min_value(T)`; signed add: `.. if B < 0 then min else max`;
        // signed sub: `.. if B < 0 then max else min`.  Template level `ir.fixed`.
        //
        // Harness citation (`V1_FAULTS`, evidence.rs): `Replace("p0.saturating_add(p1)")` /
        // `Replace("p0.saturating_sub(p1)")`.  Under `Replace` the exported function is never called,
        // so the adapter returns Ok unconditionally, hence the `["ok", V]` wrapper for a Result.
        "saturate-add" | "saturate-sub" => {
            let adding = name == "saturate-add";
            let t = match (op.params.first(), op.params.get(1)) {
                (Some((_, a)), Some((_, b))) if a.is_fixed() && a == b => a.clone(),
                _ => {
                    return Err(refuse(
                        op,
                        name,
                        &format!("{name} needs two leading parameters of one fixed-width type"),
                    ))
                }
            };
            let ok_result = matches!(&op.ret, Ty::Result(ok, _) if **ok == t);
            if op.ret != t && !ok_result {
                return Err(refuse(
                    op,
                    name,
                    &format!("{name} needs a result of the parameters' type or a Result of it"),
                ));
            }
            let fty = t.fixed().ok_or_else(|| GenError::internal("a fixed type"))?;
            let a = Ir::Var(op.params[0].0.clone());
            let b = Ir::Var(op.params[1].0.clone());
            let bound = |tag: &str| -> Result<Ir, GenError> {
                Ok(Ir::Fixed {
                    tag: crate::ir::fixed_tag(tag)
                        .ok_or_else(|| GenError::internal("a fixed-width bound tag"))?,
                    ty: Some(fty),
                    at: None,
                    args: vec![],
                })
            };
            let checked = Ir::Fixed {
                tag: crate::ir::fixed_tag(if adding { "checked_add" } else { "checked_sub" })
                    .ok_or_else(|| GenError::internal("a checked-arithmetic tag"))?,
                ty: None,
                at: Some(fty),
                args: vec![a, b.clone()],
            };
            let default = if !fty.signed() {
                bound(if adding { "max_value" } else { "min_value" })?
            } else {
                let neg = Ir::Cmp(
                    Cmp::Lt,
                    Box::new(b),
                    Box::new(Ir::Int {
                        ty: t.clone(),
                        value: 0,
                    }),
                );
                let (up, down) = (bound("max_value")?, bound("min_value")?);
                let (then, other) = if adding { (down, up) } else { (up, down) };
                Ir::If(Box::new(neg), Box::new(then), Box::new(other))
            };
            let v = fixed_node("unwrap_or", None, vec![checked, default]);
            let expr = if ok_result { Ir::Ok(Box::new(v)) } else { v };
            (Kind::Replace(expr_of(expr)), vec![Level::Fixed])
        }
        // `shift-mask` (VER04r 8.2, [D51]; kind `pre`): the last parameter `p` becomes `p mod w`, `w`
        // the FIRST parameter's width, as `unwrap_or(rem(p, w), 0)` (`rem_trunc` for a signed last
        // parameter), literals in the last parameter's own type.
        //
        // Harness citation (`V1_FAULTS`, evidence.rs): `PreLast("p % {w}")`.  Rust `%` truncates on
        // signed types, hence `rem_trunc`; `w` is the width of the first argument's type.
        "shift-mask" => {
            let Some((_, first)) = op.params.first() else {
                return Err(refuse(op, name, "shift-mask needs parameters"));
            };
            let (last_name, last) = &op.params[op.params.len() - 1];
            let (Some(f), Some(l)) = (first.fixed(), last.fixed()) else {
                return Err(refuse(
                    op,
                    name,
                    "shift-mask needs a fixed-width first and last parameter",
                ));
            };
            let lit = |v: i128| Ir::Int {
                ty: last.clone(),
                value: v,
            };
            let rem = if l.signed() {
                fixed_node("rem_trunc", Some(l), vec![Ir::Var(last_name.clone()), lit(i128::from(f.bits()))])
            } else {
                fixed_node("rem", None, vec![Ir::Var(last_name.clone()), lit(i128::from(f.bits()))])
            };
            let x = fixed_node("unwrap_or", None, vec![rem, lit(0)]);
            (
                Kind::Pre(vec![(last_name.clone(), expr_of(x))]),
                vec![Level::Fixed],
            )
        }
        _ => {
            return Err(GenError::new(
                code::FAULT,
                format!("{}/{name}", op.name),
                "the fault cannot be instantiated on this operation: an unknown fault name",
            ))
        }
    };
    finish(op, name, kind, levels)
}

/// Seal a fault: its `requires` is the sorted, duplicate-free union of `levels` and the levels of
/// the op's types (a tuple parameter or result adds `ir.tuple`, R9).
fn finish(
    op: &LoweredOp,
    name: &str,
    kind: Kind,
    mut levels: Vec<Level>,
) -> Result<Fault, GenError> {
    if op.params.iter().any(|(_, t)| t.contains_tuple()) || op.ret.contains_tuple() {
        levels.push(Level::Tuple);
    }
    if op.params.iter().any(|(_, t)| t.contains_data()) || op.ret.contains_data() {
        levels.push(Level::Data);
    }
    levels.sort_unstable();
    levels.dedup();
    Ok(Fault {
        name: name.to_string(),
        op: op.name.clone(),
        kind,
        requires: levels,
    })
}

/// Type-check a fault against the signature it perturbs (VER04r 16.17 R67, 16.19 R94, 16.20 R96 to R98).
///
/// Each expression is typed with the declared type of its position as the *expected* type (a `pre` rewrite
/// the type of the parameter it rewrites, a `post` or `replace` expression the op's result, so a zero
/// literal has the type it is the zero *of*) and must have a **common type** with it ([`crate::fault_types`]),
/// in every branch of an `if` and arm of a `match`.  `statics` are the static widths of the stand-in the
/// fault is searched on (an op with a width parameter is typed at the widths of each stand-in: a named width
/// cannot be unified with a literal one).  A `pre` rewrite that names no parameter of the op has nothing to
/// compare.
///
/// # Errors
///
/// `fault` for a pair with no common type; `ir` for an ill-typed tree.
pub fn type_fault(
    fault: &Fault,
    op: &LoweredOp,
    statics: &[(String, u64)],
    decls: &Decls,
) -> Result<(), GenError> {
    let typer = crate::fault_types::FaultTyper {
        decls,
        op: &op.name,
        branch_check: true,
    };
    let mut env: Vec<(String, Ty)> = op
        .params
        .iter()
        .map(|(n, t)| (n.clone(), t.substitute(statics)))
        .collect();
    let at = |e: GenError| e.at(&format!("{}/{}", fault.op, fault.name));
    match &fault.kind {
        Kind::Pre(rewrites) => {
            for (param, Expr::Core(ir)) in rewrites {
                let declared = env.iter().find(|(n, _)| n == param).map(|(_, t)| t.clone());
                typer.typed(ir, &env, declared.as_ref(), "rewrite").map_err(at)?;
            }
        }
        Kind::Post(Expr::Core(ir)) | Kind::Replace(Expr::Core(ir)) => {
            let ret = op.ret.substitute(statics);
            env.push((RESULT.to_string(), ret.clone()));
            typer.typed(ir, &env, Some(&ret), "expression").map_err(at)?;
        }
    }
    Ok(())
}

/// An op with a parameter named `result` is a generation error (VER04r 8.1).
///
/// # Errors
///
/// `W0701`.
pub fn check_reserved(op: &LoweredOp) -> Result<(), GenError> {
    if op.params.iter().any(|(n, _)| n == RESULT) {
        return Err(GenError::new(
            code::W0701,
            &op.name,
            "an operation may not have a parameter named `result` (the reserved name, VER04r 8.1)",
        ));
    }
    Ok(())
}

/// The state of one fault on one candidate claim (VER04r 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    /// A case differs; the chunk holding the lowest such case.
    Witnessed(u64),
    /// A stand-in exists and no admitted case differs.
    Blind,
    /// No stand-in exists.
    Unproven,
    /// A stand-in exists but the fault's level is not implemented here.
    Deferred(Level),
}

/// How a claim's subject call maps to the op's parameters: the binder name of each argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandIn {
    /// `binders[i]` is the binder passed as the op's `i`-th parameter.
    pub binders: Vec<String>,
}

/// Does `suite` admit the reference as a stand-in for the subject of `fault`?
pub fn stand_in(suite: &LoweredSuite, fault: &Fault, op: &LoweredOp) -> Option<StandIn> {
    let SuiteKind::Reference { subject, .. } = &suite.kind else {
        return None;
    };
    let Ir::Subject { op: called, args, .. } = subject else {
        return None;
    };
    if *called != fault.op || args.len() != op.params.len() {
        return None;
    }
    let mut binders = Vec::with_capacity(args.len());
    for a in args {
        match a {
            Ir::Var(v) if suite.binders.iter().any(|(b, _)| b == v) => binders.push(v.clone()),
            _ => return None,
        }
    }
    // A pre fault rewrites by parameter, but the stand-in rewrites by binder: a binder
    // passed twice must be rewritten the same way at both positions, or there is no
    // single rewritten binder to re-evaluate `expected` at.
    // ASSUMPTION (revisit in S10): this check only compares *whether* each position is
    // rewritten, not *how*.  It is sound for today's registry, where a binder passed at two
    // positions gets the same rewrite at both (`ignore-last-byte` and `ignore-high-bit`
    // rewrite every parameter of the type uniformly; `swap-choice` has one parameter).
    // `search` debug_asserts that the rewrites landing on one binder agree.  A fault with
    // distinct per-parameter rewrites must tighten this to "same expression" or give up.
    if let Kind::Pre(rewrites) = &fault.kind {
        for (i, b) in binders.iter().enumerate() {
            let rewritten = |k: usize| rewrites.iter().any(|(p, _)| *p == op.params[k].0);
            if binders
                .iter()
                .enumerate()
                .any(|(k, other)| other == b && rewritten(k) != rewritten(i))
            {
                return None;
            }
        }
    }
    Some(StandIn { binders })
}

fn lookup<'v>(vars: &'v Vars<'_>, name: &str) -> Option<&'v Value> {
    vars.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
}

/// One fault being searched on one suite.
pub struct Probe<'a> {
    /// The fault.
    pub fault: &'a Fault,
    /// Its op.
    pub op: &'a LoweredOp,
    /// The stand-in mapping.
    pub stand_in: StandIn,
}

/// Search every admitted case of `plan` for the lowest one where each probe's faulted value
/// differs from `expected`.  Returns, per probe, the product ordinal of the witness (or
/// `None` for a blind probe).  One pass serves all probes of the suite and stops as soon as
/// every probe has a witness.
///
/// # Errors
///
/// Evaluation errors, attributed to the claim and the fault.
pub fn search(
    program: &Program<'_>,
    suite: &LoweredSuite,
    plan: &SuitePlan<'_>,
    probes: &[Probe<'_>],
) -> Result<Vec<Option<u64>>, GenError> {
    let mut found: Vec<Option<u64>> = vec![None; probes.len()];
    if probes.is_empty() {
        return Ok(found);
    }
    let SuiteKind::Reference { expected, .. } = &suite.kind else {
        return Err(GenError::internal("a witness search needs a reference suite"));
    };
    let mut left = probes.len();
    let mut st = Stats::default();
    plan.scan_admitted(&mut |ordinal, vars| {
        let want = program.eval(expected, vars, &mut st)?;
        for (i, probe) in probes.iter().enumerate() {
            if found[i].is_some() {
                continue;
            }
            let at = |e: GenError| e.at(&format!("{}: {}/{}", suite.claim, probe.op.name, probe.fault.name));
            // the op's parameters, as the subject call would receive them
            let mut env: Vec<(&str, Value)> = Vec::with_capacity(probe.op.params.len() + 1);
            for ((pn, _), b) in probe.op.params.iter().zip(&probe.stand_in.binders) {
                let v = lookup(vars, b)
                    .ok_or_else(|| GenError::internal("a stand-in binder is not in scope"))?;
                env.push((pn.as_str(), v.clone()));
            }
            let faulted = match &probe.fault.kind {
                Kind::Pre(rewrites) => {
                    let mut moved: Vec<(&str, Value)> = vars.to_vec();
                    let mut rewritten_to: Vec<(String, Value)> = Vec::new();
                    for (param, expr) in rewrites {
                        let Expr::Core(ir) = expr;
                        let nv = program.eval(ir, &env, &mut st).map_err(at)?;
                        let pos = probe
                            .op
                            .params
                            .iter()
                            .position(|(n, _)| n == param)
                            .ok_or_else(|| at(GenError::internal("a rewrite names no parameter")))?;
                        let binder = &probe.stand_in.binders[pos];
                        for slot in moved.iter_mut().filter(|(n, _)| n == binder) {
                            // uniform rewrites per binder (see `stand_in`): a second rewrite
                            // landing on the same binder must agree with the first
                            debug_assert!(
                                !rewritten_to.iter().any(|(b, v)| b == binder && *v != nv),
                                "two rewrites of one binder disagree"
                            );
                            slot.1 = nv.clone();
                        }
                        rewritten_to.push((binder.clone(), nv.clone()));
                    }
                    program.eval(expected, &moved, &mut st).map_err(at)?
                }
                Kind::Post(e) | Kind::Replace(e) => {
                    let Expr::Core(ir) = e;
                    env.push((RESULT, want.clone()));
                    program.eval(ir, &env, &mut st).map_err(at)?
                }
            };
            if faulted != want {
                found[i] = Some(ordinal);
                left -= 1;
            }
        }
        Ok(left > 0)
    })?;
    Ok(found)
}
