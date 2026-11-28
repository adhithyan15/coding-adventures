//! WX-F driver adapter and obligation generation (VER04k 3.5, 3.8).
//!
//! A driven operation executes as a constructor/step/finish protocol over a
//! host object. The generated `fn op_<name>` keeps the exact adapter shape
//! (`_w`, `args: &[V]`, `fault -> V`), so every suite calls driven
//! operations through it unchanged; only the body drives instead of calling
//! one export. Obligations are emitted as `driver`-kind tests beside the
//! suites, and the manifest gains a per-operation `driver` section.
//!
//! Conversion discipline (soundness-critical): every conversion may raise
//! `Unreachable` (a harness-level skip signal) or, for `BytesN`, a loud
//! subject-defect panic. Both must bypass `error=` normalization, which only
//! covers subject panics — so ALL conversions run outside `contained()`:
//! element arguments are converted fresh per step but collected as owned
//! values, parameter/context arguments once before the drive. Step arguments
//! convert with `top = false` (owned values: a retaining subject must never
//! observe a reference into a per-iteration temporary); one-shot arguments
//! convert with `top = true` like exports.

use std::fmt::Write as _;

use super::{
    from_host, to_host, ty_of_type, unsupported, EvidenceOptions, HostCx, Ty, TyEnv,
};
use crate::binding::{HostSpec, HostTy};
use crate::codegen::CodegenError;
use crate::driver::{
    drive_elem_ty, resolve_elem_chain, validate_driver_record_strings, ArgSource, DriverArg,
    DriverRecord, DriverResult, FinishReceiver, StepReceiver, StepReturns, MAX_DRIVER_STEPS,
};
use crate::parser::OpDecl;

/// Render a host type as the Rust type of a shape pin: scalars by value,
/// byte strings by shared reference, composites owned.
fn pin_rust_ty(h: &HostTy, hosts: &HostSpec, op_span: (usize, usize)) -> Result<String, CodegenError> {
    let bad = |m: String| unsupported(m, op_span);
    Ok(match h {
        HostTy::Int(s) => s.clone(),
        HostTy::Bool => "bool".to_string(),
        HostTy::Bytes => "&[u8]".to_string(),
        HostTy::BytesN(n) => format!("&[u8; {n}]"),
        HostTy::Tuple(hs) => {
            let mut parts = Vec::new();
            for h in hs {
                parts.push(pin_rust_ty(h, hosts, op_span)?);
            }
            format!("({})", parts.join(", "))
        }
        HostTy::Opt(h) => format!("Option<{}>", pin_rust_ty(h, hosts, op_span)?),
        HostTy::List(h) => format!("Vec<{}>", pin_rust_ty(h, hosts, op_span)?),
        HostTy::Named(n) => hosts
            .types
            .get(n)
            .map(|d| d.path.clone())
            .ok_or_else(|| bad(format!("host type `{n}` is not declared")))?,
    })
}

/// Whether an argument's host value is `Copy`: scalars and shared byte
/// references. Anything else must be cloned out of a hoisted vector.
fn host_is_copy(h: &HostTy) -> bool {
    match h {
        HostTy::Int(_) | HostTy::Bool | HostTy::Bytes | HostTy::BytesN(_) => true,
        HostTy::Tuple(hs) => hs.iter().all(host_is_copy),
        HostTy::Opt(_) | HostTy::List(_) | HostTy::Named(_) => false,
    }
}

/// Build the accessor suffix navigating an element path: tuple components
/// via `.tup()[i]`, record fields via `.fields()[i]`.
fn path_suffix(chain: &[Ty], path: &[usize]) -> String {
    let mut out = String::new();
    for (depth, index) in path.iter().enumerate() {
        match &chain[depth] {
            Ty::Tuple(_) => out.push_str(&format!(".tup()[{index}]")),
            _ => out.push_str(&format!(".fields()[{index}]")),
        }
    }
    out
}

/// Convert one driver argument to a Rust expression. `elem_base` is the
/// place of the current element: `elems[i]` per step, `elems[0]` for `new`
/// and `ctx` (which run once and take the first element), or the inline
/// `V::Fixed` byte for a bytes drive. Parameters read `args[j]`; the
/// context reads `&ctxv`. Returns the expression text.
fn convert_arg(
    arg: &DriverArg,
    op: &OpDecl<'_>,
    elem_ty: Option<&Ty>,
    param_tys: &[Ty],
    elem_base: &str,
    top: bool,
    hc: &mut HostCx<'_>,
) -> Result<String, CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", op.name), op.span);
    let elem_of = |what: &str| {
        elem_ty.cloned().ok_or_else(|| {
            bad(format!("{what} takes the driven element of a Nat count"))
        })
    };
    let (warrant_ty, place) = match &arg.from {
        ArgSource::Elem => (elem_of("argument")?, elem_base.to_string()),
        ArgSource::ElemPath(path) => {
            let elem = elem_of("argument")?;
            let chain = resolve_elem_chain(&elem, path)
                .map_err(|m| unsupported(format!("operation `{}`: {m}", op.name), op.span))?;
            let leaf = chain.last().expect("chain holds the element").clone();
            (leaf, format!("{elem_base}{}", path_suffix(&chain, path)))
        }
        ArgSource::Param(name) => {
            let (j, _) = op
                .params
                .iter()
                .enumerate()
                .find(|(_, p)| p.name == name)
                .ok_or_else(|| bad(format!("`param.{name}` is not a parameter")))?;
            (param_tys[j].clone(), format!("args[{j}]"))
        }
        ArgSource::Ctx => return Ok("&ctxv".to_string()),
    };
    let Some(host) = &arg.host else {
        return Err(bad("argument names no host type".to_string()));
    };
    to_host(&warrant_ty, host, &place, top, hc).map_err(bad)
}

/// The owned Rust spelling of a host type, for hoisted conversion vectors.
/// Byte strings own (`Vec<u8>`, `[u8; N]`); the call borrows them.
fn owned_rust_ty(h: &HostTy, hosts: &HostSpec, op_span: (usize, usize)) -> Result<String, CodegenError> {
    let bad = |m: String| unsupported(m, op_span);
    Ok(match h {
        HostTy::Int(s) => s.clone(),
        HostTy::Bool => "bool".to_string(),
        HostTy::Bytes => "Vec<u8>".to_string(),
        HostTy::BytesN(n) => format!("[u8; {n}]"),
        HostTy::Tuple(hs) => {
            let mut parts = Vec::new();
            for h in hs {
                parts.push(owned_rust_ty(h, hosts, op_span)?);
            }
            format!("({})", parts.join(", "))
        }
        HostTy::Opt(h) => format!("Option<{}>", owned_rust_ty(h, hosts, op_span)?),
        HostTy::List(h) => format!("Vec<{}>", owned_rust_ty(h, hosts, op_span)?),
        HostTy::Named(n) => hosts
            .types
            .get(n)
            .map(|d| d.path.clone())
            .ok_or_else(|| bad(format!("host type `{n}` is not declared")))?,
    })
}

/// Render one argument's use at a call site from a converted temporary:
/// byte strings by shared reference, `Copy` hosts directly, the rest cloned.
fn use_arg(temp: &str, host: &HostTy) -> String {
    match host {
        HostTy::Bytes | HostTy::BytesN(_) => format!("&{temp}"),
        h if host_is_copy(h) => temp.to_string(),
        _ => format!("{temp}.clone()"),
    }
}

/// Whether a host type contains a byte string anywhere inside it.
fn contains_bytes(h: &HostTy) -> bool {
    match h {
        HostTy::Bytes | HostTy::BytesN(_) => true,
        HostTy::Tuple(hs) => hs.iter().any(contains_bytes),
        HostTy::Opt(h) | HostTy::List(h) => contains_bytes(h),
        _ => false,
    }
}

/// How a return host type pins: an exact spelling, an `AsRef<[u8]>`-flexible
/// byte string, or unpinnable (nested byte combinations have no single
/// bound; the call site still checks existence).
enum RetPin {
    Exact(String),
    BytesFlex,
    Skip,
}

fn ret_pin(
    h: &HostTy,
    hosts: &HostSpec,
    op_span: (usize, usize),
) -> Result<RetPin, CodegenError> {
    if matches!(h, HostTy::Bytes) {
        return Ok(RetPin::BytesFlex);
    }
    if contains_bytes(h) {
        return Ok(RetPin::Skip);
    }
    Ok(RetPin::Exact(pin_rust_ty(h, hosts, op_span)?))
}

