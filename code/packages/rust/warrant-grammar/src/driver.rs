//! WX-F binding-level sequence drivers: schema types and validation.
//!
//! A driven operation executes as a constructor/step/finish protocol over a
//! host object (VER04k section 3): the document stays pure (`op
//! digest_chunks(chunks: List<Bytes>) -> Bytes<20>`) while the binding says
//! how to run it (`Digest::new`, `update` per chunk, `sum1` to finish).
//!
//! Slice 1 (this module): parse `driver` records and validate their schema
//! (VER04k sections 3.2 and 3.4, rules 1-3 and 7). Every violation is W0501.
//! Slice 2 (this module): type driver records against the checked document
//! (VER04k 3.3 and 3.4 rule 5). Every inconsistency is W0711.
//! Slice 3 (this module): the fault set (VER04k 3.4 rule 4, 3.5, 3.6).
//! `faults=` must list every applicable driver fault (W0502) and none that
//! does not apply (W0701), and a driver needing a two-element exercise must
//! get one from some claim or example (W0601). Obligations and evaluation
//! follow in later slices; generation refuses driver ops explicitly until
//! then (W0704 at the evidence/codegen gates).

use crate::binding::{
    is_rust_identifier, is_rust_path, parse_host_errors, parse_host_ty, split_top, valid_host_ty,
    EvidenceBindingFile, HostErrors, HostTy, HostType, MAX_EXPORT_SEGMENTS,
};
use crate::checker::{CheckerError, CheckerErrorKind, DiagnosticCode};
use crate::codegen::{CodegenLimits, EvalValue, Evaluator};
use crate::evidence::{ty_of_type, Ty, TyEnv};
use crate::fixed::FixedTy;
use crate::parser::{Declaration, Expression, Literal, OpDecl, TypeExpr, Version, WarrantFile};

/// Longest Rust identifier accepted anywhere in a driver record (VER04k 3.4
/// rule 2): segments, method names and function names are all capped, because
/// every one is spliced into generated source.
pub const MAX_DRIVER_IDENT_BYTES: usize = 128;

/// Most `lengths=` hints on one driver (VER04k 3.4 rule 7).
pub const MAX_DRIVER_LENGTHS: usize = 32;

/// Largest single `lengths=` hint (VER04k 3.4 rule 7).
pub const MAX_DRIVER_LENGTH: u64 = 4096;

/// Deepest `elem.i.j...` component path (VER04k 3.2: nested at most 4 deep).
pub const MAX_ELEM_DEPTH: usize = 4;

/// Most steps one drive may run (VER04k 3.5): the adapter refuses before
/// calling `new` past this.
pub const MAX_DRIVER_STEPS: u64 = 65_536;

/// Receiver of a `step=` method: `mut` is `&mut self`, `own` is `self -> Self`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepReceiver {
    Mut,
    Own,
}

/// Receiver of a `finish=` method: `ref` is `&self`, `own` is `self`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReceiver {
    Ref,
    Own,
}

/// What a `step=` method returns: `unit`, a host `Result<(), E>`
/// (`result`), a `result<ht>` error union, or an emitted host value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepReturns {
    Unit,
    Result,
    ResultOf(HostTy),
    Value(HostTy),
}

/// Where one driver-call argument comes from: the driven element (`elem`,
/// optionally into tuple/record components), another parameter, or the
/// shared context value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgSource {
    Elem,
    ElemPath(Vec<usize>),
    Param(String),
    Ctx,
}

/// One argument of a `new`/`ctx`/`step` call: its source plus host type.
///
/// A bare `ctx` source has no host type at schema level (`None`); its type
/// is pinned when the record meets the document (later slice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverArg {
    pub from: ArgSource,
    pub host: Option<HostTy>,
}

/// A `new=NAME(args)` constructor call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverNew {
    pub func: String,
    pub args: Vec<DriverArg>,
}

/// A `ctx=PATH(args)[->result]` context-value call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverCtx {
    pub func: String,
    pub args: Vec<DriverArg>,
    pub returns_result: bool,
}

/// A `step=METHOD(receiver,args)[->returns]` call, run once per element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverStep {
    pub method: String,
    pub receiver: StepReceiver,
    pub args: Vec<DriverArg>,
    pub returns: StepReturns,
}

/// A `finish=METHOD(receiver)->ht` call, run once after the last step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverFinish {
    pub method: String,
    pub receiver: FinishReceiver,
    pub host: HostTy,
}

/// What the driven operation returns: the finish value, the collected step
/// values, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverResult {
    Finish,
    Collect,
    Both,
}

/// A fully parsed `driver NAME ...` record (VER04k 3.2 line form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverRecord {
    /// Host type whose associated function is `new`.
    pub state: String,
    pub ctx: Option<DriverCtx>,
    pub new: DriverNew,
    /// Driven parameter name (resolved against the document in a later slice).
    pub drive: String,
    pub step: DriverStep,
    pub finish: Option<DriverFinish>,
    pub result: DriverResult,
    pub errors: Option<HostErrors>,
    /// Block-boundary hints for `Bytes` elements.
    pub lengths: Option<Vec<u64>>,
}

/// Every method a driver record names (VER04k 3.4 rule 6): the context
/// function, constructor, step method and finish method, in that order.
/// Each counts as bound for the independent export analyzer; a method named
/// by no op and no driver is still unbound. That analyzer is not-live
/// (VER04 section 6), so nothing calls this yet — this is the one definition
/// of the set.
pub fn driver_named_methods(record: &DriverRecord) -> Vec<&str> {
    let mut out = Vec::new();
    if let Some(ctx) = &record.ctx {
        out.push(ctx.func.as_str());
    }
    out.push(record.new.func.as_str());
    out.push(record.step.method.as_str());
    if let Some(finish) = &record.finish {
        out.push(finish.method.as_str());
    }
    out
}

pub(crate) fn is_capped_ident(text: &str) -> bool {
    text.len() <= MAX_DRIVER_IDENT_BYTES && is_rust_identifier(text)
}

pub(crate) fn check_rust_path(text: &str, what: &str) -> Result<(), String> {
    if text.split("::").count() > MAX_EXPORT_SEGMENTS {
        return Err(format!(
            "{what} `{text}` has more than {MAX_EXPORT_SEGMENTS} `::` segments"
        ));
    }
    for seg in text.split("::") {
        if seg.len() > MAX_DRIVER_IDENT_BYTES || !is_rust_identifier(seg) {
            return Err(format!(
                "{what} `{text}` has a segment that is not a Rust identifier of at most {MAX_DRIVER_IDENT_BYTES} bytes"
            ));
        }
    }
    if !is_rust_path(text) {
        return Err(format!("{what} `{text}` is not a Rust path"));
    }
    Ok(())
}

