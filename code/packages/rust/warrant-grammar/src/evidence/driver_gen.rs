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
            let chain = resolve_elem_chain(&elem, path).map_err(|e| bad(e))?;
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

/// Render one argument's use at a call site from its hoisted vector.
fn use_arg(prefix: &str, i: usize, host: &HostTy) -> String {
    match host {
        HostTy::Bytes | HostTy::BytesN(_) => format!("&{prefix}{i}[step]"),
        h if host_is_copy(h) => format!("{prefix}{i}[step]"),
        _ => format!("{prefix}{i}[step].clone()"),
    }
}

/// Emit the `shape` obligation pins: `const _` items asserting every
/// fully-known method signature. Calls involving the context value carry an
/// unnameable type, so those pins are skipped: the adapter calls below still
/// check their existence.
fn shape_pins(
    record: &DriverRecord,
    hosts: &HostSpec,
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
        let ret = match &record.step.returns {
            StepReturns::Unit => "()".to_string(),
            StepReturns::Result => {
                let path = record
                    .errors
                    .as_ref()
                    .map(|e| e.path.clone())
                    .ok_or_else(|| {
                        bad("fallible step without `errors=` has no pinnable error type".to_string())
                    })?;
                format!("Result<(), {path}>")
            }
            StepReturns::Value(h) => pin_rust_ty(h, hosts, op_span)?,
            StepReturns::ResultOf(h) => {
                let path = record
                    .errors
                    .as_ref()
                    .map(|e| e.path.clone())
                    .ok_or_else(|| {
                        bad("fallible step without `errors=` has no pinnable error type".to_string())
                    })?;
                format!("Result<{}, {path}>", pin_rust_ty(h, hosts, op_span)?)
            }
        };
        let _ = writeln!(
            pins,
            "const _: fn({}) -> {ret} = {}::{};",
            params.join(", "),
            record.state,
            record.step.method
        );
    }
    // `finish`: no arguments beyond the receiver.
    if let Some(finish) = &record.finish {
        let recv = match finish.receiver {
            FinishReceiver::Ref => format!("&{}", record.state),
            FinishReceiver::Own => record.state.clone(),
        };
        let ret = pin_rust_ty(&finish.host, hosts, op_span)?;
        let _ = writeln!(
            pins,
            "const _: fn({recv}) -> {ret} = {}::{};",
            record.state, finish.method
        );
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
    validate_driver_record_strings(record).map_err(|e| bad(e))?;
    for f in faults {
        if !crate::driver::DRIVER_FAULTS.contains(&f.as_str()) && f != "missing-error" {
            return Err(bad(format!("fault `{f}` is not a driver-operation fault")));
        }
    }
    let fallible_step = matches!(
        record.step.returns,
        StepReturns::Result | StepReturns::ResultOf(_)
    );
    let missing = faults.iter().any(|f| f == "missing-error");
    if missing && !fallible_step {
        return Err(bad(format!(
            "fault `missing-error` needs a fallible step, `{n}` is total"
        )));
    }
    if matches!(record.step.receiver, StepReceiver::Own)
        && matches!(
            record.step.returns,
            StepReturns::Value(_) | StepReturns::ResultOf(_)
        )
    {
        return Err(bad("`own` step cannot emit a value".to_string()));
    }
    // Types, re-derived defensively: `generate_*` callers bypass validation.
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
    // Result-mode coherence, mirroring typing.
    let emitted_host: Option<HostTy> = match &record.step.returns {
        StepReturns::Value(h) | StepReturns::ResultOf(h) => Some(h.clone()),
        StepReturns::Unit | StepReturns::Result => None,
    };
    let collect_elem: Option<Ty> = match record.result {
        DriverResult::Finish => None,
        DriverResult::Collect | DriverResult::Both => match &mode_base {
            Ty::List(u) if !matches!(record.result, DriverResult::Both) => Some((**u).clone()),
            Ty::Tuple(ts)
                if matches!(record.result, DriverResult::Both) && ts.len() == 2 =>
            {
                match &ts[0] {
                    Ty::List(u) => Some((**u).clone()),
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
        },
    };
    if matches!(record.result, DriverResult::Finish | DriverResult::Both) && record.finish.is_none() {
        return Err(bad("result mode needs a `finish=`".to_string()));
    }
    if collect_elem.is_some() && emitted_host.is_none() {
        return Err(bad("collecting result mode needs a step that emits".to_string()));
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
    s.push_str(&shape_pins(record, &options.hosts, op.span)?);
    let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
    let _ = writeln!(s, "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{");
    let _ = writeln!(s, "    let fault = fault_for(\"{n}\", fault);");
    // Slice 5 replaces this identity with the fault plan; the binding is
    // already complete, so nothing downstream changes shape.
    let _ = writeln!(s, "    let _ = &fault;");
    // Driven inputs, owned by this frame so per-step borrows never dangle.
    match &driven_ty {
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
    let step_base = match &driven_ty {
        Ty::List(_) => "elems[step]".to_string(),
        Ty::Bytes | Ty::BytesN(_) => "V::Fixed(F::U8, ebytes[step] as i128)".to_string(),
        _ => String::new(),
    };
    let first_base = match &driven_ty {
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
            let elem = elem_ty.as_ref().ok_or_else(|| {
                bad("step takes the driven element of a Nat count".to_string())
            })?;
            let vt = owned_rust_ty(h, &options.hosts, op.span)?;
            let _ = writeln!(s, "    let mut c{k}: Vec<{vt}> = Vec::with_capacity(peril);");
            let _ = writeln!(s, "    for step in 0..peril {{");
            let e = convert_arg(a, op, Some(elem), &param_tys, &step_base, false, &mut hc)?;
            let _ = writeln!(s, "        c{k}.push({e});");
            let _ = writeln!(s, "    }}");
            step_uses.push(use_arg("c", k, h));
        } else {
            let elem_opt = elem_ty.as_ref();
            let e = convert_arg(a, op, elem_opt, &param_tys, "", true, &mut hc)?;
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
        let mut ctx_args = Vec::new();
        for (k, a) in ctx.args.iter().enumerate() {
            let elem_opt = elem_ty.as_ref();
            let e = convert_arg(a, op, elem_opt, &param_tys, &first_base, true, &mut hc)?;
            let _ = writeln!(s, "    let x{k} = {e};");
            let h = a
                .host
                .as_ref()
                .ok_or_else(|| bad("argument names no host type".to_string()))?;
            if host_is_copy(h) {
                ctx_args.push(format!("x{k}"));
            } else {
                ctx_args.push(format!("x{k}.clone()"));
            }
        }
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
    let mut new_args = Vec::new();
    for (k, a) in record.new.args.iter().enumerate() {
        let elem_opt = elem_ty.as_ref();
        let e = convert_arg(a, op, elem_opt, &param_tys, &first_base, true, &mut hc)?;
        let _ = writeln!(s, "    let w{k} = {e};");
        if matches!(a.from, ArgSource::Ctx) {
            new_args.push(format!("w{k}"));
            continue;
        }
        let h = a
            .host
            .as_ref()
            .ok_or_else(|| bad("argument names no host type".to_string()))?;
        if host_is_copy(h) {
            new_args.push(format!("w{k}"));
        } else {
            new_args.push(format!("w{k}.clone()"));
        }
    }
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
    let collect_host = emitted_host.clone();
    if collect_elem.is_some() {
        let _ = writeln!(drive, "let mut collected: Vec<V> = Vec::new();");
    }
    let _ = writeln!(drive, "for step in 0..peril {{");
    let _ = writeln!(drive, "    driver_note_call();");
    let step_args = step_uses.join(", ");
    match (&record.step.receiver, &record.step.returns) {
        (StepReceiver::Mut, StepReturns::Unit) => {
            let _ = writeln!(drive, "    st.{m}({step_args});", m = record.step.method);
        }
        (StepReceiver::Own, StepReturns::Unit) => {
            let _ = writeln!(
                drive,
                "    st = st.{m}({step_args});",
                m = record.step.method
            );
        }
        (StepReceiver::Mut, StepReturns::Result) => {
            let errors = record.errors.as_ref().expect("checked fallible");
            let mapped = errors_expr("e", &errors.path, &errors.map);
            let _ = writeln!(drive, "    match st.{m}({step_args}) {{", m = record.step.method);
            let _ = writeln!(drive, "        Ok(()) => {{}},");
            if missing {
                let _ = writeln!(drive, "        Err(_) if fault == Fault::MissingError => {{}},");
            }
            let _ = writeln!(drive, "        Err(e) => return {mapped},");
            let _ = writeln!(drive, "    }}");
        }
        (StepReceiver::Mut, StepReturns::Value(_)) => {
            if collect_elem.is_some() {
                let u = collect_elem.as_ref().expect("checked collect mode");
                let h = collect_host.as_ref().expect("checked collect mode");
                let v = from_host(u, h, "sv", &mut hc).map_err(|e| bad(e))?;
                let _ = writeln!(
                    drive,
                    "    let sv = st.{m}({step_args});",
                    m = record.step.method
                );
                let _ = writeln!(drive, "    collected.push({v});");
            } else {
                // `finish` mode discards emitted values; the call still runs.
                let _ = writeln!(drive, "    st.{m}({step_args});", m = record.step.method);
            }
        }
        (StepReceiver::Mut, StepReturns::ResultOf(_)) => {
            let errors = record.errors.as_ref().expect("checked fallible");
            let mapped = errors_expr("e", &errors.path, &errors.map);
            if collect_elem.is_some() {
                let _ = writeln!(
                    drive,
                    "    let sv = match st.{m}({step_args}) {{",
                    m = record.step.method
                );
                let _ = writeln!(drive, "        Ok(v) => v,");
                if missing {
                    let _ = writeln!(
                        drive,
                        "        Err(_) if fault == Fault::MissingError => continue,"
                    );
                }
                let _ = writeln!(drive, "        Err(e) => return {mapped},");
                let _ = writeln!(drive, "    }};");
                let u = collect_elem.as_ref().expect("checked collect mode");
                let h = collect_host.as_ref().expect("checked collect mode");
                let v = from_host(u, h, "sv", &mut hc).map_err(|e| bad(e))?;
                let _ = writeln!(drive, "    collected.push({v});");
            } else {
                let _ = writeln!(drive, "    match st.{m}({step_args}) {{", m = record.step.method);
                let _ = writeln!(drive, "        Ok(_) => {{}},");
                if missing {
                    let _ = writeln!(drive, "        Err(_) if fault == Fault::MissingError => {{}},");
                }
                let _ = writeln!(drive, "        Err(e) => return {mapped},");
                let _ = writeln!(drive, "    }}");
            }
        }
        (StepReceiver::Own, StepReturns::Value(_) | StepReturns::ResultOf(_)) => {
            return Err(bad("`own` step cannot emit a value".to_string()));
        }
        (StepReceiver::Own, StepReturns::Result) => {
            let errors = record.errors.as_ref().expect("checked fallible");
            let mapped = errors_expr("e", &errors.path, &errors.map);
            let _ = writeln!(drive, "    st = match st.{m}({step_args}) {{", m = record.step.method);
            let _ = writeln!(drive, "        Ok(st) => st,");
            if missing {
                let _ = writeln!(
                    drive,
                    "        Err(_) if fault == Fault::MissingError => st,"
                );
            }
            let _ = writeln!(drive, "        Err(e) => return {mapped},");
            let _ = writeln!(drive, "    }};");
        }
    }
    let _ = writeln!(drive, "}}");
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
            let v = from_host(&mode_base, &finish.host, "f", &mut hc).map_err(|e| bad(e))?;
            if is_result {
                let _ = writeln!(drive, "V::Ok(Box::new({v}))");
            } else {
                let _ = writeln!(drive, "{v}");
            }
        }
        DriverResult::Collect => {
            if is_result {
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
            let f_ty = match &mode_base {
                Ty::Tuple(ts) if ts.len() == 2 => ts[1].clone(),
                _ => {
                    return Err(bad(
                        "`both` needs a `(List<U>, F)` return type".to_string(),
                    ))
                }
            };
            let v = from_host(&f_ty, &finish.host, "f", &mut hc).map_err(|e| bad(e))?;
            if is_result {
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