/// Emit the `shape` obligation pins: exact `const _` items where the whole
/// signature is nameable, `AsRef`-flexible checks where a byte string
/// returns, nothing where the context value flows (its type is opaque: the
/// adapter call below still checks existence).
fn shape_pins(
    record: &DriverRecord,
    hosts: &HostSpec,
    op_name: &str,
    op_span: (usize, usize),
) -> Result<String, CodegenError> {
    let bad = |m: String| unsupported(m, op_span);
    let mut pins = String::new();
    let uses_ctx = |args: &[DriverArg]| args.iter().any(|a| matches!(a.from, ArgSource::Ctx));
    let arg_types = |args: &[DriverArg]| -> Result<Vec<String>, CodegenError> {
        let mut out = Vec::new();
        for a in args {
            let h = a
                .host
                .as_ref()
                .ok_or_else(|| bad("argument names no host type".to_string()))?;
            out.push(pin_rust_ty(h, hosts, op_span)?);
        }
        Ok(out)
    };
    // `new`: `fn(args...) -> State`.
    if !uses_ctx(&record.new.args) {
        let args = arg_types(&record.new.args)?.join(", ");
        let _ = writeln!(
            pins,
            "const _: fn({args}) -> {} = {}::{};",
            record.state, record.state, record.new.func
        );
    }
    // `step`: receiver first, then the args; the return per `returns`.
    if !uses_ctx(&record.step.args) {
        let mut params = match record.step.receiver {
            StepReceiver::Mut => vec![format!("&mut {}", record.state)],
            StepReceiver::Own => vec![record.state.clone()],
        };
        params.extend(arg_types(&record.step.args)?);
        let sig = params.join(", ");
        let path = format!("{}::{}", record.state, record.step.method);
        let err_path = record.errors.as_ref().map(|e| e.path.clone());
        match &record.step.returns {
            StepReturns::Unit => {
                let _ = writeln!(pins, "const _: fn({sig}) = {path};");
            }
            StepReturns::Result => {
                if let Some(epath) = err_path {
                    let _ = writeln!(pins, "const _: fn({sig}) -> Result<(), {epath}> = {path};");
                }
            }
            StepReturns::Value(h) => match ret_pin(h, hosts, op_span)? {
                RetPin::Exact(r) => {
                    let _ = writeln!(pins, "const _: fn({sig}) -> {r} = {path};");
                }
                RetPin::BytesFlex => {
                    let _ = writeln!(pins, "#[allow(dead_code)]");
                    let _ = writeln!(pins, "fn _shape_step_{op_name}() {{");
                    let _ = writeln!(pins, "    fn assert_sig<F, B>(_: F)");
                    let _ = writeln!(pins, "    where");
                    let _ = writeln!(pins, "        F: FnOnce({sig}) -> B,");
                    let _ = writeln!(pins, "        B: AsRef<[u8]>,");
                    let _ = writeln!(pins, "    {{}}");
                    let _ = writeln!(pins, "    assert_sig({path});");
                    let _ = writeln!(pins, "}}");
                }
                RetPin::Skip => {}
            },
            StepReturns::ResultOf(h) => {
                if let Some(epath) = err_path {
                    match ret_pin(h, hosts, op_span)? {
                        RetPin::Exact(r) => {
                            let _ = writeln!(
                                pins,
                                "const _: fn({sig}) -> Result<{r}, {epath}> = {path};"
                            );
                        }
                        RetPin::BytesFlex => {
                            let _ = writeln!(pins, "#[allow(dead_code)]");
                            let _ = writeln!(pins, "fn _shape_step_{op_name}() {{");
                            let _ = writeln!(pins, "    fn assert_sig<F, B>(_: F)");
                            let _ = writeln!(pins, "    where");
                            let _ = writeln!(
                                pins,
                                "        F: FnOnce({sig}) -> Result<B, {epath}>,"
                            );
                            let _ = writeln!(pins, "        B: AsRef<[u8]>,");
                            let _ = writeln!(pins, "    {{}}");
                            let _ = writeln!(pins, "    assert_sig({path});");
                            let _ = writeln!(pins, "}}");
                        }
                        RetPin::Skip => {}
                    }
                }
            }
        }
    }
    // `finish`: no arguments beyond the receiver.
    if let Some(finish) = &record.finish {
        let recv = match finish.receiver {
            FinishReceiver::Ref => format!("&{}", record.state),
            FinishReceiver::Own => record.state.clone(),
        };
        let path = format!("{}::{}", record.state, finish.method);
        match ret_pin(&finish.host, hosts, op_span)? {
            RetPin::Exact(r) => {
                let _ = writeln!(pins, "const _: fn({recv}) -> {r} = {path};");
            }
            RetPin::BytesFlex => {
                let _ = writeln!(pins, "#[allow(dead_code)]");
                let _ = writeln!(pins, "fn _shape_finish_{op_name}() {{");
                let _ = writeln!(pins, "    fn assert_sig<F, B>(_: F)");
                let _ = writeln!(pins, "    where");
                let _ = writeln!(pins, "        F: FnOnce({recv}) -> B,");
                let _ = writeln!(pins, "        B: AsRef<[u8]>,");
                let _ = writeln!(pins, "    {{}}");
                let _ = writeln!(pins, "    assert_sig({path});");
                let _ = writeln!(pins, "}}");
            }
            RetPin::Skip => {}
        }
    }
    // `ctx` returns an opaque host value: unpinnable, checked at its call.
    Ok(pins)
}

/// Render the expression mapping a host error value to `V::Err`, mirroring
/// the export path: every listed variant, then a loud unmapped fallthrough.
fn errors_expr(err_place: &str, path: &str, map: &[(String, String)]) -> String {
    let mut arms = String::new();
    for (variant, case) in map {
        let _ = writeln!(
            arms,
            "{path}::{variant} {{ .. }} => V::Err(\"{case}\".into()),"
        );
    }
    format!(
        "match {err_place} {{ {arms} #[allow(unreachable_patterns)] _ => panic!(\"unmapped host error\"), }}"
    )
}