fn parse_arg_source(text: &str) -> Result<ArgSource, String> {
    if text == "ctx" {
        return Ok(ArgSource::Ctx);
    }
    if text == "elem" {
        return Ok(ArgSource::Elem);
    }
    if let Some(rest) = text.strip_prefix("elem.") {
        if rest.is_empty() {
            return Err("`elem.` needs at least one index".to_string());
        }
        let mut path = Vec::new();
        for part in rest.split('.') {
            match part.parse::<usize>() {
                Ok(i) => path.push(i),
                Err(_) => {
                    return Err(format!(
                        "element path `{text}` has a non-numeric component `{part}`"
                    ))
                }
            }
        }
        if path.len() > MAX_ELEM_DEPTH {
            return Err(format!(
                "element path `{text}` nests deeper than {MAX_ELEM_DEPTH}"
            ));
        }
        return Ok(ArgSource::ElemPath(path));
    }
    if let Some(name) = text.strip_prefix("param.") {
        if !is_capped_ident(name) {
            return Err(format!(
                "`param.` source `{text}` is not an identifier of at most {MAX_DRIVER_IDENT_BYTES} bytes"
            ));
        }
        return Ok(ArgSource::Param(name.to_string()));
    }
    Err(format!(
        "argument source `{text}` is none of `ctx`, `elem`, `elem.i...` or `param.NAME`"
    ))
}

fn parse_driver_arg(text: &str) -> Result<DriverArg, String> {
    if text == "ctx" {
        return Ok(DriverArg {
            from: ArgSource::Ctx,
            host: None,
        });
    }
    let (source, host) = text
        .split_once(':')
        .ok_or_else(|| format!("argument `{text}` is neither `ctx` nor `source:host`"))?;
    if source.is_empty() || host.is_empty() {
        return Err(format!(
            "argument `{text}` is neither `ctx` nor `source:host`"
        ));
    }
    Ok(DriverArg {
        from: parse_arg_source(source)?,
        host: Some(parse_host_ty(host).map_err(|e| format!("argument `{text}`: {e}"))?),
    })
}

/// Split `NAME(inner,inner)[->trailing]` into its three parts.
fn split_call(text: &str, what: &str) -> Result<(String, String, Option<String>), String> {
    let (head, trailing) = match text.split_once("->") {
        Some((h, t)) => (h, Some(t)),
        None => (text, None),
    };
    if trailing.is_some_and(|t| t.contains("->")) {
        return Err(format!("{what} `{text}` has more than one `->`"));
    }
    let Some(open) = head.find('(') else {
        return Err(format!("{what} `{text}` is missing its `(`"));
    };
    if !head.ends_with(')') {
        return Err(format!("{what} `{text}` is missing its `)`"));
    }
    let name = head[..open].to_string();
    let inner = head[open + 1..head.len() - 1].to_string();
    Ok((name, inner, trailing.map(str::to_string)))
}

fn parse_call_args(inner: &str, what: &str) -> Result<Vec<DriverArg>, String> {
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    // split_top: host types hold commas inside `()` and `<>`.
    split_top(inner)
        .into_iter()
        .map(|a| parse_driver_arg(a.trim()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{what}: {e}"))
}

fn parse_driver_new(text: &str) -> Result<DriverNew, String> {
    let (name, inner, trailing) = split_call(text, "`new=`")?;
    if trailing.is_some() {
        return Err("`new=` takes no `->returns`".to_string());
    }
    if !is_capped_ident(&name) {
        return Err(format!("`new=` function `{name}` is not an identifier"));
    }
    Ok(DriverNew {
        func: name,
        args: parse_call_args(&inner, "`new=`")?,
    })
}

fn parse_step_returns(text: &str) -> Result<StepReturns, String> {
    match text {
        "unit" => Ok(StepReturns::Unit),
        "result" => Ok(StepReturns::Result),
        _ => match text
            .strip_prefix("result<")
            .and_then(|t| t.strip_suffix('>'))
        {
            Some(inner) => Ok(StepReturns::ResultOf(
                parse_host_ty(inner).map_err(|e| format!("`step=` returns `{text}`: {e}"))?,
            )),
            None => Ok(StepReturns::Value(
                parse_host_ty(text).map_err(|e| format!("`step=` returns `{text}`: {e}"))?,
            )),
        },
    }
}

fn parse_driver_step(text: &str) -> Result<DriverStep, String> {
    let (method, inner, trailing) = split_call(text, "`step=`")?;
    if !is_capped_ident(&method) {
        return Err(format!("`step=` method `{method}` is not an identifier"));
    }
    let mut parts = split_top(&inner).into_iter();
    let receiver = match parts.next().map(str::trim) {
        Some("mut") => StepReceiver::Mut,
        Some("own") => StepReceiver::Own,
        _ => {
            return Err(format!(
                "`step=` receiver must be `mut` or `own`, in `{text}`"
            ))
        }
    };
    let mut args = Vec::new();
    for part in parts {
        args.push(parse_driver_arg(part.trim()).map_err(|e| format!("`step=`: {e}"))?);
    }
    let returns = match trailing {
        None => StepReturns::Unit,
        Some(r) if r.is_empty() => return Err("`step=` has an empty `->returns`".to_string()),
        Some(r) => parse_step_returns(&r)?,
    };
    Ok(DriverStep {
        method,
        receiver,
        args,
        returns,
    })
}

fn parse_driver_finish(text: &str) -> Result<DriverFinish, String> {
    let (method, inner, trailing) = split_call(text, "`finish=`")?;
    if !is_capped_ident(&method) {
        return Err(format!("`finish=` method `{method}` is not an identifier"));
    }
    let receiver = match inner.trim() {
        "ref" => FinishReceiver::Ref,
        "own" => FinishReceiver::Own,
        _ => {
            return Err(format!(
                "`finish=` receiver must be `ref` or `own`, in `{text}`"
            ))
        }
    };
    let ret = trailing
        .filter(|t| !t.is_empty())
        .ok_or_else(|| format!("`finish=` needs `->hosttype`, in `{text}`"))?;
    Ok(DriverFinish {
        method,
        receiver,
        host: parse_host_ty(&ret).map_err(|e| format!("`finish=` returns `{ret}`: {e}"))?,
    })
}

fn parse_driver_ctx(text: &str) -> Result<DriverCtx, String> {
    let (func, inner, trailing) = split_call(text, "`ctx=`")?;
    check_rust_path(&func, "`ctx=` function")?;
    let returns_result = match trailing {
        None => false,
        Some(r) if r == "result" => true,
        Some(r) => {
            return Err(format!(
                "`ctx=` returns must be `result` or absent, in `{text}` (got `{r}`)"
            ))
        }
    };
    Ok(DriverCtx {
        func,
        args: parse_call_args(&inner, "`ctx=`")?,
        returns_result,
    })
}

fn parse_driver_result(text: &str) -> Result<DriverResult, String> {
    match text {
        "finish" => Ok(DriverResult::Finish),
        "collect" => Ok(DriverResult::Collect),
        "both" => Ok(DriverResult::Both),
        _ => Err(format!(
            "`result=` must be `finish`, `collect` or `both` (got `{text}`)"
        )),
    }
}

fn parse_driver_lengths(text: &str) -> Result<Vec<u64>, String> {
    // Strict decimal integers: no hex, no underscores, no empties.
    let mut out = Vec::new();
    for part in text.split(',') {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("`lengths=` entry `{part}` is not an integer"));
        }
        let value: u64 = part
            .parse()
            .map_err(|_| format!("`lengths=` entry `{part}` is out of range"))?;
        if value > MAX_DRIVER_LENGTH {
            return Err(format!(
                "`lengths=` entry `{value}` exceeds {MAX_DRIVER_LENGTH}"
            ));
        }
        out.push(value);
    }
    if out.len() > MAX_DRIVER_LENGTHS {
        return Err(format!(
            "`lengths=` has {} entries, at most {MAX_DRIVER_LENGTHS}",
            out.len()
        ));
    }
    if out.windows(2).any(|w| w[0] >= w[1]) {
        return Err("`lengths=` must be strictly increasing".to_string());
    }
    Ok(out)
}

