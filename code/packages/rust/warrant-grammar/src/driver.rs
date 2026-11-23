//! WX-F binding-level sequence drivers: schema types and validation.
//!
//! A driven operation executes as a constructor/step/finish protocol over a
//! host object (VER04k section 3): the document stays pure (`op
//! digest_chunks(chunks: List<Bytes>) -> Bytes<20>`) while the binding says
//! how to run it (`Digest::new`, `update` per chunk, `sum1` to finish).
//!
//! Slice 1 (this module): parse `driver` records and validate their schema
//! (VER04k sections 3.2 and 3.4, rules 1-3 and 7). Every violation is W0501.
//! Typing against the document (W0711), obligations, evaluation and sabotage
//! follow in later slices; generation refuses driver ops explicitly until
//! then (W0704 at the evidence/codegen gates).

use crate::binding::{
    is_rust_identifier, is_rust_path, parse_host_errors, parse_host_ty, split_top, HostErrors,
    HostTy, MAX_EXPORT_SEGMENTS,
};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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

fn is_capped_ident(text: &str) -> bool {
    text.len() <= MAX_DRIVER_IDENT_BYTES && is_rust_identifier(text)
}

fn check_rust_path(text: &str, what: &str) -> Result<(), String> {
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