/// Generate the adapter for one driven operation (VER04k 3.5): the same
/// `fn op_<name>` shape every suite calls, with a constructor/step/finish
/// drive instead of one export call. `observed` wraps the drive in the
/// allocation window. Slice 4 accepts the six driver faults but plans them
/// as identity; the rewrites land in slice 5.
pub(crate) fn driver_adapter(
    op: &OpDecl<'_>,
    env: &TyEnv<'_>,
    record: &DriverRecord,
    err_case: Option<&str>,
    faults: &[String],
    options: &EvidenceOptions,
    observed: bool,
) -> Result<String, CodegenError> {
    let n = op.name;
    let bad = |m: String| unsupported(format!("operation `{n}`: {m}"), op.span);
    validate_driver_record_strings(record)
        .map_err(|m| unsupported(format!("operation `{n}`: {m}"), op.span))?;
    for f in faults {
        if !crate::driver::DRIVER_FAULTS.contains(&f.as_str()) && f != "missing-error" {
            return Err(bad(format!("fault `{f}` is not a driver-operation fault")));
        }
    }
    // Types, re-derived defensively: `generate_*` callers bypass validation.
    // Shared with the obligations through `DriveCx` so the two cannot diverge.
    let (cx, fallible_step) = drive_cx(op, env, record, &options.hosts, faults)?;
    let drive_index = cx.drive_index;
    // Result-mode coherence, mirroring typing.
    let collection = collect_info(&cx)?;
    if matches!(record.result, DriverResult::Finish | DriverResult::Both) && record.finish.is_none() {
        return Err(bad("result mode needs a `finish=`".to_string()));
    }
    let fallible = fallible_step || record.ctx.as_ref().is_some_and(|c| c.returns_result);
    if fallible && record.errors.is_none() {
        return Err(bad("fallible driver without `errors=` cannot map host errors".to_string()));
    }
    let mut hc = HostCx {
        op: n.to_string(),
        spec: &options.hosts,
        fields: &env.fields,
        ctr: 0,
    };
    let mut s = String::new();
    s.push_str(&shape_pins(record, &options.hosts, op.name, op.span)?);
    let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
    let _ = writeln!(s, "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{");
    let _ = writeln!(s, "    let fault = fault_for(\"{n}\", fault);");
    // Driven inputs, owned by this frame so per-step borrows never dangle.
    match &cx.driven_ty {
        Ty::List(_) => {
            let _ = writeln!(s, "    let elems: Vec<V> = match &args[{drive_index}] {{");
            let _ = writeln!(s, "        V::List(es) => es.clone(),");
            let _ = writeln!(s, "        _ => panic!(\"driver {n}: List drive is not a list\"),");
            let _ = writeln!(s, "    }};");
            let _ = writeln!(s, "    let peril: usize = elems.len();");
        }
        Ty::Bytes | Ty::BytesN(_) => {
            let _ = writeln!(s, "    let ebytes: Vec<u8> = match &args[{drive_index}] {{");
            let _ = writeln!(s, "        V::Bytes(b) => b.clone(),");
            let _ = writeln!(s, "        _ => panic!(\"driver {n}: Bytes drive is not bytes\"),");
            let _ = writeln!(s, "    }};");
            let _ = writeln!(s, "    let peril: usize = ebytes.len();");
        }
        _ => {
            let _ = writeln!(s, "    let peril: usize = match &args[{drive_index}] {{");
            let _ = writeln!(
                s,
                "        V::Int(v) => usize::try_from(*v).unwrap_or_else(|_| panic!(\"driver {n}: negative Nat drive\")),"
            );
            let _ = writeln!(s, "        _ => panic!(\"driver {n}: Nat drive is not an integer\"),");
            let _ = writeln!(s, "    }};");
        }
    }
    let _ = writeln!(
        s,
        "    if peril > {MAX_DRIVER_STEPS} {{ panic!(\"driver-steps-exceeded\"); }}"
    );
    let _ = writeln!(s, "    driver_note_steps(\"{n}\", peril);");
    // The drive plan (VER04k 3.5): identity unless one of this operation's
    // driver faults rewrites it (3.6). Arms cover only listed faults; anything
    // else (including other operations' driver faults) drives the full
    // sequence in order. Every order element is below `peril` by
    // construction, so indexing the hoisted conversions cannot panic.
    let mut arms = String::new();
    for f in faults {
        let v = match crate::evidence::fault_variant(f) {
            Some(v) => v,
            None => continue,
        };
        let arm = match f.as_str() {
            "drop-last-chunk" => {
                "((0..peril.saturating_sub(1)).collect(), false, false)".to_string()
            }
            "reorder-chunks" => "((0..peril).rev().collect(), false, false)".to_string(),
            "duplicate-last-chunk" => {
                "({ let mut o: Vec<usize> = (0..peril).collect(); if peril > 0 { o.push(peril - 1); } o }, false, false)".to_string()
            }
            "early-finish" => {
                "((if peril > 0 { vec![0usize] } else { Vec::new() }), false, false)".to_string()
            }
            "reset-per-step" => "((0..peril).collect(), true, false)".to_string(),
            "drop-last-output" => "((0..peril).collect(), false, true)".to_string(),
            _ => continue,
        };
        let _ = writeln!(arms, "        Fault::{v} => {arm},");
    }
    let _ = writeln!(
        s,
        "    let (order, reset_each, drop_output): (Vec<usize>, bool, bool) = match fault {{\n{arms}        _ => ((0..peril).collect(), false, false),\n    }};"
    );
    // Sabotage coverage: the negctl of a needy fault (3.5) requires a drive
    // of at least two elements to have executed, so a fault caught only on
    // smaller drives (or on nondeterministic constructor noise) fails its
    // control instead of passing vacuously.
    let _ = writeln!(s, "    driver_note_driven(\"{n}\", peril);");
    // `new`/`ctx` take the first element; guard the empty drive loudly.
    let takes_elem = |args: &[DriverArg]| {
        args.iter().any(|a| matches!(a.from, ArgSource::Elem | ArgSource::ElemPath(_)))
    };
    if takes_elem(&record.new.args)
        || record.ctx.as_ref().is_some_and(|c| takes_elem(&c.args))
    {
        let _ = writeln!(
            s,
            "    if peril == 0 {{ panic!(\"driver {n}: constructor takes the first element of an empty drive\"); }}"
        );
    }
    // One-shot conversions (outside containment, like exports).
    let step_base = match &cx.driven_ty {
        Ty::List(_) => "elems[step]".to_string(),
        Ty::Bytes | Ty::BytesN(_) => "V::Fixed(F::U8, ebytes[step] as i128)".to_string(),
        _ => String::new(),
    };
    let first_base = match &cx.driven_ty {
        Ty::List(_) => "elems[0]".to_string(),
        Ty::Bytes | Ty::BytesN(_) => "V::Fixed(F::U8, ebytes[0] as i128)".to_string(),
        _ => String::new(),
    };
    // Element-derived step arguments convert into hoisted vectors (owned:
    // a retaining subject must never observe a per-iteration temporary).
    // Invariant (parameter/context) step arguments convert once.
    let mut step_uses: Vec<String> = Vec::new();
    for (k, a) in record.step.args.iter().enumerate() {
        let h = a
            .host
            .as_ref()
            .ok_or_else(|| bad("argument names no host type".to_string()))?;
        if matches!(a.from, ArgSource::Elem | ArgSource::ElemPath(_)) {
            let elem = cx.elem_ty.as_ref().ok_or_else(|| {
                bad("step takes the driven element of a Nat count".to_string())
            })?;
            let vt = owned_rust_ty(h, &options.hosts, op.span)?;
            let _ = writeln!(s, "    let mut c{k}: Vec<{vt}> = Vec::with_capacity(peril);");
            let _ = writeln!(s, "    for step in 0..peril {{");
            let e = convert_arg(a, op, Some(elem), &cx.param_tys, &step_base, false, &mut hc)?;
            let _ = writeln!(s, "        c{k}.push({e});");
            let _ = writeln!(s, "    }}");
            step_uses.push(use_arg(&format!("c{k}[step]"), h));
        } else {
            let elem_opt = cx.elem_ty.as_ref();
            let e = convert_arg(a, op, elem_opt, &cx.param_tys, "", true, &mut hc)?;
            let _ = writeln!(s, "    let h{k} = {e};");
            if host_is_copy(h) {
                step_uses.push(format!("h{k}"));
            } else {
                step_uses.push(format!("h{k}.clone()"));
            }
        }
    }
    // Context value, then state: subject calls, converted arguments.
    let mut drive = String::new();
    if let Some(ctx) = &record.ctx {
        let ctx_args =
            emit_arg_conversions(&cx, &ctx.args, &first_base, "x", "    ", &mut hc, &mut s)?;
        let call = format!("{}({})", ctx.func, ctx_args.join(", "));
        if ctx.returns_result {
            let errors = record.errors.as_ref().ok_or_else(|| {
                bad("fallible ctx without `errors=` cannot map host errors".to_string())
            })?;
            let mapped = errors_expr("e", &errors.path, &errors.map);
            let _ = writeln!(
                drive,
                "let ctxv = match {call} {{ Ok(v) => v, Err(e) => return {mapped}, }};"
            );
        } else {
            let _ = writeln!(drive, "let ctxv = {call};");
        }
    }
    let new_args = emit_arg_conversions(&cx, &record.new.args, &first_base, "w", "    ", &mut hc, &mut s)?;
    let _ = writeln!(drive, "driver_note_call();");
    let _ = writeln!(
        drive,
        "let mut st = {}::{}({});",
        record.state,
        record.new.func,
        new_args.join(", ")
    );
    // The drive loop. Collected values convert inline like exports convert
    // inline: host-to-model conversion never raises `Unreachable`, so it is
    // safe inside containment.
    if collection.is_some() {
        let _ = writeln!(drive, "let mut collected: Vec<V> = Vec::new();");
    }
    let _ = writeln!(drive, "for step in order {{");
    let _ = writeln!(drive, "    driver_note_call();");
    if faults.iter().any(|f| f == "reset-per-step") {
        // `reset-per-step` (3.6): re-create the state before every step.
        // `ctx` is built once and kept; the constructor arguments are the
        // same converted values (still in scope).
        let _ = writeln!(drive, "    if reset_each {{");
        let _ = writeln!(drive, "        driver_note_call();");
        let _ = writeln!(
            drive,
            "        st = {}::{}({});",
            record.state,
            record.new.func,
            new_args.join(", ")
        );
        let _ = writeln!(drive, "    }}");
    }
    emit_step_call(
        &cx,
        &step_uses,
        collection.as_ref().map(|(u, h)| (u, h)),
        OnStepErr::ReturnMapped,
        "    ",
        &mut hc,
        &mut drive,
    )?;
    let _ = writeln!(drive, "}}");
    if collection.is_some() && faults.iter().any(|f| f == "drop-last-output") {
        // `drop-last-output` (3.6): the decoder loses its final symbol. The
        // pop runs even when collection is empty (`None`, harmless).
        let _ = writeln!(drive, "if drop_output {{ collected.pop(); }}");
    }
    // Finish and result shaping.
    match record.result {
        DriverResult::Finish => {
            let finish = record.finish.as_ref().ok_or_else(|| {
                bad("result mode needs a `finish=`".to_string())
            })?;
            let _ = writeln!(
                drive,
                "let f = st.{m}();",
                m = finish.method
            );
            let v = from_host(&cx.mode_base, &finish.host, "f", &mut hc)
                .map_err(|m| unsupported(format!("operation `{n}`: {m}"), op.span))?;
            if cx.is_result {
                let _ = writeln!(drive, "V::Ok(Box::new({v}))");
            } else {
                let _ = writeln!(drive, "{v}");
            }
        }
        DriverResult::Collect => {
            if cx.is_result {
                let _ = writeln!(drive, "V::Ok(Box::new(V::List(collected)))");
            } else {
                let _ = writeln!(drive, "V::List(collected)");
            }
        }
        DriverResult::Both => {
            let finish = record.finish.as_ref().ok_or_else(|| {
                bad("result mode needs a `finish=`".to_string())
            })?;
            let _ = writeln!(
                drive,
                "let f = st.{m}();",
                m = finish.method
            );
            let f_ty = match &cx.mode_base {
                Ty::Tuple(ts) if ts.len() == 2 => ts[1].clone(),
                _ => {
                    return Err(bad(
                        "`both` needs a `(List<U>, F)` return type".to_string(),
                    ))
                }
            };
            let v = from_host(&f_ty, &finish.host, "f", &mut hc).map_err(bad)?;
            if cx.is_result {
                let _ = writeln!(
                    drive,
                    "V::Ok(Box::new(V::Tup(vec![V::List(collected), {v}])))"
                );
            } else {
                let _ = writeln!(drive, "V::Tup(vec![V::List(collected), {v}])");
            }
        }
    }
    // Wrap the drive: allocation window inside panic normalization.
    let body = if observed {
        let mut w = String::new();
        for line in drive.lines() {
            let _ = writeln!(w, "    {line}");
        }
        format!("obs_scope(|| {{\n{w}}})")
    } else {
        let mut w = String::new();
        for line in drive.lines() {
            let _ = writeln!(w, "    {line}");
        }
        w.trim_end().to_string()
    };
    if let Some(case) = err_case {
        let _ = writeln!(s, "    let v: V = match contained(|| {{");
        for line in body.lines() {
            let _ = writeln!(s, "    {line}");
        }
        let _ = writeln!(s, "    }}) {{");
        let _ = writeln!(s, "        Some(r) => r,");
        let _ = writeln!(s, "        None => V::Err(\"{case}\".into()),");
        let _ = writeln!(s, "    }};");
        let _ = writeln!(s, "    v");
    } else {
        for line in body.lines() {
            let _ = writeln!(s, "    {line}");
        }
    }
    let _ = writeln!(s, "}}");
    let _ = writeln!(s);
    Ok(s)
}