/// Parse one `driver NAME k=v ...` line into a [`DriverRecord`].
///
/// `words` are the whitespace-separated `k=v` words after the name. Every
/// violation is an error the caller prefixes with `W0501` (VER04k 3.4).
pub fn parse_driver_record(words: &[&str]) -> Result<DriverRecord, String> {
    let mut seen = std::collections::BTreeSet::new();
    let (mut state, mut ctx, mut new, mut drive) = (None, None, None, None);
    let (mut step, mut finish, mut result, mut errors, mut lengths) =
        (None, None, None, None, None);
    for w in words {
        let (k, v) = w
            .split_once('=')
            .ok_or_else(|| format!("malformed driver option `{w}` (want `k=v`)"))?;
        if v.is_empty() {
            return Err(format!("driver option `{k}=` needs a value"));
        }
        if !seen.insert(k) {
            return Err(format!("driver option `{k}=` appears twice"));
        }
        match k {
            "state" => {
                check_rust_path(v, "`state=`")?;
                state = Some(v.to_string());
            }
            "ctx" => ctx = Some(parse_driver_ctx(v)?),
            "new" => new = Some(parse_driver_new(v)?),
            "drive" => {
                if !is_capped_ident(v) {
                    return Err(format!(
                        "`drive=` parameter `{v}` is not an identifier of at most {MAX_DRIVER_IDENT_BYTES} bytes"
                    ));
                }
                drive = Some(v.to_string());
            }
            "step" => step = Some(parse_driver_step(v)?),
            "finish" => finish = Some(parse_driver_finish(v)?),
            "result" => result = Some(parse_driver_result(v)?),
            "errors" => {
                let parsed =
                    parse_host_errors(v).map_err(|e| format!("`errors=`: {e}"))?;
                // The shared host-errors parser caps nothing; the error-enum
                // path is spliced into generated source later, so cap it here
                // rather than widening the shared parser's contract.
                check_rust_path(&parsed.path, "`errors=` path")?;
                errors = Some(parsed)
            }
            "lengths" => lengths = Some(parse_driver_lengths(v)?),
            _ => return Err(format!("unknown driver option `{k}=`")),
        }
    }
    let missing = |k: &str| format!("driver record needs `{k}=`");
    Ok(DriverRecord {
        state: state.ok_or_else(|| missing("state"))?,
        ctx,
        new: new.ok_or_else(|| missing("new"))?,
        drive: drive.ok_or_else(|| missing("drive"))?,
        step: step.ok_or_else(|| missing("step"))?,
        finish,
        result: result.ok_or_else(|| missing("result"))?,
        errors,
        lengths,
    })
}

// ---------------------------------------------------------------------------
// Slice 2: typing against the checked document (VER04k 3.3, 3.4 rule 5).
// ---------------------------------------------------------------------------

/// Most `errors=` map entries on one driver (VER04k 3.3 rule 4).
pub const MAX_DRIVER_ERROR_ENTRIES: usize = 64;

/// The `proofs_not_live=` reason a driven operation with proof obligations
/// must carry (VER04k 3.4 rule 5).
pub const DRIVER_NOT_LIVE_REASON: &str = "driver";

fn w0711(message: String, span: (usize, usize)) -> CheckerError {
    CheckerError {
        code: DiagnosticCode::W0711,
        kind: CheckerErrorKind::DriverInconsistency(message),
        span,
    }
}

/// Render a host type in binding-file syntax for diagnostics.
fn host_name(h: &HostTy) -> String {
    match h {
        HostTy::Int(s) => s.clone(),
        HostTy::Bool => "bool".to_string(),
        HostTy::Bytes => "bytes".to_string(),
        HostTy::BytesN(n) => format!("bytes<{n}>"),
        HostTy::Tuple(hs) => {
            let inner: Vec<String> = hs.iter().map(host_name).collect();
            format!("({})", inner.join(","))
        }
        HostTy::Opt(h) => format!("opt<{}>", host_name(h)),
        HostTy::List(h) => format!("list<{}>", host_name(h)),
        HostTy::Named(n) => n.clone(),
    }
}