/// Context for emitting a drive: everything the adapter and the
/// `finish-observational` obligation share, so the two cannot diverge.
struct DriveCx<'a> {
    op: &'a OpDecl<'a>,
    record: &'a DriverRecord,
    hosts: &'a HostSpec,
    drive_index: usize,
    driven_ty: Ty,
    elem_ty: Option<Ty>,
    param_tys: Vec<Ty>,
    mode_base: Ty,
    is_result: bool,
    missing: bool,
}

/// Build the drive context, re-deriving types defensively like the adapter.
fn drive_cx<'a>(
    op: &'a OpDecl<'a>,
    env: &'a TyEnv<'a>,
    record: &'a DriverRecord,
    hosts: &'a HostSpec,
    faults: &[String],
) -> Result<(DriveCx<'a>, bool), CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", op.name), op.span);
    let drive_index = op
        .params
        .iter()
        .position(|p| p.name == record.drive)
        .ok_or_else(|| bad(format!("driver drives `{}` with no such parameter", record.drive)))?;
    let param_tys: Vec<Ty> = op
        .params
        .iter()
        .map(|p| ty_of_type(&p.param_type, env, true, p.span, 0))
        .collect::<Result<_, _>>()?;
    let driven_ty = param_tys[drive_index].clone();
    match &driven_ty {
        Ty::List(_) | Ty::Bytes | Ty::BytesN(_) | Ty::Nat | Ty::NatCount => {}
        _ => {
            return Err(bad(format!(
                "driver drives `{}` of non-sequence type",
                record.drive
            )))
        }
    }
    let elem_ty = drive_elem_ty(&driven_ty);
    let return_ty = ty_of_type(&op.return_type, env, true, op.span, 0)?;
    let (mode_base, is_result) = match &return_ty {
        Ty::Res(t, _, _) => ((**t).clone(), true),
        t => (t.clone(), false),
    };
    let missing = faults.iter().any(|f| f == "missing-error");
    let fallible_step = matches!(
        record.step.returns,
        StepReturns::Result | StepReturns::ResultOf(_)
    );
    if missing && !fallible_step {
        return Err(bad(format!(
            "fault `missing-error` needs a fallible step, `{}` is total",
            op.name
        )));
    }
    Ok((
        DriveCx {
            op,
            record,
            hosts,
            drive_index,
            driven_ty,
            elem_ty,
            param_tys,
            mode_base,
            is_result,
            missing,
        },
        fallible_step,
    ))
}

/// Emit the statements converting one call's arguments: `prefix` names the
/// temporaries (`nw`, `xw`, `sw`), `elem_base` places the element. Returns
/// the use expressions in argument order.
fn emit_arg_conversions(
    cx: &DriveCx<'_>,
    args: &[DriverArg],
    elem_base: &str,
    prefix: &str,
    indent: &str,
    hc: &mut HostCx<'_>,
    out: &mut String,
) -> Result<Vec<String>, CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span);
    let mut uses = Vec::new();
    for (k, a) in args.iter().enumerate() {
        if matches!(a.from, ArgSource::Ctx) {
            // The context needs no temporary: it is already a binding.
            uses.push("&ctxv".to_string());
            continue;
        }
        let e = convert_arg(
            a,
            cx.op,
            cx.elem_ty.as_ref(),
            &cx.param_tys,
            elem_base,
            true,
            hc,
        )?;
        let _ = writeln!(out, "{indent}let {prefix}{k} = {e};");
        let h = a
            .host
            .as_ref()
            .ok_or_else(|| bad("argument names no host type".to_string()))?;
        if host_is_copy(h) {
            uses.push(format!("{prefix}{k}"));
        } else {
            uses.push(format!("{prefix}{k}.clone()"));
        }
    }
    Ok(uses)
}

/// What a fallible step does with a host error: the adapter returns the
/// mapped `V::Err`; an obligation skips the sequence (its function returns
/// a verdict, not a value).
#[derive(Clone, Copy, PartialEq, Eq)]
enum OnStepErr {
    ReturnMapped,
    SkipSeq,
}

/// The collect element type/host for collect/both modes, validating
/// result-mode coherence (mirrors typing for `generate_*` callers).
fn collect_info(cx: &DriveCx<'_>) -> Result<Option<(Ty, HostTy)>, CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span);
    let emitted: Option<&HostTy> = match &cx.record.step.returns {
        StepReturns::Value(h) | StepReturns::ResultOf(h) => Some(h),
        StepReturns::Unit | StepReturns::Result => None,
    };
    match cx.record.result {
        DriverResult::Finish => Ok(None),
        DriverResult::Collect | DriverResult::Both => {
            let u = match &cx.mode_base {
                Ty::List(u) if !matches!(cx.record.result, DriverResult::Both) => (**u).clone(),
                Ty::Tuple(ts)
                    if matches!(cx.record.result, DriverResult::Both) && ts.len() == 2 =>
                {
                    match &ts[0] {
                        Ty::List(u) => (**u).clone(),
                        _ => {
                            return Err(bad(
                                "`both` needs a `(List<U>, F)` return type".to_string(),
                            ))
                        }
                    }
                }
                _ => {
                    return Err(bad(
                        "result mode does not match the return type".to_string(),
                    ))
                }
            };
            let h = emitted.cloned().ok_or_else(|| {
                bad("collecting result mode needs a step that emits".to_string())
            })?;
            Ok(Some((u, h)))
        }
    }
}