/// Whether host type `h` can carry generator type `w` under the WX-A host
/// mapping: the exact predicate `to_host`/`from_host` implement, plus the
/// two v0 scalar leaves (drivers route scalars through host types where the
/// export path never does). Mirrored here so refusal precedes generation.
fn host_carries(w: &Ty, h: &HostTy) -> bool {
    match (w, h) {
        (Ty::Bool, HostTy::Bool) => true,
        (Ty::U8, HostTy::Int(rt)) if rt == "u8" => true,
        (Ty::U64, HostTy::Int(rt)) if rt == "u64" => true,
        (Ty::Fx(f) | Ty::FxEdge(f), HostTy::Int(rt)) if rt == f.suffix() => true,
        // A `Nat` bound to a host `usize`/`u32` declares its width; values
        // the host cannot hold are unreachable by type (VER04j 4.6).
        (Ty::Nat | Ty::NatCount, HostTy::Int(rt)) if rt.starts_with('u') => true,
        (Ty::Bytes | Ty::BytesN(_), HostTy::Bytes) => true,
        (Ty::BytesN(n), HostTy::BytesN(m)) if n == m => true,
        (Ty::List(t), HostTy::List(hh)) => host_carries(t, hh),
        (Ty::Opt(t), HostTy::Opt(hh)) => host_carries(t, hh),
        (Ty::Tuple(ts), HostTy::Tuple(hs)) if ts.len() == hs.len() => {
            ts.iter().zip(hs.iter()).all(|(t, h)| host_carries(t, h))
        }
        (Ty::Data { name, .. }, HostTy::Named(hn)) if hn == name => true,
        _ => false,
    }
}

/// The first `Named` host type in `h` with no `struct`/`enum` declaration,
/// if any (generation could not build its adapter).
fn undeclared_named<'h>(
    h: &'h HostTy,
    declared: &std::collections::BTreeMap<String, crate::binding::HostType>,
) -> Option<&'h str> {
    match h {
        HostTy::Named(n) if !declared.contains_key(n) => Some(n.as_str()),
        HostTy::Named(_) => None,
        HostTy::Tuple(hs) => hs.iter().find_map(|h| undeclared_named(h, declared)),
        HostTy::Opt(h) | HostTy::List(h) => undeclared_named(h, declared),
        _ => None,
    }
}

/// Re-validate every string a driver record splices into generated source.
/// The line parser checks these, but a caller may build a manifest by hand,
/// so generation checks again: anything else would be code injection.
pub(crate) fn validate_driver_record_strings(record: &DriverRecord) -> Result<(), String> {
    check_rust_path(&record.state, "`state=`")?;
    if let Some(ctx) = &record.ctx {
        check_rust_path(&ctx.func, "`ctx=` function")?;
    }
    for (what, name) in [
        ("`new=` function", record.new.func.as_str()),
        ("`step=` method", record.step.method.as_str()),
        ("`drive=` parameter", record.drive.as_str()),
    ] {
        if !is_capped_ident(name) {
            return Err(format!("{what} `{name}` is not an identifier"));
        }
    }
    if let Some(finish) = &record.finish {
        if !is_capped_ident(&finish.method) {
            return Err(format!(
                "`finish=` method `{}` is not an identifier",
                finish.method
            ));
        }
    }
    if let Some(errors) = &record.errors {
        check_rust_path(&errors.path, "`errors=` path")?;
        for (variant, case) in &errors.map {
            if !is_rust_identifier(variant) || !is_rust_identifier(case) {
                return Err(format!(
                    "`errors=` entry `{variant}={case}` must name identifiers"
                ));
            }
        }
    }
    for arg in record
        .new
        .args
        .iter()
        .chain(record.ctx.iter().flat_map(|c| c.args.iter()))
        .chain(record.step.args.iter())
    {
        if let ArgSource::Param(name) = &arg.from {
            if !is_capped_ident(name) {
                return Err(format!("`param.` source `{name}` is not an identifier"));
            }
        }
    }
    for h in record_host_types(record) {
        if !valid_host_ty(h) {
            return Err(format!(
                "host type `{}` could not have come from the bindings grammar",
                host_name(h)
            ));
        }
    }
    Ok(())
}

/// Every host type a record names: argument hosts, step returns, finish.
fn record_host_types(record: &DriverRecord) -> Vec<&HostTy> {
    let mut out = Vec::new();
    for arg in record
        .new
        .args
        .iter()
        .chain(record.ctx.iter().flat_map(|c| c.args.iter()))
        .chain(record.step.args.iter())
    {
        out.extend(arg.host.as_ref());
    }
    match &record.step.returns {
        StepReturns::Value(h) | StepReturns::ResultOf(h) => out.push(h),
        StepReturns::Unit | StepReturns::Result => {}
    }
    out.extend(record.finish.as_ref().map(|f| &f.host));
    out
}

/// Render a generator type for diagnostics (parameter sources keep their
/// written syntax; only leaves reached through paths need this).
fn ty_name(t: &Ty) -> String {
    match t {
        Ty::Bool => "Bool".to_string(),
        Ty::U8 => "U8".to_string(),
        Ty::U64 => "U64".to_string(),
        Ty::Nat | Ty::NatCount => "Nat".to_string(),
        Ty::Bytes => "Bytes".to_string(),
        Ty::BytesHinted(_) => "Bytes".to_string(),
        Ty::ListDriven => "List".to_string(),
        Ty::BytesN(n) => format!("Bytes<{n}>"),
        Ty::Fx(f) | Ty::FxEdge(f) => f.suffix().to_uppercase(),
        Ty::Tuple(ts) => {
            let inner: Vec<String> = ts.iter().map(ty_name).collect();
            format!("({})", inner.join(", "))
        }
        Ty::Data { name, .. } => name.clone(),
        Ty::Opt(t) => format!("Option<{}>", ty_name(t)),
        Ty::Res(t, _, _) => format!("Result<{}>", ty_name(t)),
        Ty::List(t) => format!("List<{}>", ty_name(t)),
    }
}

/// The element type of a driven shape: `Some` for lists (the element) and
/// fixed/byte strings (`U8` bytes), `None` for a `Nat` count driver (steps
/// with no element).
pub(crate) fn drive_elem_ty(driven_ty: &Ty) -> Option<Ty> {
    match driven_ty {
        Ty::List(t) => Some((**t).clone()),
        Ty::Bytes | Ty::BytesN(_) => Some(Ty::Fx(FixedTy::U8)),
        Ty::Nat | Ty::NatCount => None,
        _ => None,
    }
}

/// Resolve an `elem.i.j...` path through tuple components and record fields
/// (by index), returning the type chain from the element down to the leaf
/// (generation walks the same chain to build value accessors).
pub(crate) fn resolve_elem_chain(elem: &Ty, path: &[usize]) -> Result<Vec<Ty>, String> {
    // The parser caps paths at `MAX_ELEM_DEPTH`, but the record fields are
    // public, so a hand-built record may carry a longer one. Refuse it here
    // rather than walking it: the walk always ends in `Err`, but the cap
    // keeps a hostile length from buying resolution work.
    if path.len() > MAX_ELEM_DEPTH {
        return Err(format!(
            "element path nests {} deep, at most {MAX_ELEM_DEPTH}",
            path.len()
        ));
    }
    let mut chain = vec![elem.clone()];
    for (depth, index) in path.iter().enumerate() {
        let current = chain.last().expect("chain holds the element");
        let next = match current {
            Ty::Tuple(ts) => ts.get(*index).cloned().ok_or_else(|| {
                format!(
                    "element path reaches component {index} of a {}-tuple",
                    ts.len()
                )
            })?,
            Ty::Data { name, ctors, record } => {
                if !record {
                    return Err(format!(
                        "element path reaches `{name}`, which is a variant, not a record"
                    ));
                }
                let fields = ctors.first().map(|(_, fs)| fs).ok_or_else(|| {
                    format!("element path reaches record `{name}` with no constructor")
                })?;
                fields.get(*index).cloned().ok_or_else(|| {
                    format!(
                        "element path reaches field {index} of record `{name}` with {} fields",
                        fields.len()
                    )
                })?
            }
            other => {
                return Err(format!(
                    "element path descends into `{}` at depth {depth}, which has no components",
                    ty_name(other)
                ))
            }
        };
        chain.push(next);
    }
    Ok(chain)
}

/// Resolve an `elem.i.j...` path to its leaf type.
fn resolve_elem_path(elem: &Ty, path: &[usize]) -> Result<Ty, String> {
    resolve_elem_chain(elem, path).map(|mut chain| chain.pop().expect("chain holds the element"))
}

/// Convert a declared type through the generator's own resolver. A checked
/// document converts cleanly; anything else fails closed as W0711.
fn driver_ty(
    t: &TypeExpr<'_>,
    env: &TyEnv<'_>,
    op_span: (usize, usize),
    what: &str,
) -> Result<Ty, CheckerError> {
    ty_of_type(t, env, true, op_span, 0).map_err(|e| {
        w0711(
            format!("{what} has a type the generator cannot resolve: {e}"),
            op_span,
        )
    })
}