/// Emit one step call with `uses` as the argument expressions, appending to
/// `out` at `indent`. When `collect` holds the element type/host and the
/// step emits, converts the value and pushes it onto `collected`.
fn emit_step_call(
    cx: &DriveCx<'_>,
    uses: &[String],
    collect: Option<(&Ty, &HostTy)>,
    on_err: OnStepErr,
    indent: &str,
    hc: &mut HostCx<'_>,
    out: &mut String,
) -> Result<(), CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span);
    let m = &cx.record.step.method;
    let args = uses.join(", ");
    let missing_arm = |out: &mut String, action: &str| {
        if cx.missing {
            let _ = writeln!(out, "{indent}    Err(_) if fault == Fault::MissingError => {action},");
        }
    };
    let errors_mapped = || -> Result<String, CodegenError> {
        let errors = cx
            .record
            .errors
            .as_ref()
            .ok_or_else(|| bad("fallible step without `errors=`".to_string()))?;
        Ok(errors_expr("e", &errors.path, &errors.map))
    };
    let err_arm = |out: &mut String| -> Result<(), CodegenError> {
        match on_err {
            OnStepErr::ReturnMapped => {
                let mapped = errors_mapped()?;
                let _ = writeln!(out, "{indent}    Err(e) => return {mapped},");
            }
            OnStepErr::SkipSeq => {
                let _ = writeln!(out, "{indent}    Err(_) => continue 'seq,");
            }
        }
        Ok(())
    };
    match (&cx.record.step.receiver, &cx.record.step.returns) {
        (StepReceiver::Mut, StepReturns::Unit) => {
            let _ = writeln!(out, "{indent}st.{m}({args});");
            Ok(())
        }
        (StepReceiver::Own, StepReturns::Unit) => {
            let _ = writeln!(out, "{indent}st = st.{m}({args});");
            Ok(())
        }
        (StepReceiver::Mut, StepReturns::Result) => {
            let _ = writeln!(out, "{indent}match st.{m}({args}) {{");
            let _ = writeln!(out, "{indent}    Ok(()) => {{}},");
            missing_arm(out, "{}");
            err_arm(out)?;
            let _ = writeln!(out, "{indent}}}");
            Ok(())
        }
        (StepReceiver::Own, StepReturns::Result) => {
            let _ = writeln!(out, "{indent}st = match st.{m}({args}) {{");
            let _ = writeln!(out, "{indent}    Ok(st) => st,");
            missing_arm(out, "st");
            err_arm(out)?;
            let _ = writeln!(out, "{indent}}}");
            Ok(())
        }
        (StepReceiver::Mut, StepReturns::Value(_)) => {
            if let Some((u, h)) = collect {
                let v = from_host(u, h, "sv", hc)
                    .map_err(|m| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span))?;
                let _ = writeln!(out, "{indent}let sv = st.{m}({args});");
                let _ = writeln!(out, "{indent}collected.push({v});");
            } else {
                let _ = writeln!(out, "{indent}st.{m}({args});");
            }
            Ok(())
        }
        (StepReceiver::Mut, StepReturns::ResultOf(_)) => {
            if let Some((u, h)) = collect {
                let v = from_host(u, h, "sv", hc)
                    .map_err(|m| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span))?;
                let _ = writeln!(out, "{indent}let sv = match st.{m}({args}) {{");
                let _ = writeln!(out, "{indent}    Ok(v) => v,");
                missing_arm(out, "continue");
                err_arm(out)?;
                let _ = writeln!(out, "{indent}}};");
                let _ = writeln!(out, "{indent}collected.push({v});");
            } else {
                let _ = writeln!(out, "{indent}match st.{m}({args}) {{");
                let _ = writeln!(out, "{indent}    Ok(_) => {{}},");
                missing_arm(out, "{}");
                err_arm(out)?;
                let _ = writeln!(out, "{indent}}}");
            }
            Ok(())
        }
        (StepReceiver::Own, StepReturns::Value(_) | StepReturns::ResultOf(_)) => Err(bad(
            "`own` step cannot emit a value".to_string(),
        )),
    }
}

/// Render a minimal `V` expression for a type: empty collections, zero
/// scalars. Total: every generator type has a minimal value.
fn minimal_v(t: &Ty) -> String {
    match t {
        Ty::Bool => "V::Bool(false)".to_string(),
        Ty::U8 | Ty::U64 => "V::Fixed(F::U8, 0)".to_string(),
        Ty::Fx(f) | Ty::FxEdge(f) => format!("V::Fixed(F::{}, 0)", f.name()),
        Ty::Nat | Ty::NatCount => "V::Int(0)".to_string(),
        Ty::Bytes | Ty::BytesN(_) | Ty::BytesHinted(_) => "V::Bytes(vec![])".to_string(),
        Ty::Tuple(cs) => {
            let items: Vec<String> = cs.iter().map(minimal_v).collect();
            format!("V::Tup(vec![{}])", items.join(", "))
        }
        Ty::Data { name, ctors, .. } => {
            let (ctor, fields) = ctors
                .first()
                .map(|(c, fs)| (c.as_str(), fs.as_slice()))
                .unwrap_or(("", &[]));
            let items: Vec<String> = fields.iter().map(minimal_v).collect();
            format!(
                "V::Con(\"{name}\", 0, \"{ctor}\", vec![{}])",
                items.join(", ")
            )
        }
        Ty::Opt(_) => "V::Opt(None)".to_string(),
        Ty::Res(t, _, _) => format!("V::Ok(Box::new({}))", minimal_v(t)),
        Ty::List(_) | Ty::ListDriven => "V::List(vec![])".to_string(),
    }
}

/// Render a non-empty `V` expression: `minimal_v`, except byte strings and
/// lists, which get one minimal element.
fn nonempty_v(t: &Ty) -> String {
    match t {
        Ty::Bytes | Ty::BytesN(_) | Ty::BytesHinted(_) => "V::Bytes(vec![0xa5])".to_string(),
        Ty::List(u) => format!("V::List(vec![{}])", minimal_v(u)),
        Ty::ListDriven => format!(
            "V::List(vec![{}])",
            minimal_v(&Ty::Bytes)
        ),
        _ => minimal_v(t),
    }
}

/// Emit a state-only drive of `driven_expr` into `out` at `indent`: input
/// extraction, conversions, context, constructor and loop, leaving the
/// state in `st`. Shared by the `finish-observational` main check and
/// control so both drive identically; the adapter's drive uses the same
/// conversion and call emitters. Must be emitted inside a `'seq` loop:
/// aborted sequences skip to the next one.
fn emit_state_drive(
    cx: &DriveCx<'_>,
    driven_expr: &str,
    indent: &str,
    hc: &mut HostCx<'_>,
    out: &mut String,
) -> Result<(), CodegenError> {
    let bad = |m: String| unsupported(format!("operation `{}`: {m}", cx.op.name), cx.op.span);
    let n = cx.op.name;
    match &cx.driven_ty {
        Ty::List(_) => {
            let _ = writeln!(
                out,
                "{indent}let elems: Vec<V> = match &({driven_expr}) {{"
            );
            let _ = writeln!(out, "{indent}    V::List(es) => es.clone(),");
            let _ = writeln!(
                out,
                "{indent}    _ => panic!(\"driver {n}: List drive is not a list\"),"
            );
            let _ = writeln!(out, "{indent}}};");
            let _ = writeln!(out, "{indent}let peril: usize = elems.len();");
        }
        Ty::Bytes | Ty::BytesN(_) => {
            let _ = writeln!(
                out,
                "{indent}let ebytes: Vec<u8> = match &({driven_expr}) {{"
            );
            let _ = writeln!(out, "{indent}    V::Bytes(b) => b.clone(),");
            let _ = writeln!(out, "{indent}    _ => panic!(\"driver {n}: Bytes drive is not bytes\"),");
            let _ = writeln!(out, "{indent}}};");
            let _ = writeln!(out, "{indent}let peril: usize = ebytes.len();");
        }
        _ => {
            let _ = writeln!(
                out,
                "{indent}let peril: usize = match &({driven_expr}) {{"
            );
            let _ = writeln!(
                out,
                "{indent}    V::Int(v) => usize::try_from(*v).unwrap_or_else(|_| panic!(\"driver {n}: negative Nat drive\")),"
            );
            let _ = writeln!(
                out,
                "{indent}    _ => panic!(\"driver {n}: Nat drive is not an integer\"),"
            );
            let _ = writeln!(out, "{indent}}};");
        }
    }
    let _ = writeln!(out, "{indent}driver_note_steps(\"{n}\", peril);");
    let step_base = match &cx.driven_ty {
        Ty::List(_) => "elems[step]".to_string(),
        Ty::Bytes | Ty::BytesN(_) => "V::Fixed(F::U8, ebytes[step] as i128)".to_string(),
        _ => String::new(),
    };
    let first_base = match &cx.driven_ty {
        Ty::List(_) => "elems[0]".to_string(),
        Ty::Bytes | Ty::BytesN(_) => "V::Fixed(F::U8, ebytes[0] as i128)".to_string(),
        _ => String::new(),
    };
    // Element-derived step arguments convert into hoisted owned vectors.
    let mut step_uses: Vec<String> = Vec::new();
    for (k, a) in cx.record.step.args.iter().enumerate() {
        let h = a
            .host
            .as_ref()
            .ok_or_else(|| bad("argument names no host type".to_string()))?;
        if matches!(a.from, ArgSource::Elem | ArgSource::ElemPath(_)) {
            let elem = cx.elem_ty.as_ref().ok_or_else(|| {
                bad("step takes the driven element of a Nat count".to_string())
            })?;
            let vt = owned_rust_ty(h, cx.hosts, cx.op.span)?;
            let _ = writeln!(
                out,
                "{indent}let mut c{k}: Vec<{vt}> = Vec::with_capacity(peril);"
            );
            let _ = writeln!(out, "{indent}for step in 0..peril {{");
            let e = convert_arg(a, cx.op, Some(elem), &cx.param_tys, &step_base, false, hc)?;
            let _ = writeln!(out, "{indent}    c{k}.push({e});");
            let _ = writeln!(out, "{indent}}}");
            step_uses.push(use_arg(&format!("c{k}[step]"), h));
        } else {
            let e = convert_arg(
                a,
                cx.op,
                cx.elem_ty.as_ref(),
                &cx.param_tys,
                "",
                true,
                hc,
            )?;
            let _ = writeln!(out, "{indent}let h{k} = {e};");
            if host_is_copy(h) {
                step_uses.push(format!("h{k}"));
            } else {
                step_uses.push(format!("h{k}.clone()"));
            }
        }
    }
    if let Some(ctx) = &cx.record.ctx {
        let ctx_args = emit_arg_conversions(cx, &ctx.args, &first_base, "x", indent, hc, out)?;
        let call = format!("{}({})", ctx.func, ctx_args.join(", "));
        if ctx.returns_result {
            let _ = writeln!(
                out,
                "{indent}let ctxv = match {call} {{ Ok(v) => v, Err(_) => continue 'seq, }};"
            );
        } else {
            let _ = writeln!(out, "{indent}let ctxv = {call};");
        }
    }
    let new_args = emit_arg_conversions(cx, &cx.record.new.args, &first_base, "w", indent, hc, out)?;
    let _ = writeln!(out, "{indent}driver_note_call();");
    let _ = writeln!(
        out,
        "{indent}let mut st = {}::{}({});",
        cx.record.state,
        cx.record.new.func,
        new_args.join(", ")
    );
    let collection = collect_info(cx)?;
    if collection.is_some() {
        let _ = writeln!(out, "{indent}let mut collected: Vec<V> = Vec::new();");
    }
    let _ = writeln!(out, "{indent}for step in 0..peril {{");
    let _ = writeln!(out, "{indent}    driver_note_call();");
    emit_step_call(
        cx,
        &step_uses,
        collection.as_ref().map(|(u, h)| (u, h)),
        OnStepErr::SkipSeq,
        &format!("{indent}    "),
        hc,
        out,
    )?;
    let _ = writeln!(out, "{indent}}}");
    Ok(())
}

/// The obligations of one driven operation (VER04k 3.5): `shape` and `empty`
/// always; `empty-element` for list-of-bytes/list-of-lists drives;
/// `deterministic` always; `finish-observational` for `ref` finishes;
/// `steps-cap` always.
pub(crate) fn driver_obligations(record: &DriverRecord, driven_ty: &Ty) -> Vec<&'static str> {
    let mut out = vec!["shape", "empty"];
    if matches!(driven_ty, Ty::List(t) if matches!(**t, Ty::Bytes | Ty::BytesN(_) | Ty::List(_))) {
        out.push("empty-element");
    }
    out.push("deterministic");
    if record
        .finish
        .as_ref()
        .is_some_and(|f| matches!(f.receiver, FinishReceiver::Ref))
    {
        out.push("finish-observational");
    }
    out.push("steps-cap");
    out
}

/// Render the argument template all obligation drives share: every parameter
/// filled, the driven slot holding a placeholder the drive overwrites.
fn template_expr(cx: &DriveCx<'_>) -> String {
    let items: Vec<String> = cx
        .param_tys
        .iter()
        .enumerate()
        .map(|(j, t)| {
            if j == cx.drive_index {
                "V::Bool(false)".to_string()
            } else {
                format!("domain_take({}, 1).into_iter().next().expect(\"parameter domain is inhabited\")", t.rust())
            }
        })
        .collect();
    format!("vec![{}]", items.join(", "))
}