/// Validate one driver record against its operation (VER04k 3.3), reading
/// types through the generator's resolver so the verdict agrees with
/// generation by construction. Returns the driven type and the drive
/// parameter's index for the fault-set checks.
fn check_driver_against_op(
    op: &OpDecl<'_>,
    record: &DriverRecord,
    env: &TyEnv<'_>,
    declared: &std::collections::BTreeMap<String, HostType>,
) -> Result<(Ty, usize), CheckerError> {
    let op_name = op.name;
    let bad = |message: String| w0711(message, op.span);
    // Every `Named` host type must be declared by a `struct`/`enum` line:
    // generation builds its adapter from the declaration, so a phantom name
    // agrees with nothing.
    for h in record_host_types(record) {
        if let Some(name) = undeclared_named(h, declared) {
            return Err(bad(format!(
                "driver of `{op_name}` names host type `{name}`, which no `struct`/`enum` line declares"
            )));
        }
    }
    // Rule 5 (first half): no static parameters on a driven operation.
    if !op.static_params.is_empty() {
        return Err(bad(format!(
            "operation `{op_name}` has static parameters, which a driver cannot instantiate"
        )));
    }
    // An `own` step returns the state (`self -> Self`), so it cannot also
    // emit a value: there is only one return slot.
    if matches!(record.step.receiver, StepReceiver::Own)
        && matches!(
            record.step.returns,
            StepReturns::Value(_) | StepReturns::ResultOf(_)
        )
    {
        return Err(bad(format!(
            "step of `{op_name}` takes `own` but also emits a value; `own` returns only the state"
        )));
    }
    // Rule 1: the driven parameter and its element type (`None` is a `Nat`
    // count driver: steps with no element).
    let drive_index = op.params.iter().position(|p| p.name == record.drive);
    let Some(drive_index) = drive_index else {
        return Err(bad(format!(
            "driver drives `{}` but operation `{op_name}` has no such parameter",
            record.drive
        )));
    };
    let driven = &op.params[drive_index];
    let driven_ty = driver_ty(
        &driven.param_type,
        env,
        op.span,
        &format!("driven parameter `{}` of `{op_name}`", record.drive),
    )?;
    let elem_ty: Option<Ty> = match &driven_ty {
        Ty::List(_) | Ty::Bytes | Ty::BytesN(_) | Ty::Nat | Ty::NatCount => {
            drive_elem_ty(&driven_ty)
        }
        other => {
            return Err(bad(format!(
                "driver drives `{}` of type {}, which is neither `List<T>`, `Bytes` nor `Nat`",
                record.drive,
                ty_name(other)
            )))
        }
    };
    // Rule 2: every argument's source resolves and its host type carries it.
    let mut calls: Vec<(&str, &[DriverArg])> =
        vec![("new", &record.new.args), ("step", &record.step.args)];
    if let Some(ctx) = &record.ctx {
        calls.push(("ctx", &ctx.args));
    }
    let mut consumed: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (call, args) in calls {
        for (i, arg) in args.iter().enumerate() {
            let at = format!("{call} argument {i}");
            match &arg.from {
                ArgSource::Elem | ArgSource::ElemPath(_) => {
                    let Some(elem) = &elem_ty else {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes the driven element, but `{}` is a `Nat` count with no element",
                            record.drive
                        )));
                    };
                    let path = match &arg.from {
                        ArgSource::ElemPath(p) => p.as_slice(),
                        _ => &[],
                    };
                    let leaf = resolve_elem_path(elem, path)
                        .map_err(|e| bad(format!("`{at}` of `{op_name}`: {e}")))?;
                    let Some(host) = &arg.host else {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes the driven element but names no host type"
                        )));
                    };
                    if !host_carries(&leaf, host) {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` binds host `{}` to `{}`, which it cannot carry",
                            host_name(host),
                            ty_name(&leaf)
                        )));
                    }
                }
                ArgSource::Param(name) => {
                    let param = op.params.iter().find(|p| p.name == name);
                    let Some(param) = param else {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes `param.{name}`, which is not a parameter"
                        )));
                    };
                    consumed.insert(name.as_str());
                    let Some(host) = &arg.host else {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes `param.{name}` but names no host type"
                        )));
                    };
                    let param_ty = driver_ty(
                        &param.param_type,
                        env,
                        op.span,
                        &format!("`{at}` of `{op_name}`"),
                    )?;
                    if !host_carries(&param_ty, host) {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` binds host `{}` to `{}`, which it cannot carry",
                            host_name(host),
                            ty_name(&param_ty)
                        )));
                    }
                }
                ArgSource::Ctx => {
                    if record.ctx.is_none() {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes the context value, but the driver has no `ctx=`"
                        )));
                    }
                    if call == "ctx" {
                        return Err(bad(format!(
                            "`{at}` of `{op_name}` takes the context value to build the context"
                        )));
                    }
                    // The context value is a host-built opaque value with no
                    // static Warrant type, so no carry check applies.
                }
            }
        }
    }
    // Rule 3: every non-driven runtime parameter is consumed.
    for p in &op.params {
        if p.name != record.drive && !consumed.contains(p.name) {
            return Err(bad(format!(
                "parameter `{}` of `{op_name}` is consumed by neither `new`, `ctx` nor a step argument",
                p.name
            )));
        }
    }
    // Rule 4: the result mode against the return type.
    let fallible = matches!(
        record.step.returns,
        StepReturns::Result | StepReturns::ResultOf(_)
    ) || record.ctx.as_ref().is_some_and(|c| c.returns_result);
    let return_ty = driver_ty(
        &op.return_type,
        env,
        op.span,
        &format!("return type of `{op_name}`"),
    )?;
    let (mode_base, err_cases) = match &return_ty {
        Ty::Res(t, cases, _) => (&**t, Some(cases.as_slice())),
        t => (t, None),
    };
    if fallible && err_cases.is_none() {
        return Err(bad(format!(
            "driver of `{op_name}` is fallible but the operation does not return `Result`"
        )));
    }
    let emitted: Option<&HostTy> = match &record.step.returns {
        StepReturns::Value(h) | StepReturns::ResultOf(h) => Some(h),
        StepReturns::Unit | StepReturns::Result => None,
    };
    match record.result {
        DriverResult::Finish => {
            let Some(finish) = &record.finish else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `finish` but has no `finish=`"
                )));
            };
            if !host_carries(mode_base, &finish.host) {
                return Err(bad(format!(
                    "`finish` of `{op_name}` returns host `{}` which cannot carry `{}`",
                    host_name(&finish.host),
                    ty_name(mode_base)
                )));
            }
        }
        DriverResult::Collect => {
            let Some(h) = emitted else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `collect` but the step emits no value"
                )));
            };
            let Ty::List(u) = mode_base else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `collect` but the operation returns `{}`",
                    ty_name(mode_base)
                )));
            };
            if !host_carries(u, h) {
                return Err(bad(format!(
                    "`collect` of `{op_name}` gathers host `{}` which cannot carry `{}`",
                    host_name(h),
                    ty_name(u)
                )));
            }
        }
        DriverResult::Both => {
            let Some(finish) = &record.finish else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `both` but has no `finish=`"
                )));
            };
            let Some(h) = emitted else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `both` but the step emits no value"
                )));
            };
            let Ty::Tuple(ts) = mode_base else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `both` but the operation returns `{}`",
                    ty_name(mode_base)
                )));
            };
            let [l, f] = ts.as_slice() else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `both` but the operation returns a {}-tuple, not `(List<U>, F)`",
                    ts.len()
                )));
            };
            let Ty::List(u) = l else {
                return Err(bad(format!(
                    "driver of `{op_name}` returns `both` but the tuple's first component is `{}`, not a list",
                    ty_name(l)
                )));
            };
            if !host_carries(u, h) {
                return Err(bad(format!(
                    "`both` of `{op_name}` gathers host `{}` which cannot carry `{}`",
                    host_name(h),
                    ty_name(u)
                )));
            }
            if !host_carries(f, &finish.host) {
                return Err(bad(format!(
                    "`both` of `{op_name}` finishes with host `{}` which cannot carry `{}`",
                    host_name(&finish.host),
                    ty_name(f)
                )));
            }
        }
    }
    // Rule 4 (`errors=` map): every listed host variant maps to a case of E.
    if let Some(errors) = &record.errors {
        if errors.map.len() > MAX_DRIVER_ERROR_ENTRIES {
            return Err(bad(format!(
                "`errors=` of `{op_name}` maps {} variants, at most {MAX_DRIVER_ERROR_ENTRIES}",
                errors.map.len()
            )));
        }
        let Some(cases) = err_cases else {
            return Err(bad(format!(
                "`errors=` of `{op_name}` maps host errors but the operation does not return `Result`"
            )));
        };
        for (variant, case) in &errors.map {
            if !cases.iter().any(|c| c == case) {
                return Err(bad(format!(
                    "`errors=` of `{op_name}` maps host variant `{variant}` to `{case}`, which is not a case of the error type"
                )));
            }
        }
    }
    if fallible && record.errors.is_none() {
        return Err(bad(format!(
            "driver of `{op_name}` is fallible but has no `errors=` map"
        )));
    }
    // VER04k 3.4 rule 7 (typing half): `lengths=` only on `Bytes` elements.
    if record.lengths.is_some() {
        let driven_is_bytes = matches!(driven_ty, Ty::Bytes | Ty::BytesN(_));
        let elem_is_bytes = elem_ty
            .as_ref()
            .is_some_and(|t| matches!(t, Ty::Bytes));
        if !driven_is_bytes && !elem_is_bytes {
            return Err(bad(format!(
                "`lengths=` of `{op_name}` needs `Bytes` elements or a `Bytes` driven parameter"
            )));
        }
    }
    Ok((driven_ty, drive_index))
}

/// Whether operation `op` carries proof obligations a driver cannot lower:
/// a secret `labels` row or any `bound` row (VER04k 3.4 rule 5).
fn op_needs_driver_not_live(file: &WarrantFile<'_>, op: &str) -> bool {
    file.declarations.iter().any(|d| match d {
        Declaration::Labels(l) if l.name == op => l.rows.iter().any(|r| r.is_secret),
        Declaration::Bound(b) if b.name == op => !b.rows.is_empty(),
        _ => false,
    })
}