/// Emit the `obl_<op>_<name>` test functions for one driven operation,
/// returning them with the obligation names for suite registration.
pub(crate) fn obligation_fns(
    op: &OpDecl<'_>,
    env: &TyEnv<'_>,
    record: &DriverRecord,
    faults: &[String],
    options: &EvidenceOptions,
) -> Result<(String, Vec<&'static str>), CodegenError> {
    let n = op.name;
    let bad = |m: String| unsupported(format!("operation `{n}`: {m}"), op.span);
    validate_driver_record_strings(record)
        .map_err(|m| unsupported(format!("operation `{n}`: {m}"), op.span))?;
    let (cx, _) = drive_cx(op, env, record, &options.hosts, faults)?;
    let mut hc = HostCx {
        op: n.to_string(),
        spec: &options.hosts,
        fields: &env.fields,
        ctr: 0,
    };
    let obligations = driver_obligations(record, &cx.driven_ty);
    let template = template_expr(&cx);
    let idx = cx.drive_index;
    // A constructor taking the first element cannot run on an empty drive;
    // obligations skip those sequences, and claims exclude them with
    // refinements the same way.
    let takes_elem = |args: &[DriverArg]| {
        args.iter()
            .any(|a| matches!(a.from, ArgSource::Elem | ArgSource::ElemPath(_)))
    };
    let ctor_takes_elem = takes_elem(&record.new.args)
        || record.ctx.as_ref().is_some_and(|c| takes_elem(&c.args));
    let mut s = String::new();
    for ob in &obligations {
        match *ob {
            "shape" => {
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_shape(_fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(s, "    (Verdict::Pass, Stats::default())");
                let _ = writeln!(s, "}}");
            }
            "empty" => {
                let empty = match &cx.driven_ty {
                    Ty::List(_) => "V::List(vec![])".to_string(),
                    Ty::Bytes | Ty::BytesN(_) => "V::Bytes(vec![])".to_string(),
                    _ => "V::Int(0)".to_string(),
                };
                // A constructor taking the first element cannot run on an
                // empty drive; the adapter refuses loudly, and the
                // obligation expects exactly that refusal (mirroring how
                // `steps-cap` expects its own).
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_empty(fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(s, "    let mut args = {template};");
                let _ = writeln!(s, "    args[{idx}] = {empty};");
                if ctor_takes_elem {
                    let _ = writeln!(
                        s,
                        "    let r = catch_unwind(AssertUnwindSafe(|| op_{n}(&[], &args, fault)));"
                    );
                    let _ = writeln!(s, "    match r {{");
                    let _ = writeln!(s, "        Err(p) => {{");
                    let _ = writeln!(
                        s,
                        "            let msg = p.downcast_ref::<String>().cloned().unwrap_or_default() + p.downcast_ref::<&str>().unwrap_or(&\"\");"
                    );
                    let _ = writeln!(
                        s,
                        "            if !msg.contains(\"takes the first element of an empty drive\") {{"
                    );
                    let _ = writeln!(s, "                let mut s = Stats::default();");
                    let _ = writeln!(s, "                s.cases = 1;");
                    let _ = writeln!(
                        s,
                        "                return (Verdict::fail(\"driver-wrong-refusal\", \"empty drive refused wrongly\"), s);"
                    );
                    let _ = writeln!(s, "            }}");
                    let _ = writeln!(s, "        }}");
                    let _ = writeln!(s, "        Ok(_) => {{");
                    let _ = writeln!(s, "            let mut s = Stats::default();");
                    let _ = writeln!(s, "            s.cases = 1;");
                    let _ = writeln!(
                        s,
                        "            return (Verdict::fail(\"driver-no-refusal\", \"empty drive with element constructor proceeded\"), s);"
                    );
                    let _ = writeln!(s, "        }}");
                    let _ = writeln!(s, "    }}");
                } else {
                    let _ = writeln!(s, "    let _ = op_{n}(&[], &args, fault);");
                }
                let _ = writeln!(s, "    let mut s = Stats::default();");
                let _ = writeln!(s, "    s.cases = 1;");
                let _ = writeln!(s, "    (Verdict::Pass, s)");
                let _ = writeln!(s, "}}");
            }
            "empty-element" => {
                let elem = match &cx.driven_ty {
                    Ty::List(t) => t.as_ref().clone(),
                    _ => Ty::Bytes,
                };
                let (a0, a1) = match &elem {
                    Ty::List(u) => (
                        "V::List(vec![])".to_string(),
                        format!(
                            "V::List(vec![domain_take({}, 1).into_iter().next().expect(\"element domain is inhabited\")])",
                            u.rust()
                        ),
                    ),
                    _ => (
                        "V::Bytes(vec![])".to_string(),
                        "V::Bytes(vec![0x00])".to_string(),
                    ),
                };
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_empty_element(fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(s, "    let alphabet: Vec<V> = vec![{a0}, {a1}];");
                let _ = writeln!(s, "    let template: Vec<V> = {template};");
                let _ = writeln!(s, "    let mut cases = 0u64;");
                let _ = writeln!(
                    s,
                    "    let drive = |elems: Vec<V>| {{ let mut args = template.clone(); args[{idx}] = V::List(elems); op_{n}(&[], &args, fault) }};"
                );
                for len in 1..=3usize {
                    let loops: Vec<String> = (0..len)
                        .map(|d| format!("v{d}"))
                        .collect();
                    let mut body = String::new();
                    for (d, v) in loops.iter().enumerate() {
                        let _ = writeln!(
                            body,
                            "{}for {v} in &alphabet {{",
                            "    ".repeat(d + 1)
                        );
                    }
                    let elems = loops
                        .iter()
                        .map(|v| format!("{v}.clone()"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = writeln!(
                        body,
                        "{}let _ = drive(vec![{elems}]); cases += 1;",
                        "    ".repeat(len + 1)
                    );
                    for d in (0..len).rev() {
                        let _ = writeln!(body, "{}}}", "    ".repeat(d + 1));
                    }
                    s.push_str(&body);
                }
                let _ = writeln!(s, "    let mut s = Stats::default();");
                let _ = writeln!(s, "    s.cases = cases;");
                let _ = writeln!(s, "    (Verdict::Pass, s)");
                let _ = writeln!(s, "}}");
            }
            "deterministic" => {
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_deterministic(fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(
                    s,
                    "    let seqs = domain_take({}, 16);",
                    cx.driven_ty.rust()
                );
                let _ = writeln!(s, "    let template: Vec<V> = {template};");
                let _ = writeln!(
                    s,
                    "    let drive = |v: V| {{ let mut args = template.clone(); args[{idx}] = v; op_{n}(&[], &args, fault) }};"
                );
                let _ = writeln!(s, "    let mut cases = 0u64;");
                let _ = writeln!(s, "    let mut first: Option<V> = None;");
                let _ = writeln!(s, "    let mut sensitive = false;");
                let _ = writeln!(s, "    let mut distinct = 0usize;");
                if ctor_takes_elem {
                    let _ = writeln!(s, "    let mut completed = 0u64;");
                }
                let _ = writeln!(s, "    for (i, s0) in seqs.iter().enumerate() {{");
                if ctor_takes_elem {
                    let _ = writeln!(
                        s,
                        "        if matches!(s0, V::List(es) if es.is_empty()) || matches!(s0, V::Bytes(b) if b.is_empty()) {{ continue; }}"
                    );
                }
                let _ = writeln!(s, "        let r1 = drive(s0.clone());");
                let _ = writeln!(s, "        let r2 = drive(s0.clone());");
                let _ = writeln!(s, "        cases += 2;");
                let _ = writeln!(s, "        if r1 != r2 {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = cases;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-nondeterministic\", \"same sequence drove different results\"), s);"
                );
                let _ = writeln!(s, "        }}");
                let _ = writeln!(
                    s,
                    "        if seqs[..i].iter().all(|b| b != s0) {{"
                );
                let _ = writeln!(s, "            distinct += 1;");
                let _ = writeln!(s, "            match &first {{");
                let _ = writeln!(s, "                None => first = Some(r1),");
                let _ = writeln!(
                    s,
                    "                Some(f) => if f != &r1 {{ sensitive = true; }},"
                );
                let _ = writeln!(s, "            }}");
                let _ = writeln!(s, "        }}");
                if ctor_takes_elem {
                    let _ = writeln!(s, "        completed += 1;");
                }
                let _ = writeln!(s, "    }}");
                if ctor_takes_elem {
                    let _ = writeln!(s, "    if completed == 0 {{");
                    let _ = writeln!(s, "        let mut s = Stats::default();");
                    let _ = writeln!(s, "        s.cases = cases;");
                    let _ = writeln!(
                        s,
                        "        return (Verdict::fail(\"driver-no-drive-completed\", \"no drive completed\"), s);"
                    );
                    let _ = writeln!(s, "    }}");
                }
                let _ = writeln!(s, "    if distinct >= 2 && !sensitive {{");
                let _ = writeln!(s, "        let mut s = Stats::default();");
                let _ = writeln!(s, "        s.cases = cases;");
                let _ = writeln!(
                    s,
                    "        return (Verdict::fail(\"driver-control-insensitive\", \"no two distinct sequences drove different results\"), s);"
                );
                let _ = writeln!(s, "    }}");
                let _ = writeln!(s, "    let mut s = Stats::default();");
                let _ = writeln!(s, "    s.cases = cases;");
                let _ = writeln!(s, "    (Verdict::Pass, s)");
                let _ = writeln!(s, "}}");
            }
            "finish-observational" => {
                let finish = record.finish.as_ref().expect("ref finish for finish-observational");
                let f_ty = match &cx.mode_base {
                    Ty::Tuple(ts) if ts.len() == 2 => ts[1].clone(),
                    t => t.clone(),
                };
                let f_host = finish.host.clone();
                let is_both = matches!(record.result, DriverResult::Both);
                let conv = |v: &str, hc: &mut HostCx<'_>| -> Result<String, CodegenError> {
                    from_host(&f_ty, &f_host, v, hc)
                        .map_err(|m| unsupported(format!("operation `{n}`: {m}"), op.span))
                };
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_finish_observational(fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(
                    s,
                    "    let seqs = domain_take({}, 16);",
                    cx.driven_ty.rust()
                );
                let _ = writeln!(s, "    let template: Vec<V> = {template};");
                let _ = writeln!(s, "    let mut cases = 0u64;");
                let _ = writeln!(s, "    let mut completed = 0u64;");
                let _ = writeln!(s, "    let mut controlled = false;");
                let extra = match cx.elem_ty.as_ref() {
                    Some(t) => nonempty_v(t),
                    None => "V::Int(0)".to_string(),
                };
                let _ = writeln!(s, "    let extra = {extra};");
                // The label exists only when a fallible call can skip to it;
                // otherwise clippy would flag it in generated code.
                let fallible_drive = matches!(
                    record.step.returns,
                    StepReturns::Result | StepReturns::ResultOf(_)
                ) || record.ctx.as_ref().is_some_and(|c| c.returns_result);
                if fallible_drive {
                    let _ = writeln!(s, "    'seq: for s0 in &seqs {{");
                } else {
                    let _ = writeln!(s, "    for s0 in &seqs {{");
                }
                // A constructor taking the first element cannot run on an
                // empty drive; skip those sequences.
                if ctor_takes_elem {
                    let _ = writeln!(
                        s,
                        "        if matches!(s0, V::List(es) if es.is_empty()) || matches!(s0, V::Bytes(b) if b.is_empty()) {{ continue; }}"
                    );
                }
                // Main: drive, finish twice, compare. Each drive runs in its
                // own block so temporaries never collide.
                if is_both {
                    let _ = writeln!(s, "        let (va, vb, rcollected) = {{");
                } else {
                    let _ = writeln!(s, "        let (va, vb) = {{");
                }
                let _ = writeln!(s, "            let mut args = template.clone();");
                let _ = writeln!(s, "            args[{idx}] = s0.clone();");
                let mut main = String::new();
                emit_state_drive(&cx, &format!("args[{idx}].clone()"), "            ", &mut hc, &mut main)?;
                for line in main.lines() {
                    let _ = writeln!(s, "{line}");
                }
                let _ = writeln!(s, "            let fa = st.{}();", finish.method);
                let _ = writeln!(s, "            let fb = st.{}();", finish.method);
                let va = conv("fa", &mut hc)?;
                let vb = conv("fb", &mut hc)?;
                if is_both {
                    let _ = writeln!(s, "            ({va}, {vb}, collected)");
                } else {
                    let _ = writeln!(s, "            ({va}, {vb})");
                }
                let _ = writeln!(s, "        }};");
                let _ = writeln!(s, "        cases += 1;");
                let _ = writeln!(s, "        if va != vb {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = cases;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-finish-mutating\", \"finish observed different values\"), s);"
                );
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "        completed += 1;");
                // Cross-check: the replicated drive must agree with the
                // adapter, tuple-destructured for both-mode shaping. The
                // adapter wraps `Result` payloads; unwrap one layer first so
                // fallible drivers compare like-for-like (a completed
                // replica never yields `Err`, and neither does the adapter
                // on the same input unless the subject diverges).
                let _ = writeln!(s, "        let mut args = template.clone();");
                let _ = writeln!(s, "        args[{idx}] = s0.clone();");
                let _ = writeln!(s, "        let r_adapter = op_{n}(&[], &args, fault);");
                let _ = writeln!(s, "        let shaped = match &r_adapter {{");
                let _ = writeln!(s, "            V::Ok(b) => b.as_ref(),");
                let _ = writeln!(s, "            v => v,");
                let _ = writeln!(s, "        }};");
                if is_both {
                    let _ = writeln!(s, "        let replica_ok = match shaped {{");
                    let _ = writeln!(
                        s,
                        "            V::Tup(ps) if ps.len() == 2 => ps[0] == V::List(rcollected) && ps[1] == va,"
                    );
                    let _ = writeln!(s, "            _ => false,");
                    let _ = writeln!(s, "        }};");
                } else {
                    let _ = writeln!(s, "        let replica_ok = shaped == &va;");
                }
                let _ = writeln!(s, "        if !replica_ok {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = cases;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-replica-diverged\", \"replicated drive disagrees with adapter\"), s);"
                );
                let _ = writeln!(s, "        }}");
                // Control: fresh drive, finish, one extra step, finish. The
                // extra element's conversions hoist above the drive so a
                // retaining subject never observes a temporary that drops
                // before it.
                let mut uses = Vec::new();
                let mut pre = String::new();
                for (k, a) in cx.record.step.args.iter().enumerate() {
                    let h = a.host.as_ref().ok_or_else(|| {
                        bad("argument names no host type".to_string())
                    })?;
                    if matches!(a.from, ArgSource::Elem | ArgSource::ElemPath(_)) {
                        let elem = cx.elem_ty.as_ref().ok_or_else(|| {
                            bad("step takes the driven element of a Nat count".to_string())
                        })?;
                        let e = convert_arg(
                            a,
                            cx.op,
                            Some(elem),
                            &cx.param_tys,
                            "extra",
                            false,
                            &mut hc,
                        )?;
                        let _ = writeln!(pre, "            let q{k} = {e};");
                        uses.push(use_arg(&format!("q{k}"), h));
                    } else {
                        uses.push(format!("h{k}"));
                    }
                }
                let _ = writeln!(s, "        let differed = {{");
                let _ = writeln!(s, "            let mut args = template.clone();");
                let _ = writeln!(s, "            args[{idx}] = s0.clone();");
                s.push_str(&pre);
                let mut ctl = String::new();
                emit_state_drive(&cx, &format!("args[{idx}].clone()"), "            ", &mut hc, &mut ctl)?;
                for line in ctl.lines() {
                    let _ = writeln!(s, "{line}");
                }
                let _ = writeln!(s, "            let g1 = st.{}();", finish.method);
                emit_step_call(&cx, &uses, None, OnStepErr::SkipSeq, "            ", &mut hc, &mut s)?;
                let _ = writeln!(s, "            let g2 = st.{}();", finish.method);
                let vg1 = conv("g1", &mut hc)?;
                let vg2 = conv("g2", &mut hc)?;
                let _ = writeln!(s, "            let vg1v = {vg1};");
                let _ = writeln!(s, "            let vg2v = {vg2};");
                let _ = writeln!(s, "            vg1v != vg2v");
                let _ = writeln!(s, "        }};");
                let _ = writeln!(s, "        if differed {{ controlled = true; }}");
                let _ = writeln!(s, "    }}");
                let _ = writeln!(s, "    if completed == 0 {{");
                let _ = writeln!(s, "        let mut s = Stats::default();");
                let _ = writeln!(s, "        s.cases = cases;");
                let _ = writeln!(
                    s,
                    "        return (Verdict::fail(\"driver-no-drive-completed\", \"no drive completed\"), s);"
                );
                let _ = writeln!(s, "    }}");
                let _ = writeln!(s, "    if !controlled {{");
                let _ = writeln!(s, "        let mut s = Stats::default();");
                let _ = writeln!(s, "        s.cases = cases;");
                let _ = writeln!(
                    s,
                    "        return (Verdict::fail(\"driver-control-dead\", \"extra step never changed the finish\"), s);"
                );
                let _ = writeln!(s, "    }}");
                let _ = writeln!(s, "    let mut s = Stats::default();");
                let _ = writeln!(s, "    s.cases = cases;");
                let _ = writeln!(s, "    (Verdict::Pass, s)");
                let _ = writeln!(s, "}}");
            }
            "steps-cap" => {
                let cap = MAX_DRIVER_STEPS;
                let big = match &cx.driven_ty {
                    Ty::List(t) => format!(
                        "V::List(vec![{}; {}])",
                        minimal_v(t),
                        cap + 1
                    ),
                    Ty::Bytes | Ty::BytesN(_) => format!("V::Bytes(vec![0u8; {}])", cap + 1),
                    _ => format!("V::Int({})", cap + 1),
                };
                let edge = match &cx.driven_ty {
                    Ty::List(t) => format!("V::List(vec![{}; {}])", minimal_v(t), cap),
                    Ty::Bytes | Ty::BytesN(_) => format!("V::Bytes(vec![0u8; {cap}])", cap = cap),
                    _ => format!("V::Int({cap})"),
                };
                let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
                let _ = writeln!(
                    s,
                    "pub fn obl_{n}_steps_cap(fault: Fault) -> (Verdict, Stats) {{"
                );
                let _ = writeln!(s, "    let template: Vec<V> = {template};");
                let _ = writeln!(s, "    driver_take_calls();");
                let _ = writeln!(s, "    let mut args = template.clone();");
                let _ = writeln!(s, "    args[{idx}] = {big};");
                let _ = writeln!(
                    s,
                    "    let r = catch_unwind(AssertUnwindSafe(|| op_{n}(&[], &args, fault)));"
                );
                let _ = writeln!(s, "    let calls = driver_take_calls();");
                let _ = writeln!(s, "    match r {{");
                let _ = writeln!(s, "        Err(p) => {{");
                let _ = writeln!(
                    s,
                    "            let msg = p.downcast_ref::<String>().cloned().unwrap_or_default() + p.downcast_ref::<&str>().unwrap_or(&\"\");"
                );
                let _ = writeln!(
                    s,
                    "            if !msg.contains(\"driver-steps-exceeded\") {{"
                );
                let _ = writeln!(s, "                let mut s = Stats::default();");
                let _ = writeln!(s, "                s.cases = 1;");
                let _ = writeln!(
                    s,
                    "                return (Verdict::fail(\"driver-wrong-refusal\", \"cap refusal had the wrong shape\"), s);"
                );
                let _ = writeln!(s, "            }}");
                let _ = writeln!(s, "            if calls != 0 {{");
                let _ = writeln!(s, "                let mut s = Stats::default();");
                let _ = writeln!(s, "                s.cases = 1;");
                let _ = writeln!(
                    s,
                    "                return (Verdict::fail(\"driver-called-before-refusal\", \"adapter called subject before refusing\"), s);"
                );
                let _ = writeln!(s, "            }}");
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "        Ok(_) => {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = 1;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-no-refusal\", \"adapter drove past the cap\"), s);"
                );
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "    }}");
                // Control: exactly the cap proceeds and calls are observed.
                let _ = writeln!(s, "    driver_take_calls();");
                let _ = writeln!(s, "    let mut args = template.clone();");
                let _ = writeln!(s, "    args[{idx}] = {edge};");
                let _ = writeln!(
                    s,
                    "    let r = catch_unwind(AssertUnwindSafe(|| op_{n}(&[], &args, fault)));"
                );
                let _ = writeln!(s, "    let calls = driver_take_calls();");
                let _ = writeln!(s, "    match r {{");
                let _ = writeln!(s, "        Ok(_) if calls > 0 => {{}}");
                let _ = writeln!(s, "        Ok(_) => {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = 2;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-control-blind\", \"cap drive observed no call\"), s);"
                );
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "        Err(_) => {{");
                let _ = writeln!(s, "            let mut s = Stats::default();");
                let _ = writeln!(s, "            s.cases = 2;");
                let _ = writeln!(
                    s,
                    "            return (Verdict::fail(\"driver-control-refused\", \"cap drive refused\"), s);"
                );
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "    }}");
                let _ = writeln!(s, "    let mut s = Stats::default();");
                let _ = writeln!(s, "    s.cases = 2;");
                let _ = writeln!(s, "    (Verdict::Pass, s)");
                let _ = writeln!(s, "}}");
            }
            other => {
                return Err(bad(format!("unknown obligation `{other}`")));
            }
        }
    }
    Ok((s, obligations))
}