/// Validate every driver record against its operation (VER04k 3.3 rules 1-6
/// and 3.4 rule 5), reading types through the generator's own resolver so
/// the verdict agrees with generation by construction. The caller passes a
/// document that already checked: this runs beside
/// [`crate::binding::validate_bindings`]. Non-driver bindings are untouched,
/// and generation still refuses all driver ops (W0704) until the evaluation
/// slice lands.
pub fn validate_driver_consistency(
    file: &WarrantFile<'_>,
    bindings: &EvidenceBindingFile,
) -> Result<(), CheckerError> {
    let driven: Vec<&OpDecl<'_>> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Op(op)
                if bindings
                    .manifest
                    .bindings
                    .get(op.name)
                    .is_some_and(|b| b.driver.is_some()) =>
            {
                Some(op)
            }
            _ => None,
        })
        .collect();
    if driven.is_empty() {
        return Ok(());
    }
    // Rule 5 (second half): drivers need a v1 document.
    if file.version != Version::V1 {
        let op = driven[0];
        return Err(w0711(
            format!(
                "operation `{}` uses a driver, which needs a v1 document",
                op.name
            ),
            op.span,
        ));
    }
    let env = TyEnv::new(file);
    for op in driven {
        let Some(binding) = bindings.manifest.bindings.get(op.name) else {
            continue;
        };
        let Some(record) = binding.driver.as_ref() else {
            continue;
        };
        let (driven_ty, drive_index) =
            check_driver_against_op(op, record, &env, &bindings.hosts.types)?;
        check_driver_fault_set(op, record, &binding.negative_controls, &driven_ty)?;
        // VER04k 3.4 rule 5: obligations need the opt-in.
        if op_needs_driver_not_live(file, op.name) {
            let opted_in = bindings
                .proofs_not_live
                .get(op.name)
                .is_some_and(|r| r == DRIVER_NOT_LIVE_REASON);
            if !opted_in {
                return Err(w0711(
                    format!(
                        "operation `{}` has proof obligations a driver cannot lower; add `proofs_not_live=driver`",
                        op.name
                    ),
                    op.span,
                ));
            }
        }
        check_driver_exercised(file, op, drive_index, &driven_ty, record.result)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Slice 3: the fault set (VER04k 3.4 rule 4, 3.5, 3.6).
// ---------------------------------------------------------------------------

/// The six driver faults of VER04k 3.6, in table order.
pub const DRIVER_FAULTS: [&str; 6] = [
    "drop-last-chunk",
    "reorder-chunks",
    "duplicate-last-chunk",
    "early-finish",
    "reset-per-step",
    "drop-last-output",
];

/// The faults that need a two-element exercise to be catchable (VER04k 3.5).
pub(crate) const NEEDY_FAULTS: [&str; 3] = ["reorder-chunks", "early-finish", "reset-per-step"];

fn w_fault(code: DiagnosticCode, message: String, span: (usize, usize)) -> CheckerError {
    CheckerError {
        code,
        kind: CheckerErrorKind::DriverFaultSet(message),
        span,
    }
}

/// Whether the driven shape can hold two elements (VER04k 3.6 "applies
/// when"). A `Nat` count can drive two steps; whether reorder applies to it
/// is decided separately below.
fn shape_holds_two(driven_ty: &Ty) -> bool {
    match driven_ty {
        Ty::List(_) | Ty::Bytes | Ty::Nat | Ty::NatCount => true,
        Ty::BytesN(n) => *n >= 2,
        _ => false,
    }
}

/// Whether a driver fault applies to a driver of this shape and result mode
/// (VER04k 3.6 table).
fn driver_fault_applies(fault: &str, driven_ty: &Ty, result: DriverResult) -> bool {
    let multi = shape_holds_two(driven_ty);
    let collects = matches!(result, DriverResult::Collect | DriverResult::Both);
    let finishes = matches!(result, DriverResult::Finish | DriverResult::Both);
    match fault {
        "drop-last-chunk" | "duplicate-last-chunk" => true,
        // Reorder needs two distinguishable elements; `Nat` steps take no
        // element, so every step is identical and the rewrite is always the
        // identity (W0701 if listed, never required).
        "reorder-chunks" => multi && !matches!(driven_ty, Ty::Nat | Ty::NatCount),
        "early-finish" => multi,
        "reset-per-step" => multi && finishes,
        "drop-last-output" => collects,
        _ => false,
    }
}

/// Check the `faults=` set of one driven operation (VER04k 3.4 rule 4):
/// every applicable driver fault listed, none inapplicable one. Faults
/// outside the six driver faults are judged by the existing per-fault
/// machinery, not here.
fn check_driver_fault_set(
    op: &OpDecl<'_>,
    record: &DriverRecord,
    faults: &[String],
    driven_ty: &Ty,
) -> Result<(), CheckerError> {
    let bad = |code, message: String| w_fault(code, message, op.span);
    let mut missing = Vec::new();
    for fault in DRIVER_FAULTS {
        let listed = faults.iter().any(|f| f == fault);
        let applies = driver_fault_applies(fault, driven_ty, record.result);
        if !applies && listed {
            return Err(bad(
                DiagnosticCode::W0701,
                format!(
                    "`faults=` of `{}` lists `{fault}`, which does not apply to this driver",
                    op.name
                ),
            ));
        }
        if applies && !listed {
            missing.push(fault);
        }
    }
    if !missing.is_empty() {
        return Err(bad(
            DiagnosticCode::W0502,
            format!(
                "`faults=` of `{}` is missing driver fault{}: {}",
                op.name,
                if missing.len() == 1 { "" } else { "s" },
                missing.join(", ")
            ),
        ));
    }
    Ok(())
}

/// Whether element expression `e` of element type `t` can be non-empty.
/// Scalars, tuples, options and records have no empty value; only `Bytes`
/// and lists can be provably empty, and only literals prove it.
fn maybe_nonempty(e: &Expression<'_>, t: &Ty) -> bool {
    match e {
        Expression::Grouped { inner, .. } => maybe_nonempty(inner, t),
        Expression::Literal {
            lit: Literal::Bytes(s),
            ..
        } if matches!(t, Ty::Bytes | Ty::BytesN(_)) => !s.is_empty(),
        Expression::List { elements, .. } if matches!(t, Ty::List(_)) => !elements.is_empty(),
        _ => true,
    }
}

/// Whether a ground value drives a two-element exercise. A value of the
/// wrong shape is a mistyped argument the checker already refused; never
/// pile a W0601 onto it.
fn value_drives_big(v: &EvalValue, driven: &Ty) -> bool {
    match (driven, v) {
        (Ty::List(t), EvalValue::List(es)) => {
            es.len() >= 2 && es.iter().filter(|e| value_nonempty(e, t)).count() >= 2
        }
        (Ty::Bytes | Ty::BytesN(_), EvalValue::Bytes(b)) => b.len() >= 2,
        (Ty::Nat | Ty::NatCount, EvalValue::Nat(n)) => *n >= 2,
        (Ty::Nat | Ty::NatCount, EvalValue::Int(n)) => *n >= 2,
        (Ty::Nat | Ty::NatCount, EvalValue::U8(n)) => *n >= 2,
        (Ty::Nat | Ty::NatCount, EvalValue::U64(n)) => *n >= 2,
        (Ty::Nat | Ty::NatCount, EvalValue::Fixed(_, n)) => *n >= 2,
        _ => true,
    }
}

/// Whether a ground element value of element type `t` is non-empty.
fn value_nonempty(v: &EvalValue, t: &Ty) -> bool {
    match (t, v) {
        (Ty::Bytes | Ty::BytesN(_), EvalValue::Bytes(b)) => !b.is_empty(),
        (Ty::List(_), EvalValue::List(es)) => !es.is_empty(),
        _ => true,
    }
}

/// Whether a driven argument can drive a two-element exercise (VER04k 3.5).
/// Ground arguments (literals, literal arithmetic, `len` of literals,
/// nullary models over literals) are measured EXACTLY through the ground
/// evaluator, so `fill(5 - 5)` and `fill(z())` with `z() = 0` are caught.
/// Binder-dependent arguments fall back to the structural rules below, true
/// unless PROVABLY too small: a model over binders may still drive big
/// (VER04k 3.7's own claims do), so opacity counts as exercisable and the
/// runtime coverage control (evaluation slice) is the backstop for
/// semantically-small-but-syntactically-opaque drives.
fn drives_big(arg: &Expression<'_>, driven: &Ty, file: &WarrantFile<'_>) -> bool {
    let mut evaluator = Evaluator::new(file, CodegenLimits::default());
    if let Ok(value) = evaluator.eval(arg, &std::collections::HashMap::new()) {
        return value_drives_big(&value, driven);
    }
    match arg {
        Expression::Grouped { inner, .. } => drives_big(inner, driven, file),
        Expression::List { elements, .. } => match driven {
            Ty::List(t) => {
                elements.len() >= 2
                    && elements.iter().filter(|e| maybe_nonempty(e, t)).count() >= 2
            }
            // A list literal for a non-list driven type is a type error the
            // checker already refused; never pile a W0601 onto it.
            _ => true,
        },
        Expression::Literal { lit, .. } => match (driven, lit) {
            // Bytes literals are even-length hex; length is half of it.
            (Ty::Bytes | Ty::BytesN(_), Literal::Bytes(s)) => s.len() / 2 >= 2,
            (Ty::Nat | Ty::NatCount, Literal::U64(v)) => *v >= 2,
            (Ty::Nat | Ty::NatCount, Literal::Int(v)) => *v >= 2,
            (Ty::Nat | Ty::NatCount, Literal::Fixed(_, v)) => *v >= 2,
            // Anything else is a mistyped literal; the checker owns it.
            _ => true,
        },
        // A bare name or qualified path ranges over its domain (which holds
        // two for every type this check runs on), and a call is opaque:
        // both count as exercisable. Narrow fixed widths never reach here:
        // their needy set is empty, so the vacuity check is skipped.
        Expression::Reference { .. } => true,
        Expression::Conditional {
            then_branch,
            else_branch,
            ..
        } => drives_big(then_branch, driven, file) || drives_big(else_branch, driven, file),
        Expression::Match { arms, .. } => arms.iter().any(|a| drives_big(&a.body, driven, file)),
        // Binary, unary, tuple, projection: opaque or mistyped; the checker
        // owns mistyped, opacity counts as exercisable.
        _ => true,
    }
}

/// Whether expression `e` contains a call to `op` whose driven argument can
/// drive two elements. The recursion follows expression structure, which the
/// parser caps at 128 deep, so even an unchecked document cannot overflow it.
fn call_drives_big(
    e: &Expression<'_>,
    op: &str,
    drive_index: usize,
    driven_ty: &Ty,
    file: &WarrantFile<'_>,
) -> bool {
    match e {
        Expression::Reference { name, args, .. } => {
            let Some(args) = args else {
                return false;
            };
            if name.segments.as_slice() == [op] {
                if let Some(arg) = args.get(drive_index) {
                    if drives_big(arg, driven_ty, file) {
                        return true;
                    }
                }
            }
            // Not a big-driving call to this op; nested calls may still be.
            args.iter()
                .any(|a| call_drives_big(a, op, drive_index, driven_ty, file))
        }
        Expression::Grouped { inner, .. } => call_drives_big(inner, op, drive_index, driven_ty, file),
        Expression::Binary { left, right, .. } => {
            call_drives_big(left, op, drive_index, driven_ty, file)
                || call_drives_big(right, op, drive_index, driven_ty, file)
        }
        Expression::Unary { operand, .. } => call_drives_big(operand, op, drive_index, driven_ty, file),
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            call_drives_big(condition, op, drive_index, driven_ty, file)
                || call_drives_big(then_branch, op, drive_index, driven_ty, file)
                || call_drives_big(else_branch, op, drive_index, driven_ty, file)
        }
        Expression::Match { target, arms, .. } => {
            call_drives_big(target, op, drive_index, driven_ty, file)
                || arms
                    .iter()
                    .any(|a| call_drives_big(&a.body, op, drive_index, driven_ty, file))
        }
        Expression::Quantified { body, .. } => call_drives_big(body, op, drive_index, driven_ty, file),
        Expression::List { elements, .. } | Expression::Tuple { elements, .. } => elements
            .iter()
            .any(|a| call_drives_big(a, op, drive_index, driven_ty, file)),
        Expression::Project { base, .. } => call_drives_big(base, op, drive_index, driven_ty, file),
        Expression::Literal { .. } => false,
    }
}

/// Fail with W0601 unless some claim or example calls `op` with a
/// two-element exercise on the driven parameter (VER04k 3.5). Skipped when
/// no two-element fault applies: there is nothing to catch.
fn check_driver_exercised(
    file: &WarrantFile<'_>,
    op: &OpDecl<'_>,
    drive_index: usize,
    driven_ty: &Ty,
    result: DriverResult,
) -> Result<(), CheckerError> {
    let needy = NEEDY_FAULTS
        .iter()
        .any(|f| driver_fault_applies(f, driven_ty, result));
    if !needy {
        return Ok(());
    }
    let exercised = file.declarations.iter().any(|d| {
        let body = match d {
            Declaration::Claim(c) => &c.body,
            Declaration::Example(e) => &e.body,
            _ => return false,
        };
        call_drives_big(body, op.name, drive_index, driven_ty, file)
    });
    if exercised {
        Ok(())
    } else {
        Err(w_fault(
            DiagnosticCode::W0601,
            format!(
                "operation `{}` is never called with a two-element sequence (driver not exercised)",
                op.name
            ),
            op.span,
        ))
    }
}
